use std::path::Path;

use tauri_plugin_opener::OpenerExt;
use walkdir::WalkDir;
use yal_core::{AppInfo, Command};

#[tauri::command]
pub fn run_cmd(app: tauri::AppHandle, cmd: Command) -> Result<(), String> {
    match cmd {
        Command::App(app_info) => run_app_cmd(app, app_info),
    }
}

fn run_app_cmd(app: tauri::AppHandle, AppInfo { path, .. }: AppInfo) -> Result<(), String> {
    app.opener()
        .open_path(path, None::<&str>)
        .map_err(|e| e.to_string())
}

pub fn get_cmds() -> Vec<Command> {
    let app_infos = get_app_info().unwrap_or_default();
    app_infos.into_iter().map(Command::App).collect()
}

fn read_app_name(bundle_path: &Path) -> String {
    let plist_path = bundle_path.join("Contents").join("Info.plist");
    if let Ok(v) = plist::Value::from_file(&plist_path) {
        if let Some(d) = v.as_dictionary() {
            for key in ["CFBundleDisplayName", "CFBundleName", "Bundle name"] {
                if let Some(pl) = d.get(key).and_then(|v| v.as_string()) {
                    return pl.to_string();
                }
            }
        }
    }
    // Fallback to folder name without ".app"
    bundle_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_string()
}

fn collect_apps_in(dir: &Path) -> Vec<AppInfo> {
    if !dir.exists() {
        return Vec::new();
    }
    let mut out = Vec::new();
    for entry in WalkDir::new(dir).max_depth(2).into_iter().flatten() {
        let path = entry.path();
        if path.is_dir() && path.extension().and_then(|e| e.to_str()) == Some("app") {
            let name = read_app_name(path);
            out.push(AppInfo {
                name,
                path: path.to_string_lossy().into_owned(),
            });
        }
    }
    out
}

fn get_app_info() -> Result<Vec<AppInfo>, String> {
    let mut apps: Vec<AppInfo> = Vec::new();
    apps.append(&mut collect_apps_in(Path::new("/Applications")));
    apps.append(&mut collect_apps_in(Path::new("/System/Applications")));
    if let Some(home) = dirs::home_dir() {
        apps.append(&mut collect_apps_in(&home.join("Applications")));
    }
    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(apps)
}
