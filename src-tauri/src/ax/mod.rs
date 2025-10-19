mod mission_control_emu;

use crate::application_tree::{ApplicationTreeActor, SearchParam, SearchResult};
use crate::display::DisplayManagerActor;
use crate::focus::FocusManagerActor;
use anyhow::Result;
use kameo::prelude::Message;
use kameo::{actor::ActorRef, Actor};
use lightsky::{DisplayId, Lightsky, SpaceId, WindowId};
use mission_control_emu::MissionControlEmu;
use serde::{Deserialize, Serialize};
use std::thread;

#[derive(Actor)]
pub struct AXActor {
    ax: AX,
}

impl AXActor {
    pub fn new(
        display_manager_ref: ActorRef<DisplayManagerActor>,
        focus_manager_ref: ActorRef<FocusManagerActor>,
        application_tree_ref: ActorRef<ApplicationTreeActor>,
    ) -> Self {
        let ax = AX::new(display_manager_ref, focus_manager_ref, application_tree_ref);
        Self { ax }
    }
}

#[derive(Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
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
    pub lightsky: Lightsky,
    pub application_tree_ref: ActorRef<ApplicationTreeActor>,
    display_manager_ref: ActorRef<DisplayManagerActor>,
    mc: MissionControlEmu,
    focus_manager_ref: ActorRef<FocusManagerActor>,
}

impl AX {
    pub fn new(
        display_manager_ref: ActorRef<DisplayManagerActor>,
        focus_manager_ref: ActorRef<FocusManagerActor>,
        application_tree_ref: ActorRef<ApplicationTreeActor>,
    ) -> Self {
        let lightsky = Lightsky::new().expect("Failed to initialize Lightsky");
        let mc = MissionControlEmu::new();
        Self {
            lightsky,
            application_tree_ref,
            display_manager_ref,
            focus_manager_ref,
            mc,
        }
    }

    pub async fn current_display_space(&self) -> DisplaySpace {
        let display_id = self
            .display_manager_ref
            .ask(crate::display::ActiveDisplayRequest)
            .await
            .unwrap()
            .unwrap_or_else(|| {
                panic!("Failed to get active display ID");
            });

        let space_id = self.lightsky.current_space();

        DisplaySpace {
            display_id,
            space_id,
        }
    }

    pub async fn refresh(&mut self) {
        let _ = self
            .application_tree_ref
            .ask(crate::application_tree::RefreshTree)
            .await;
    }

    #[allow(dead_code)]
    pub async fn focus_display(&self, display_id: &DisplayId) -> Option<()> {
        self.display_manager_ref
            .ask(crate::display::FocusDisplayCenter {
                display_id: display_id.clone(),
            })
            .await
            .unwrap()
    }

    pub async fn focus_space(&self, space_id: SpaceId) -> Option<()> {
        let target_display_id = self
            .application_tree_ref
            .ask(crate::application_tree::FindDisplayFromSpace { space_id })
            .await
            .unwrap()?;

        let target_space_index = self
            .application_tree_ref
            .ask(crate::application_tree::FindSpaceIndex { space_id })
            .await
            .unwrap()?;

        let current_display_space = self.current_display_space().await;

        if target_display_id != current_display_space.display_id {
            log::info!(
                "Switch display: {} -> {}",
                current_display_space.display_id,
                target_display_id
            );
            let _ = self
                .display_manager_ref
                .ask(crate::display::FocusDisplayCenter {
                    display_id: target_display_id.clone(),
                });
            thread::sleep(std::time::Duration::from_millis(40));
        }

        let current_space_index = self
            .application_tree_ref
            .ask(crate::application_tree::FindSpaceIndex {
                space_id: current_display_space.space_id,
            })
            .await
            .unwrap()?;

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

    pub async fn try_focus_app(&mut self, app_name: &str) {
        log::info!("Trying to focus app: {}", app_name);
        if let Some(res) = self
            .application_tree_ref
            .ask(SearchParam::ByName(app_name.to_string()))
            .await
            .unwrap()
            .first()
        {
            let SearchResult {
                pid,
                window_id,
                space_id,
                ..
            } = res;
            let _ = self.focus_space(*space_id).await;
            let _ = self
                .focus_manager_ref
                .ask(crate::focus::FocusWindow {
                    pid: *pid,
                    window_id: Some(*window_id),
                })
                .await;
        }
    }

    pub async fn focus_window(&mut self, window_id: WindowId) {
        log::info!("Focusing window_id: {}", window_id);
        if let Some(res) = self
            .application_tree_ref
            .ask(SearchParam::ByWindowId(window_id))
            .await
            .unwrap()
            .first()
            .cloned()
        {
            let SearchResult {
                pid,
                window_id,
                space_id,
                title,
                app_name,
                ..
            } = res;

            log::info!(
                "Focusing window: pid={}, window_id={}, space_id={}, app_name={}, title={:?}",
                pid,
                window_id,
                space_id,
                app_name,
                title
            );
            let _ = self.focus_space(space_id).await;
            let _ = self
                .focus_manager_ref
                .ask(crate::focus::FocusWindow {
                    pid,
                    window_id: Some(window_id),
                })
                .await;
        }

        self.refresh().await;
    }
}

pub struct RefreshAX;

impl Message<RefreshAX> for AXActor {
    type Reply = ();

    async fn handle(
        &mut self,
        _msg: RefreshAX,
        _ctx: &mut kameo::prelude::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.ax.refresh().await;
    }
}

pub struct TryFocusApp {
    pub app_name: String,
}

impl Message<TryFocusApp> for AXActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: TryFocusApp,
        _ctx: &mut kameo::prelude::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.ax.try_focus_app(&msg.app_name).await;
    }
}

pub struct FocusWindow {
    pub window_id: WindowId,
}

impl Message<FocusWindow> for AXActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: FocusWindow,
        _ctx: &mut kameo::prelude::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.ax.focus_window(msg.window_id).await;
    }
}

pub struct CurrentDisplaySpace;

impl Message<CurrentDisplaySpace> for AXActor {
    type Reply = Result<DisplaySpace>;

    async fn handle(
        &mut self,
        _msg: CurrentDisplaySpace,
        _ctx: &mut kameo::prelude::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        Ok(self.ax.current_display_space().await)
    }
}
