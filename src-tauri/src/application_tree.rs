use core_foundation::array::CFArrayRef;
use core_foundation::number::CFNumberRef;
use core_graphics::display::CFDictionaryRef;
use core_graphics::window::{
    kCGNullWindowID, kCGWindowListOptionOnScreenOnly, CGWindowListCopyWindowInfo,
};
use kameo::prelude::Message;
use kameo::Actor;
use lightsky::{DisplayId, Lightsky, SpaceId, WindowId};

use core_foundation::base::{CFTypeRef, TCFType};
use core_foundation::string::CFString;

#[derive(Actor)]
pub struct ApplicationTreeActor {
    tree: ApplicationTree,
    ls: Lightsky,
}

impl ApplicationTreeActor {
    pub fn new(ls: Lightsky) -> Self {
        let tree = ApplicationTree::new(&ls);
        Self { tree, ls }
    }
}

impl Message<SearchParam> for ApplicationTreeActor {
    type Reply = Vec<SearchResult>;

    async fn handle(
        &mut self,
        param: SearchParam,
        _ctx: &mut kameo::prelude::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.tree.search(param)
    }
}

pub struct RefreshTree;

impl Message<RefreshTree> for ApplicationTreeActor {
    type Reply = ();

    async fn handle(
        &mut self,
        _msg: RefreshTree,
        _ctx: &mut kameo::prelude::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.tree = ApplicationTree::new(&self.ls);
    }
}

pub struct FindDisplayFromSpace {
    pub space_id: SpaceId,
}

impl Message<FindDisplayFromSpace> for ApplicationTreeActor {
    type Reply = Option<DisplayId>;

    async fn handle(
        &mut self,
        msg: FindDisplayFromSpace,
        _ctx: &mut kameo::prelude::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.tree.find_display_from_space(msg.space_id)
    }
}

pub struct FindSpaceIndex {
    pub space_id: SpaceId,
}

impl Message<FindSpaceIndex> for ApplicationTreeActor {
    type Reply = Option<usize>;

    async fn handle(
        &mut self,
        msg: FindSpaceIndex,
        _ctx: &mut kameo::prelude::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.tree.find_space_index(msg.space_id)
    }
}

pub struct ApplicationTree {
    pub displays: Vec<DisplayNode>,
}

impl std::fmt::Display for ApplicationTree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for display in &self.displays {
            writeln!(f, "Display ID: {}", display.id)?;
            for space in &display.spaces {
                writeln!(f, "  Space ID: {}", space.id)?;
                for window in &space.windows {
                    writeln!(
                        f,
                        "    Window ID: {}, PID: {}, App: {}, Title: {}, Focused: {}",
                        window.id,
                        window.pid,
                        window.app_name,
                        window.title.as_deref().unwrap_or("<No Title>"),
                        window.is_focused
                    )?;
                }
            }
        }
        Ok(())
    }
}

pub struct DisplayNode {
    pub id: DisplayId,
    pub spaces: Vec<SpaceNode>,
}

pub struct SpaceNode {
    pub id: SpaceId,
    pub index: usize,
    pub windows: Vec<WindowNode>,
}

pub struct WindowNode {
    pub id: WindowId,
    pub title: Option<String>,
    pub pid: i32,
    pub app_name: String,
    pub is_focused: bool,
}

#[derive(Clone)]
#[allow(dead_code)]
pub struct SearchResult {
    pub display_id: DisplayId,
    pub space_id: SpaceId,
    pub space_index: usize,
    pub window_id: WindowId,
    pub title: Option<String>,
    pub pid: i32,
    pub app_name: String,
    pub is_focused: bool,
}

impl std::fmt::Display for SearchResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Display ID: {}, Space ID: {}, Window ID: {}, PID: {}, App: {}, Title: {}, Focused: {}",
            self.display_id,
            self.space_id,
            self.window_id,
            self.pid,
            self.app_name,
            self.title.as_deref().unwrap_or("<No Title>"),
            self.is_focused
        )
    }
}

#[allow(dead_code)]
pub enum SearchParam {
    All,
    ByPid(i32),
    ByWindowId(WindowId),
    BySpaceId(SpaceId),
    ByDisplayId(DisplayId),
    ByName(String),
    Focused,
}

impl ApplicationTree {
    pub fn new(ls: &Lightsky) -> ApplicationTree {
        build_application_tree(ls)
    }

