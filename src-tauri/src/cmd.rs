use std::sync::{Arc, RwLock};

use lightsky::WindowId;
use tauri::Manager;
use tauri_plugin_opener::OpenerExt;
use yal_core::{AppInfo, Command, WindowTarget};

use crate::{ax::AX, cmd::app::get_app_info};

mod app;

#[tauri::command]
pub fn run_cmd(app: tauri::AppHandle, cmd: Command) -> Result<(), String> {
    match cmd {
        Command::App(app_info) => run_app_cmd(app, app_info),
        Command::Switch(target) => run_switch_cmd(app, target),
    }
}

fn run_app_cmd(app: tauri::AppHandle, AppInfo { path, .. }: AppInfo) -> Result<(), String> {
    app.opener()
        .open_path(path, None::<&str>)
        .map_err(|e| e.to_string())
}

fn run_switch_cmd(app: tauri::AppHandle, target: WindowTarget) -> Result<(), String> {
    let ax = app.state::<Arc<RwLock<AX>>>();
    let mut ax = ax.write().unwrap();
    ax.focus_window(WindowId(target.window_id));
    Ok(())
}

pub fn get_cmds(app: &tauri::AppHandle) -> Vec<Command> {
    let ax = app.state::<Arc<RwLock<AX>>>();
    let ax = ax.read().unwrap();

    let app_infos = get_app_info()
        .unwrap_or_default()
        .into_iter()
        .map(Command::App)
        .collect::<Vec<Command>>();

    let switch_targets = ax
        .application_tree
        .flatten()
        .into_iter()
        .map(|res| WindowTarget {
            app_name: res.app_name,
            title: res.title,
            pid: res.pid,
            window_id: res.window_id.0,
        })
        .map(Command::Switch)
        .collect::<Vec<Command>>();

    [app_infos, switch_targets].concat()
}
