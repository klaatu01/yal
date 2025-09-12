use core_graphics::display::CGDirectDisplayID;
use lightsky::Lightsky;
use objc2::rc::Retained;
use objc2_app_kit::NSScreen;
use objc2_foundation::{MainThreadMarker, NSDictionary, NSNumber, NSString};

use core_foundation::base::TCFType;
use core_foundation::string::CFString;
use core_foundation::uuid::{CFUUIDRef, CFUUID};

use crate::ax::application_tree::ApplicationTree;
mod application_tree;

extern "C" {
    fn CGDisplayCreateUUIDFromDisplayID(display: CGDirectDisplayID) -> CFUUIDRef;
}

pub struct AX {
    pub lightsky: Lightsky,
    pub application_tree: ApplicationTree,
}

impl AX {
    pub fn new() -> Self {
        let lightsky = Lightsky::new().expect("Failed to initialize Lightsky");
        let application_tree = ApplicationTree::new(&lightsky);
        AX {
            lightsky,
            application_tree,
        }
    }

    pub fn refresh(&mut self) {
        self.application_tree = ApplicationTree::new(&self.lightsky);
    }
}

/// Return the system’s screens as CoreGraphics display IDs.
pub fn get_displays() -> Vec<CGDirectDisplayID> {
    let mtm = MainThreadMarker::new().expect("Failed to get main thread marker");
    NSScreen::screens(mtm)
        .iter()
        .filter_map(|screen| screen_display_id(&screen))
        .collect()
}

/// Extract `CGDirectDisplayID` (NSScreenNumber) from an NSScreen.
pub fn screen_display_id(screen: &NSScreen) -> Option<CGDirectDisplayID> {
    // deviceDescription: NSDictionary<NSDeviceDescriptionKey, id>
    let desc: Retained<NSDictionary<NSString, objc2::runtime::AnyObject>> =
        screen.deviceDescription();
    let key = NSString::from_str("NSScreenNumber");
    let val = desc.objectForKey(&key)?;
    let num: &NSNumber = val.downcast_ref()?;
    Some(num.unsignedIntValue() as CGDirectDisplayID)
}

/// Convert an NSScreen to its CoreGraphics/ColorSync UUID string.
pub fn display_uuid_for_screen(screen: &NSScreen) -> Option<String> {
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
        Some(s.to_string())
    }
}

/// UUID string for the active (main) screen.
pub fn active_display_uuid() -> Option<String> {
    let mtm = MainThreadMarker::new()?;
    let screen = NSScreen::mainScreen(mtm)?;
    display_uuid_for_screen(&screen)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn build_ax() {
        let mut _ax = AX::new();
        println!("{}", _ax.application_tree);
        let focused = _ax
            .application_tree
            .search(application_tree::SearchParam::ByFocused);
        assert!(focused.len() <= 1);
        let focused_window = focused.first().unwrap();
        println!(
            "Focused window: pid={} app_name={} title={:?}",
            focused_window.pid, focused_window.app_name, focused_window.title
        );
    }
}
