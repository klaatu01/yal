use tauri_plugin_opener::OpenerExt;
use yal_core::{AppInfo, Command, WindowTarget};

use crate::cmd::{
    app::get_app_info,
    // hide::{set_previous_focus_state, FocusState},
};

mod app;
pub mod switch;

#[tauri::command]
pub fn run_cmd(app: tauri::AppHandle, cmd: Command) -> Result<(), String> {
    match cmd {
        Command::App(app_info) => run_app_cmd(app, app_info),
        Command::Switch(target) => {
            let pid = target.pid;
            let window_id = target.window_id;
            // set_previous_focus_state(
            //     &app,
            //     FocusState {
            //         prev_pid: Some(pid),
            //         window_id: Some(window_id),
            //     },
            // );
            // hide::hide(&app, hide::HideBehavior::FocusNew { pid, window_id });

            Ok(())
        }
    }
}

fn run_app_cmd(app: tauri::AppHandle, AppInfo { path, .. }: AppInfo) -> Result<(), String> {
    app.opener()
        .open_path(path, None::<&str>)
        .map_err(|e| e.to_string())
}

fn run_switch_cmd(target: WindowTarget) -> Result<(), String> {
    switch::focus_switch_target(&target)
}

pub fn get_cmds(app: &tauri::AppHandle) -> Vec<Command> {
    let app_infos = get_app_info()
        .unwrap_or_default()
        .into_iter()
        .map(Command::App)
        .collect::<Vec<Command>>();
    let switch_targets = switch::list_switch_targets(app)
        .into_iter()
        .map(Command::Switch)
        .collect::<Vec<Command>>();
    [app_infos, switch_targets].concat()
}
