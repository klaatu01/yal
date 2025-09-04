use serde::Serialize;
use std::path::Path;
#[cfg(target_os = "macos")]
use tauri::ActivationPolicy;
use tauri::{Manager, WindowEvent};
use walkdir::WalkDir;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[derive(Serialize, Clone)]
struct AppInfo {
    name: String,
    path: String,
}

fn read_app_name(bundle_path: &Path) -> String {
    // /Applications/Foo.app/Contents/Info.plist
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

fn collect_apps_in(dir: &Path, out: &mut Vec<AppInfo>) {
    if !dir.exists() {
        return;
    }
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
}

#[tauri::command]
fn list_apps() -> Result<Vec<AppInfo>, String> {
    let mut apps = Vec::new();
    collect_apps_in(Path::new("/Applications"), &mut apps);
    collect_apps_in(Path::new("/System/Applications"), &mut apps);
    if let Some(home) = dirs::home_dir() {
        collect_apps_in(&home.join("Applications"), &mut apps);
    }

    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(apps)
}

#[tauri::command]
fn open_app(app: tauri::AppHandle, path: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_path(path, None::<&str>)
        .map_err(|e| e.to_string())
}

pub fn run() {
    tauri::Builder::default()
        // start at login so it’s “always running” after you log in
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        // make sure only one instance ever runs
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.show();
                let _ = win.set_focus();
            }
        }))
        // register the ONE global shortcut with a toggle handler
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                // use "alt+space" to avoid Spotlight conflict; switch to "cmd+space" only if you disable Spotlight’s hotkey
                .with_shortcut("cmd+space")
                .unwrap()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        if let Some(win) = app.get_webview_window("main") {
                            if win.is_visible().unwrap_or(false) {
                                let _ = win.hide();
                            } else {
                                let _ = win.show();
                                let _ = win.set_focus();
                            }
                        }
                    }
                })
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .on_window_event(|win, ev| match ev {
            WindowEvent::Focused(false) => {
                let _ = win.hide();
            }
            WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                let _ = win.hide();
            }
            _ => {}
        })
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(ActivationPolicy::Accessory);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![greet, list_apps, open_app])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
