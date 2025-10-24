use core_foundation::array::CFArrayRef;
use core_foundation::base::{CFTypeRef, TCFType};
use core_foundation::number::CFNumber;
use core_foundation::number::CFNumberRef;
use core_foundation::string::{CFString, CFStringRef};
use core_graphics::display::CFDictionaryRef;
use core_graphics::window::{
    kCGNullWindowID, kCGWindowListOptionOnScreenOnly, CGWindowListCopyWindowInfo,
};
use kameo::prelude::Message;
use kameo::Actor;
use lightsky::WindowId;
use objc2_app_kit::{NSApplicationActivationOptions, NSRunningApplication};
use std::{ffi::c_void, ptr};

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
    ) -> i32;
    fn AXUIElementSetAttributeValue(
        element: AXUIElementRef,
        attribute: CFStringRef,
        value: CFTypeRef,
    ) -> i32;
    fn AXUIElementPerformAction(element: AXUIElementRef, action: CFStringRef) -> i32;
}

extern "C" {
    fn CFArrayGetCount(theArray: CFArrayRef) -> isize;
    fn CFArrayGetValueAtIndex(theArray: CFArrayRef, idx: isize) -> *const c_void;
    fn CFRelease(cf: CFTypeRef);
    fn CFRetain(cf: CFTypeRef) -> CFTypeRef; // <-- add this
}

#[derive(Actor)]
pub struct FocusManagerActor {
    app_handle: tauri::AppHandle,
    focus_window_id: Option<WindowId>,
}

pub struct FocusWindow {
    pub pid: i32,
    pub window_id: Option<WindowId>,
    pub title: Option<String>,
}

impl Message<FocusWindow> for FocusManagerActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: FocusWindow,
        _ctx: &mut kameo::prelude::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.focus_window_id = msg.window_id;
        self.focus(msg.pid, msg.window_id, msg.title);
    }
}

pub struct SetFocusWindowId {
    pub window_id: Option<WindowId>,
}

impl Message<SetFocusWindowId> for FocusManagerActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: SetFocusWindowId,
        _ctx: &mut kameo::prelude::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.focus_window_id = msg.window_id;
    }
}

pub struct GetFocusWindowId;

impl Message<GetFocusWindowId> for FocusManagerActor {
    type Reply = Option<WindowId>;

    async fn handle(
        &mut self,
        _msg: GetFocusWindowId,
        _ctx: &mut kameo::prelude::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        log::info!("Getting focus window id: {:?}", self.focus_window_id);
        self.focus_window_id
    }
}

pub struct InitFocus;

impl Message<InitFocus> for FocusManagerActor {
    type Reply = Option<WindowId>;

    async fn handle(
        &mut self,
        _msg: InitFocus,
        _ctx: &mut kameo::prelude::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        unsafe {
            if let Some(win) = self.focused_window_id() {
                log::info!("Initial focused window: {:?}", win);
                self.focus_window_id = Some(win);
                Some(win)
            } else {
                log::info!("No focused window found");
                None
            }
        }
    }
}

impl FocusManagerActor {
    pub fn new(app_handle: tauri::AppHandle) -> Self {
        Self {
            app_handle,
            focus_window_id: None,
        }
    }

    pub unsafe fn focused_window_id(&self) -> Option<WindowId> {
        let info = CGWindowListCopyWindowInfo(kCGWindowListOptionOnScreenOnly, kCGNullWindowID);
        if info.is_null() {
            return None;
        }

        let count = CFArrayGetCount(info);
        if count <= 0 {
            return None;
        }

        let dict_ref = CFArrayGetValueAtIndex(info, 0) as CFDictionaryRef;

        if dict_ref.is_null() {
            return None;
        }

        let key = CFString::from_static_string("kCGWindowNumber");
        let value: CFTypeRef =
            *core_foundation::dictionary::CFDictionary::wrap_under_get_rule(dict_ref).find(&key)?;

        let num_ref: CFNumberRef = value as CFNumberRef;
        let num = core_foundation::number::CFNumber::wrap_under_get_rule(num_ref);

        num.to_i64().map(|n| WindowId(n as u32))
    }

