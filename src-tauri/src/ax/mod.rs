use core_graphics::display::CGDirectDisplayID;
use lightsky::{DisplayId, Lightsky, SpaceId, WindowId};
use objc2::rc::Retained;
use objc2_app_kit::{NSApplicationActivationOptions, NSRunningApplication, NSScreen};
use objc2_foundation::{MainThreadMarker, NSDictionary, NSNumber, NSString};

use core_foundation::array::CFArrayRef;
use core_foundation::base::{CFTypeRef, TCFType};
use core_foundation::number::CFNumber;
use core_foundation::string::{CFString, CFStringRef};
use core_foundation::uuid::{CFUUIDRef, CFUUID};

use crate::ax::application_tree::{ApplicationTree, SearchParam, SearchResult};
mod application_tree;

/* ----------------------------- CoreGraphics FFI ----------------------------- */

extern "C" {
    fn CGDisplayCreateUUIDFromDisplayID(display: CGDirectDisplayID) -> CFUUIDRef;
    // Fallback to the primary display if AppKit mainScreen is unavailable:
    fn CGMainDisplayID() -> CGDirectDisplayID;
}

/* ---------------------------- CoreFoundation FFI ---------------------------- */

use std::thread;
use std::{ffi::c_void, ptr};

extern "C" {
    fn CFArrayGetCount(theArray: CFArrayRef) -> isize;
    fn CFArrayGetValueAtIndex(theArray: CFArrayRef, idx: isize) -> *const c_void;
    fn CFRelease(cf: CFTypeRef);
}

/* --------------------------- Quartz EventPosting FFI ------------------------ */

use core_graphics::event::{CGEvent, CGEventFlags, CGEventTapLocation, CGKeyCode};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use core_graphics::geometry::{CGPoint, CGRect};

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGWarpMouseCursorPosition(newCursorPosition: CGPoint) -> i32; // CGError
    fn CGDisplayBounds(display: CGDirectDisplayID) -> CGRect;
    fn CGGetActiveDisplayList(
        max_displays: u32,
        active_displays: *mut CGDirectDisplayID,
        display_count: *mut u32,
    ) -> i32; // CGError
}

/* ------------------------------ Accessibility FFI ------------------------------ */
// Avoid extern types (experimental). Use an opaque enum + pointer alias.
#[allow(non_camel_case_types)]
enum __AXUIElement {}
type AXUIElementRef = *mut __AXUIElement;

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXUIElementCreateApplication(pid: i32) -> AXUIElementRef;

    fn AXUIElementCopyAttributeValue(
        element: AXUIElementRef,
        attribute: CFStringRef,
        value: *mut CFTypeRef,
    ) -> i32; // AXError (0 == kAXErrorSuccess)

    fn AXUIElementSetAttributeValue(
        element: AXUIElementRef,
        attribute: CFStringRef,
        value: CFTypeRef,
    ) -> i32; // AXError

    fn AXUIElementPerformAction(element: AXUIElementRef, action: CFStringRef) -> i32; // AXError
}

/* --------------------------------- Types --------------------------------- */

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct DisplaySpace {
    pub display_id: DisplayId,
    pub space_id: SpaceId,
}

impl std::fmt::Display for DisplaySpace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "CurrentDisplaySpace:")?;
        writeln!(f, "  display_id: {}", self.display_id.0)?;
        writeln!(f, "  space_id: {}", self.space_id)
    }
}

pub struct AX {
    app: tauri::AppHandle,
    pub current_display_space: DisplaySpace,
    pub lightsky: Lightsky,
    pub application_tree: ApplicationTree,
}

impl std::fmt::Display for AX {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "AX:")?;
        write!(f, "{}", self.current_display_space)?;
        write!(f, "{}", self.application_tree)
    }
}

/* ----------------------------- Free helpers ----------------------------- */

/// Extract `CGDirectDisplayID` (NSScreenNumber) from an NSScreen.
fn screen_display_id(screen: &NSScreen) -> Option<CGDirectDisplayID> {
    // deviceDescription: NSDictionary<NSDeviceDescriptionKey, id>
    let desc: Retained<NSDictionary<NSString, objc2::runtime::AnyObject>> =
        screen.deviceDescription();
    let key = NSString::from_str("NSScreenNumber");
    let val = desc.objectForKey(&key)?;
    let num: &NSNumber = val.downcast_ref()?;
    Some(num.unsignedIntValue() as CGDirectDisplayID)
}

