use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use leptos::task::spawn_local;
use leptos::web_sys;
use leptos::{ev::KeyboardEvent, prelude::*};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use yal_core::{
    AppConfig, Command, CommandKind, Field, Form, Node, Popup, SelectField, Shortcut,
    ShortcutCommand, SliderField, TextField, Theme,
};

use leptos::prelude::AnyView; // type-erased view to unify match/if arms
use leptos::prelude::CollectView;

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

fn init_theme_listener() {
    spawn_local(async move {
        let cb = Closure::<dyn FnMut(js_sys::Object)>::new(move |evt_obj: js_sys::Object| {
            if let Ok(payload) = js_sys::Reflect::get(&evt_obj, &JsValue::from_str("payload")) {
                if let Ok(theme) = serde_wasm_bindgen::from_value::<Theme>(payload) {
                    crate::ui::apply_theme_cfg(&theme);
                }
            }
        });

        let _unlisten = tauri_listen("theme://applied", cb.as_ref().unchecked_ref()).await;
        cb.forget();
    });
}

fn init_config_listener(set_shortcuts: WriteSignal<Vec<Shortcut>>) {
    spawn_local(async move {
        let cb = Closure::<dyn FnMut(js_sys::Object)>::new(move |evt_obj: js_sys::Object| {
            if let Ok(payload) = js_sys::Reflect::get(&evt_obj, &JsValue::from_str("payload")) {
                if let Ok(cfg) = serde_wasm_bindgen::from_value::<AppConfig>(payload) {
                    if let Some(window_cfg) = &cfg.window {
                        crate::ui::apply_window_cfg(window_cfg);
                    }
                    if let Some(font_cfg) = &cfg.font {
                        crate::ui::apply_font_cfg(font_cfg);
                    }
                    if let Some(keys_cfg) = &cfg.keys {
                        if let Some(shortcuts) = &keys_cfg.shortcuts {
                            set_shortcuts.set(shortcuts.clone());
                        }
                    }
                }
            }
        });

        let _unlisten = tauri_listen("config://updated", cb.as_ref().unchecked_ref()).await;
        cb.forget();
    });
}

