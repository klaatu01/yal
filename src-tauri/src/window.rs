use objc2::runtime::AnyObject;
use objc2_app_kit::{NSApp, NSEvent, NSScreen, NSWindow, NSWindowCollectionBehavior};
use objc2_foundation::{MainThreadMarker, NSPoint, NSRect};

use tauri::{LogicalSize, Manager, Size};

use yal_core::{AlignH, AlignV, AppConfig};

pub fn apply_window_size(app: &tauri::AppHandle, cfg: &AppConfig) {
    if let Some(win) = app.get_webview_window("main") {
        if let Some(window_cfg) = &cfg.window {
            if let (Some(w), Some(h)) = (window_cfg.w_width, window_cfg.w_height) {
                let _ = win.set_size(Size::Logical(LogicalSize {
                    width: w,
                    height: h,
                }));
            }
        }
    }
}

#[inline]
fn point_in_rect(p: NSPoint, r: NSRect) -> bool {
    p.x >= r.origin.x
        && p.x <= r.origin.x + r.size.width
        && p.y >= r.origin.y
        && p.y <= r.origin.y + r.size.height
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

pub fn position_main_window_on_mouse_display(app: &tauri::AppHandle, cfg: &AppConfig) {
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

                let (ah, av, mx, my) = if let Some(window_cfg) = &cfg.window {
                    (
                        window_cfg.align_h.unwrap_or(AlignH::Center),
                        window_cfg.align_v.unwrap_or(AlignV::Center),
                        window_cfg.margin_x.unwrap_or(12.0),
                        window_cfg.margin_y.unwrap_or(12.0),
                    )
                } else {
                    (AlignH::Center, AlignV::Center, 12.0, 12.0)
                };

                let top_left = compute_top_left_for_alignment(sf, wf, ah, av, mx, my);
                nswin.setFrameTopLeftPoint(top_left);
            }
        }
    });
}

pub fn reveal_on_active_space(app: &tauri::AppHandle, cfg: &AppConfig) {
    // remember_current_frontmost(app);
    position_main_window_on_mouse_display(app, cfg);

    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show();
        let _ = app.run_on_main_thread(|| unsafe {
            let mtm = MainThreadMarker::new_unchecked();
            NSApp(mtm).activate();
        });
        let _ = win.set_focus();
    }
}
