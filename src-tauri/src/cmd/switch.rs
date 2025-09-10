#![allow(clippy::missing_safety_doc)]
use core_foundation::{
    array::CFArrayRef,
    base::{CFIndex, CFTypeRef, TCFType},
    boolean::CFBoolean,
    string::CFString,
};

use lightsky::Lightsky;
use objc2::rc::Retained;
use objc2_app_kit::{NSApplicationActivationOptions, NSRunningApplication};

use std::ffi::c_void;
use std::ptr;

use yal_core::WindowTarget;

extern "C" {
    fn CFArrayGetCount(theArray: CFArrayRef) -> CFIndex;
    fn CFArrayGetValueAtIndex(theArray: CFArrayRef, idx: CFIndex) -> *const c_void;
}

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGPreflightScreenCaptureAccess() -> bool;
    fn CGRequestScreenCaptureAccess() -> bool;
}

pub fn list_switch_targets() -> Vec<WindowTarget> {
    let _ = ensure_cg_permission_prompt();
    let _ = ensure_ax_permission_prompt();
    let sky = match Lightsky::new() {
        Ok(sky) => sky,
        Err(e) => {
            log::error!("Failed to initialize Lightsky: {}", e);
            return Vec::new();
        }
    };

    let displays = match sky.list_all_spaces() {
        Ok(displays) => displays,
        Err(e) => {
            log::error!("Failed to list spaces: {}", e);
            return Vec::new();
        }
    };

    let mut targets = Vec::new();
    for display in displays {
        println!("{}:", display);
        for space in display.spaces {
            println!("  Space ID: {}", space.id);
            println!("  Windows:");
            match sky.windows_in_spaces_app_only_with_titles(
                &[space.id],
                lightsky::WindowListOptions::all(),
            ) {
                Ok(wl) => {
                    for w in &wl {
                        targets.push(WindowTarget {
                            app_name: w.owner_name.clone().unwrap_or_default(),
                            title: w.title.clone(),
                            pid: w.pid.unwrap(),
                            label: if let Some(title) = &w.title {
                                format!("{} - {}", w.owner_name.clone().unwrap_or_default(), title)
                            } else {
                                w.owner_name.clone().unwrap_or_default()
                            },
                        });
                    }
                }
                Err(e) => {
                    log::error!("Failed to list windows in space {}: {}", space.id, e);
                    continue;
                }
            };
        }
    }
    targets
}

pub fn focus_switch_target(t: &WindowTarget) -> Result<(), String> {
    let _ = ensure_ax_permission_prompt();

    match &t.title {
        None => activate_app_by_pid(t.pid),
        Some(title) => {
            if ax_focus_window_by_title(t.pid, title).is_err() {
                activate_app_by_pid(t.pid)
            } else {
                Ok(())
            }
        }
    }
}

fn cfstr(s: &str) -> CFString {
    CFString::new(s)
}

#[allow(non_camel_case_types)]
type AXUIElementRef = *const c_void;

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrustedWithOptions(options: CFTypeRef) -> bool;

    fn AXUIElementCreateApplication(pid: i32) -> AXUIElementRef;
    fn AXUIElementCopyAttributeValue(
        element: AXUIElementRef,
        attr: CFTypeRef,
        out: *mut CFTypeRef,
    ) -> i32;
    fn AXUIElementSetAttributeValue(
        element: AXUIElementRef,
        attr: CFTypeRef,
        value: CFTypeRef,
    ) -> i32;
    fn AXUIElementPerformAction(element: AXUIElementRef, action: CFTypeRef) -> i32;
}

fn cfbool(b: bool) -> CFBoolean {
    if b {
        CFBoolean::true_value()
    } else {
        CFBoolean::false_value()
    }
}

fn ensure_cg_permission_prompt() -> bool {
    unsafe {
        if CGPreflightScreenCaptureAccess() {
            true
        } else {
            // Shows the system dialog. Note: user may need to relaunch the host app for access to take effect.
            CGRequestScreenCaptureAccess()
        }
    }
}

fn ensure_ax_permission_prompt() -> bool {
    let key = cfstr("AXTrustedCheckOptionPrompt");
    let val = cfbool(true);
    let opts = core_foundation::dictionary::CFDictionary::from_CFType_pairs(&[(
        key.as_CFType(),
        val.as_CFType(),
    )]);
    unsafe { AXIsProcessTrustedWithOptions(opts.as_concrete_TypeRef() as _) }
}

fn ax_focus_window_by_title(pid: i32, target_title: &str) -> Result<(), String> {
    let app = unsafe { AXUIElementCreateApplication(pid) };
    if app.is_null() {
        return Err("AXUIElementCreateApplication returned null".into());
    }

    let mut windows_ref: CFTypeRef = ptr::null();
    let ax_windows = cfstr("AXWindows");
    let err = unsafe {
        AXUIElementCopyAttributeValue(app, ax_windows.as_concrete_TypeRef() as _, &mut windows_ref)
    };
    if err != 0 || windows_ref.is_null() {
        return Err("Failed to read AXWindows".into());
    }

    let windows_arr = windows_ref as CFArrayRef;
    if windows_arr.is_null() {
        return Err("AXWindows not an array".into());
    }

    let count = unsafe { CFArrayGetCount(windows_arr) };
    let ax_title = cfstr("AXTitle");
    let ax_raise = cfstr("AXRaise");
    let ax_main = cfstr("AXMain");

    for i in 0..count {
        let w_ref = unsafe { CFArrayGetValueAtIndex(windows_arr, i) } as AXUIElementRef;
        if w_ref.is_null() {
            continue;
        }

        let mut title_ref: CFTypeRef = ptr::null();
        let t_err = unsafe {
            AXUIElementCopyAttributeValue(
                w_ref,
                ax_title.as_concrete_TypeRef() as _,
                &mut title_ref,
            )
        };
        if t_err != 0 || title_ref.is_null() {
            continue;
        }
        let title = unsafe { CFString::wrap_under_create_rule(title_ref as _) }.to_string();

        if title == target_title {
            let r = unsafe { AXUIElementPerformAction(w_ref, ax_raise.as_concrete_TypeRef() as _) };
            if r != 0 {
                return Err("AXRaise failed".into());
            }
            let _ = unsafe {
                AXUIElementSetAttributeValue(
                    w_ref,
                    ax_main.as_concrete_TypeRef() as _,
                    CFBoolean::true_value().as_CFTypeRef(),
                )
            };
            return Ok(());
        }
    }

    Err("Window title not found via AX".into())
}

fn activate_app_by_pid(pid: i32) -> Result<(), String> {
    unsafe {
        let app: Option<Retained<NSRunningApplication>> =
            NSRunningApplication::runningApplicationWithProcessIdentifier(pid);
        if let Some(app) = app {
            app.activateWithOptions(NSApplicationActivationOptions::ActivateAllWindows);
            Ok(())
        } else {
            Err("NSRunningApplication not found".into())
        }
    }
}
