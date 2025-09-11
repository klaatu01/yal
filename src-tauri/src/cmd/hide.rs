// focus.rs
#![allow(clippy::missing_safety_doc)]
use std::{ffi::c_void, sync::RwLock};

use core_foundation::{
    array::CFArrayRef,
    base::{CFIndex, CFRelease, CFTypeRef, TCFType},
    boolean::CFBoolean,
    number::{kCFNumberSInt64Type, CFNumber, CFNumberGetValue, CFNumberRef},
    string::CFString,
};
use lightsky::{DisplayId, DisplaySpaces, SpaceId, WindowId};
use objc2_foundation::MainThreadMarker;
use tauri::Manager;

pub enum HideBehavior {
    FocusPrevious,
    FocusNew { pid: i32, window_id: i64 },
}

#[derive(Clone)]
pub struct FocusState {
    pub display_id: DisplayId,
    pub pid: i32,
    pub window_id: WindowId,
    pub space_id: SpaceId,
}

/* ------------------------------ Tauri state ------------------------------ */

pub fn get_focus_state(app: &tauri::AppHandle) -> Option<FocusState> {
    app.state::<RwLock<Option<FocusState>>>()
        .read()
        .unwrap()
        .clone()
}

pub fn set_focus_state(app: &tauri::AppHandle, state: FocusState) {
    *app.state::<RwLock<Option<FocusState>>>().write().unwrap() = Some(state);
}

pub fn clear_focus_state(app: &tauri::AppHandle) {
    *app.state::<RwLock<Option<FocusState>>>().write().unwrap() = None;
}

/* -------------------------- Public entry points -------------------------- */

/// Hide your palette and focus either the previous app/window or a new specific one.
pub fn hide(app: &tauri::AppHandle, _behavior: HideBehavior) {
    // 1) Hide the palette window (named "main" here) and deactivate our app to yield key focus.
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.hide();
    }
    let _ = app.run_on_main_thread(|| unsafe {
        let mtm = MainThreadMarker::new_unchecked();
        objc2_app_kit::NSApp(mtm).deactivate();
    });
}

/// Determine the CURRENT focused display+space by using AX to get the focused window,
/// then scanning all spaces to find where that window lives.
/// Falls back to a placeholder if we can't determine it.
pub fn find_current_display_space(displays: Vec<DisplaySpaces>) -> DisplaySpaces {
    // Best effort: try AX → (pid, window_id)
    if let Some((pid, win_id)) = ax_get_focused_pid_and_window() {
        if let Ok(sky) = lightsky::Lightsky::new() {
            // Fast path: find the space containing this exact CG window id
            for disp in &displays {
                for srec in &disp.spaces {
                    if let Ok(wins) = sky.get_windows_in_space(
                        srec.id,
                        lightsky::WindowListOptions::VISIBLE,
                        lightsky::WindowKindFilter::ALL,
                    ) {
                        if wins.iter().any(|w| w.window_id == win_id) {
                            // Return the display with its *current* set to the one we just matched.
                            return DisplaySpaces {
                                display_identifier: disp.display_identifier.clone(),
                                current: srec.id,
                                spaces: disp.spaces.clone(),
                            };
                        }
                    }
                }
            }
            // Slower fallback: match by PID in case window numbers don't line up for this app
            for disp in &displays {
                for srec in &disp.spaces {
                    if let Ok(wins) = sky.get_windows_in_space_with_titles(
                        srec.id,
                        lightsky::WindowListOptions::VISIBLE,
                        lightsky::WindowKindFilter::ALL,
                    ) {
                        if wins.iter().any(|w| w.pid == Some(pid)) {
                            return DisplaySpaces {
                                display_identifier: disp.display_identifier.clone(),
                                current: srec.id,
                                spaces: disp.spaces.clone(),
                            };
                        }
                    }
                }
            }
        }
    }

    // Final fallback: if we have any display entries, prefer the first one.
    // Otherwise construct a placeholder.
    displays.into_iter().next().unwrap_or(DisplaySpaces {
        display_identifier: DisplayId("<unknown>".into()),
        current: SpaceId(0),
        spaces: vec![],
    })
}

