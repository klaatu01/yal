#![cfg(target_os = "macos")]

use anyhow::{Result, anyhow};
use bitflags::bitflags;
use core_foundation::{
    array::{
        CFArrayCreate, CFArrayGetCount, CFArrayGetValueAtIndex, CFArrayRef, kCFTypeArrayCallBacks,
    },
    base::{CFRelease, CFTypeRef, TCFType},
    dictionary::{CFDictionaryGetValue, CFDictionaryRef},
    number::{CFNumber, CFNumberGetValue, CFNumberRef, kCFNumberSInt64Type},
    string::CFString,
};
use core_graphics::window::CGWindowListCopyWindowInfo;
use lightsky_sys::{SLSConnectionID, SkylightSymbols};
use serde::{Deserialize, Serialize};

use std::{collections::HashMap, ffi::c_void, ptr};

/* ----------------------------- SkyLight heuristics ---------------------------- */
// Private SkyLight heuristics (observed; may vary by macOS)
const TAG_HAS_TITLEBAR_LIKE: u64 = 0x0400_0000_0000_0000;
const TAG_MINIMIZED_1: u64 = 0x1000_0000_0000_0000;
const TAG_MINIMIZED_2: u64 = 0x0300_0000_0000_0000;

/* -------------------------------- Public types -------------------------------- */
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SpaceId(pub u64);

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct DisplayId(pub String);

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WindowId(pub u32);

impl std::fmt::Display for WindowId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::fmt::Display for DisplayId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::fmt::Display for SpaceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpaceType {
    User,
    System,
    Fullscreen,
    Other(i32),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisplaySpaces {
    /// e.g. "Display-1" / a UUID-like string (varies by macOS)
    pub display_identifier: DisplayId,
    pub current: SpaceId,
    pub spaces: Vec<SpaceRecord>,
}

impl std::fmt::Display for DisplaySpaces {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Display: {}", self.display_identifier.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpaceRecord {
    pub id: SpaceId,
    pub kind: SpaceType,
    pub is_current_on_display: bool,
}

bitflags! {
    /// Private API bits passed to SLS window copy routines; vary by macOS.
    #[derive(Default, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct WindowListOptions: u32 {
        /// “Visible-ish” windows (typical)
        const VISIBLE = 0x2;
        /// Broader set: often includes minimized/off-space windows
        const INCLUDE_MINIMIZED = 0x7;
    }
}

/* ------------------------------ Kind filtering -------------------------------- */