/// Convert an NSScreen to its CoreGraphics/ColorSync UUID string.
fn display_uuid_for_screen(screen: &NSScreen) -> Option<DisplayId> {
    let did = screen_display_id(screen)?;
    unsafe {
        let uuid_ref = CGDisplayCreateUUIDFromDisplayID(did);
        if uuid_ref.is_null() {
            return None;
        }
        let uuid = CFUUID::wrap_under_create_rule(uuid_ref);
        let s_ref = core_foundation::uuid::CFUUIDCreateString(
            core_foundation::base::kCFAllocatorDefault,
            uuid.as_concrete_TypeRef(),
        );
        let s = CFString::wrap_under_create_rule(s_ref);
        Some(DisplayId(s.to_string()))
    }
}

/// Fallback: UUID string for the **primary** display via CoreGraphics (works off main thread).
fn primary_display_uuid_via_cg() -> Option<DisplayId> {
    unsafe {
        let did = CGMainDisplayID();
        if did == 0 {
            return None;
        }
        let uuid_ref = CGDisplayCreateUUIDFromDisplayID(did);
        if uuid_ref.is_null() {
            return None;
        }
        let uuid = CFUUID::wrap_under_create_rule(uuid_ref);
        let s_ref = core_foundation::uuid::CFUUIDCreateString(
            core_foundation::base::kCFAllocatorDefault,
            uuid.as_concrete_TypeRef(),
        );
        let s = CFString::wrap_under_create_rule(s_ref);
        Some(DisplayId(s.to_string()))
    }
}

/// UUID string for the active (main) screen, executed on the main thread.
/// Falls back to the primary display if AppKit returns nothing.
fn active_display_id(app: &tauri::AppHandle) -> Option<DisplayId> {
    let (tx, rx) = std::sync::mpsc::channel();

    // Ignore the Result; if dispatch fails we’ll just get a recv() error below.
    let _ = app.run_on_main_thread(move || {
        let result = MainThreadMarker::new()
            .and_then(|mtm| NSScreen::mainScreen(mtm))
            .and_then(|screen| display_uuid_for_screen(&screen))
            .or_else(|| primary_display_uuid_via_cg());
        let _ = tx.send(result);
    });

    rx.recv().ok().flatten()
}

/* --------- Display & Mission Control (keyboard emulation) helpers ---------- */

const KC_CTRL: CGKeyCode = 59; // kVK_Control
const KC_LEFT: CGKeyCode = 123; // kVK_LeftArrow
const KC_RIGHT: CGKeyCode = 124; // kVK_RightArrow
const KC_1: CGKeyCode = 18;
const KC_2: CGKeyCode = 19;
const KC_3: CGKeyCode = 20;
const KC_4: CGKeyCode = 21;
const KC_5: CGKeyCode = 23;
const KC_6: CGKeyCode = 22;
const KC_7: CGKeyCode = 26;
const KC_8: CGKeyCode = 28;
const KC_9: CGKeyCode = 25;
const KC_0: CGKeyCode = 29;

fn kc_for_digit_1_to_10(n: usize) -> Option<CGKeyCode> {
    match n {
        1 => Some(KC_1),
        2 => Some(KC_2),
        3 => Some(KC_3),
        4 => Some(KC_4),
        5 => Some(KC_5),
        6 => Some(KC_6),
        7 => Some(KC_7),
        8 => Some(KC_8),
        9 => Some(KC_9),
        10 => Some(KC_0),
        _ => None,
    }
}

fn post_key(k: CGKeyCode, down: bool) -> bool {
    let Some(src) = CGEventSource::new(CGEventSourceStateID::HIDSystemState).ok() else {
        return false;
    };
    if let Some(e) = CGEvent::new_keyboard_event(src, k, down).ok() {
        e.post(CGEventTapLocation::HID);
        return true;
    }
    false
}

fn ctrl_combo(arrow: CGKeyCode) -> bool {
    // Ctrl down
    if !post_key(KC_CTRL, true) {
        return false;
    }
    std::thread::sleep(std::time::Duration::from_millis(2));

    // Arrow down + up (no flags needed because Control is physically held)
    let _ = post_key(arrow, true);
    std::thread::sleep(std::time::Duration::from_millis(2));
    let _ = post_key(arrow, false);

    // Ctrl up
    std::thread::sleep(std::time::Duration::from_millis(2));
    post_key(KC_CTRL, false)
}