fn init_cmd_list_listener(set_cmd_list: WriteSignal<Vec<Command>>, reset: impl Fn() + 'static) {
    spawn_local(async move {
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

fn init_popup_listeners(set_popup: WriteSignal<Option<Popup>>) {
    spawn_local(async move {
        let on_show = Closure::<dyn FnMut(js_sys::Object)>::new({
            move |evt_obj: js_sys::Object| {
                if let Ok(payload) = js_sys::Reflect::get(&evt_obj, &JsValue::from_str("payload")) {
                    if let Ok(popup) = serde_wasm_bindgen::from_value::<Popup>(payload) {
                        set_popup.set(Some(popup));
                    }
                }
            }
        });
        let on_close = Closure::<dyn FnMut(js_sys::Object)>::new({
            move |_evt_obj: js_sys::Object| {
                set_popup.set(None);
            }
        });

        let _u1 = tauri_listen("popup://show", on_show.as_ref().unchecked_ref()).await;
        let _u2 = tauri_listen("popup://close", on_close.as_ref().unchecked_ref()).await;
        on_show.forget();
        on_close.forget();
    });
}

fn load_config(set_shortcuts: WriteSignal<Vec<Shortcut>>) {
    spawn_local(async move {
        let config = invoke(
            "get_config",
            serde_wasm_bindgen::to_value(&Empty {}).unwrap(),
        )
        .await;
        if let Ok(cfg) = serde_wasm_bindgen::from_value::<AppConfig>(config) {
            if let Some(window_cfg) = &cfg.window {
                crate::ui::apply_window_cfg(window_cfg);
            }
            if let Some(font_cfg) = &cfg.font {
                crate::ui::apply_font_cfg(font_cfg);
            }
            if let Some(keys_cfg) = &cfg.keys {
                if let Some(shortcuts) = &keys_cfg.shortcuts {
                    set_shortcuts.set(shortcuts.clone());
                }
            }
        }
    });
}

fn load_theme() {
    spawn_local(async move {
        let config = invoke(
            "get_theme",
            serde_wasm_bindgen::to_value(&Empty {}).unwrap(),
        )
        .await;
        if let Ok(cfg) = serde_wasm_bindgen::from_value::<Theme>(config) {
            crate::ui::apply_theme_cfg(&cfg);
        }
    });
}

/* ------------------------------ Filtering ----------------------------------- */

fn fuzzy_filter_commands(cmds: &[Command], query: &str) -> Vec<Command> {
    let matcher = SkimMatcherV2::default();
    let mut scored: Vec<(Command, i64)> = cmds
        .iter()
        .filter_map(|cmd| {
            matcher
                .fuzzy_match(&cmd.name(), query)
                .map(|score| (cmd.clone(), score))
        })
        .collect();

    scored.sort_by(|a, b| {
        b.1.cmp(&a.1)
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

/* ------------------------------ Popup renderers ------------------------------ */

#[component]
fn RenderNode(
    node: Node,
    set_form_values: WriteSignal<std::collections::HashMap<String, serde_json::Value>>,
) -> AnyView {
    match node {
        Node::VStack { gap, children } => view! {
            <div class="yal-vstack" style=move || format!("gap:{}rem;", gap.unwrap_or(0.5))>
              {
                children.into_iter()
                  .map(|n| view!{ <RenderNode node=n set_form_values=set_form_values /> })
                  .collect_view()
              }
            </div>
        }.into_any(),

        Node::HStack { gap, children } => view! {
            <div class="yal-hstack" style=move || format!("gap:{}rem;", gap.unwrap_or(0.5))>
              {
                children.into_iter()
                  .map(|n| view!{ <RenderNode node=n set_form_values=set_form_values /> })
                  .collect_view()
              }
            </div>
        }.into_any(),

        Node::Grid { cols, gap, children } => view! {
            <div class="yal-grid"
                 style=move || format!("grid-template-columns:repeat({cols},1fr);gap:{}rem;", gap.unwrap_or(0.5))>
              {
                children.into_iter()
                  .map(|n| view!{ <RenderNode node=n set_form_values=set_form_values /> })
                  .collect_view()
              }
            </div>
        }.into_any(),

        Node::Markdown { md } => view! { <div class="yal-md">{ md }</div> }.into_any(),

        Node::Text { text, .. } => view! { <div class="yal-text">{ text }</div> }.into_any(),

        Node::Form(form) => view! {
            <RenderForm
              form=form
                set_form_values=set_form_values
            />
        }.into_any(),

        Node::Html { html } => view! { <div class="yal-md" inner_html=html></div> }.into_any(),

        Node::Image { src, alt, w, h } => {
            let style = format!(
                "{}{}",
                w.map(|v| format!("width:{v}px;")).unwrap_or_default(),
                h.map(|v| format!("height:{v}px;")).unwrap_or_default()
            );
            view! { <img class="yal-img" src=src alt=alt.unwrap_or_default() style=style /> }.into_any()
        }
    }
}

// --- Select ---
// --- Select as list (command-palette style) ---
#[component]
fn RenderSelectField(
    field: SelectField,
    set_form_values: WriteSignal<std::collections::HashMap<String, serde_json::Value>>,
) -> AnyView {
    let name = field.name.clone();
    let options = field.options.clone();
    let len = options.len();

    // local selection index
    let (sel, set_sel) = signal(0usize);

    // seed initial value (first option or null)
    Effect::new({
        let name = name.clone();
        let options = options.clone();
        move |_| {
            let initial_val = options
                .first()
                .map(|o| o.value.clone())
                .unwrap_or(serde_json::Value::Null);
            set_form_values.update(|m| {
                m.entry(name.clone()).or_insert(initial_val);
            });
        }
    });

    // whenever sel changes, update the form value
    Effect::new({
        let name = name.clone();
        let options = options.clone();
        move |_| {
            let i = sel.get();
            if let Some(opt) = options.get(i) {
                set_form_values.update(|m| {
                    m.insert(name.clone(), opt.value.clone());
                });
            }
        }
    });

    // inside RenderSelectField
    let on_keydown = move |e: web_sys::KeyboardEvent| {
        let key = e.key();
        match key.as_str() {
            // vim-style
            "j" => {
                e.prevent_default();
                e.stop_propagation();
                if len > 0 {
                    set_sel.update(|i| *i = (*i + 1).min(len - 1));
                }
            }
            "k" => {
                e.prevent_default();
                e.stop_propagation();
                set_sel.update(|i| *i = i.saturating_sub(1));
            }

            // arrows still work if the list is focused
            "ArrowDown" => {
                e.prevent_default();
                e.stop_propagation();
                if len > 0 {
                    set_sel.update(|i| *i = (*i + 1).min(len - 1));
                }
            }
            "ArrowUp" => {
                e.prevent_default();
                e.stop_propagation();
                set_sel.update(|i| *i = i.saturating_sub(1));
            }

            // let Enter bubble to the popup's global submit
            _ => {}
        }
    };

    view! {
      // Use your "results" list look & feel
      <ul class="results yal-form-control"
          tabindex="0"
          role="listbox"
          aria-label=name.clone()
          on:keydown=on_keydown
      >
        {
          options.into_iter().enumerate().map({
            let name = name.clone();
            move |(i, opt)| {
            let is_sel = move || sel.get() == i;
            let label = opt.label.clone();
            let value = opt.value.clone();
            view! {
              <li
                role="option"
                aria-selected=move || is_sel().to_string()
                class:is-selected=move || is_sel()
                on:mousemove=move |_| { set_sel.set(i); }      // hover moves highlight (optional)
                on:click={
                let name = name.clone();
                move |_| {                             // click selects (and value Effect updates)
                    set_sel.set(i);
                    set_form_values.update(|m| { m.insert(name.clone(), value.clone()); });
                }
                }
              >
                { label.to_lowercase() }
              </li>
            }
          }}).collect_view()
        }
      </ul>
    }
    .into_any()
}

#[component]
fn RenderTextField(
    field: TextField,
    set_form_values: WriteSignal<std::collections::HashMap<String, serde_json::Value>>,
) -> AnyView {
    let name = field.name.clone();
    let placeholder = field.placeholder.clone().unwrap_or_default();
    let initial_value = "";

    // seed initial value once
    Effect::new({
        let name = name.clone();
        move |_| {
            set_form_values.update(|m| {
                m.entry(name.clone())
                    .or_insert(serde_json::Value::String(initial_value.to_string()));
            });
        }
    });

    let on_input = Rc::new(move |name: String, v: String| {
        set_form_values.update(|m| {
            m.insert(name, serde_json::Value::String(v));
        });
    });

    view! {
      <input
        type="text"
        class="yal-input yal-form-control"
        name=name.clone()
        prop:value=initial_value
        placeholder=placeholder
        prop:spellcheck=false
        prop:autocorrect="off"
        prop:autocapitalize="off"
        autocomplete="off"
        on:input=move |ev| {
          let v = event_target_value(&ev);
          on_input(name.clone(), v);
        }
      />
    }
    .into_any()
}

#[component]
fn RenderSlider(
    field: SliderField,
    set_form_values: WriteSignal<std::collections::HashMap<String, serde_json::Value>>,
) -> AnyView {
    let name = field.name.clone();
    let min = field.min;
    let max = field.max;
    let step = field.step;
    let initial = field.value.unwrap_or(min);

    let on_input = Rc::new(move |name: String, v: String| {
        if let Ok(val) = v.parse::<f64>() {
            set_form_values.update(|m| {
                m.insert(
                    name,
                    serde_json::Value::Number(serde_json::Number::from_f64(val).unwrap()),
                );
            });
        }
    });

    view! {
        <input
          type="range"
          class="yal-form-control"
          name=name.clone()
          prop:min=min
          prop:max=max
          prop:step=step
          prop:value=initial
          on:input=move |ev| {
            let v = event_target_value(&ev);
            on_input(name.clone(), v);
          }
        />
    }
    .into_any()
}

#[component]
fn RenderForm(
    form: Form,
    set_form_values: WriteSignal<std::collections::HashMap<String, serde_json::Value>>,
) -> AnyView {
    view! {
          <form class="yal-form">
            {
              form.fields.into_iter().map(|field| {
                match field {
                  Field::Text(f) => {
                    view! { <RenderTextField field=f set_form_values=set_form_values /> }.into_any()
                  }
                  Field::Select(f) => {
                    view! { <RenderSelectField field=f set_form_values=set_form_values /> }.into_any()
                  }
                  Field::Slider(f) => {
                    view! { <RenderSlider field=f set_form_values=set_form_values /> }.into_any()
                  }
                }
              }).collect_view()
            }
          </form>
    }
    .into_any()
}

/* ------------------------- Focus & slider helpers --------------------------- */

fn focus_first_form_control_now() {
    let Some(win) = web_sys::window() else { return };
    let Some(doc) = win.document() else { return };
    let list = doc.get_elements_by_class_name("yal-form-control");
    if list.length() == 0 {
        return;
    }
    if let Some(el) = list.item(0) {
        if let Some(he) = el.dyn_ref::<web_sys::HtmlElement>() {
            let _ = he.focus();
        }
    }
}

fn raf_focus_first_form_control() {
    if let Some(win) = web_sys::window() {
        let cb = Closure::<dyn FnMut()>::new(focus_first_form_control_now);
        let _ = win.request_animation_frame(cb.as_ref().unchecked_ref());
        cb.forget();
    }
}

fn focus_move(delta: i32) {
    if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
        if let Ok(list) = doc.query_selector_all(".yal-form-control") {
            let len = list.length() as i32;
            if len == 0 {
                return;
            }

            // find active index
            let active = doc.active_element();
            let mut idx: i32 = -1;
            for i in 0..len {
                if let Some(el) = list.item(i as u32) {
                    if let Some(ae) = &active {
                        if ae.is_same_node(Some(&el)) {
                            idx = i;
                            break;
                        }
                    }
                }
            }
            let next = if idx < 0 {
                0
            } else {
                let mut n = idx + delta;
                if n < 0 {
                    n = 0;
                }
                if n >= len {
                    n = len - 1;
                }
                n
            };

            if let Some(el) = list.item(next as u32) {
                let _ = el.dyn_ref::<web_sys::HtmlElement>().map(|h| h.focus());
            }
        }
    }
}

fn active_is_range() -> Option<web_sys::HtmlInputElement> {
    let doc = web_sys::window()?.document()?;
    let ae = doc.active_element()?;
    let input: web_sys::HtmlInputElement = ae.dyn_into().ok()?;
    if input.type_().to_lowercase() == "range" {
        Some(input)
    } else {
        None
    }
}

fn nudge_active_slider(delta: f64) {
    if let Some(input) = active_is_range() {
        let step = input.step().parse::<f64>().unwrap_or(1.0);
        let min = input.min().parse::<f64>().unwrap_or(0.0);
        let max = input.max().parse::<f64>().unwrap_or(100.0);
        let cur = input.value().parse::<f64>().unwrap_or(min);
        let mut v = cur + delta * step;
        if v < min {
            v = min;
        }
        if v > max {
            v = max;
        }
        input.set_value(&v.to_string());
        if let Ok(ev) = web_sys::Event::new("input") {
            let _ = input.dispatch_event(&ev);
        }
    }
}

/* -------------------------------- Popup shell ------------------------------- */

#[derive(Serialize, Deserialize)]
pub struct PopupReponseArgs {
    id: String,
    value: serde_json::Value,
}

#[component]
fn PopupView(popup: ReadSignal<Option<Popup>>, set_popup: WriteSignal<Option<Popup>>) -> AnyView {
    let p = popup.get().unwrap();
    let (form_values, set_form_values) =
        signal(std::collections::HashMap::<String, serde_json::Value>::new());

    // Popup-level key handling: Escape, Enter, ctrl+n/p, h/l, slider nudge
    let popup_keydown = move |e: web_sys::KeyboardEvent| {
        let key = e.key();
        match key.as_str() {
            "Escape" => {
                e.prevent_default();
                set_popup.set(None);
            }
            "Enter" => {
                e.prevent_default();

                // Prefer submitting the first <Form> (this path merges the current slider value via `submit_action`)
                if let Some(p) = popup.get() {
                    spawn_local(async move {
                        let values = form_values.get();
                        let args = serde_wasm_bindgen::to_value(&PopupReponseArgs {
                            id: p.id.unwrap(),
                            value: serde_json::Value::Object(values.into_iter().collect()),
                        })
                        .unwrap();
                        let _ = invoke("plugin_api_response_handler", args).await;
                        set_popup.set(None);
                    });
                }
            }

            // ctrl+n / ctrl+p → move focus
            "n" if e.ctrl_key() => {
                focus_move(1);
            }
            "p" if e.ctrl_key() => {
                focus_move(-1);
            }

            // h / l → either nudge slider if focused, else move focus
            "h" => {
                if active_is_range().is_some() {
                    nudge_active_slider(-1.0);
                }
            }
            "l" => {
                if active_is_range().is_some() {
                    nudge_active_slider(1.0);
                }
            }

            // ArrowLeft/Right on range are handled by the browser already.
            _ => {}
        }
    };

    Effect::new(move |_| {
        raf_focus_first_form_control();
    });

    Effect::new(move |_| {
        let _ = popup.get(); // track
        raf_focus_first_form_control();
    });

    view! {
      <div class="yal-popup-overlay" on:keydown=popup_keydown tabindex="0">
          <div
            class="yal-popup"
            style=move || {
              let w = p.width.unwrap_or(75.0);
              let height_css = if let Some(h) = p.height {
                  format!("height:{}%;", h)
              } else {
                  "height:auto;".to_string()
              };
              format!("width:{}%;{}", w, height_css)
            }
          >
          <div class="yal-popup-header">
            { p.title.clone().unwrap_or_default() }
          </div>
          <div class="yal-popup-body">
            {
              p.content.iter().cloned()
                .map(|n| view!{ <RenderNode node=n set_form_values=set_form_values.clone() /> })
                .collect_view()
            }
          </div>
        </div>
      </div>
    }
    .into_any()
}

/* ---------------------------------- App ------------------------------------- */

fn focus_search_now() {
    let Some(win) = web_sys::window() else { return };
    let Some(doc) = win.document() else { return };
    if let Some(el) = doc.get_element_by_id("search") {
        if let Some(he) = el.dyn_ref::<web_sys::HtmlElement>() {
            let _ = he.focus();
        }
    }
}

/// rAF to make sure the input is in the DOM & visible before focusing.
fn raf_focus_search() {
    if let Some(win) = web_sys::window() {
        let cb = Closure::<dyn FnMut()>::new(focus_search_now);
        let _ = win.request_animation_frame(cb.as_ref().unchecked_ref());
        cb.forget();
    }
}

fn norm_token(t: &str) -> &str {
    match t {
        // modifiers
        "control" | "ctrl" => "ctrl",
        "alt" | "option" | "opt" => "alt",
        "shift" => "shift",
        "cmd" | "command" | "meta" | "super" | "win" => "cmd",

        // special keys (aliases)
        "esc" | "escape" => "esc",
        "enter" | "return" => "enter",
        "space" => "space",
        "pgup" | "pageup" => "pageup",
        "pgdn" | "pagedown" => "pagedown",
        "arrowup" | "up" => "up",
        "arrowdown" | "down" => "down",
        "arrowleft" | "left" => "left",
        "arrowright" | "right" => "right",
        "plus" | "+" => "plus",
        _ => t,
    }
}

// Normalize a *config* string like "Ctrl + S" → "ctrl+s"
fn normalize_combo_string(s: &str) -> String {
    let mut parts: Vec<String> = s
        .split('+')
        .map(|p| norm_token(&p.trim().to_ascii_lowercase()).to_string())
        .collect();

    // Extract modifiers, dedupe, and sort for a stable order
    let mut mods: Vec<String> = vec![];
    let mut key: Option<String> = None;

    for p in parts.drain(..) {
        match p.as_str() {
            "ctrl" | "alt" | "shift" | "cmd" => {
                if !mods.contains(&p) {
                    mods.push(p.to_string())
                }
            }
            other => {
                // the final non-mod token is considered the key
                key = Some(other.to_string());
            }
        }
    }

    mods.sort_unstable_by(|a, b| {
        ["ctrl", "alt", "shift", "cmd"]
            .iter()
            .position(|x| x == a)
            .cmp(&["ctrl", "alt", "shift", "cmd"].iter().position(|x| x == b))
    });

    let k = key.unwrap_or_default();
    if mods.is_empty() {
        k
    } else {
        format!("{}+{}", mods.join("+"), k)
    }
}

// Build canonical combo from a KeyboardEvent
fn combo_from_event(ev: &KeyboardEvent) -> Option<String> {
    // ignore pure modifier presses
    let raw_key = ev.key(); // e.g. "s", "S", "Escape", "ArrowUp", "+", "F5"
    let lower = raw_key.to_ascii_lowercase();

    // map to canonical key token
    let key = match lower.as_str() {
        "shift" | "control" | "alt" | "meta" => return None, // just a modifier, no key yet
        "escape" => "esc".to_string(),
        "enter" | "return" => "enter".to_string(),
        " " | "spacebar" | "space" => "space".to_string(),
        "arrowup" => "up".to_string(),
        "arrowdown" => "down".to_string(),
        "arrowleft" => "left".to_string(),
        "arrowright" => "right".to_string(),
        "+" => "plus".to_string(),
        // function keys: key could be "F1".."F24"
        k if k.starts_with('f') && k.len() <= 3 && k[1..].chars().all(|c| c.is_ascii_digit()) => {
            k.to_string()
        }
        // single printable
        k if k.len() == 1 => k.to_string(),
        // common names
        "tab" | "backspace" | "delete" | "insert" | "home" | "end" | "pageup" | "pagedown"
        | "minus" | "equals" | "comma" | "period" | "slash" | "backslash" | "semicolon"
        | "quote" | "bracketleft" | "bracketright" | "grave" => lower.clone(),
        _ => lower.clone(), // fall back (you can tighten this if desired)
    };

    let mut mods: Vec<&str> = vec![];
    if ev.ctrl_key() {
        mods.push("ctrl");
    }
    if ev.alt_key() {
        mods.push("alt");
    }
    if ev.shift_key() {
        mods.push("shift");
    }
    if ev.meta_key() {
        mods.push("cmd");
    } // macOS Command

    mods.sort_unstable_by(|a, b| {
        ["ctrl", "alt", "shift", "cmd"]
            .iter()
            .position(|x| x == a)
            .cmp(&["ctrl", "alt", "shift", "cmd"].iter().position(|x| x == b))
    });

    Some(if mods.is_empty() {
        key
    } else {
        format!("{}+{}", mods.join("+"), key)
    })
}

#[component]
pub fn App() -> impl IntoView {
    let (cmds, set_cmd_list) = signal(Vec::<Command>::new());
    let (query, set_query) = signal(String::new());
    let (selected, set_selected) = signal(0usize);
    let (filter, set_filter) = signal(Option::<CommandKind>::None);
    let (shortcuts, set_shortcuts) = signal(Vec::<Shortcut>::new());

    let reset = move || {
        set_selected.set(0);
        set_query.set(String::new());
    };

    let (popup, set_popup) = signal::<Option<Popup>>(None);

    load_config(set_shortcuts);
    load_theme();
    init_config_listener(set_shortcuts);
    init_theme_listener();
    init_cmd_list_listener(set_cmd_list, reset);
    init_popup_listeners(set_popup);

    let filtered = Memo::new(move |_| {
        let q = query.get();
        let list = cmds.get();
        let filter = filter.get();
        filter_memoized_commands(&list, &q, selected.get(), &set_selected, filter)
    });

    let prefix_text = Memo::new(move |_| match filter.get() {
        Some(CommandKind::App) => "open".to_string(),
        Some(CommandKind::Switch) => "switch".to_string(),
        Some(CommandKind::Theme) => "theme".to_string(),
        Some(CommandKind::Plugin) => "plugin".to_string(),
        None => String::new(),
    });

    let shortcut_map = Memo::new(move |_| {
        let mut m: HashMap<String, ShortcutCommand> = HashMap::new();
        for s in shortcuts.get() {
            let k = normalize_combo_string(&s.combination);
            m.insert(k, s.command.clone());
        }
        m
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

    // Key handlers when the popup is NOT open (launcher palette)
    let pallet_keys = move |ev: KeyboardEvent| {
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
            "t" if ev.ctrl_key() => {
                ev.prevent_default();
                set_filter.update(|f| {
                    *f = match f {
                        Some(CommandKind::Theme) => None,
                        _ => Some(CommandKind::Theme),
                    }
                });
            }
            "e" if ev.ctrl_key() => {
                ev.prevent_default();
                set_filter.update(|f| {
                    *f = match f {
                        Some(CommandKind::Plugin) => None,
                        _ => Some(CommandKind::Plugin),
                    }
                });
            }
            "Escape" => {
                // hide entire window only when no popup is open
                spawn_local(async move {
                    let _ = invoke(
                        "hide_window",
                        serde_wasm_bindgen::to_value(&Empty {}).unwrap(),
                    )
                    .await;
                });
            }
            _ => {
                if let Some(combo) = combo_from_event(&ev) {
                    if let Some(sc) = shortcut_map.get().get(&combo).cloned() {
                        ev.prevent_default();
                        let cmd = Command::Plugin {
                            plugin_name: sc.plugin,
                            command_name: sc.command,
                            args: None,
                        };
                        spawn_local(async move {
                            let args = serde_wasm_bindgen::to_value(&RunCmdArgs { cmd }).unwrap();
                            let _ = invoke("run_cmd", args).await;
                        });
                    }
                }
            }
        }
    };

    // Key navigation on the input
    let on_key = move |ev: KeyboardEvent| {
        if popup.get().is_some() {
            // When popup is open, keys are handled on the popup overlay.
            // Let them bubble; no action here.
        } else {
            pallet_keys(ev);
        }
    };

    let on_input = move |ev| set_query.set(event_target_value(&ev));

    Effect::new(move |_| {
        if popup.get().is_none() {
            raf_focus_search();
        }
    });

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

      <Show when=move || popup.get().is_some()>
        <PopupView popup=popup set_popup=set_popup />
      </Show>
    }
}