    pub fn focus(&self, pid: i32, window_id: Option<WindowId>, title: Option<String>) {
        let _ = self.app_handle.run_on_main_thread(move || unsafe {
            if let Some(app) = NSRunningApplication::runningApplicationWithProcessIdentifier(pid) {
                let _ = app.activateWithOptions(NSApplicationActivationOptions::ActivateAllWindows);
            }
            if window_id.is_none() && title.is_none() {
                log::warn!("No window_id or title provided for focus()");
                return;
            }

            let app_ax = AXUIElementCreateApplication(pid);
            if app_ax.is_null() {
                return;
            }

            let ax_windows = CFString::from_static_string("AXWindows");
            let ax_focused_window = CFString::from_static_string("AXFocusedWindow");
            let ax_window_number = CFString::from_static_string("AXWindowNumber");
            let ax_title = CFString::from_static_string("AXTitle");
            let ax_raise = CFString::from_static_string("AXRaise");

            let mut windows_val: CFTypeRef = ptr::null();
            if AXUIElementCopyAttributeValue(
                app_ax,
                ax_windows.as_concrete_TypeRef(),
                &mut windows_val,
            ) != 0
                || windows_val.is_null()
            {
                CFRelease(app_ax as CFTypeRef);
                log::warn!("Failed to get AXWindows for pid {}", pid);
                return;
            }

            let windows_array: CFArrayRef = windows_val as CFArrayRef;
            let count = CFArrayGetCount(windows_array);
            let target_num: Option<i64> = window_id.map(|w| w.0 as i64);
            let target_title = title.as_ref().map(|s| s.as_str());

            let mut matched_window: Option<AXUIElementRef> = None;

            for i in 0..count {
                let w_ref = CFArrayGetValueAtIndex(windows_array, i) as AXUIElementRef;
                if w_ref.is_null() {
                    continue;
                }

                let mut matched = false;

                // Try by AXWindowNumber
                if let Some(n) = target_num {
                    let mut num_val: CFTypeRef = ptr::null();
                    let status = AXUIElementCopyAttributeValue(
                        w_ref,
                        ax_window_number.as_concrete_TypeRef(),
                        &mut num_val,
                    );
                    if status == 0 && !num_val.is_null() {
                        let cfnum = CFNumber::wrap_under_create_rule(num_val as _);
                        if let Some(win_n) = cfnum.to_i64() {
                            if win_n == n {
                                log::info!(
                                    "Matched by AXWindowNumber: idx={} number={} target={}",
                                    i,
                                    win_n,
                                    n
                                );
                                matched = true;
                            }
                        } else {
                            log::info!("AXWindowNumber present but not convertible at idx={}", i);
                        }
                    } else {
                        log::warn!("Failed to read AXWindowNumber at idx={}", i);
                    }
                }

                // Fallback: AXTitle
                if !matched {
                    if let Some(t) = target_title {
                        let mut title_val: CFTypeRef = ptr::null();
                        let status = AXUIElementCopyAttributeValue(
                            w_ref,
                            ax_title.as_concrete_TypeRef(),
                            &mut title_val,
                        );
                        if status == 0 && !title_val.is_null() {
                            let cfstr = CFString::wrap_under_create_rule(title_val as _);
                            let current = cfstr.to_string();
                            if current == t {
                                log::info!("Matched by AXTitle: idx={} title='{}'", i, current);
                                matched = true;
                            } else {
                                log::info!(
                                    "AXTitle mismatch: idx={} title='{}' target='{}'",
                                    i,
                                    current,
                                    t
                                );
                            }
                        } else {
                            log::info!("No AXTitle for window at idx={}", i);
                        }
                    }
                }

                if matched {
                    // RETAIN before breaking; the array will be released soon.
                    let retained = CFRetain(w_ref as CFTypeRef) as AXUIElementRef;
                    matched_window = Some(retained);
                    log::info!("Found matching window at index {}", i);
                    break;
                }
            }

            // Now it's safe to release the array of windows.
            CFRelease(windows_val);

            if let Some(w_ref) = matched_window {
                let set_status = AXUIElementSetAttributeValue(
                    app_ax,
                    ax_focused_window.as_concrete_TypeRef(),
                    w_ref as CFTypeRef,
                );
                if set_status != 0 {
                    log::warn!(
                        "AXUIElementSetAttributeValue(AXFocusedWindow) failed with {}",
                        set_status
                    );
                }

                let raise_status = AXUIElementPerformAction(w_ref, ax_raise.as_concrete_TypeRef());
                if raise_status != 0 {
                    log::warn!(
                        "AXUIElementPerformAction(AXRaise) failed with {}",
                        raise_status
                    );
                } else {
                    log::info!("Focused window performed AXRaise");
                }

                // Release our retain
                CFRelease(w_ref as CFTypeRef);
            } else {
                log::warn!(
                    "No matching window found for pid {} (id={:?}, title={:?})",
                    pid,
                    window_id,
                    title
                );
            }

            CFRelease(app_ax as CFTypeRef);
        });
    }
}