/// Post Ctrl+<digit> (1..=10 where 10 -> '0' key) to jump directly to Desktop N on current display.
fn press_ctrl_digit(n: usize) -> bool {
    let Some(key) = kc_for_digit_1_to_10(n) else {
        return false;
    };
    ctrl_combo(key)
}

fn press_ctrl_left() -> bool {
    ctrl_combo(KC_LEFT)
}

fn press_ctrl_right() -> bool {
    ctrl_combo(KC_RIGHT)
}

/// Find the CoreGraphics display ID for a given ColorSync UUID string (DisplayId).
fn cg_display_id_for_uuid(uuid: &DisplayId) -> Option<CGDirectDisplayID> {
    unsafe {
        let mut ids = [0u32; 16];
        let mut count: u32 = 0;
        if CGGetActiveDisplayList(ids.len() as u32, ids.as_mut_ptr(), &mut count) != 0 {
            return None;
        }
        for &did in &ids[..count as usize] {
            let cf_uuid = CGDisplayCreateUUIDFromDisplayID(did);
            if cf_uuid.is_null() {
                continue;
            }
            let s_ref = core_foundation::uuid::CFUUIDCreateString(
                core_foundation::base::kCFAllocatorDefault,
                cf_uuid,
            );
            let s = CFString::wrap_under_create_rule(s_ref);
            if s.to_string() == uuid.0 {
                return Some(did);
            }
        }
        None
    }
}

/* ------------------------------- AX impl -------------------------------- */

impl AX {
    pub fn new(app: tauri::AppHandle) -> Self {
        let lightsky = Lightsky::new().expect("Failed to initialize Lightsky");
        let application_tree = ApplicationTree::new(&lightsky);

        let current_display = active_display_id(&app).expect("Failed to get active display ID");
        let current_space = lightsky.current_space();

        AX {
            app,
            current_display_space: DisplaySpace {
                display_id: current_display,
                space_id: current_space,
            },
            lightsky,
            application_tree,
        }
    }

    pub fn refresh(&mut self) {
        self.application_tree = ApplicationTree::new(&self.lightsky);
        self.current_display_space = DisplaySpace {
            display_id: active_display_id(&self.app).expect("Failed to get active display ID"),
            space_id: self.lightsky.current_space(),
        };
    }

    /// Move the input focus to a specific **display** by warping the mouse
    /// to its center. This selects which display Mission Control shortcuts
    /// will act on (with "Displays have separate Spaces" enabled).
    pub fn focus_display(&self, display_id: DisplayId) -> Option<()> {
        let did = cg_display_id_for_uuid(&display_id)?;
        unsafe {
            let bounds = CGDisplayBounds(did);
            let center = CGPoint::new(
                bounds.origin.x + bounds.size.width / 2.0,
                bounds.origin.y + bounds.size.height / 2.0,
            );
            let _ = CGWarpMouseCursorPosition(center);
        }
        Some(())
    }

    /// Emulates Mission Control to switch to a different Space on the target display.
    /// - If the target Space is on another display, first focus that display (by warping cursor).
    /// - Then jump directly to the Space index using Ctrl+<number> if 1..=10, else fall back to arrows.
    pub fn focus_space(&self, space_id: SpaceId) -> Option<()> {
        log::info!("Focusing space_id: {}", space_id);
        // Where is the target space?
        let target_display_id = self.application_tree.find_display_from_space(space_id)?;
        let target_space_index = self.application_tree.find_space_index(space_id)?;

        // If we're not on that display, focus it first so Mission Control shortcuts act there.
        if target_display_id != self.current_display_space.display_id {
            log::info!(
                "Switching display: {} -> {}",
                self.current_display_space.display_id,
                target_display_id
            );
            let _ = self.focus_display(target_display_id.clone());
            // tiny delay so WindowServer updates "active display"
            thread::sleep(std::time::Duration::from_millis(40));
        }

        // if target_space_index < 10 {
        //     let worked = press_ctrl_digit(target_space_index + 1);
        //     if !worked {
        //         log::warn!("Failed to post Ctrl+{} key event", target_space_index + 1);
        //     }
        //     log::info!("Pressed Ctrl+{}", target_space_index);
        //     thread::sleep(std::time::Duration::from_millis(1000));
        //     return Some(());
        // }

        // Fallback: approximate with left/right. We need a relative delta; best effort:
        // Attempt to compute current index on *current* (now-target) display.
        let current_idx = self
            .application_tree
            .find_space_index(self.current_display_space.space_id)
            .unwrap_or(target_space_index);

        let diff = (target_space_index as isize) - (current_idx as isize);
        if diff > 0 {
            for _ in 0..diff {
                log::info!("Pressing Ctrl+Right");
                let _ = press_ctrl_right();
            }
        } else if diff < 0 {
            for _ in 0..(-diff) {
                log::info!("Pressing Ctrl+Left");
                let _ = press_ctrl_left();
            }
        }
        Some(())
    }

