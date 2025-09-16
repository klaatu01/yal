use core_foundation::base::TCFType;
use core_foundation::string::CFString;
use core_foundation::uuid::CFUUID;
use core_graphics::display::CGDirectDisplayID;
use core_graphics::geometry::{CGPoint, CGRect};
use lightsky::DisplayId;
use objc2::rc::Retained;
use objc2_app_kit::NSScreen;
use objc2_foundation::{MainThreadMarker, NSDictionary, NSNumber, NSString};

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGDisplayCreateUUIDFromDisplayID(display: CGDirectDisplayID) -> *const std::ffi::c_void; // CFUUIDRef
    fn CGMainDisplayID() -> CGDirectDisplayID;
    fn CGDisplayBounds(display: CGDirectDisplayID) -> CGRect;
    fn CGGetActiveDisplayList(
        max_displays: u32,
        active_displays: *mut CGDirectDisplayID,
        display_count: *mut u32,
    ) -> i32; // CGError
    fn CGWarpMouseCursorPosition(newCursorPosition: CGPoint) -> i32; // CGError
}

pub struct DisplayManager;

impl DisplayManager {
    pub fn new() -> Self {
        Self
    }

    pub fn active_display_id(&self, app: &tauri::AppHandle) -> Option<DisplayId> {
        let (tx, rx) = std::sync::mpsc::channel();

        let _ = app.run_on_main_thread(move || {
            let result = MainThreadMarker::new()
                .and_then(NSScreen::mainScreen)
                .and_then(|screen| display_uuid_for_screen(&screen))
                .or_else(primary_display_uuid_via_cg);
            let _ = tx.send(result);
        });

        rx.recv().ok().flatten()
    }

    pub fn focus_display_center(&self, display_uuid: &DisplayId) -> Option<()> {
        let did = cg_display_id_for_uuid(display_uuid)?;
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
}

fn screen_display_id(screen: &NSScreen) -> Option<CGDirectDisplayID> {
    let desc: Retained<NSDictionary<NSString, objc2::runtime::AnyObject>> =
        screen.deviceDescription();
    let key = NSString::from_str("NSScreenNumber");
    let val = desc.objectForKey(&key)?;
    let num: &NSNumber = val.downcast_ref()?;
    Some(num.unsignedIntValue() as CGDirectDisplayID)
}

fn display_uuid_for_screen(screen: &NSScreen) -> Option<DisplayId> {
    let did = screen_display_id(screen)?;
    unsafe {
        let uuid_ref = CGDisplayCreateUUIDFromDisplayID(did);
        if uuid_ref.is_null() {
            return None;
        }
        let uuid = CFUUID::wrap_under_create_rule(uuid_ref as _);
        let s_ref = core_foundation::uuid::CFUUIDCreateString(
            core_foundation::base::kCFAllocatorDefault,
            uuid.as_concrete_TypeRef(),
        );
        let s = CFString::wrap_under_create_rule(s_ref);
        Some(DisplayId(s.to_string()))
    }
}

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
        let uuid = CFUUID::wrap_under_create_rule(uuid_ref as _);
        let s_ref = core_foundation::uuid::CFUUIDCreateString(
            core_foundation::base::kCFAllocatorDefault,
            uuid.as_concrete_TypeRef(),
        );
        let s = CFString::wrap_under_create_rule(s_ref);
        Some(DisplayId(s.to_string()))
    }
}

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
                cf_uuid as _,
            );
            let s = CFString::wrap_under_create_rule(s_ref);
            if s == uuid.0 {
                return Some(did);
            }
        }
        None
    }
}
