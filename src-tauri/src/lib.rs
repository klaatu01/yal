use tauri::{ActivationPolicy, Emitter, Manager, WindowEvent};

mod ax;
mod cmd;
mod config;
mod window;

use crate::{
    ax::AX,
    cmd::{run_cmd, theme::ThemeManager},
};

use std::sync::{Arc, RwLock};

use config::load_config;
use yal_core::{AppConfig, Theme};

#[tauri::command]
fn get_theme(
    app: tauri::AppHandle,
    theme_manager: tauri::State<Arc<RwLock<ThemeManager>>>,
) -> Option<Theme> {
    let config = current_cfg_or_default(&app);
    let theme_manager = theme_manager.read().unwrap();
    let themes = theme_manager.load_themes();
    config
        .theme
        .as_ref()
        .and_then(|name| themes.iter().find(|t| t.name.as_deref() == Some(name)))
        .cloned()
}

#[tauri::command]
fn get_config(state: tauri::State<Arc<RwLock<AppConfig>>>) -> Result<AppConfig, String> {
    Ok(state.read().unwrap().clone())
}

#[tauri::command]
fn reload_config(
    app: tauri::AppHandle,
    state: tauri::State<Arc<RwLock<AppConfig>>>,
) -> Result<AppConfig, String> {
    let cfg = load_config();
    window::apply_window_size(&app, &cfg);
    *state.write().unwrap() = cfg.clone();
    Ok(cfg)
}

#[tauri::command]
fn hide_window(app: tauri::AppHandle) -> Result<(), String> {
    hide_palette_window(&app);
    Ok(())
}

fn publish_cmd_list(app: &tauri::AppHandle) {
    let cmds: Vec<_> = cmd::get_cmds(app);
    let _ = app.emit("commands://updated", cmds);
}

fn reveal_palette(app: &tauri::AppHandle) {
    let cfg = current_cfg_or_default(app);
    window::reveal_on_active_space(app, &cfg);
}

fn hide_palette_window(app: &tauri::AppHandle) {
    app.hide().ok();
}

fn current_cfg_or_default(app: &tauri::AppHandle) -> AppConfig {
    app.try_state::<Arc<RwLock<AppConfig>>>()
        .map(|s| s.read().unwrap().clone())
        .unwrap_or_default()
}

fn spawn_config_watcher(app: &tauri::AppHandle, state: Arc<RwLock<AppConfig>>) {
    use notify::{RecursiveMode, Watcher};
    use std::{sync::mpsc, time::Duration};

    let app_handle = app.clone();
    let cfg_path = config::config_path();
    let watch_dir = cfg_path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| std::env::current_dir().unwrap());

    std::thread::spawn(move || {
        let (tx, rx) = mpsc::channel();

        let mut watcher =
            notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
                let _ = tx.send(res);
            })
            .expect("failed to create file watcher");

        watcher
            .watch(&watch_dir, RecursiveMode::NonRecursive)
            .expect("failed to watch config directory");

        let mut last_reload = std::time::Instant::now();

        while let Ok(res) = rx.recv() {
            match res {
                Ok(event) => {
                    let relevant = event.paths.iter().any(|p| p == &cfg_path);
                    if !relevant {
                        continue;
                    }

                    if last_reload.elapsed() < Duration::from_millis(120) {
                        continue;
                    }
                    last_reload = std::time::Instant::now();
                    std::thread::sleep(Duration::from_millis(50));

                    let new_cfg = config::load_config();

                    {
                        let mut lock = state.write().unwrap();
                        *lock = new_cfg.clone();
                    }

                    if let Some(theme_name) = &new_cfg.theme {
                        let theme_manager = app_handle.state::<Arc<RwLock<ThemeManager>>>();
                        let mut theme_manager = theme_manager.write().unwrap();
                        theme_manager.apply_theme(&app_handle, theme_name);
                    }

                    window::apply_window_size(&app_handle, &new_cfg);

                    window::position_main_window_on_mouse_display(&app_handle, &new_cfg);

                    let _ = app_handle.emit("config://updated", new_cfg);
                }
                Err(err) => eprintln!("watch error: {err:?}"),
            }
        }
    });
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_macos_permissions::init())
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.show();
                let _ = win.set_focus();
            }
        }))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_shortcut("cmd+space")
                .unwrap()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        if let Some(win) = app.get_webview_window("main") {
                            if win.is_visible().unwrap_or(false) {
                                hide_palette_window(app);
                            } else {
                                let handle = win.app_handle();
                                {
                                    let _ax = handle.state::<Arc<RwLock<AX>>>();
                                    let mut _ax = _ax.write().unwrap();
                                    _ax.refresh();
                                }
                                publish_cmd_list(app);
                                reveal_palette(app);
                            }
                        }
                    }
                })
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .on_window_event(|win, ev| match ev {
            WindowEvent::Focused(false) => {
                let handle = win.app_handle();
                let _ax = handle.state::<Arc<RwLock<AX>>>();
                let mut _ax = _ax.write().unwrap();
                let focused = _ax.get_focused_window();
                if let Some(focus) = focused {
                    _ax.focus_window(focus);
                }
                hide_palette_window(win.app_handle());
            }
            WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                hide_palette_window(win.app_handle());
            }
            _ => {}
        })
        .setup(|app| {
            tauri::async_runtime::block_on(async {
                if !tauri_plugin_macos_permissions::check_accessibility_permission().await {
                    tauri_plugin_macos_permissions::request_accessibility_permission().await;
                }
                if !tauri_plugin_macos_permissions::check_screen_recording_permission().await {
                    tauri_plugin_macos_permissions::request_screen_recording_permission().await;
                }
            });
            let cfg = load_config();
            window::apply_window_size(app.handle(), &cfg);
            app.manage(Arc::new(RwLock::new(cfg)));
            app.set_activation_policy(ActivationPolicy::Accessory);
            app.manage(Arc::new(RwLock::new(AX::new(app.handle().clone()))));
            app.manage(Arc::new(RwLock::new(ThemeManager::new())));
            let cfg_state = app.state::<Arc<RwLock<AppConfig>>>().inner().clone();
            spawn_config_watcher(&app.handle().clone(), cfg_state);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            run_cmd,
            hide_window,
            get_config,
            reload_config,
            get_theme
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
