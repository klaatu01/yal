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

use std::{ffi::c_void, ptr};

// Private SkyLight heuristics (observed; may vary by macOS)
const TAG_ON_ACTIVE_SPACE: u64 = 0x1;
const TAG_VISIBLE_ON_ALL_SPACES_AND_ELIGIBLE: u64 = 0x2;
const TAG_HAS_TITLEBAR_LIKE: u64 = 0x0400_0000_0000_0000;
const TAG_MINIMIZED_1: u64 = 0x1000_0000_0000_0000;
const TAG_MINIMIZED_2: u64 = 0x0300_0000_0000_0000;
const TAG_ELIGIBLE_BIT: u64 = 0x8000_0000;

// ---------- Public types ----------
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpaceId(pub u64);

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
    pub display_identifier: Option<String>,
    pub current: Option<SpaceId>,
    pub spaces: Vec<SpaceRecord>,
}

impl std::fmt::Display for DisplaySpaces {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(id) = &self.display_identifier {
            write!(f, "Display: {}", id)
        } else {
            write!(f, "Display: <unknown>")
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpaceRecord {
    pub id: SpaceId,
    pub kind: SpaceType,
    pub is_current_on_display: bool,
}

bitflags! {
    /// Private API bits; vary by macOS.
    pub struct WindowListOptions: u32 {
        /// “Visible-ish” windows (typical)
        const VISIBLE = 0x2;
        /// Broader set: often includes minimized/off-space windows
        const INCLUDE_MINIMIZED = 0x7;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowInfo {
    pub window_id: u32,
    pub parent_window_id: u32,
    pub level: i32,
    pub tags: u64,
    pub attributes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Window {
    pub info: WindowInfo,
    pub pid: Option<i32>,
    pub owner_name: Option<String>,
    pub title: Option<String>,
}

// ---------- Wrapper ----------
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

    pub fn windows_in_spaces_app_only_with_titles(
        &self,
        spaces: &[SpaceId],
        options: WindowListOptions,
    ) -> Result<Vec<Window>> {
        let wins = self.windows_in_spaces_app_only(spaces, options)?;
        let cg = build_cg_index();

        let mut out = Vec::with_capacity(wins.len());
        for info in wins {
            let pid_owner_title = cg.get(&info.window_id).cloned();
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

    /// Query windows for the given Space IDs.
    /// Note: title/pid/etc. come from AX/CG; this returns WindowServer facts (ids, tags, levels).
    pub fn windows_in_spaces(
        &self,
        spaces: &[SpaceId],
        options: WindowListOptions,
    ) -> Result<Vec<WindowInfo>> {
        unsafe {
            // Build CFArray<CFNumber(SInt64)>
            let nums: Vec<CFNumber> = spaces.iter().map(|s| CFNumber::from(s.0 as i64)).collect();
            let mut raw: Vec<*const c_void> = nums
                .iter()
                .map(|n| n.as_concrete_TypeRef() as *const c_void)
                .collect();

            let cf_spaces = CFArrayCreate(
                ptr::null(),
                raw.as_mut_ptr(),
                raw.len() as isize,
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

            let mut out = Vec::new();
            while (self.syms.SLSWindowIteratorAdvance)(iter) {
                let wid = (self.syms.SLSWindowIteratorGetWindowID)(iter);
                let par = (self.syms.SLSWindowIteratorGetParentID)(iter);
                let lvl = (self.syms.SLSWindowIteratorGetLevel)(iter);
                let tag = (self.syms.SLSWindowIteratorGetTags)(iter);
                let att = (self.syms.SLSWindowIteratorGetAttributes)(iter);
                out.push(WindowInfo {
                    window_id: wid,
                    parent_window_id: par,
                    level: lvl,
                    tags: tag,
                    attributes: att,
                });
            }

            CFRelease(iter);
            CFRelease(query);
            CFRelease(list as CFTypeRef);

            Ok(out)
        }
    }

    pub fn windows_in_spaces_app_only(
        &self,
        spaces: &[SpaceId],
        options: WindowListOptions,
    ) -> Result<Vec<WindowInfo>> {
        let all = self.windows_in_spaces(spaces, options)?;
        Ok(all
            .into_iter()
            .filter(|w| self.is_application_window(w))
            .collect())
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

    #[inline]
    fn is_application_window(&self, w: &WindowInfo) -> bool {
        // 1) Only top-level windows
        if w.parent_window_id != 0 {
            return false;
        }

        // 2) Accept common “app window” levels.
        //    (Normal = 0, floating/utility often = 3, some fullscreen cases = 8)
        match w.level {
            0 | 3 | 8 => {}
            _ => return false,
        }

        let tags = w.tags;
        let attrs = w.attributes;

        // 3) Must be in an active/eligible state for the Space we asked about
        let on_space_or_globally_visible = (tags & TAG_ON_ACTIVE_SPACE) != 0
            || ((tags & TAG_VISIBLE_ON_ALL_SPACES_AND_ELIGIBLE) != 0
                && (tags & TAG_ELIGIBLE_BIT) != 0);

        if !on_space_or_globally_visible {
            return false;
        }

        // 4) “Looks like” a standard app window:
        //    Either attributes say “standardish” OR a high-bit tag we often see on normal windows.
        let standardish = (attrs & 0x2) != 0 || (tags & TAG_HAS_TITLEBAR_LIKE) != 0;

        if standardish {
            return true;
        }

        // 5) Secondary pattern seen for minimized/alt states (optional, keeps e.g. minimized app windows
        //    if your `options` allowed them). If you *don’t* want minimized ones, comment this out.
        if (attrs == 0 || attrs == 1)
            && ((tags & TAG_MINIMIZED_1) != 0 || (tags & TAG_MINIMIZED_2) != 0)
            && on_space_or_globally_visible
        {
            return true;
        }

        false
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
                    display_identifier,
                    current: current_space_id,
                    spaces: spaces_vec,
                });
            }

            // Release the top-level object we got from SkyLight
            CFRelease(plist);

            Ok(out)
        }
    }
}

fn build_cg_index() -> std::collections::HashMap<u32, (Option<i32>, Option<String>, Option<String>)>
{
    use std::collections::HashMap;
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