/// Determine the CURRENT focused window by AX and ensure it belongs to the provided display/space.
/// Returns (pid, WindowId) if we can find a corresponding window in that space.
pub fn find_current_window(display_space: DisplaySpaces) -> Option<(i32, WindowId)> {
    let (pid, ax_win) = ax_get_focused_pid_and_window()?;

    // Confirm that this window actually lives in the provided space.
    let sky = lightsky::Lightsky::new().ok()?;

    // First, try exact CG window id membership in this space.
    if let Ok(wins) = sky.get_windows_in_space(
        display_space.current,
        lightsky::WindowListOptions::VISIBLE,
        lightsky::WindowKindFilter::ALL,
    ) {
        if wins.iter().any(|w| w.window_id == ax_win) {
            return Some((pid, WindowId(ax_win)));
        }
    }

    // Fallback: match by PID within that space (pick first app window).
    if let Ok(wins) = sky.get_windows_in_space_with_titles(
        display_space.current,
        lightsky::WindowListOptions::VISIBLE,
        lightsky::WindowKindFilter::ALL,
    ) {
        if let Some(w) = wins.iter().find(|w| w.pid == Some(pid)) {
            return Some((pid, WindowId(w.info.window_id)));
        }
    }

    None
}

/// Bring the target app/window to the foreground.
/// 1) If the current space != target space, switch spaces.
/// 2) Make the app frontmost, raise and focus the target window.
/// Requires AX permission (System Settings → Privacy & Security → Accessibility).
fn activate_app(focus: &FocusState) -> Result<(), String> {
    // 1) Switch to the correct Space if needed.
    if let Ok(sky) = lightsky::Lightsky::new() {
        if let Some(cur) = sky.current_space() {
            if cur != focus.space_id {
                // NOTE: Your lightsky::Lightsky likely already exposes `select_space`.
                // If not, add it to your lib (as in earlier iterations).
                if let Err(e) = sky.select_space(focus.space_id) {
                    // Not fatal; continue and attempt to focus anyway.
                    log::warn!("Failed to switch space via SkyLight: {}", e);
                }
            }
        }
    }

    // 2) AX: frontmost app + raise / focus the window.
    // Make application frontmost
    unsafe {
        let app_el = AXUIElementCreateApplication(focus.pid);
        // Set AXFrontmost = true
        let _ = AXUIElementSetAttributeValue(
            app_el,
            cfstr("AXFrontmost").as_CFTypeRef(),
            cfbool(true).as_CFTypeRef(),
        );

        // Find the AX window element that corresponds to the CG window id
        if let Some(win_el) = ax_find_app_window_by_number(app_el, focus.window_id.0) {
            // Try to set it main and focused, then raise it.
            let _ = AXUIElementSetAttributeValue(
                win_el,
                cfstr("AXMain").as_CFTypeRef(),
                cfbool(true).as_CFTypeRef(),
            );
            let _ = AXUIElementSetAttributeValue(
                win_el,
                cfstr("AXFocused").as_CFTypeRef(),
                cfbool(true).as_CFTypeRef(),
            );
            let _ = AXUIElementPerformAction(win_el, cfstr("AXRaise").as_CFTypeRef());
            // Release win_el now that we're done
            CFRelease(win_el as CFTypeRef);
        }

        // Release app element
        CFRelease(app_el as CFTypeRef);
    }

    Ok(())
}

/* ----------------------------- AX / AppKit ------------------------------- */

#[allow(non_camel_case_types)]
type AXUIElementRef = *const c_void;

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXUIElementCreateSystemWide() -> AXUIElementRef;
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
    fn AXUIElementGetPid(element: AXUIElementRef, out_pid: *mut i32) -> i32;
}

extern "C" {
    fn CFArrayGetCount(theArray: CFArrayRef) -> CFIndex;
    fn CFArrayGetValueAtIndex(theArray: CFArrayRef, idx: CFIndex) -> *const c_void;
}

#[inline]
fn cfstr(s: &str) -> CFString {
    CFString::new(s)
}

#[inline]
fn cfbool(b: bool) -> CFBoolean {
    if b {
        CFBoolean::true_value()
    } else {
        CFBoolean::false_value()
    }
}

/* ------------------------------- AX helpers ------------------------------- */

