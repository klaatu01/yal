use core_foundation::array::CFArrayRef;
use core_foundation::base::{CFTypeRef, TCFType};
use core_foundation::number::CFNumber;
use core_foundation::string::{CFString, CFStringRef};
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
}

pub struct FocusManager;

impl FocusManager {
    pub fn new() -> Self {
        Self
    }

    /// Bring app to foreground and (optionally) focus/raise a specific window by AXWindowNumber.
    pub fn focus(&self, app: &tauri::AppHandle, pid: i32, window_id: Option<WindowId>) {
        // Activate app on main thread
        let _ = app.run_on_main_thread(move || unsafe {
            if let Some(app) = NSRunningApplication::runningApplicationWithProcessIdentifier(pid) {
                let _ = app.activateWithOptions(NSApplicationActivationOptions::ActivateAllWindows);
            }
        });

        if let Some(window_id) = window_id {
            unsafe {
                let app_ax: AXUIElementRef = AXUIElementCreateApplication(pid);
                if app_ax.is_null() {
                    return;
                }

                let ax_windows = CFString::from_static_string("AXWindows");
                let ax_focused_window = CFString::from_static_string("AXFocusedWindow");
                let ax_window_number = CFString::from_static_string("AXWindowNumber");
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
                    return;
                }

                let windows_array: CFArrayRef = windows_val as CFArrayRef;
                let count = CFArrayGetCount(windows_array);
                let target_num: i64 = window_id.0 as i64;

                let mut matched_window: Option<AXUIElementRef> = None;

                for i in 0..count {
                    let w_ref = CFArrayGetValueAtIndex(windows_array, i) as AXUIElementRef;
                    if w_ref.is_null() {
                        continue;
                    }

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

                    let cfnum = CFNumber::wrap_under_create_rule(num_val as _);
                    if let Some(n) = cfnum.to_i64() {
                        if n == target_num {
                            matched_window = Some(w_ref);
                            break;
                        }
                    }
                }

                // Release the windows array we copied
                CFRelease(windows_val);

                if let Some(w_ref) = matched_window {
                    let _ = AXUIElementSetAttributeValue(
                        app_ax,
                        ax_focused_window.as_concrete_TypeRef(),
                        w_ref as CFTypeRef,
                    );
                    let _ = AXUIElementPerformAction(w_ref, ax_raise.as_concrete_TypeRef());
                }

                CFRelease(app_ax as CFTypeRef);
            }
        }
    }
}