    /// Find the target window by id, switch Space to it, then focus it.
    pub fn focus_window(&mut self, window_id: WindowId) {
        let results = self
            .application_tree
            .search(SearchParam::ByWindowId(window_id));

        if let Some(res) = results.first() {
            let SearchResult {
                pid,
                window_id,
                space_id,
                ..
            } = res;

            let _ = self.focus_space(*space_id);

            self.focus(*pid, Some(*window_id));
        }

        self.refresh();
    }

    pub fn get_focused_window(&self) -> Option<WindowId> {
        let results = self.application_tree.search(SearchParam::ByFocused);
        results.first().map(|res| res.window_id)
    }

    /// Focus a specific window for a PID (assumes we're already on the correct Space).
    /// Steps:
    ///   - Activate app with NSRunningApplication (main thread)
    ///   - Enumerate AXWindows and match AXWindowNumber to `window_id`
    ///   - Set AXFocusedWindow and perform AXRaise on it
    pub fn focus(&self, pid: i32, window_id: Option<WindowId>) {
        // 1) Bring the app to the foreground on the main thread.
        let _ = self.app.run_on_main_thread(move || unsafe {
            if let Some(app) = NSRunningApplication::runningApplicationWithProcessIdentifier(pid) {
                let opts = NSApplicationActivationOptions::ActivateAllWindows;
                let _ = app.activateWithOptions(opts);
            }
        });

        if let Some(window_id) = window_id {
            // 2) Use AX to focus and raise the exact window.
            unsafe {
                let app_ax: AXUIElementRef = AXUIElementCreateApplication(pid);
                if app_ax.is_null() {
                    return;
                }

                // CFStrings for attributes/actions
                let ax_windows = CFString::from_static_string("AXWindows");
                let ax_focused_window = CFString::from_static_string("AXFocusedWindow");
                let ax_window_number = CFString::from_static_string("AXWindowNumber");
                let ax_raise = CFString::from_static_string("AXRaise");

                // Get windows array
                let mut windows_val: CFTypeRef = ptr::null();
                if AXUIElementCopyAttributeValue(
                    app_ax,
                    ax_windows.as_concrete_TypeRef(),
                    &mut windows_val,
                ) != 0
                    || windows_val.is_null()
                {
                    // Couldn't read windows; clean up and bail
                    CFRelease(app_ax as CFTypeRef);
                    return;
                }

                let windows_array: CFArrayRef = windows_val as CFArrayRef;
                let count = CFArrayGetCount(windows_array);

                // Compare against your WindowId
                let target_num: i64 = window_id.0 as i64;

                let mut matched_window: Option<AXUIElementRef> = None;

                for i in 0..count {
                    let w_ref = CFArrayGetValueAtIndex(windows_array, i) as AXUIElementRef;
                    if w_ref.is_null() {
                        continue;
                    }

                    // Read AXWindowNumber
                    let mut num_val: CFTypeRef = ptr::null();
                    if AXUIElementCopyAttributeValue(
                        w_ref,
                        ax_window_number.as_concrete_TypeRef(),
                        &mut num_val,
                    ) != 0
                        || num_val.is_null()
                    {
                        continue;
                    }

                    // Convert CFNumber -> i64
                    let cfnum = CFNumber::wrap_under_create_rule(num_val as _);
                    if let Some(n) = cfnum.to_i64() {
                        if n == target_num {
                            matched_window = Some(w_ref);
                            // cfnum drops here (releases num_val)
                            break;
                        }
                    }
                    // cfnum drops here and releases num_val if not matched
                }

                // Release the windows array we "copied"
                CFRelease(windows_val);

                if let Some(w_ref) = matched_window {
                    // Focus the window
                    let _ = AXUIElementSetAttributeValue(
                        app_ax,
                        ax_focused_window.as_concrete_TypeRef(),
                        w_ref as CFTypeRef,
                    );

                    // Raise the specific window
                    let _ = AXUIElementPerformAction(w_ref, ax_raise.as_concrete_TypeRef());
                }

                // Release the app element we created
                CFRelease(app_ax as CFTypeRef);
            }
        }
    }
}
