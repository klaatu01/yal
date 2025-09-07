use leptos::web_sys::window;
use wasm_bindgen::JsCast;
use yal_core::AppConfig;

pub fn apply(cfg: &AppConfig) {
    if let Some(doc) = window().and_then(|w| w.document()) {
        if let Some(root_el) = doc.document_element() {
            // Cast <html> Element -> HtmlElement to use the inherent web_sys::HtmlElement::style()
            let html_el: leptos::web_sys::HtmlElement = root_el.unchecked_into();
            let style = html_el.style();

            // Backgrounds
            if let Some(v) = &cfg.bg_color {
                let _ = style.set_property("--bg", v);
            }
            if let Some(v) = &cfg.fg_color {
                // highlight background
                let _ = style.set_property("--hl", v);
            }

            // Text colors
            if let Some(v) = &cfg.bg_font_color {
                // normal text (on --bg)
                let _ = style.set_property("--text", v);
            }
            if let Some(v) = &cfg.fg_font_color {
                // text on highlight
                let _ = style.set_property("--hl-text", v);
            }

            // Font family / size via CSS variables
            if let Some(v) = &cfg.font {
                let _ = style.set_property("--font", v);
            }
            if let Some(px) = cfg.font_size {
                let _ = style.set_property("--fs", &format!("{px}px")); // e.g. "14px"
            }

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
