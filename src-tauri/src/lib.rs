use serde::Serialize;
use std::path::Path;
use tauri::{ActivationPolicy, Emitter};
use tauri::{Manager, WindowEvent};
use walkdir::WalkDir;
mod config;

#[cfg(target_os = "macos")]
use objc2::runtime::AnyObject;
#[cfg(target_os = "macos")]
use objc2_app_kit::{NSApp, NSEvent, NSScreen, NSWindow, NSWindowCollectionBehavior};
#[cfg(target_os = "macos")]
use objc2_foundation::{MainThreadMarker, NSPoint, NSRect};
use std::sync::{Arc, RwLock};
use tauri::LogicalSize;
use tauri::Size;

use config::{load_config, AppConfig};

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
    apply_window_size(&app, &cfg);
    *state.write().unwrap() = cfg.clone();
    Ok(cfg)
}

fn apply_window_size(app: &tauri::AppHandle, cfg: &AppConfig) {
    if let Some(win) = app.get_webview_window("main") {
        if let (Some(w), Some(h)) = (cfg.w_width, cfg.w_height) {
            let _ = win.set_size(Size::Logical(LogicalSize {
                width: w,
                height: h,
            }));
        }
    }
}

#[derive(Serialize, Clone)]
struct AppInfo {
    name: String,
    path: String,
}

#[cfg(target_os = "macos")]
#[inline]
fn point_in_rect(p: NSPoint, r: NSRect) -> bool {
    p.x >= r.origin.x
        && p.x <= r.origin.x + r.size.width
        && p.y >= r.origin.y
        && p.y <= r.origin.y + r.size.height
}

#[cfg(target_os = "macos")]
fn reveal_on_active_space(app: &tauri::AppHandle) {
    // Helper: is a point inside a rect?
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || unsafe {
        let mtm = MainThreadMarker::new_unchecked();

        if let Some(win) = handle.get_webview_window("main") {
            // Get NSWindow*

            use objc2::rc::Retained;
            let ptr = win.ns_window().expect("missing NSWindow");
            let any = &*(ptr as *mut AnyObject);
            let nswin: &NSWindow = any.downcast_ref::<NSWindow>().expect("not an NSWindow");

            // Follow the active Space when activated.
            let mut behavior = nswin.collectionBehavior();
            behavior.insert(NSWindowCollectionBehavior::MoveToActiveSpace);

            // Optional: make it show even over fullscreen Spaces, or on *all* Spaces.
            // behavior.insert(NSWindowCollectionBehavior::FullScreenAuxiliary);
            // behavior.insert(NSWindowCollectionBehavior::CanJoinAllSpaces);

            nswin.setCollectionBehavior(behavior);

            // Find the screen under the mouse.
            let mouse = NSEvent::mouseLocation();
            let screens = NSScreen::screens(mtm);
            let mut target: Option<Retained<NSScreen>> = None;
            let mut first: Option<Retained<NSScreen>> = None;

            for s in screens.iter() {
                if first.is_none() {
                    first = Some(s.clone());
                }
                if point_in_rect(mouse, s.frame()) {
                    target = Some(s);
                    break;
                }
            }
            let target = target.or(first).expect("no NSScreen available");
            let sf = target.frame(); // screen frame (global coords)
            let wf = nswin.frame(); // current window frame

            // Center window on the chosen screen. Note: setFrameTopLeftPoint uses top-left origin.
            let x = sf.origin.x + (sf.size.width - wf.size.width) / 2.0;
            let y_top = sf.origin.y + sf.size.height - (sf.size.height - wf.size.height) / 2.0;
            nswin.setFrameTopLeftPoint(NSPoint { x, y: y_top });

            // Show + activate on that display/Space.
            let _ = win.show();
            NSApp(mtm).activate();
            nswin.makeKeyAndOrderFront(None);
            let _ = win.set_focus();
        }
    });
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

#[tauri::command]
fn hide_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("main") {
        win.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn run() {
    tauri::Builder::default()
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
                                let _ = win.hide();
                            } else {
                                reveal_on_active_space(app);
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
            let cfg = load_config();
            apply_window_size(app.handle(), &cfg);
            app.manage(Arc::new(RwLock::new(cfg)));
            #[cfg(target_os = "macos")]
            app.set_activation_policy(ActivationPolicy::Accessory);

            {
                use notify::{RecursiveMode, Watcher};
                use std::{sync::mpsc, time::Duration};

                let app_handle = app.handle().clone();
                let state = app.state::<Arc<RwLock<AppConfig>>>().inner().clone();
                let cfg_path = config::config_path();
                let watch_dir = cfg_path
                    .parent()
                    .map(|p| p.to_path_buf())
                    .unwrap_or_else(|| std::env::current_dir().unwrap());

                std::thread::spawn(move || {
                    let (tx, rx) = mpsc::channel();

                    let mut watcher = notify::recommended_watcher(
                        move |res: Result<notify::Event, notify::Error>| {
                            let _ = tx.send(res);
                        },
                    )
                    .expect("failed to create file watcher");

                    watcher
                        .watch(&watch_dir, RecursiveMode::NonRecursive)
                        .expect("failed to watch config directory");

                    // Simple debounce to avoid partial writes
                    let mut last_reload = std::time::Instant::now();

                    while let Ok(res) = rx.recv() {
                        match res {
                            Ok(event) => {
                                // Only care if the changed path is the config file
                                let relevant = event.paths.iter().any(|p| p == &cfg_path);
                                if !relevant {
                                    continue;
                                }

                                // debounce ~120ms
                                if last_reload.elapsed() < Duration::from_millis(120) {
                                    continue;
                                }
                                last_reload = std::time::Instant::now();
                                std::thread::sleep(Duration::from_millis(50));

                                // Reload + apply + push
                                let new_cfg = config::load_config();

                                {
                                    let mut lock = state.write().unwrap();
                                    *lock = new_cfg.clone();
                                }

                                // If your window sizing must happen on the main thread on macOS,
                                // you can wrap this with `app_handle.run_on_main_thread(...)`.
                                apply_window_size(&app_handle, &new_cfg);

                                // Push to the frontend
                                let _ = app_handle.emit("config://updated", new_cfg);
                            }
                            Err(err) => eprintln!("watch error: {err:?}"),
                        }
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_apps,
            open_app,
            hide_window,
            get_config,
            reload_config
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