    fn search_by_pid(&self, pid: i32) -> Vec<SearchResult> {
        self.flatten()
            .into_iter()
            .filter(|res| res.pid == pid)
            .collect()
    }

    fn search_by_window_id(&self, window_id: WindowId) -> Vec<SearchResult> {
        self.flatten()
            .into_iter()
            .filter(|res| res.window_id == window_id)
            .collect()
    }

    fn search_by_space_id(&self, space_id: SpaceId) -> Vec<SearchResult> {
        self.flatten()
            .into_iter()
            .filter(|res| res.space_id == space_id)
            .collect()
    }

    fn search_by_display_id(&self, display_id: DisplayId) -> Vec<SearchResult> {
        self.flatten()
            .into_iter()
            .filter(|res| res.display_id == display_id)
            .collect()
    }

    pub fn find_display_from_space(&self, space_id: SpaceId) -> Option<DisplayId> {
        for display in &self.displays {
            for space in &display.spaces {
                if space.id == space_id {
                    return Some(display.id.clone());
                }
            }
        }
        None
    }

    pub fn find_space_index(&self, space_id: SpaceId) -> Option<usize> {
        for display in &self.displays {
            for space in &display.spaces {
                if space.id == space_id {
                    return Some(space.index);
                }
            }
        }
        None
    }

    pub fn flatten(&self) -> Vec<SearchResult> {
        let mut results = Vec::new();
        for display in &self.displays {
            for space in &display.spaces {
                for window in &space.windows {
                    results.push(SearchResult {
                        display_id: display.id.clone(),
                        space_id: space.id,
                        window_id: window.id,
                        title: window.title.clone(),
                        pid: window.pid,
                        app_name: window.app_name.clone(),
                        is_focused: window.is_focused,
                        space_index: space.index,
                    });
                }
            }
        }
        results
    }

    fn search_by_focused(&self) -> Vec<SearchResult> {
        self.flatten()
            .into_iter()
            .filter(|res| res.is_focused)
            .collect()
    }

    pub fn search(&self, param: SearchParam) -> Vec<SearchResult> {
        match param {
            SearchParam::All => self.flatten(),
            SearchParam::ByPid(pid) => self.search_by_pid(pid),
            SearchParam::ByWindowId(window_id) => self.search_by_window_id(window_id),
            SearchParam::BySpaceId(space_id) => self.search_by_space_id(space_id),
            SearchParam::ByDisplayId(display_id) => self.search_by_display_id(display_id),
            SearchParam::Focused => self.search_by_focused(),
            SearchParam::ByName(name) => self
                .flatten()
                .into_iter()
                .filter(|res| res.app_name == name)
                .collect(),
        }
    }
}

extern "C" {
    fn CFArrayGetCount(theArray: CFArrayRef) -> isize;
    fn CFArrayGetValueAtIndex(theArray: CFArrayRef, idx: isize) -> *const std::ffi::c_void;
}

pub fn focused_window_id() -> Option<WindowId> {
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
            *core_foundation::dictionary::CFDictionary::wrap_under_get_rule(dict_ref).find(&key)?;

        let num_ref: CFNumberRef = value as CFNumberRef;
        let num = core_foundation::number::CFNumber::wrap_under_get_rule(num_ref);

        num.to_i64().map(|n| WindowId(n as u32))
    }
}

pub fn build_application_tree(ls: &Lightsky) -> ApplicationTree {
    let focused_window_id = focused_window_id();
    let all = ls.list_all_spaces().unwrap_or_default();
    let mut display_nodes = Vec::new();
    for display in all {
        let mut space_nodes = Vec::new();
        for space in display.spaces {
            let mut window_nodes = Vec::new();
            let windows = ls
                .get_windows_in_space_with_titles(
                    space.id,
                    lightsky::WindowListOptions::all(),
                    lightsky::WindowKindFilter::APP,
                )
                .unwrap_or_default();
            for window in windows {
                window_nodes.push(WindowNode {
                    id: window.info.window_id,
                    title: window.title,
                    pid: window.pid,
                    app_name: window.owner_name.unwrap_or_default(),
                    is_focused: Some(window.info.window_id) == focused_window_id,
                });
            }
            space_nodes.push(SpaceNode {
                id: space.id,
                index: space.index,
                windows: window_nodes,
            });
        }
        display_nodes.push(DisplayNode {
            id: display.display_identifier,
            spaces: space_nodes,
        });
    }

    ApplicationTree {
        displays: display_nodes,
    }
}
