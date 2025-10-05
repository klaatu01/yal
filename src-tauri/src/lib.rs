use kameo::{actor::ActorRef, Actor};
use tauri::{ActivationPolicy, Emitter, Manager, WindowEvent};

mod application_tree;
mod ax;
mod cmd;
mod config;
mod display;
mod focus;
mod window;

use crate::{
    ax::AXActor,
    cmd::{
        run_cmd,
        theme::{self, ThemeManagerActor},
    },
};

use config::load_config;
use yal_core::{AppConfig, Theme};

#[tauri::command]
async fn get_theme(app: tauri::AppHandle) -> Result<Option<Theme>, String> {
    let theme_manager = app.state::<ActorRef<ThemeManagerActor>>();
    let cfg_ref = app.state::<ActorRef<config::ConfigActor>>();
    let cfg = cfg_ref
        .ask(config::GetConfig)
        .await
        .map_err(|e| e.to_string())?;
    let themes = theme_manager.ask(theme::LoadThemes).await.unwrap();
    if let Some(name) = &cfg.theme {
        let mut theme_iter = themes.iter();
        if let Some(theme) = theme_iter.find(|t| t.name.as_deref() == Some(name)) {
            return Ok(Some(theme.clone()));
        }
        Ok(None)
    } else {
        Ok(None)
    }
}

#[tauri::command]
async fn get_config(app: tauri::AppHandle) -> Result<AppConfig, String> {
    let cfg_ref = app.state::<ActorRef<config::ConfigActor>>();
    let cfg = cfg_ref
        .ask(config::GetConfig)
        .await
        .map_err(|e| e.to_string())?;
    Ok(cfg)
}

#[tauri::command]
async fn reload_config(app: tauri::AppHandle) -> Result<AppConfig, String> {
    let cfg_ref = app.state::<ActorRef<config::ConfigActor>>();
    cfg_ref
        .ask(config::ReloadConfig)
        .await
        .map_err(|e| e.to_string())?;

    let cfg = cfg_ref
        .ask(config::GetConfig)
        .await
        .map_err(|e| e.to_string())?;

    window::apply_window_size(&app, &cfg);
    Ok(cfg)
}

#[tauri::command]
fn hide_window(app: tauri::AppHandle) -> Result<(), String> {
    hide_palette_window(&app);
    Ok(())
}

async fn publish_cmd_list(app: &tauri::AppHandle) {
    let cmd_handle = app.state::<ActorRef<cmd::CommandActor>>();
    cmd_handle.tell(cmd::PublishCommands).await.unwrap();
}

async fn reveal_palette(app: &tauri::AppHandle) {
    let cfg = current_cfg_or_default(app).await;
    window::reveal_on_active_space(app, &cfg);
}

fn hide_palette_window(app: &tauri::AppHandle) {
    app.hide().ok();
}

async fn current_cfg_or_default(app: &tauri::AppHandle) -> AppConfig {
    let cfg_ref = app.state::<ActorRef<config::ConfigActor>>();
    cfg_ref.ask(config::GetConfig).await.unwrap_or_default()
}

fn spawn_config_watcher(
    app: &tauri::AppHandle,
    config_actor_ref: ActorRef<config::ConfigActor>,
    theme_manager_ref: ActorRef<ThemeManagerActor>,
) {
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

                    tauri::async_runtime::block_on(async {
                        config_actor_ref.tell(config::ReloadConfig).await.unwrap();

                        let new_cfg = config_actor_ref.ask(config::GetConfig).await.unwrap();

                        if let Some(theme_name) = &new_cfg.theme {
                            theme_manager_ref
                                .tell(theme::ApplyTheme {
                                    theme_name: theme_name.clone(),
                                })
                                .await
                                .unwrap();
                        }

                        window::apply_window_size(&app_handle, &new_cfg);

                        window::position_main_window_on_mouse_display(&app_handle, &new_cfg);

                        let _ = app_handle.emit("config://updated", new_cfg);
                    });
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
                                tauri::async_runtime::block_on(async {
                                    let ax_handle = app.state::<ActorRef<AXActor>>();
                                    ax_handle.tell(crate::ax::RefreshAX).await.unwrap();
                                    publish_cmd_list(app).await;
                                    reveal_palette(app).await;
                                });
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
                let ax_ref = handle.state::<ActorRef<AXActor>>();
                tauri::async_runtime::block_on(async {
                    let focused = ax_ref.ask(crate::ax::GetFocusedWindow).await.unwrap();
                    if let Some(focus) = focused {
                        ax_ref
                            .tell(crate::ax::FocusWindow { window_id: focus })
                            .await
                            .unwrap();
                    }
                });
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

            tauri::async_runtime::block_on(async {
                let cmd_actor =
                    cmd::CommandActor::spawn(cmd::CommandActor::new(app.handle().clone()));

                let application_tree_actor = application_tree::ApplicationTreeActor::spawn(
                    application_tree::ApplicationTreeActor::new(lightsky::Lightsky::new().unwrap()),
                );

                let focus_manager_actor = focus::FocusManagerActor::spawn(
                    focus::FocusManagerActor::new(app.handle().clone()),
                );

                let display_manager_actor = display::DisplayManagerActor::spawn(
                    display::DisplayManagerActor::new(app.handle().clone()),
                );

                let ax_actor = AXActor::spawn(AXActor::new(
                    app.handle().clone(),
                    display_manager_actor.clone(),
                    focus_manager_actor.clone(),
                    application_tree_actor.clone(),
                ));

                let config_actor = config::ConfigActor::spawn(config::ConfigActor::new());

                let theme_manager_actor = theme::ThemeManagerActor::spawn(
                    theme::ThemeManagerActor::new(app.handle().clone()),
                );

                spawn_config_watcher(
                    &app.handle().clone(),
                    config_actor.clone(),
                    theme_manager_actor.clone(),
                );

                app.manage(cmd_actor);
                app.manage(application_tree_actor);
                app.manage(focus_manager_actor);
                app.manage(display_manager_actor);
                app.manage(ax_actor);
                app.manage(theme_manager_actor);
                app.manage(config_actor);
            });
            app.set_activation_policy(ActivationPolicy::Accessory);
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
