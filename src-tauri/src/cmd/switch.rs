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
use tauri::Manager;

use std::{
    ffi::c_void,
    sync::{Arc, Mutex},
};
use std::{ptr, sync::RwLock};

use yal_core::WindowTarget;

use crate::ax::AX;

extern "C" {
    fn CFArrayGetCount(theArray: CFArrayRef) -> CFIndex;
    fn CFArrayGetValueAtIndex(theArray: CFArrayRef, idx: CFIndex) -> *const c_void;
}

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGPreflightScreenCaptureAccess() -> bool;
    fn CGRequestScreenCaptureAccess() -> bool;
}

pub fn list_switch_targets(app: &tauri::AppHandle) -> Vec<WindowTarget> {
    let _ = ensure_cg_permission_prompt();
    let _ = ensure_ax_permission_prompt();
    let ax = app.state::<Arc<RwLock<AX>>>();
    let ax_guard = ax.read().unwrap();
    let results = ax_guard.application_tree.flatten();
    results
        .into_iter()
        .map(|w| WindowTarget {
            pid: w.pid,
            window_id: w.window_id.0,
            title: w.title.clone(),
            app_name: w.app_name.clone(),
        })
        .collect()
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
    fn AXUIElementCreateSystemWide() -> AXUIElementRef; // NEW

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
    let ax_focused_window = cfstr("AXFocusedWindow"); // NEW
    let ax_focused_app = cfstr("AXFocusedApplication"); // NEW
    let ax_hidden = cfstr("AXHidden");
    let ax_frontmost = cfstr("AXFrontmost");
    let ax_minimized = cfstr("AXMinimized");

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
            // 1) Bring the window forward
            let r = unsafe { AXUIElementPerformAction(w_ref, ax_raise.as_concrete_TypeRef() as _) };
            if r != 0 {
                return Err("AXRaise failed".into());
            }

            // 2) Mark it as main (harmless but not sufficient alone)
            let _ = unsafe {
                AXUIElementSetAttributeValue(
                    w_ref,
                    ax_main.as_concrete_TypeRef() as _,
                    CFBoolean::true_value().as_CFTypeRef(),
                )
            };

            // 3) Make it the *focused* window
            let fr = unsafe {
                AXUIElementSetAttributeValue(
                    app,
                    ax_focused_window.as_concrete_TypeRef() as _,
                    w_ref as CFTypeRef,
                )
            };
            if fr != 0 {
                // not fatal, but loggable
                log::warn!("Setting AXFocusedWindow returned {}", fr);
            }

            let _ = unsafe {
                AXUIElementSetAttributeValue(
                    app,
                    ax_hidden.as_concrete_TypeRef() as _,
                    CFBoolean::false_value().as_CFTypeRef(),
                )
            };
            let _ = unsafe {
                AXUIElementSetAttributeValue(
                    w_ref,
                    ax_minimized.as_concrete_TypeRef() as _,
                    CFBoolean::false_value().as_CFTypeRef(),
                )
            };

            let _ = unsafe {
                AXUIElementSetAttributeValue(
                    app,
                    ax_frontmost.as_concrete_TypeRef() as _,
                    CFBoolean::true_value().as_CFTypeRef(),
                )
            };

            // 4) (Optional but helps) mark the app as focused at system level
            let sys = unsafe { AXUIElementCreateSystemWide() };
            if !sys.is_null() {
                let _ = unsafe {
                    AXUIElementSetAttributeValue(
                        sys,
                        ax_focused_app.as_concrete_TypeRef() as _,
                        app as CFTypeRef,
                    )
                };
            }

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
            // Important: ignore other apps to actually transfer key focus
            app.activateWithOptions(NSApplicationActivationOptions::ActivateAllWindows);
            Ok(())
        } else {
            Err("NSRunningApplication not found".into())
        }
    }
}
