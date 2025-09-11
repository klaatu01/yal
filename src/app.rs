use leptos::task::spawn_local;
use leptos::{ev::KeyboardEvent, prelude::*};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast; // for unchecked_into / dyn_into
use yal_core::{AppConfig, Command, CommandKind};

// NEW: fuzzy matcher imports
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "event"], js_name = listen)]
    async fn tauri_listen(event: &str, callback: &js_sys::Function);
}

#[derive(Serialize, Deserialize)]
struct RunCmdArgs {
    cmd: Command,
}

#[derive(Serialize, Deserialize)]
struct Empty {}

/* --------------------------------- Events ----------------------------------- */

fn init_config_listener() {
    leptos::task::spawn_local(async move {
        let cb = Closure::<dyn FnMut(js_sys::Object)>::new(move |evt_obj: js_sys::Object| {
            if let Ok(payload) = js_sys::Reflect::get(&evt_obj, &JsValue::from_str("payload")) {
                if let Ok(cfg) = serde_wasm_bindgen::from_value::<AppConfig>(payload) {
                    crate::ui::apply(&cfg);
                }
            }
        });

        let _unlisten = tauri_listen("config://updated", cb.as_ref().unchecked_ref()).await;
        cb.forget();
    });
}

fn init_cmd_list_listener(set_cmd_list: WriteSignal<Vec<Command>>, reset: impl Fn() + 'static) {
    leptos::task::spawn_local(async move {
        let cb = Closure::<dyn FnMut(js_sys::Object)>::new(move |evt_obj: js_sys::Object| {
            if let Ok(payload) = js_sys::Reflect::get(&evt_obj, &JsValue::from_str("payload")) {
                if let Ok(cmds) = serde_wasm_bindgen::from_value::<Vec<Command>>(payload) {
                    reset();
                    set_cmd_list.set(cmds);
                }
            }
        });

        let _unlisten = tauri_listen("commands://updated", cb.as_ref().unchecked_ref()).await;
        cb.forget();
    });
}

fn load_config() {
    spawn_local(async move {
        let config = invoke(
            "get_config",
            serde_wasm_bindgen::to_value(&Empty {}).unwrap(),
        )
        .await;
        if let Ok(cfg) = serde_wasm_bindgen::from_value::<AppConfig>(config) {
            crate::ui::apply(&cfg);
        }
    });
}

fn fuzzy_filter_commands(cmds: &[Command], query: &str) -> Vec<Command> {
    let matcher = SkimMatcherV2::default();
    let mut scored: Vec<(Command, i64)> = cmds
        .iter()
        .filter_map(|cmd| {
            matcher
                .fuzzy_match(cmd.name(), query)
                .map(|score| (cmd.clone(), score))
        })
        .collect();

    scored.sort_by(|a, b| {
        b.1.cmp(&a.1) // higher score first
            .then_with(|| a.0.name().to_lowercase().cmp(&b.0.name().to_lowercase()))
    });

    scored.into_iter().map(|(cmd, _)| cmd).collect()
}

fn filter_memoized_commands(
    cmds: &[Command],
    query: &str,
    selected: usize,
    set_selected: &WriteSignal<usize>,
    filter: Option<CommandKind>,
) -> Vec<Command> {
    let commands = if let Some(kind) = filter {
        cmds.iter()
            .filter(|c| kind.is_kind(c))
            .cloned()
            .collect::<Vec<_>>()
    } else {
        cmds.to_vec()
    };

    let v: Vec<Command> = if query.trim().is_empty() {
        let mut all = commands.to_vec();
        all.sort_by_key(|a| a.name().to_lowercase());
        all
    } else {
        fuzzy_filter_commands(&commands, query)
    };

    if !v.is_empty() && selected >= v.len() {
        set_selected.set(v.len() - 1);
    }
    v
}

/* --------------------------------- Component ---------------------------------- */
#[component]
pub fn App() -> impl IntoView {
    let (cmds, set_cmd_list) = signal(Vec::<Command>::new());
    let (query, set_query) = signal(String::new());
    let (selected, set_selected) = signal(0usize);
    let (filter, set_filter) = signal(Option::<CommandKind>::None);

    let reset = move || {
        set_selected.set(0);
        set_query.set(String::new());
    };

    load_config();
    init_config_listener();
    init_cmd_list_listener(set_cmd_list, reset);

    let filtered = Memo::new(move |_| {
        let q = query.get();
        let list = cmds.get();
        let filter = filter.get();
        filter_memoized_commands(&list, &q, selected.get(), &set_selected, filter)
    });

    let prefix_text = Memo::new(move |_| match filter.get() {
        Some(CommandKind::App) => "open".to_string(),
        Some(CommandKind::Switch) => "switch".to_string(),
        None => String::new(),
    });

    let open_selected = move || {
        if let Some(cmd) = filtered.get().get(selected.get()).cloned() {
            spawn_local(async move {
                let args = serde_wasm_bindgen::to_value(&RunCmdArgs { cmd: cmd.clone() }).unwrap();
                let _ = invoke("run_cmd", args).await;
            });
        }
    };

    let increment_selected = move || {
        let len = filtered.get().len();
        if len > 0 {
            set_selected.update(|i| *i = (*i + 1).min(len - 1));
        }
    };

    let decrement_selected = move || {
        set_selected.update(|i| *i = i.saturating_sub(1));
    };

    // Key navigation on the input
    let on_key = move |ev: KeyboardEvent| {
        let key = ev.key();
        match key.as_str() {
            "ArrowDown" => {
                ev.prevent_default();
                increment_selected();
            }
            "ArrowUp" => {
                ev.prevent_default();
                decrement_selected();
            }
            "n" if ev.ctrl_key() => {
                ev.prevent_default();
                increment_selected();
            }
            "p" if ev.ctrl_key() => {
                ev.prevent_default();
                decrement_selected();
            }
            "Enter" => {
                ev.prevent_default();
                open_selected();
            }
            "y" if ev.ctrl_key() => {
                ev.prevent_default();
                open_selected();
            }
            "f" if ev.ctrl_key() => {
                ev.prevent_default();
                set_filter.update(|f| {
                    *f = match f {
                        Some(CommandKind::Switch) => None,
                        _ => Some(CommandKind::Switch),
                    }
                });
            }
            "o" if ev.ctrl_key() => {
                ev.prevent_default();
                set_filter.update(|f| {
                    *f = match f {
                        Some(CommandKind::App) => None,
                        _ => Some(CommandKind::App),
                    }
                });
            }
            "Escape" => {
                spawn_local(async move {
                    let _ = invoke(
                        "hide_window",
                        serde_wasm_bindgen::to_value(&Empty {}).unwrap(),
                    )
                    .await;
                });
            }
            _ => {}
        }
    };

    let on_input = move |ev| set_query.set(event_target_value(&ev));

    view! {
      <div id="bar">
        <Show when=move || !prefix_text.get().is_empty()>
          <span class="input-prefix">
            { move || format!("{} ", prefix_text.get()) }
          </span>
        </Show>

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
          filtered.get().into_iter().enumerate().map(|(i, cmd)| {
            let is_sel = i == sel;
            view! {
              <li class:is-selected=is_sel>
                {
                  if filter.get().is_none() { cmd.to_string() } else { cmd.name().to_string() }.to_lowercase()
                }
              </li>
            }
          }).collect_view()
        }}
      </ul>
    }
}
