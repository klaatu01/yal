use tauri::{ActivationPolicy, Emitter, Manager, WindowEvent};

mod cmd;
mod config;

use crate::cmd::run_cmd;

use objc2::runtime::AnyObject;
use objc2_app_kit::{
    NSApp, NSApplicationActivationOptions, NSEvent, NSRunningApplication, NSScreen, NSWindow,
    NSWindowCollectionBehavior, NSWorkspace,
};
use objc2_foundation::{MainThreadMarker, NSPoint, NSRect};

use std::sync::{Arc, RwLock};
use tauri::{LogicalSize, Size};

use config::load_config;
use yal_core::{AlignH, AlignV, AppConfig};

// ========== Shared: Config state & Tauri commands ==========

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

#[tauri::command]
fn hide_window(app: tauri::AppHandle) -> Result<(), String> {
    hide_palette_window(&app);
    Ok(())
}

// ========== Shared: Window sizing & publishing ==========

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

fn publish_cmd_list(app: &tauri::AppHandle) {
    let cmds = cmd::get_cmds();
    let _ = app.emit("commands://updated", cmds);
}

// ========== Cross-platform window show/hide entry points ==========

fn reveal_palette(app: &tauri::AppHandle) {
    reveal_on_active_space(app);
}

fn hide_palette_window(app: &tauri::AppHandle) {
    hide_and_focus_previous(app);
}

fn hide_without_focus_restore_cross(app: &tauri::AppHandle) {
    hide_without_focus_restore(app);
}

#[derive(Default)]
struct FocusState {
    /// PID of the app that was frontmost before we activated our palette window.
    prev_pid: Option<i32>,
}

#[inline]
fn point_in_rect(p: NSPoint, r: NSRect) -> bool {
    p.x >= r.origin.x
        && p.x <= r.origin.x + r.size.width
        && p.y >= r.origin.y
        && p.y <= r.origin.y + r.size.height
}

fn hide_without_focus_restore(app: &tauri::AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.hide();
    }
}

/// Hide the main window and restore focus to whatever was frontmost before
/// we activated our window. Falls back to `NSApp.deactivate()` if unknown.
fn hide_and_focus_previous(app: &tauri::AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.hide();
    }

    // Read remembered PID (if any)
    let pid_opt = app
        .try_state::<Arc<RwLock<FocusState>>>()
        .map(|s| s.read().unwrap().prev_pid);

    let _ = app.run_on_main_thread(move || unsafe {
        let mtm = MainThreadMarker::new_unchecked();

        if let Some(Some(pid)) = pid_opt {
            if let Some(prev) = NSRunningApplication::runningApplicationWithProcessIdentifier(pid) {
                prev.activateWithOptions(NSApplicationActivationOptions::ActivateAllWindows);
                return;
            }
        }

        NSApp(mtm).deactivate();
    });
}

fn clamp(v: f64, lo: f64, hi: f64) -> f64 {
    if v < lo {
        lo
    } else if v > hi {
        hi
    } else {
        v
    }
}

fn compute_top_left_for_alignment(
    sf: NSRect, // screen frame (global coords)
    wf: NSRect, // current window frame (size)
    ah: AlignH,
    av: AlignV,
    mx: f64,
    my: f64,
) -> NSPoint {
    // Horizontal placement
    let mut x = match ah {
        AlignH::Left => sf.origin.x + mx,
        AlignH::Center => sf.origin.x + (sf.size.width - wf.size.width) / 2.0,
        AlignH::Right => sf.origin.x + sf.size.width - wf.size.width - mx,
    };
    let min_x = sf.origin.x;
    let max_x = sf.origin.x + sf.size.width - wf.size.width;
    x = clamp(x, min_x, max_x);

    // Vertical placement: NSWindow::setFrameTopLeftPoint uses top-left coordinates
    let mut y = match av {
        AlignV::Top => sf.origin.y + sf.size.height - my,
        AlignV::Center => sf.origin.y + sf.size.height - (sf.size.height - wf.size.height) / 2.0,
        AlignV::Bottom => sf.origin.y + wf.size.height + my,
    };
    let min_y = sf.origin.y + wf.size.height; // bottom edge + window height
    let max_y = sf.origin.y + sf.size.height; // top edge
    y = clamp(y, min_y, max_y);

    NSPoint { x, y }
}

fn remember_frontmost_pid(app: &tauri::AppHandle) {
    if let Some(state) = app.try_state::<Arc<RwLock<FocusState>>>() {
        unsafe {
            let ws = NSWorkspace::sharedWorkspace();
            // This call is thread-affine, but we only read the PID; safe enough in practice.
            if let Some(front) = ws.frontmostApplication() {
                state.write().unwrap().prev_pid = Some(front.processIdentifier());
            }
        };
    }
}

fn position_main_window_on_mouse_display(app: &tauri::AppHandle, cfg: &AppConfig) {
    let _ = app.run_on_main_thread({
        let cfg = cfg.clone();
        let app = app.clone();
        move || unsafe {
            use objc2::rc::Retained;
            let mtm = MainThreadMarker::new_unchecked();

            if let Some(win) = app.get_webview_window("main") {
                // Obtain NSWindow*
                let ptr = win.ns_window().expect("missing NSWindow");
                let any = &*(ptr as *mut AnyObject);
                let nswin: &NSWindow = any.downcast_ref::<NSWindow>().expect("not an NSWindow");

                // Follow active Space when activated.
                let mut behavior = nswin.collectionBehavior();
                behavior.insert(NSWindowCollectionBehavior::MoveToActiveSpace);
                nswin.setCollectionBehavior(behavior);

                // Target screen under mouse (fall back to first screen).
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

                // Alignment & margins
                let ah = cfg.align_h.unwrap_or(AlignH::Center);
                let av = cfg.align_v.unwrap_or(AlignV::Center);
                let mx = cfg.margin_x.unwrap_or(12.0);
                let my = cfg.margin_y.unwrap_or(12.0);

                let top_left = compute_top_left_for_alignment(sf, wf, ah, av, mx, my);
                nswin.setFrameTopLeftPoint(top_left);
            }
        }
    });
}

fn reveal_on_active_space(app: &tauri::AppHandle) {
    remember_frontmost_pid(app);
    position_main_window_on_mouse_display(app, &current_cfg_or_default(app));

    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show();
        let _ = app.run_on_main_thread(|| unsafe {
            let mtm = MainThreadMarker::new_unchecked();
            NSApp(mtm).activate();
        });
        let _ = win.set_focus();
    }
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

                    apply_window_size(&app_handle, &new_cfg);

                    position_main_window_on_mouse_display(&app_handle, &new_cfg);

                    let _ = app_handle.emit("config://updated", new_cfg);
                }
                Err(err) => eprintln!("watch error: {err:?}"),
            }
        }
    });
}

// ========== Entrypoint ==========

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
                                hide_palette_window(app);
                            } else {
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
                // On blur, hide without bringing previous app to front (mac) to avoid focus ping-pong
                hide_without_focus_restore_cross(win.app_handle());
            }
            WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                hide_palette_window(win.app_handle());
            }
            _ => {}
        })
        .setup(|app| {
            let cfg = load_config();
            apply_window_size(app.handle(), &cfg);
            app.manage(Arc::new(RwLock::new(cfg)));
            app.set_activation_policy(ActivationPolicy::Accessory);
            app.manage(Arc::new(RwLock::new(FocusState::default())));

            let cfg_state = app.state::<Arc<RwLock<AppConfig>>>().inner().clone();
            spawn_config_watcher(&app.handle().clone(), cfg_state);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            run_cmd,
            hide_window,
            get_config,
            reload_config
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
