use leptos::task::spawn_local;
use leptos::{ev::KeyboardEvent, prelude::*};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

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

#[component]
pub fn App() -> impl IntoView {
    let (apps, set_apps) = signal(Vec::<AppInfo>::new());
    let (query, set_query) = signal(String::new());
    let (selected, set_selected) = signal(0usize);

    // Fetch app list on mount
    spawn_local(async move {
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
        // keep selection in bounds
        if !v.is_empty() && selected.get() >= v.len() {
            set_selected.set(v.len() - 1);
        }
        v
    });

    // Key navigation on the input
    let on_key = move |ev: KeyboardEvent| {
        let key = ev.key();
        let len = filtered.get().len();
        if key == "ArrowDown" && len > 0 {
            ev.prevent_default();
            set_selected.update(|i| *i = (*i + 1).min(len - 1));
        } else if key == "ArrowUp" && len > 0 {
            ev.prevent_default();
            set_selected.update(|i| *i = i.saturating_sub(1));
        } else if key == "Enter" {
            if let Some(app) = filtered.get().get(selected.get()).cloned() {
                spawn_local(async move {
                    let args =
                        serde_wasm_bindgen::to_value(&OpenAppArgs { path: &app.path }).unwrap();
                    let _ = invoke("open_app", args).await;
                });
            }
        }
    };

    let on_input = move |ev| set_query.set(event_target_value(&ev));

    view! {
        <main class="container">
            <h1>"Launch an app"</h1>

            <input
              id="search"
              placeholder="Type to fuzzy find…"
              on:input=on_input
              on:keydown=on_key
              autofocus
            />

            <ul class="results">
                { move || {
                    let sel = selected.get();
                    filtered.get().into_iter().enumerate().map(|(i, app)| {
                        let is_sel = i == sel;
                        view! {
                          <li class:is-selected=is_sel>{format!("{}  —  {}", app.name, app.path)}</li>
                        }
                    }).collect_view()
                }}
            </ul>
        </main>
    }
}
