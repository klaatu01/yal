use leptos::web_sys::window;
use wasm_bindgen::JsCast;
use yal_core::{FontConfig, Theme, WindowConfig};

pub fn apply_theme_cfg(cfg: &Theme) {
    if let Some(doc) = window().and_then(|w| w.document()) {
        if let Some(root_el) = doc.document_element() {
            let html_el: leptos::web_sys::HtmlElement = root_el.unchecked_into();
            let style = html_el.style();

            if let Some(v) = &cfg.bg_color {
                let _ = style.set_property("--bg", v);
            }
            if let Some(v) = &cfg.fg_color {
                let _ = style.set_property("--hl", v);
            }

            if let Some(v) = &cfg.bg_font_color {
                let _ = style.set_property("--text", v);
            }
            if let Some(v) = &cfg.fg_font_color {
                let _ = style.set_property("--hl-text", v);
            }
        }
    }
}

pub fn apply_window_cfg(cfg: &WindowConfig) {
    if let Some(doc) = window().and_then(|w| w.document()) {
        if let Some(root_el) = doc.document_element() {
            let html_el: leptos::web_sys::HtmlElement = root_el.unchecked_into();
            let style = html_el.style();

            if let Some(pad) = &cfg.padding {
                let _ = style.set_property("--pad", &format!("{pad}px"));
            }

            if let Some(lh) = &cfg.line_height {
                let _ = style.set_property("--lh", &format!("{lh}"));
            }

            if let Some(rad) = &cfg.w_radius {
                let _ = style.set_property("--radius", &format!("{rad}px"));
            }
        }
    }
}

pub fn apply_font_cfg(cfg: &FontConfig) {
    if let Some(doc) = window().and_then(|w| w.document()) {
        if let Some(root_el) = doc.document_element() {
            let html_el: leptos::web_sys::HtmlElement = root_el.unchecked_into();
            let style = html_el.style();

            if let Some(v) = &cfg.font {
                let _ = style.set_property("--font", v);
            }
            if let Some(px) = cfg.font_size {
                let _ = style.set_property("--fs", &format!("{px}px")); // e.g. "14px"
            }
        }
    }
}
