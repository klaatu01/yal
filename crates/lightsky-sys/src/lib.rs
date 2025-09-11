#![allow(non_camel_case_types, non_snake_case)]

use anyhow::Result;
use core_foundation::{array::CFArrayRef, base::CFTypeRef, string::CFStringRef};
use libloading::{Library, Symbol};
use std::ffi::c_void;

// --- Opaque CF / private types ---
pub type SLSConnectionID = i32;
pub type CFUUIDRef = *const c_void;

// --- Symbol table holding plain fn pointers and the Libraries to keep them alive ---
pub struct SkylightSymbols {
    _sky: Library, // SkyLight.framework

    // Connections
    pub SLSMainConnectionID: unsafe extern "C" fn() -> SLSConnectionID,

    // Windows in spaces
    pub SLSCopyWindowsWithOptionsAndTags: unsafe extern "C" fn(
        conn: SLSConnectionID,
        cid: i32,
        spaces: CFArrayRef,
        options: u32,
        set_tags: *mut u64,
        clear_tags: *mut u64,
    ) -> CFArrayRef,

    pub SLSWindowQueryWindows:
        unsafe extern "C" fn(conn: SLSConnectionID, windows: CFArrayRef, count: i32) -> CFTypeRef,

    pub SLSWindowQueryResultCopyWindows: unsafe extern "C" fn(query: CFTypeRef) -> CFTypeRef,

    pub SLSWindowIteratorAdvance: unsafe extern "C" fn(iter: CFTypeRef) -> bool,
    pub SLSWindowIteratorGetWindowID: unsafe extern "C" fn(iter: CFTypeRef) -> u32,
    pub SLSWindowIteratorGetParentID: unsafe extern "C" fn(iter: CFTypeRef) -> u32,
    pub SLSWindowIteratorGetLevel: unsafe extern "C" fn(iter: CFTypeRef) -> i32,
    pub SLSWindowIteratorGetTags: unsafe extern "C" fn(iter: CFTypeRef) -> u64,
    pub SLSWindowIteratorGetAttributes: unsafe extern "C" fn(iter: CFTypeRef) -> u64,
    pub SLSCopyManagedDisplayForSpace:
        unsafe extern "C" fn(conn: SLSConnectionID, sid: u64) -> CFStringRef,
    pub SLSSpaceGetType: unsafe extern "C" fn(conn: SLSConnectionID, sid: u64) -> i32,
    pub SLSCopyActiveSpace: Option<unsafe extern "C" fn(SLSConnectionID) -> u64>,
    pub CGSCopyManagedDisplaySpaces: Option<unsafe extern "C" fn(SLSConnectionID) -> CFTypeRef>,
}

impl SkylightSymbols {
    pub fn load() -> Result<Self> {
        unsafe {
            // load DSOs
            let sky =
                Library::new("/System/Library/PrivateFrameworks/SkyLight.framework/SkyLight")?;

            // helpers to copy function pointer values out of Symbols
            macro_rules! req {
                ($lib:expr, $t:ty, $name:literal) => {{
                    let sym: Symbol<$t> = $lib.get(concat!($name, "\0").as_bytes())?;
                    *sym // copy the fn pointer
                }};
            }
            macro_rules! opt {
                ($lib:expr, $t:ty, $name:literal) => {{
                    $lib.get::<$t>(concat!($name, "\0").as_bytes())
                        .ok()
                        .map(|s| *s)
                }};
            }

            // required SkyLight
            let SLSMainConnectionID = req!(
                sky,
                unsafe extern "C" fn() -> SLSConnectionID,
                "SLSMainConnectionID"
            );
            let SLSCopyWindowsWithOptionsAndTags = req!(
                sky,
                unsafe extern "C" fn(
                    SLSConnectionID,
                    i32,
                    CFArrayRef,
                    u32,
                    *mut u64,
                    *mut u64,
                ) -> CFArrayRef,
                "SLSCopyWindowsWithOptionsAndTags"
            );
            let SLSWindowQueryWindows = req!(
                sky,
                unsafe extern "C" fn(SLSConnectionID, CFArrayRef, i32) -> CFTypeRef,
                "SLSWindowQueryWindows"
            );
            let SLSWindowQueryResultCopyWindows = req!(
                sky,
                unsafe extern "C" fn(CFTypeRef) -> CFTypeRef,
                "SLSWindowQueryResultCopyWindows"
            );
            let SLSWindowIteratorAdvance = req!(
                sky,
                unsafe extern "C" fn(CFTypeRef) -> bool,
                "SLSWindowIteratorAdvance"
            );
            let SLSWindowIteratorGetWindowID = req!(
                sky,
                unsafe extern "C" fn(CFTypeRef) -> u32,
                "SLSWindowIteratorGetWindowID"
            );
            let SLSWindowIteratorGetParentID = req!(
                sky,
                unsafe extern "C" fn(CFTypeRef) -> u32,
                "SLSWindowIteratorGetParentID"
            );
            let SLSWindowIteratorGetLevel = req!(
                sky,
                unsafe extern "C" fn(CFTypeRef) -> i32,
                "SLSWindowIteratorGetLevel"
            );
            let SLSWindowIteratorGetTags = req!(
                sky,
                unsafe extern "C" fn(CFTypeRef) -> u64,
                "SLSWindowIteratorGetTags"
            );
            let SLSWindowIteratorGetAttributes = req!(
                sky,
                unsafe extern "C" fn(CFTypeRef) -> u64,
                "SLSWindowIteratorGetAttributes"
            );

            let SLSCopyManagedDisplayForSpace = req!(
                sky,
                unsafe extern "C" fn(SLSConnectionID, u64) -> CFStringRef,
                "SLSCopyManagedDisplayForSpace"
            );
            let SLSSpaceGetType = req!(
                sky,
                unsafe extern "C" fn(SLSConnectionID, u64) -> i32,
                "SLSSpaceGetType"
            );
            let SLSCopyActiveSpace = opt!(
                sky,
                unsafe extern "C" fn(SLSConnectionID) -> u64,
                "SLSCopyActiveSpace"
            );

            let CGSCopyManagedDisplaySpaces = opt!(
                sky,
                unsafe extern "C" fn(SLSConnectionID) -> CFTypeRef,
                "CGSCopyManagedDisplaySpaces"
            );

            Ok(Self {
                _sky: sky,

                SLSMainConnectionID,
                SLSCopyWindowsWithOptionsAndTags,
                SLSWindowQueryWindows,
                SLSWindowQueryResultCopyWindows,
                SLSWindowIteratorAdvance,
                SLSWindowIteratorGetWindowID,
                SLSWindowIteratorGetParentID,
                SLSWindowIteratorGetLevel,
                SLSWindowIteratorGetTags,
                SLSWindowIteratorGetAttributes,
                SLSCopyManagedDisplayForSpace,
                SLSSpaceGetType,
                SLSCopyActiveSpace,
                CGSCopyManagedDisplaySpaces,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Only runs on macOS
    #[cfg(target_os = "macos")]
    #[test]
    fn can_load_skylight_and_get_connection() {
        let syms = SkylightSymbols::load().expect("SkyLight symbols should load");
        let conn = unsafe { (syms.SLSMainConnectionID)() };
        // It's fine if it's 0 on some systems; we just want the call to succeed.
        assert!(
            conn >= 0,
            "connection id should be non-negative, got {conn}"
        );
    }
}
