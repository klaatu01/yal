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
    fn AXUIElementCreateSystemWide() -> AXUIElementRef; // ← add this
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

#[derive(Actor)]
pub struct FocusManagerActor {
    app_handle: tauri::AppHandle,
    focus_window_id: Option<WindowId>,
}

pub struct FocusWindow {
    pub pid: i32,
    pub window_id: Option<WindowId>,
}

impl Message<FocusWindow> for FocusManagerActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: FocusWindow,
        _ctx: &mut kameo::prelude::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.focus_window_id = msg.window_id;
        self.focus(&self.app_handle, msg.pid, msg.window_id);
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

impl FocusManagerActor {
    pub fn new(app_handle: tauri::AppHandle) -> Self {
        Self {
            app_handle,
            focus_window_id: None,
        }
    }

    pub fn focused_window_id(&self) -> Option<WindowId> {
        unsafe {
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
                *core_foundation::dictionary::CFDictionary::wrap_under_get_rule(dict_ref)
                    .find(&key)?;

            let num_ref: CFNumberRef = value as CFNumberRef;
            let num = core_foundation::number::CFNumber::wrap_under_get_rule(num_ref);

            num.to_i64().map(|n| WindowId(n as u32))
        }
    }

    pub fn focus(&self, app: &tauri::AppHandle, pid: i32, window_id: Option<WindowId>) {
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
