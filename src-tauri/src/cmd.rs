use tauri_plugin_opener::OpenerExt;
use yal_core::{AppInfo, Command, WindowTarget};

use crate::cmd::app::get_app_info;

mod app;
pub mod switch;

#[tauri::command]
pub fn run_cmd(app: tauri::AppHandle, cmd: Command) -> Result<(), String> {
    app.hide().unwrap();
    match cmd {
        Command::App(app_info) => run_app_cmd(app, app_info),
        Command::Switch(target) => run_switch_cmd(target),
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

pub fn get_cmds() -> Vec<Command> {
    let app_infos = get_app_info()
        .unwrap_or_default()
        .into_iter()
        .map(Command::App)
        .collect::<Vec<Command>>();
    let switch_targets = switch::list_switch_targets()
        .into_iter()
        .map(Command::Switch)
        .collect::<Vec<Command>>();
    [app_infos, switch_targets].concat()
}