bitflags! {
    /// Filter which kinds of windows you want back.
    /// You can OR these together, e.g. `APP | UTILITY`.
    #[derive(Default, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct WindowKindFilter: u32 {
        /// Standard application windows (top-level, normal-ish).
        const APP        = 0b0000_0001;
        /// Floating/utility/panel-like.
        const UTILITY    = 0b0000_0010;
        /// Fullscreen style windows.
        const FULLSCREEN = 0b0000_0100;
        /// Minimized windows (as inferred from tags).
        const MINIMIZED  = 0b0000_1000;
        /// Anything that doesn’t match other buckets.
        const OTHER      = 0b0001_0000;

        /// Convenience: include everything.
        const ALL        = u32::MAX;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowInfo {
    pub window_id: WindowId,
    pub parent_window_id: u32,
    pub level: i32,
    pub tags: u64,
    pub attributes: u64,
    pub space_id: SpaceId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Window {
    pub info: WindowInfo,
    pub pid: Option<i32>,
    pub owner_name: Option<String>,
    pub title: Option<String>,
}

/// Grouping helper for the "all spaces" sweep.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PerSpaceWindows {
    pub display_identifier: DisplayId,
    pub space: SpaceRecord,
    pub windows: Vec<WindowInfo>,
}

/* ---------------------------------- Wrapper ---------------------------------- */
pub struct Lightsky {
    syms: SkylightSymbols,
    conn: SLSConnectionID,
}

impl Lightsky {
    pub fn new() -> Result<Self> {
        // All dlsym happens inside lightsky-sys
        let syms = SkylightSymbols::load()?;
        let conn = unsafe { (syms.SLSMainConnectionID)() };
        Ok(Self { syms, conn })
    }

    /* ----------------------------- Space management ---------------------------- */

    pub fn current_space(&self) -> Option<SpaceId> {
        unsafe {
            if let Some(copy_active) = self.syms.SLSCopyActiveSpace {
                let sid = copy_active(self.conn);
                if sid != 0 {
                    return Some(SpaceId(sid));
                }
            }
            None
        }
    }

    /// Get the type of a Space (user/system/fullscreen).
    pub fn space_type(&self, sid: SpaceId) -> SpaceType {
        let t = unsafe { (self.syms.SLSSpaceGetType)(self.conn, sid.0) };
        match t {
            0 => SpaceType::User,
            2 => SpaceType::System,
            4 => SpaceType::Fullscreen,
            x => SpaceType::Other(x),
        }
    }

    /// Get the display UUID string for a Space (useful to map to a display id via ColorSync/CoreGraphics elsewhere).
    pub fn display_uuid_for_space(&self, sid: SpaceId) -> Option<String> {
        unsafe {
            let s = (self.syms.SLSCopyManagedDisplayForSpace)(self.conn, sid.0);
            if s.is_null() {
                return None;
            }
            // SLS*Copy* → Create rule → wrap_under_create_rule will release on drop
            let cf = CFString::wrap_under_create_rule(s);
            Some(cf.to_string())
        }
    }

    /// Discover all Spaces grouped by display via CGSCopyManagedDisplaySpaces.
    /// Returns one entry per display: identifier (if present), current space on that display,
    /// and a list of spaces with their types and whether they are current.
    pub fn list_all_spaces(&self) -> Result<Vec<DisplaySpaces>> {
        unsafe {
            let Some(copy_spaces) = self.syms.CGSCopyManagedDisplaySpaces else {
                return Err(anyhow!(
                    "CGSCopyManagedDisplaySpaces not available on this macOS"
                ));
            };

            // SkyLight returns a retained object (typically CFArray of display dicts).
            let plist: CFTypeRef = copy_spaces(self.conn);
            if plist.is_null() {
                return Err(anyhow!("CGSCopyManagedDisplaySpaces returned null"));
            }

            // Treat top-level as CFArrayRef
            let displays_arr = plist as CFArrayRef;
            let dcount = CFArrayGetCount(displays_arr);

            // Common keys (best-effort across macOS versions)
            let k_display_id = CFString::new("Display Identifier");
            let k_spaces = CFString::new("Spaces");
            let k_current = CFString::new("Current Space");
            let k_id64 = CFString::new("id64");

            fn dict_get(the_dict: CFDictionaryRef, key: &CFString) -> CFTypeRef {
                unsafe {
                    CFDictionaryGetValue(the_dict, key.as_concrete_TypeRef() as *const c_void)
                        as CFTypeRef
                }
            }

            fn num_to_i64(n: CFNumberRef) -> Option<i64> {
                let mut out: i64 = 0;
                let ok = unsafe {
                    CFNumberGetValue(n, kCFNumberSInt64Type, &mut out as *mut i64 as *mut c_void)
                };
                if ok { Some(out) } else { None }
            }

            let mut out = Vec::new();

            for i in 0..dcount {
                // Each element should be a CFDictionaryRef (display dictionary)
                let dv = CFArrayGetValueAtIndex(displays_arr, i) as CFTypeRef;
                if dv.is_null() {
                    continue;
                }
                let disp_dict = dv as CFDictionaryRef;

                // Display identifier (optional string)
                let display_identifier = {
                    let v = dict_get(disp_dict, &k_display_id);
                    if !v.is_null() {
                        // v is CFStringRef
                        let s = CFString::wrap_under_get_rule(v as _);
                        Some(s.to_string())
                    } else {
                        None
                    }
                };

                // Current Space id64 (optional)
                let current_space_id = {
                    let cur = dict_get(disp_dict, &k_current) as CFDictionaryRef;
                    if !cur.is_null() {
                        let id = dict_get(cur, &k_id64) as CFNumberRef;
                        if !id.is_null() {
                            num_to_i64(id).map(|v| SpaceId(v as u64))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                };

                // Spaces array → list SpaceRecord
                let mut spaces_vec: Vec<SpaceRecord> = Vec::new();
                let spaces_val = dict_get(disp_dict, &k_spaces) as CFArrayRef;
                if !spaces_val.is_null() {
                    let scount = CFArrayGetCount(spaces_val);
                    for j in 0..scount {
                        let sv = CFArrayGetValueAtIndex(spaces_val, j) as CFDictionaryRef;
                        if sv.is_null() {
                            continue;
                        }
                        let idnum = dict_get(sv, &k_id64) as CFNumberRef;
                        if idnum.is_null() {
                            continue;
                        }
                        if let Some(id64) = num_to_i64(idnum) {
                            let sid = SpaceId(id64 as u64);
                            let kind = self.space_type(sid);
                            let is_cur = current_space_id.map(|c| c == sid).unwrap_or(false);
                            spaces_vec.push(SpaceRecord {
                                id: sid,
                                kind,
                                is_current_on_display: is_cur,
                            });
                        }
                    }
                }

                out.push(DisplaySpaces {
                    display_identifier: display_identifier
                        .map(DisplayId)
                        .expect("at least one display should have an ID"),
                    current: current_space_id
                        .expect("at least one display should have a current space"),
                    spaces: spaces_vec,
                });
            }

            // Release the top-level object we got from SkyLight
            CFRelease(plist);

            Ok(out)
        }
    }

    /* ------------------------- Window queries (modular) ------------------------ */

    /// Core worker: query windows **in a single Space**.
    /// Populates `space_id` on each `WindowInfo` and filters by `kinds`.
    pub fn get_windows_in_space(
        &self,
        space: SpaceId,
        options: WindowListOptions,
        kinds: WindowKindFilter,
    ) -> Result<Vec<WindowInfo>> {
        unsafe {
            // Build CFArray<CFNumber(SInt64)> with ONE entry (this space)
            let num = CFNumber::from(space.0 as i64);
            let mut raw: [*const c_void; 1] = [num.as_concrete_TypeRef() as *const c_void];

            let cf_spaces = CFArrayCreate(
                ptr::null(),
                raw.as_mut_ptr(),
                1isize,
                &kCFTypeArrayCallBacks,
            );

            let mut set_tags: u64 = 0;
            let mut clear_tags: u64 = 0;

            let list = (self.syms.SLSCopyWindowsWithOptionsAndTags)(
                self.conn,
                0, // cid filter: 0 = all clients
                cf_spaces,
                options.bits(),
                &mut set_tags,
                &mut clear_tags,
            );
            CFRelease(cf_spaces as CFTypeRef);

            if list.is_null() {
                return Ok(vec![]);
            }

            let count = CFArrayGetCount(list) as i32;
            let query = (self.syms.SLSWindowQueryWindows)(self.conn, list, count);
            if query.is_null() {
                CFRelease(list as CFTypeRef);
                return Err(anyhow!("SLSWindowQueryWindows returned null"));
            }
            let iter = (self.syms.SLSWindowQueryResultCopyWindows)(query);
            if iter.is_null() {
                CFRelease(query);
                CFRelease(list as CFTypeRef);
                return Err(anyhow!("SLSWindowQueryResultCopyWindows returned null"));
            }

            // Empty filter means "ALL"
            let kinds = if kinds.is_empty() {
                WindowKindFilter::ALL
            } else {
                kinds
            };

            let mut out = Vec::new();
            while (self.syms.SLSWindowIteratorAdvance)(iter) {
                let wid = (self.syms.SLSWindowIteratorGetWindowID)(iter);
                let par = (self.syms.SLSWindowIteratorGetParentID)(iter);
                let lvl = (self.syms.SLSWindowIteratorGetLevel)(iter);
                let tag = (self.syms.SLSWindowIteratorGetTags)(iter);
                let att = (self.syms.SLSWindowIteratorGetAttributes)(iter);

                let info = WindowInfo {
                    window_id: WindowId(wid),
                    parent_window_id: par,
                    level: lvl,
                    tags: tag,
                    attributes: att,
                    space_id: space,
                };

                // New: mask-based classification. A window can belong to multiple buckets.
                let mask = classify_window_mask(&info);
                if !(mask & kinds).is_empty() {
                    out.push(info);
                }
            }

            CFRelease(iter);
            CFRelease(query);
            CFRelease(list as CFTypeRef);

            Ok(out)
        }
    }

    /// Best-effort: which Space IDs currently contain this window.
    /// Uses SLSCopySpacesForWindows if available; otherwise returns empty (unknown).
    pub fn spaces_for_window(&self, wid: WindowId) -> Result<Vec<SpaceId>> {
        unsafe {
            let Some(copy_spaces_for_windows) = self.syms.SLSCopySpacesForWindows else {
                return Ok(Vec::new());
            };

            // Build CFArray<CFNumber> of a single window id (as SInt64)
            let num = CFNumber::from(wid.0 as i32);
            let mut raw: [*const c_void; 1] = [num.as_concrete_TypeRef() as *const c_void];
            let cf_wins = CFArrayCreate(
                ptr::null(),
                raw.as_mut_ptr(),
                1isize,
                &kCFTypeArrayCallBacks,
            );

            let cf = copy_spaces_for_windows(self.conn, cf_wins, 1);
            CFRelease(cf_wins as CFTypeRef);

            if cf.is_null() {
                return Ok(Vec::new());
            }

            // Result is CFArray of CFNumber(id64)
            let arr = cf as CFArrayRef;
            let count = CFArrayGetCount(arr);
            let mut out = Vec::with_capacity(count as usize);
            for i in 0..count {
                let v = CFArrayGetValueAtIndex(arr, i) as CFTypeRef;
                if !v.is_null() {
                    let n = v as CFNumberRef;
                    let mut id64: i64 = 0;
                    let ok = CFNumberGetValue(
                        n,
                        kCFNumberSInt64Type,
                        &mut id64 as *mut i64 as *mut c_void,
                    );
                    if ok {
                        out.push(SpaceId(id64 as u64));
                    }
                }
            }
            CFRelease(cf);
            Ok(out)
        }
    }

    /// Move a specific window to a target Space.
    ///
    /// If `from` is `Some(space)`, we remove from that space only.
    /// If `from` is `None`, we attempt to discover all existing spaces for the window
    /// and remove it from all of them except `to`.
    pub fn move_window_to_space(&self, window: WindowId, from: SpaceId, to: SpaceId) -> Result<()> {
        unsafe {
            // CFArray[window_id as SInt32]
            let wid32: i32 = window.0 as i32;
            let win_num = CFNumber::from(wid32); // SInt32
            let mut wraw: [*const c_void; 1] = [win_num.as_concrete_TypeRef() as *const c_void];
            let cf_windows: CFArrayRef = CFArrayCreate(
                ptr::null(),
                wraw.as_mut_ptr(),
                1isize,
                &kCFTypeArrayCallBacks,
            );

            // CFArray[to as SInt64]  (spaces are 64-bit ids)
            let to_num = CFNumber::from(to.0 as i64);
            let mut sraw_to: [*const c_void; 1] = [to_num.as_concrete_TypeRef() as *const c_void];
            let cf_space_to: CFArrayRef = CFArrayCreate(
                ptr::null(),
                sraw_to.as_mut_ptr(),
                1isize,
                &kCFTypeArrayCallBacks,
            );

            let from_num = CFNumber::from(from.0 as i64);
            let mut sraw_from: [*const c_void; 1] =
                [from_num.as_concrete_TypeRef() as *const c_void];
            let cf_space_from: CFArrayRef = CFArrayCreate(
                ptr::null(),
                sraw_from.as_mut_ptr(),
                1isize,
                &kCFTypeArrayCallBacks,
            );

            // 1) Preferred atomic API if present
            if let Some(f) = self.syms.SLSSpaceAddWindowsAndRemoveFromSpaces {
                log::info!("Using SLSSpaceAddWindowsAndRemoveFromSpaces");
                f(
                    self.conn,
                    cf_windows,
                    cf_space_to,
                    if from != to {
                        cf_space_from
                    } else {
                        ptr::null() as CFArrayRef
                    },
                );
            }

            // Optional nudge (some apps surface this in older macOS)
            if let Some(show_spaces) = self.syms.SLSShowSpaces {
                log::info!("Nudging with SLSShowSpaces");
                show_spaces(self.conn, cf_space_to);
            }

            CFRelease(cf_space_from as CFTypeRef);
            CFRelease(cf_space_to as CFTypeRef);
            CFRelease(cf_windows as CFTypeRef);
        }

        // Verify – if we can read memberships, ensure the window is now on `to`
        let now = self.spaces_for_window(window).unwrap_or_default();
        if !now.contains(&to) {
            return Err(anyhow!(
                "move_window_to_space: window {} still not on Space {} (now in {:?})",
                window.0,
                to.0,
                now
            ));
        }
        Ok(())
    }

    pub fn change_space_focus(&self, display: String, space: SpaceId, to: SpaceId) -> Result<()> {
        // show the target space first

        unsafe {
            let show = CFArrayCreate(
                ptr::null(),
                [CFNumber::from(to.0 as i64).as_concrete_TypeRef() as *const c_void].as_mut_ptr(),
                1isize,
                &kCFTypeArrayCallBacks,
            );

            let hide = CFArrayCreate(
                ptr::null(),
                [CFNumber::from(space.0 as i64).as_concrete_TypeRef() as *const c_void]
                    .as_mut_ptr(),
                1isize,
                &kCFTypeArrayCallBacks,
            );

            // if let Some(show_spaces) = self.syms.SLSShowSpaces {
            //     log::info!("Showing target space {}", to.0);
            //     show_spaces(self.conn, show);
            // }

            // if let Some(hide_spaces) = self.syms.SLSHideSpaces {
            //     log::info!("Hiding old space {}", space.0);
            //     hide_spaces(self.conn, hide);
            // }

            log::info!("Setting display {} current space to {}", display, space.0);

            let disp_cf = CFString::new(&display);
            let res = (self.syms.SLSManagedDisplaySetCurrentSpace)(
                self.conn,
                disp_cf.as_concrete_TypeRef(),
                to.0,
            );
            if res != 0 {
                return Err(anyhow!(
                    "SLSManagedDisplaySetCurrentSpace failed with code {}",
                    res
                ));
            }
            CFRelease(show as _);
            CFRelease(hide as _);
        }

        Ok(())
    }

    /// Filter + annotate with PID/owner/title (single space).
    pub fn get_windows_in_space_with_titles(
        &self,
        space: SpaceId,
        options: WindowListOptions,
        kinds: WindowKindFilter,
    ) -> Result<Vec<Window>> {
        let wins = self.get_windows_in_space(space, options, kinds)?;
        let cg = build_cg_index();

        let mut out = Vec::with_capacity(wins.len());
        for info in wins {
            let pid_owner_title = cg.get(&(info.window_id.0)).cloned();
            let (pid, owner_name, title) = pid_owner_title.unwrap_or((None, None, None));
            out.push(Window {
                info,
                pid,
                owner_name,
                title,
            });
        }
        Ok(out)
    }

    /// Sweep **all spaces**: calls `list_all_spaces()` first, then queries each space,
    /// applying the same `options` and `kinds` to each. Returns windows grouped by space.
    pub fn get_windows_in_spaces(
        &self,
        options: WindowListOptions,
        kinds: WindowKindFilter,
    ) -> Result<Vec<PerSpaceWindows>> {
        let displays = self.list_all_spaces()?;
        let mut out = Vec::new();

        for disp in displays.iter() {
            let disp_id = disp.display_identifier.clone();
            for space_rec in disp.spaces.iter().cloned() {
                let windows = self.get_windows_in_space(space_rec.id, options, kinds)?;
                out.push(PerSpaceWindows {
                    display_identifier: disp_id.clone(),
                    space: space_rec,
                    windows,
                });
            }
        }

        Ok(out)
    }
}

/* --------------------------- Window classification -------------------------- */

/// Return a bitmask of all kinds this window plausibly belongs to.
/// Notes:
/// - Do **not** require TAG_ON_ACTIVE_SPACE or "eligible" – membership is enforced by the SLS query.
/// - Off-current-space windows can look “minimized” in tag space on some OS builds.
///   We therefore *add* MINIMIZED when those bits are set, but still also classify as APP/UTILITY
///   based on level/parent/titlebar heuristics so APP-only filters still find them.
fn classify_window_mask(w: &WindowInfo) -> WindowKindFilter {
    let mut mask = WindowKindFilter::empty();

    let tags = w.tags;
    let attrs = w.attributes;

    // Minimized?
    if (tags & TAG_MINIMIZED_1) != 0 || (tags & TAG_MINIMIZED_2) != 0 {
        mask |= WindowKindFilter::MINIMIZED;
    }

    let top_level = w.parent_window_id == 0;
    let standardish = (attrs & 0x2) != 0 || (tags & TAG_HAS_TITLEBAR_LIKE) != 0;

    if top_level {
        if w.level >= 8 {
            // Fullscreen-style layers
            mask |= WindowKindFilter::FULLSCREEN;
        } else if w.level == 3 {
            // Utility/panel
            mask |= WindowKindFilter::UTILITY;
        } else if w.level == 0 && standardish {
            // Normal app windows only if they look "standard"
            mask |= WindowKindFilter::APP;
        } else {
            mask |= WindowKindFilter::OTHER;
        }
    } else {
        mask |= WindowKindFilter::OTHER;
    }

    if mask.is_empty() {
        mask |= WindowKindFilter::OTHER;
    }
    mask
}

/* ------------------------------ CG helpers (CGS) ------------------------------ */

type CGIndexMap = HashMap<u32, (Option<i32>, Option<String>, Option<String>)>;

fn build_cg_index() -> CGIndexMap {
    let mut map = HashMap::new();

    unsafe {
        // 0 == kCGWindowListOptionAll, 0 == kCGNullWindowID
        let arr: CFArrayRef = CGWindowListCopyWindowInfo(0, 0);
        if arr.is_null() {
            return map;
        }

        let count = CFArrayGetCount(arr);

        // CFString keys used in the window dictionaries
        let k_num = CFString::new("kCGWindowNumber");
        let k_owner_pid = CFString::new("kCGWindowOwnerPID");
        let k_owner_name = CFString::new("kCGWindowOwnerName");
        let k_name = CFString::new("kCGWindowName");

        for i in 0..count {
            let dict_ref = CFArrayGetValueAtIndex(arr, i) as CFDictionaryRef;
            if dict_ref.is_null() {
                continue;
            }

            let win_id = dict_get_i64(dict_ref, &k_num).map(|v| v as u32);
            let pid = dict_get_i64(dict_ref, &k_owner_pid).map(|v| v as i32);
            let owner = dict_get_string(dict_ref, &k_owner_name);
            let title = dict_get_string(dict_ref, &k_name);

            if let Some(wid) = win_id {
                map.insert(wid, (pid, owner, title));
            }
        }

        CFRelease(arr as CFTypeRef);
    }

    map
}

#[inline]
fn dict_get_i64(dict: CFDictionaryRef, key: &CFString) -> Option<i64> {
    unsafe {
        let v: CFTypeRef =
            CFDictionaryGetValue(dict, key.as_concrete_TypeRef() as *const c_void) as CFTypeRef;
        if v.is_null() {
            return None;
        }
        let n: CFNumberRef = v as CFNumberRef;
        let mut out: i64 = 0;
        // Using SInt64 works for 32-bit values too; CF will convert if representable.
        let ok = CFNumberGetValue(n, kCFNumberSInt64Type, &mut out as *mut i64 as *mut c_void);
        if ok { Some(out) } else { None }
    }
}

#[inline]
fn dict_get_string(dict: CFDictionaryRef, key: &CFString) -> Option<String> {
    unsafe {
        let v: CFTypeRef =
            CFDictionaryGetValue(dict, key.as_concrete_TypeRef() as *const c_void) as CFTypeRef;
        if v.is_null() {
            return None;
        }
        let s = CFString::wrap_under_get_rule(v as _);
        Some(s.to_string())
    }
}
