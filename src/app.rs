use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use leptos::task::spawn_local;
use leptos::web_sys;
use leptos::{ev::KeyboardEvent, prelude::*};
use serde::{Deserialize, Serialize};
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use yal_core::{
    Action, AppConfig, Command, CommandKind, Field, Form, Node, Popup, Presentation, Theme,
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

fn init_config_listener() {
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

fn load_config() {
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
    on_input: Rc<dyn Fn(String, String)>,
    submit_action: Rc<dyn Fn(Action)>,
) -> AnyView {
    match node {
        Node::VStack { gap, children } => view! {
            <div class="yal-vstack" style=move || format!("gap:{}rem;", gap.unwrap_or(0.5))>
              {
                children.into_iter()
                  .map(|n| view!{ <RenderNode node=n on_input=on_input.clone() submit_action=submit_action.clone() /> })
                  .collect_view()
              }
            </div>
        }.into_any(),

        Node::HStack { gap, children } => view! {
            <div class="yal-hstack" style=move || format!("gap:{}rem;", gap.unwrap_or(0.5))>
              {
                children.into_iter()
                  .map(|n| view!{ <RenderNode node=n on_input=on_input.clone() submit_action=submit_action.clone() /> })
                  .collect_view()
              }
            </div>
        }.into_any(),

        Node::Grid { cols, gap, children } => view! {
            <div class="yal-grid"
                 style=move || format!("grid-template-columns:repeat({cols},1fr);gap:{}rem;", gap.unwrap_or(0.5))>
              {
                children.into_iter()
                  .map(|n| view!{ <RenderNode node=n on_input=on_input.clone() submit_action=submit_action.clone() /> })
                  .collect_view()
              }
            </div>
        }.into_any(),

        Node::Markdown { md } => view! { <div class="yal-md">{ md }</div> }.into_any(),

        Node::Text { text, .. } => view! { <div class="yal-text">{ text }</div> }.into_any(),

        Node::Form(form) => view! {
            <RenderForm
              form=form
              on_input=on_input.clone()
              on_submit=submit_action.clone()
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

#[component]
fn RenderForm(
    form: Form,
    on_input: Rc<dyn Fn(String, String)>,
    on_submit: Rc<dyn Fn(Action)>,
) -> AnyView {
    let submit_on_enter = form.submit_on_enter.unwrap_or(true);

    view! {
          <form class="yal-form" on:submit=move |ev| {
              ev.prevent_default();
              (on_submit)(form.submit.clone());
            }>
            {
              form.fields.into_iter().map(|f| match f {
                Field::Text { name, label, placeholder, multiline, rows, .. } => {
                  let label_v = label.unwrap_or_else(|| name.clone());
                  if multiline.unwrap_or(false) {
                    view!{
                      <label>{label_v}
                        <textarea
                          class="yal-textarea yal-form-control"
                          rows=rows.unwrap_or(4)
                          on:input={
                          let on_input = on_input.clone();
                          move |e| on_input(name.clone(), event_target_value(&e))
                          }
                          placeholder=placeholder.unwrap_or_default()
                        />
                      </label>
                    }.into_any()
                  } else {
                    view!{
                      <label>{label_v}
                        <input
                          class="yal-input yal-form-control"
                          type="text"
                          placeholder=placeholder.unwrap_or_default()
                          on:input={
                          let on_input = on_input.clone();
                          move |e| on_input(name.clone(), event_target_value(&e))
                          }
                          on:keydown={
                          let on_submit = on_submit.clone();
                          let submit = form.submit.clone();
                          move |ke: KeyboardEvent| {
                            if submit_on_enter && ke.key() == "Enter" {
                              ke.prevent_default();
                              on_submit(submit.clone());
                            }
                          }
                          }
                        />
                      </label>
                    }.into_any()
                  }
                }

                // -------- Slider support + h/l nudging ----------
    Field::Slider { name, label, min, max, step, value, show_value } => {
      let initial = value.unwrap_or(min);
      let (cur, set_cur) = signal(initial);
      let stp = step;
      let mn = min;
      let mx = max;

      {
        let name = name.clone();
        let on_input = on_input.clone();
        Effect::new(move |_| {
          on_input(name.clone(), initial.to_string());
        });
      }

      let fmt_val = move || {
          let v = cur.get();
          if (v - v.round()).abs() < 1e-9 { format!("{:.0}%", v) }
          else { format!("{}%", v) }
      };

      // helper: adjust and emit input
      let adjust = {
        let name = name.clone();
        let on_input = on_input.clone();
        move |delta: f64, elem: web_sys::HtmlInputElement| {
          let mut v = cur.get() + delta;
          if v < mn { v = mn; }
          if v > mx { v = mx; }
          set_cur.set(v);
          elem.set_value(&v.to_string());
          if let Ok(ev) = web_sys::Event::new("input") {
            let _ = elem.dispatch_event(&ev);
          }
          on_input(name.clone(), v.to_string());
        }
      };

      view! {
        <label>
          { label.clone().unwrap_or_else(|| name.clone()) }
          <div class="yal-slider-row">
            <input
              class="yal-slider yal-form-control"
              type="range"
              prop:min=mn
              prop:max=mx
              prop:step=stp
              prop:value=cur.get()
              on:input={
                let on_input = on_input.clone();
                move |e| {
                  if let Ok(v) = event_target_value(&e).parse::<f64>() {
                    set_cur.set(v);
                    on_input(name.clone(), v.to_string());
                  }
                }
              }
              on:keydown=move |ke: KeyboardEvent| {
                let key = ke.key();
                if key == "h" || key == "l" {
                  if let Some(t) = ke.target() {
                    if let Ok(input) = t.dyn_into::<web_sys::HtmlInputElement>() {
                      ke.prevent_default();
                      let delta = if key == "h" { -stp } else { stp };
                      adjust(delta, input);
                    }
                  }
                }
              }
            />
            {
              move || if show_value.unwrap_or(true) {
                view!{ <span class="yal-slider-val">{ fmt_val() }</span> }.into_any()
              } else {
                view!{ <span></span> }.into_any()
              }
            }
          </div>
        </label>
      }.into_any()
    }

                // TODO: Number / Select / Checkbox / RadioGroup / Hidden
                _ => view!{ <div>"unsupported field"</div> }.into_any(),
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

#[component]
fn PopupView(popup: ReadSignal<Option<Popup>>, set_popup: WriteSignal<Option<Popup>>) -> AnyView {
    let p = popup.get().unwrap();
    let (form_values, set_form_values) =
        signal(std::collections::HashMap::<String, serde_json::Value>::new());

    // cloneable handlers
    let on_input = Rc::new(move |name: String, v: String| {
        set_form_values.update(|m| {
            m.insert(name, serde_json::Value::String(v));
        });
    });

    let submit_action = Rc::new(move |action: Action| {
        let args = match &action {
            Action::Command { args, .. } => args.clone(),
            _ => serde_json::Value::Null,
        };
        let fields = serde_json::to_value(form_values.get()).unwrap_or(serde_json::json!({}));
        let merged = match args {
            serde_json::Value::Object(mut m) => {
                m.insert("fields".into(), fields);
                serde_json::Value::Object(m)
            }
            _ => serde_json::json!({ "fields": fields }),
        };

        if let Action::Command {
            plugin,
            command,
            presentation,
            ..
        } = action
        {
            let cmd = Command::Plugin {
                plugin_name: plugin,
                command_name: command,
                args: Some(merged),
            };
            spawn_local(async move {
                let args = serde_wasm_bindgen::to_value(&RunCmdArgs { cmd }).unwrap();
                let _ = invoke("run_cmd", args).await;
            });
            match presentation {
                Presentation::ClosePopup => set_popup.set(None),
                Presentation::KeepPopup | Presentation::ReplacePopup => { /* backend will emit show/close */
                }
            }
        }
    });

    let _submit_action = submit_action.clone();
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
                    if let Some(submit) = find_first_form_submit(&p.content) {
                        let submit_action_enter = _submit_action.clone();
                        (submit_action_enter)(submit);
                        return;
                    }

                    // Fallback: if no form exists, use the first footer action (old behavior)
                    if let Some(Action::Command {
                        plugin,
                        command,
                        args,
                        ..
                    }) = p.actions.first().cloned()
                    {
                        let cmd = Command::Plugin {
                            plugin_name: plugin,
                            command_name: command,
                            args: Some(args),
                        };
                        spawn_local(async move {
                            let args = serde_wasm_bindgen::to_value(&RunCmdArgs { cmd }).unwrap();
                            let _ = invoke("run_cmd", args).await;
                        });
                    }
                }
            }

            // ctrl+n / ctrl+p → move focus
            "n" if e.ctrl_key() => {
                e.prevent_default();
                focus_move(1);
            }
            "p" if e.ctrl_key() => {
                e.prevent_default();
                focus_move(-1);
            }

            // h / l → either nudge slider if focused, else move focus
            "h" => {
                if active_is_range().is_some() {
                    e.prevent_default();
                    nudge_active_slider(-1.0);
                } else {
                    e.prevent_default();
                    focus_move(-1);
                }
            }
            "l" => {
                if active_is_range().is_some() {
                    e.prevent_default();
                    nudge_active_slider(1.0);
                } else {
                    e.prevent_default();
                    focus_move(1);
                }
            }

            // ArrowLeft/Right on range are handled by the browser already.
            _ => {}
        }
    };

    // Find the first Form node in the popup (recurses through layout) and return its submit action.
    fn find_first_form_submit(nodes: &[Node]) -> Option<Action> {
        use Node::*;
        for n in nodes {
            match n {
                Form(f) => return Some(f.submit.clone()),
                VStack { children, .. } | HStack { children, .. } | Grid { children, .. } => {
                    if let Some(a) = find_first_form_submit(children) {
                        return Some(a);
                    }
                }
                _ => {}
            }
        }
        None
    }

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
                .map(|n| view!{ <RenderNode node=n on_input=on_input.clone() submit_action=submit_action.clone() /> })
                .collect_view()
            }
          </div>
          <div class="yal-popup-footer">
            {
              p.actions.iter().cloned().map(|a| {
                // Pretty labels for quick-set volume: show "0%/50%/100%" if args.fields.vol present
                let label = match &a {
                  Action::Command{ args, command, .. } => {
                    if let serde_json::Value::Object(map) = args {
                      if let Some(serde_json::Value::Object(fields)) = map.get("fields") {
                        if let Some(v) = fields.get("vol") {
                          if let Some(n) = v.as_f64().or_else(|| v.as_i64().map(|i| i as f64)) {
                            format!("{}%", n.round() as i64)
                          } else { command.clone() }
                        } else { command.clone() }
                      } else { command.clone() }
                    } else { command.clone() }
                  }
                  Action::OpenUrl{ url, .. } => format!("Open {}", url),
                  Action::CopyToClipboard{ .. } => "Copy".into(),
                };
                let submit_action_btn = submit_action.clone();
                view!{
                  <button class="yal-btn yal-form-control" on:click=move |_| submit_action_btn(a.clone())>{label}</button>
                }
              }).collect_view()
            }
          </div>
        </div>
      </div>
    }.into_any()
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

    let (popup, set_popup) = signal::<Option<Popup>>(None);

    load_config();
    load_theme();
    init_config_listener();
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
            _ => {}
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
