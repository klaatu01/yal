#![allow(non_camel_case_types, non_snake_case)]

use anyhow::Result;
use core_foundation::{array::CFArrayRef, base::CFTypeRef, string::CFStringRef};
use libloading::{Library, Symbol};
use std::ffi::c_void;

pub type SLSConnectionID = i32;
pub type CFUUIDRef = *const c_void;

pub struct SkylightSymbols {
    _sky: Library,

    pub SLSMainConnectionID: unsafe extern "C" fn() -> SLSConnectionID,

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
    pub SLSGetActiveSpace: unsafe extern "C" fn(SLSConnectionID) -> u64,
    pub CGSCopyManagedDisplaySpaces: Option<unsafe extern "C" fn(SLSConnectionID) -> CFTypeRef>,
    pub CGSAddWindowsToSpaces:
        unsafe extern "C" fn(conn: SLSConnectionID, windows: CFArrayRef, spaces: CFArrayRef),
    pub CGSRemoveWindowsFromSpaces:
        unsafe extern "C" fn(conn: SLSConnectionID, windows: CFArrayRef, spaces: CFArrayRef),

    pub SLSCopySpacesForWindows: Option<
        unsafe extern "C" fn(conn: SLSConnectionID, windows: CFArrayRef, count: i32) -> CFTypeRef,
    >,
    pub SLSSpaceAddWindowsAndRemoveFromSpaces: Option<
        unsafe extern "C" fn(
            conn: SLSConnectionID,
            windows: CFArrayRef,
            add_to: CFArrayRef,
            remove_from: CFArrayRef,
        ),
    >,
    pub SLSMoveWindowsToManagedSpace:
        Option<unsafe extern "C" fn(conn: SLSConnectionID, windows: CFArrayRef, space: u64)>,
    pub SLSShowSpaces: Option<unsafe extern "C" fn(conn: SLSConnectionID, spaces: CFArrayRef)>,
    pub SLSManagedDisplaySetCurrentSpace:
        unsafe extern "C" fn(conn: SLSConnectionID, display: CFStringRef, space: u64) -> i32,
    pub SLSHideSpaces: Option<unsafe extern "C" fn(conn: SLSConnectionID, spaces: CFArrayRef)>,
}

impl SkylightSymbols {
    pub fn load() -> Result<Self> {
        unsafe {
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
            let SLSGetActiveSpace = req!(
                sky,
                unsafe extern "C" fn(SLSConnectionID) -> u64,
                "SLSGetActiveSpace"
            );

            let CGSCopyManagedDisplaySpaces = opt!(
                sky,
                unsafe extern "C" fn(SLSConnectionID) -> CFTypeRef,
                "CGSCopyManagedDisplaySpaces"
            );

            let CGSAddWindowsToSpaces = req!(
                sky,
                unsafe extern "C" fn(SLSConnectionID, CFArrayRef, CFArrayRef),
                "CGSAddWindowsToSpaces"
            );
            let CGSRemoveWindowsFromSpaces = req!(
                sky,
                unsafe extern "C" fn(SLSConnectionID, CFArrayRef, CFArrayRef),
                "CGSRemoveWindowsFromSpaces"
            );

            let SLSCopySpacesForWindows = opt!(
                sky,
                unsafe extern "C" fn(SLSConnectionID, CFArrayRef, i32) -> CFTypeRef,
                "SLSCopySpacesForWindows"
            );
            let SLSSpaceAddWindowsAndRemoveFromSpaces = opt!(
                sky,
                unsafe extern "C" fn(SLSConnectionID, CFArrayRef, CFArrayRef, CFArrayRef),
                "SLSSpaceAddWindowsAndRemoveFromSpaces"
            );
            let SLSMoveWindowsToManagedSpace = opt!(
                sky,
                unsafe extern "C" fn(SLSConnectionID, CFArrayRef, u64),
                "SLSMoveWindowsToManagedSpace"
            );
            let SLSShowSpaces = opt!(
                sky,
                unsafe extern "C" fn(SLSConnectionID, CFArrayRef),
                "SLSShowSpaces"
            );
            let SLSManagedDisplaySetCurrentSpace = req!(
                sky,
                unsafe extern "C" fn(SLSConnectionID, CFStringRef, u64) -> i32,
                "SLSManagedDisplaySetCurrentSpace"
            );
            let SLSHideSpaces = opt!(
                sky,
                unsafe extern "C" fn(SLSConnectionID, CFArrayRef),
                "SLSHideSpaces"
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
                SLSGetActiveSpace,
                CGSCopyManagedDisplaySpaces,
                CGSAddWindowsToSpaces,
                CGSRemoveWindowsFromSpaces,
                SLSCopySpacesForWindows,
                SLSSpaceAddWindowsAndRemoveFromSpaces,
                SLSMoveWindowsToManagedSpace,
                SLSShowSpaces,
                SLSHideSpaces,
                SLSManagedDisplaySetCurrentSpace,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "macos")]
    #[test]
    fn can_load_skylight_and_get_connection() {
        let syms = SkylightSymbols::load().expect("SkyLight symbols should load");
        let conn = unsafe { (syms.SLSMainConnectionID)() };
        assert!(
            conn >= 0,
            "connection id should be non-negative, got {conn}"
        );
    }
}
