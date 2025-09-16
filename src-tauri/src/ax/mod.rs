mod application_tree;
mod display;
mod focus;
mod mission_control_emu;

use application_tree::{ApplicationTree, SearchParam, SearchResult};
use display::DisplayManager;
use focus::FocusManager;
use lightsky::{DisplayId, Lightsky, SpaceId, WindowId};
use mission_control_emu::MissionControlEmu;
use std::thread;

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct DisplaySpace {
    pub display_id: DisplayId,
    pub space_id: SpaceId,
}

impl std::fmt::Display for DisplaySpace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "CurrentDisplaySpace:")?;
        writeln!(f, "  display_id: {}", self.display_id.0)?;
        writeln!(f, "  space_id: {}", self.space_id)
    }
}

pub struct AX {
    app: tauri::AppHandle,
    pub lightsky: Lightsky,
    pub application_tree: ApplicationTree,
    pub current_display_space: DisplaySpace,
    display: DisplayManager,
    mc: MissionControlEmu,
    focus: FocusManager,
}

impl std::fmt::Display for AX {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "AX:")?;
        write!(f, "{}", self.current_display_space)?;
        write!(f, "{}", self.application_tree)
    }
}

impl AX {
    pub fn new(app: tauri::AppHandle) -> Self {
        let display = DisplayManager::new();
        let mc = MissionControlEmu::new();
        let focus = FocusManager::new();

        let lightsky = Lightsky::new().expect("Failed to initialize Lightsky");
        let application_tree = ApplicationTree::new(&lightsky);

        let current_display = display
            .active_display_id(&app)
            .expect("Failed to get active display ID");
        let current_space = lightsky.current_space();

        Self {
            app,
            lightsky,
            application_tree,
            current_display_space: DisplaySpace {
                display_id: current_display,
                space_id: current_space,
            },
            display,
            mc,
            focus,
        }
    }

    pub fn refresh(&mut self) {
        self.application_tree = ApplicationTree::new(&self.lightsky);
        self.current_display_space = DisplaySpace {
            display_id: self
                .display
                .active_display_id(&self.app)
                .expect("Failed to get active display ID"),
            space_id: self.lightsky.current_space(),
        };
    }

    #[allow(dead_code)]
    pub fn focus_display(&self, display_id: &DisplayId) -> Option<()> {
        self.display.focus_display_center(display_id)
    }

    pub fn focus_space(&self, space_id: SpaceId) -> Option<()> {
        log::info!("Focusing space_id: {}", space_id);

        let target_display_id = self.application_tree.find_display_from_space(space_id)?;
        let target_space_index = self.application_tree.find_space_index(space_id)?;

        if target_display_id != self.current_display_space.display_id {
            log::info!(
                "Switch display: {} -> {}",
                self.current_display_space.display_id,
                target_display_id
            );
            let _ = self.display.focus_display_center(&target_display_id);
            thread::sleep(std::time::Duration::from_millis(40));
        }

        let current_space_index = self
            .application_tree
            .find_space_index(self.current_display_space.space_id)?;

        if target_space_index == current_space_index {
            log::info!("Already on target space");
            return Some(());
        }

        if target_space_index <= 9 {
            log::info!("Press Ctrl+{}", target_space_index + 1);
            let _ = self.mc.press_ctrl_digit(target_space_index + 1);
            thread::sleep(std::time::Duration::from_millis(200));
            return Some(());
        } else {
            // Move to 10th space first, then move left/right as needed
            let _ = self.mc.press_ctrl_digit(10);
            thread::sleep(std::time::Duration::from_millis(250));
            let diff = (target_space_index as isize) - 9;
            if diff > 0 {
                log::info!("Move right {} times", diff);
                for _ in 0..diff {
                    let _ = self.mc.press_ctrl_right();
                }
            } else if diff < 0 {
                log::info!("Move left {} times", -diff);
                for _ in 0..(-diff) {
                    let _ = self.mc.press_ctrl_left();
                }
            }
        }

        Some(())
    }

    pub fn try_focus_app(&mut self, app_name: &str) {
        if let Some(res) = self
            .application_tree
            .search(SearchParam::ByName(app_name.to_string()))
            .first()
        {
            let SearchResult {
                pid,
                window_id,
                space_id,
                ..
            } = res;
            let _ = self.focus_space(*space_id);
            self.focus.focus(&self.app, *pid, Some(*window_id));
        }
    }

    pub fn focus_window(&mut self, window_id: WindowId) {
        if let Some(res) = self
            .application_tree
            .search(SearchParam::ByWindowId(window_id))
            .first()
            .cloned()
        {
            let SearchResult {
                pid,
                window_id,
                space_id,
                ..
            } = res;

            let _ = self.focus_space(space_id);
            self.focus.focus(&self.app, pid, Some(window_id));
        }

        self.refresh();
    }

    pub fn get_focused_window(&self) -> Option<WindowId> {
        self.application_tree
            .search(SearchParam::Focused)
            .first()
            .map(|res| res.window_id)
    }
}