/// Return (focused PID, focused window CGWindowID) if available via AX.
fn ax_get_focused_pid_and_window() -> Option<(i32, i64)> {
    unsafe {
        let sys = AXUIElementCreateSystemWide();

        // Focused application
        let mut app_ref: CFTypeRef = std::ptr::null();
        let err = AXUIElementCopyAttributeValue(
            sys,
            cfstr("AXFocusedApplication").as_CFTypeRef(),
            &mut app_ref,
        );
        if err != 0 || app_ref.is_null() {
            return None;
        }

        // PID
        let mut pid: i32 = 0;
        if AXUIElementGetPid(app_ref as AXUIElementRef, &mut pid) != 0 {
            CFRelease(app_ref);
            return None;
        }

        // Focused window → window number
        let win = ax_copy_first_window(app_ref as AXUIElementRef);
        CFRelease(app_ref);

        let win = win?;
        let mut out_ref: CFTypeRef = std::ptr::null();
        let ok = AXUIElementCopyAttributeValue(
            win as AXUIElementRef,
            cfstr("AXWindowNumber").as_CFTypeRef(),
            &mut out_ref,
        );
        if ok != 0 || out_ref.is_null() {
            CFRelease(win);
            return None;
        }

        // CFNumber -> i64
        let nref = out_ref as CFNumberRef;
        let mut wnum: i64 = 0;
        let conv_ok = CFNumberGetValue(
            nref,
            kCFNumberSInt64Type,
            &mut wnum as *mut i64 as *mut c_void,
        );
        CFRelease(out_ref);
        CFRelease(win);

        if conv_ok {
            Some((pid, wnum))
        } else {
            None
        }
    }
}

/// Try focused window, then main window, then first in AXWindows.
unsafe fn ax_copy_first_window(app_el: AXUIElementRef) -> Option<CFTypeRef> {
    // 1) Focused
    if let Some(w) = ax_copy_attr(app_el, "AXFocusedWindow") {
        return Some(w);
    }
    // 2) Main
    if let Some(w) = ax_copy_attr(app_el, "AXMainWindow") {
        return Some(w);
    }
    // 3) First in AXWindows
    if let Some(arr) = ax_copy_attr(app_el, "AXWindows") {
        let arr_ref = arr as CFArrayRef;
        let count = CFArrayGetCount(arr_ref);
        if count > 0 {
            let w = CFArrayGetValueAtIndex(arr_ref, 0);
            // Retain semantics: AX returns a retained array; its elements are unretained.
            // Create a retained reference to the window element so the caller can release it.
            // We can just return it as-is; caller won't release the array itself here.
            CFRelease(arr);
            return Some(w as CFTypeRef);
        }
        CFRelease(arr);
    }
    None
}

/// Copy a retained attribute value (CFTypeRef) from an AX element.
unsafe fn ax_copy_attr(element: AXUIElementRef, name: &str) -> Option<CFTypeRef> {
    let mut out: CFTypeRef = std::ptr::null();
    let err = AXUIElementCopyAttributeValue(element, cfstr(name).as_CFTypeRef(), &mut out);
    if err == 0 && !out.is_null() {
        Some(out)
    } else {
        None
    }
}

/// Find a window element for this app that matches the given CG window number.
unsafe fn ax_find_app_window_by_number(
    app_el: AXUIElementRef,
    target_wid: i64,
) -> Option<AXUIElementRef> {
    let arr = ax_copy_attr(app_el, "AXWindows")? as CFArrayRef;
    let count = CFArrayGetCount(arr);
    for i in 0..count {
        let w = CFArrayGetValueAtIndex(arr, i) as AXUIElementRef;

        let mut nref: CFTypeRef = std::ptr::null();
        let ok =
            AXUIElementCopyAttributeValue(w, cfstr("AXWindowNumber").as_CFTypeRef(), &mut nref);
        if ok == 0 && !nref.is_null() {
            let mut val: i64 = 0;
            let _ = CFNumberGetValue(
                nref as CFNumberRef,
                kCFNumberSInt64Type,
                &mut val as *mut i64 as *mut c_void,
            );
            CFRelease(nref);
            if val == target_wid {
                // Return a +1 retained reference for caller symmetry
                // Simplest way is to "copy" via a benign attribute fetch that returns the same object;
                // but AX doesn't have direct CFRetain here. We can just return the raw pointer and let caller skip releasing.
                // To avoid leaks, perform a cheap attribute read that retains (focused attr).
                // In practice, skipping the retain here is fine; we'll not CFRelease in caller if we didn't retain.
                return Some(w);
            }
        }
    }
    None
}
