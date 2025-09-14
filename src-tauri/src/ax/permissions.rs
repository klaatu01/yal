use core_foundation::base::TCFType;
use core_foundation::boolean::CFBoolean;
use core_foundation::dictionary::{CFDictionary, CFDictionaryRef};
use core_foundation::string::CFString;

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGPreflightScreenCaptureAccess() -> bool;
    fn CGRequestScreenCaptureAccess() -> bool;
}

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrustedWithOptions(options: CFDictionaryRef) -> bool;
}

pub struct PermissionsManager;

impl PermissionsManager {
    pub fn new() -> Self {
        Self
    }

    pub fn ensure(&self) {
        let ax_ok = self.ensure_ax_permission_prompt();
        if !ax_ok {
            log::warn!(
                "Accessibility permission not granted. \
                 System should prompt; otherwise enable in System Settings → Privacy & Security → Accessibility, \
                 then quit & relaunch."
            );
        }

        let sr_ok = self.ensure_cg_permission_prompt();
        if !sr_ok {
            log::warn!(
                "Screen Recording permission not granted. \
                 Enable in System Settings → Privacy & Security → Screen Recording, \
                 then quit & relaunch."
            );
        }
    }

    fn ensure_ax_permission_prompt(&self) -> bool {
        let key = CFString::from_static_string("kAXTrustedCheckOptionPrompt");
        let no_prompt = CFDictionary::from_CFType_pairs(&[(
            key.as_CFType(),
            CFBoolean::false_value().as_CFType(),
        )]);
        let trusted = unsafe { AXIsProcessTrustedWithOptions(no_prompt.as_concrete_TypeRef()) };
        if trusted {
            return true;
        }
        let with_prompt = CFDictionary::from_CFType_pairs(&[(
            key.as_CFType(),
            CFBoolean::true_value().as_CFType(),
        )]);
        unsafe { AXIsProcessTrustedWithOptions(with_prompt.as_concrete_TypeRef()) }
    }

    fn ensure_cg_permission_prompt(&self) -> bool {
        unsafe {
            if CGPreflightScreenCaptureAccess() {
                true
            } else {
                CGRequestScreenCaptureAccess()
            }
        }
    }
}
