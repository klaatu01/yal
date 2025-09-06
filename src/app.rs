use leptos::task::spawn_local;
use leptos::web_sys::window;
use leptos::{ev::KeyboardEvent, prelude::*};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast; // for unchecked_into / dyn_into

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
struct AppInfo {
    name: String,
    path: String,
}

#[derive(Serialize, Deserialize)]
struct OpenAppArgs<'a> {
    path: &'a str,
}

#[derive(Serialize, Deserialize)]
struct Empty {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
struct UiConfig {
    font: Option<String>,
    font_size: Option<f32>,     // in px
    bg_color: Option<String>,   // normal background
    fg_color: Option<String>,   // highlight background
    font_color: Option<String>, // (legacy) fallback for normal text if font_bg_color missing
    // NEW:
    font_fg_color: Option<String>, // text color ON highlight background
    font_bg_color: Option<String>, // text color ON normal background
    w_width: Option<f64>,
    w_height: Option<f64>,
}

fn apply_ui_config(cfg: &UiConfig) {
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
            if let Some(v) = &cfg.font_bg_color {
                // normal text (on --bg)
                let _ = style.set_property("--text", v);
            } else if let Some(v) = &cfg.font_color {
                // backwards-compat fallback if user didn't set font_bg_color
                let _ = style.set_property("--text", v);
            }
            if let Some(v) = &cfg.font_fg_color {
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
            // If you want a custom line-height from config later, also set `--lh` here.
        }
    }
}

/* --------------------------------- Fuzzy match -------------------------------- */

fn fuzzy_match(text: &str, pattern: &str) -> bool {
    if pattern.is_empty() {
        return true;
    }
    let p = pattern.to_lowercase();
    let mut pc = p.chars();
    let mut next = pc.next();
    for c in text.to_lowercase().chars() {
        if Some(c) == next {
            next = pc.next();
            if next.is_none() {
                return true;
            }
        }
    }
    false
}

async fn fetch_and_apply_config() {
    let cfg_val = invoke("get_config", JsValue::NULL).await;
    if let Ok(cfg) = serde_wasm_bindgen::from_value::<UiConfig>(cfg_val) {
        apply_ui_config(&cfg);
    }
}

/* --------------------------------- Component ---------------------------------- */
#[component]
pub fn App() -> impl IntoView {
    let (apps, set_apps) = signal(Vec::<AppInfo>::new());
    let (query, set_query) = signal(String::new());
    let (selected, set_selected) = signal(0usize);

    spawn_local(async move {
        fetch_and_apply_config().await;

        let js = invoke(
            "list_apps",
            serde_wasm_bindgen::to_value(&Empty {}).unwrap(),
        )
        .await;
        let list: Vec<AppInfo> = serde_wasm_bindgen::from_value(js).unwrap_or_default();
        set_apps.set(list);
    });

    // Derived filtered list
    let filtered = Memo::new(move |_| {
        let q = query.get();
        let list = apps.get();
        let v: Vec<AppInfo> = list
            .into_iter()
            .filter(|a| fuzzy_match(&a.name, &q))
            .collect();
        if !v.is_empty() && selected.get() >= v.len() {
            set_selected.set(v.len() - 1);
        }
        v
    });

    let reset = move || {
        set_selected.set(0);
        set_query.set(String::new());
    };

    // Key navigation on the input
    let on_key = move |ev: KeyboardEvent| {
        let key = ev.key();
        let len = filtered.get().len();
        match key.as_str() {
            "ArrowDown" => {
                ev.prevent_default();
                if len > 0 {
                    set_selected.update(|i| *i = (*i + 1).min(len - 1));
                }
            }
            "ArrowUp" => {
                ev.prevent_default();
                if len > 0 {
                    set_selected.update(|i| *i = i.saturating_sub(1));
                }
            }
            "Enter" => {
                if let Some(app) = filtered.get().get(selected.get()).cloned() {
                    spawn_local(async move {
                        let args =
                            serde_wasm_bindgen::to_value(&OpenAppArgs { path: &app.path }).unwrap();
                        let _ = invoke("open_app", args).await;
                    });
                }
                reset();
            }
            "Escape" => {
                spawn_local(async move {
                    let _ = invoke(
                        "hide_window",
                        serde_wasm_bindgen::to_value(&Empty {}).unwrap(),
                    )
                    .await;
                });
                reset();
            }
            _ => {}
        }
    };

    let on_input = move |ev| set_query.set(event_target_value(&ev));

    view! {
        // Top "bar" like dmenu
        <div id="bar">
            <input
              id="search"
              prop:value=move || query.get()
              on:input=on_input
              on:keydown=on_key
              prop:spellcheck=false
              prop:autocorrect="off"
              prop:autocapitalize="off"
              autofocus
            />
        </div>

        <ul class="results">
            { move || {
                let sel = selected.get();
                filtered.get().into_iter().enumerate().map(|(i, app)| {
                    let is_sel = i == sel;
                    view! {
                        <li class:is-selected=is_sel>
                            { app.name.to_lowercase() }
                        </li>
                    }
                }).collect_view()
            }}
        </ul>
    }
}
