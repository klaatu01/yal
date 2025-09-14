use std::{
    sync::{Arc, RwLock},
    thread,
};

use lightsky::WindowId;
use tauri::Manager;
use tauri_plugin_opener::OpenerExt;
use yal_core::{AppInfo, Command, WindowTarget};

use crate::{ax::AX, cmd::app::get_app_info};

mod app;
pub mod theme;

#[tauri::command]
pub fn run_cmd(app: tauri::AppHandle, cmd: Command) -> Result<(), String> {
    match cmd {
        Command::App(app_info) => run_app_cmd(app, app_info),
        Command::Switch(target) => run_switch_cmd(app, target),
        Command::Theme(theme) => run_theme_cmd(app, theme),
    }
}

fn run_app_cmd(app: tauri::AppHandle, AppInfo { path, name }: AppInfo) -> Result<(), String> {
    app.opener()
        .open_path(path, None::<&str>)
        .map_err(|e| e.to_string())?;
    thread::sleep(std::time::Duration::from_millis(500));
    let ax = app.state::<Arc<RwLock<AX>>>();
    let mut ax = ax.write().unwrap();
    ax.refresh();
    ax.try_focus_app(&name);
    Ok(())
}

fn run_switch_cmd(app: tauri::AppHandle, target: WindowTarget) -> Result<(), String> {
    let ax = app.state::<Arc<RwLock<AX>>>();
    let mut ax = ax.write().unwrap();
    ax.focus_window(WindowId(target.window_id));
    Ok(())
}

fn run_theme_cmd(app: tauri::AppHandle, theme: String) -> Result<(), String> {
    let theme_manager = app.state::<Arc<RwLock<theme::ThemeManager>>>();
    let mut theme_manager = theme_manager.write().unwrap();
    theme_manager.apply_theme(&app, &theme);
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

    let themes = app
        .state::<Arc<RwLock<theme::ThemeManager>>>()
        .read()
        .unwrap()
        .load_themes()
        .into_iter()
        .filter_map(|t| t.name)
        .map(Command::Theme)
        .collect::<Vec<Command>>();

    [app_infos, switch_targets, themes].concat()
}
