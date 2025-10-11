use std::thread;

use kameo::{
    actor::ActorRef,
    prelude::{Context, Message},
    Actor,
};
use lightsky::WindowId;
use tauri::{Emitter, Manager};
use tauri_plugin_opener::OpenerExt;
use yal_core::{AppInfo, Command, WindowTarget};

use crate::{
    application_tree,
    ax::{self, AXActor, AX},
    cmd::app::get_app_info,
};

mod app;
pub mod theme;

#[derive(Actor)]
pub struct CommandActor {
    app_handle: tauri::AppHandle,
}

impl Message<Command> for CommandActor {
    type Reply = Result<(), String>;

    async fn handle(&mut self, cmd: Command, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        match cmd {
            Command::App(app_info) => self.run_app_cmd(app_info).await,
            Command::Switch(target) => self.run_switch_cmd(target).await,
            Command::Theme(theme) => self.run_theme_cmd(theme).await,
        }
    }
}

pub struct GetCommands;

impl Message<GetCommands> for CommandActor {
    type Reply = Vec<Command>;

    async fn handle(
        &mut self,
        _msg: GetCommands,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.get_cmds().await
    }
}

pub struct PublishCommands;

impl Message<PublishCommands> for CommandActor {
    type Reply = ();

    async fn handle(
        &mut self,
        _msg: PublishCommands,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let cmds: Vec<_> = self.get_cmds().await;
        let _ = self.app_handle.emit("commands://updated", cmds);
    }
}

impl CommandActor {
    pub fn new(app_handle: tauri::AppHandle) -> Self {
        Self { app_handle }
    }

    async fn run_app_cmd(&self, AppInfo { path, name }: AppInfo) -> Result<(), String> {
        self.app_handle
            .opener()
            .open_path(path, None::<&str>)
            .map_err(|e| e.to_string())?;
        thread::sleep(std::time::Duration::from_millis(500));

        let ax_ref = self.app_handle.state::<ActorRef<AXActor>>();
        ax_ref.tell(ax::RefreshAX).await.unwrap();
        ax_ref
            .tell(ax::TryFocusApp {
                app_name: name.clone(),
            })
            .await
            .unwrap();

        log::info!("launched app: {}", name);

        if name == "Screenshot" {
            self.app_handle.hide().map_err(|e| e.to_string())?;
        }

        Ok(())
    }

    async fn run_switch_cmd(&self, target: WindowTarget) -> Result<(), String> {
        let ax_ref = self.app_handle.state::<ActorRef<AXActor>>();
        ax_ref
            .ask(ax::FocusWindow {
                window_id: WindowId(target.window_id),
            })
            .await
            .unwrap();
        Ok(())
    }

    async fn run_theme_cmd(&self, theme: String) -> Result<(), String> {
        let theme_ref = self
            .app_handle
            .state::<ActorRef<theme::ThemeManagerActor>>();
        theme_ref
            .tell(theme::ApplyTheme { theme_name: theme })
            .await
            .unwrap();
        Ok(())
    }

    pub async fn get_cmds(&self) -> Vec<Command> {
        let app_infos = get_app_info()
            .unwrap_or_default()
            .into_iter()
            .map(Command::App)
            .collect::<Vec<Command>>();

        let application_tree_ref = self
            .app_handle
            .state::<ActorRef<application_tree::ApplicationTreeActor>>();

        let switch_targets = application_tree_ref
            .ask(application_tree::SearchParam::All)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|res| WindowTarget {
                app_name: res.app_name,
                title: res.title,
                pid: res.pid,
                window_id: res.window_id.0,
            })
            .map(Command::Switch)
            .collect::<Vec<Command>>();

        let theme_ref = self
            .app_handle
            .state::<ActorRef<theme::ThemeManagerActor>>();

        let themes = theme_ref
            .ask(theme::LoadThemes)
            .await
            .unwrap_or_default()
            .into_iter()
            .filter_map(|t| t.name)
            .map(Command::Theme)
            .collect::<Vec<Command>>();

        [app_infos, switch_targets, themes].concat()
    }
}

#[tauri::command]
pub async fn run_cmd(app: tauri::AppHandle, cmd: Command) -> Result<(), String> {
    let handle = app.state::<ActorRef<CommandActor>>();
    handle.ask(cmd).await.map_err(|e| e.to_string())?;
    Ok(())
}
