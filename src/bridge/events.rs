use crate::bridge::invoke::{api_respond, get_config, get_theme};
use crate::ui::theme::{apply_font_cfg, apply_theme_cfg, apply_window_cfg};
use leptos::prelude::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::{closure::Closure, JsCast};
use yal_core::{
    AppConfig, FrontendRequest, Prompt, PromptRequest, PromptResponse, Shortcut, Theme,
};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "event"], js_name = listen)]
    async fn tauri_listen(event: &str, callback: &js_sys::Function);
}

pub fn prime_config(set_shortcuts: WriteSignal<Vec<Shortcut>>) {
    leptos::task::spawn_local(async move {
        if let Some(cfg) = get_config().await {
            if let Some(w) = &cfg.window {
                apply_window_cfg(w);
            }
            if let Some(f) = &cfg.font {
                apply_font_cfg(f);
            }
            if let Some(keys_cfg) = &cfg.keys {
                if let Some(shortcuts) = &keys_cfg.shortcuts {
                    set_shortcuts.set(shortcuts.clone());
                }
            }
        }
    });
}

pub fn prime_theme() {
    leptos::task::spawn_local(async move {
        if let Some(t) = get_theme().await {
            apply_theme_cfg(&t);
        }
    });
}

pub fn init_theme_listener() {
    leptos::task::spawn_local(async move {
        let cb = Closure::<dyn FnMut(js_sys::Object)>::new(move |evt_obj: js_sys::Object| {
            if let Ok(payload) = js_sys::Reflect::get(&evt_obj, &JsValue::from_str("payload")) {
                if let Ok(theme) = serde_wasm_bindgen::from_value::<Theme>(payload) {
                    apply_theme_cfg(&theme);
                }
            }
        });
        let _unlisten = tauri_listen("theme://applied", cb.as_ref().unchecked_ref()).await;
        cb.forget();
    });
}

pub fn init_config_listener(set_shortcuts: WriteSignal<Vec<Shortcut>>) {
    leptos::task::spawn_local(async move {
        let cb = Closure::<dyn FnMut(js_sys::Object)>::new(move |evt_obj: js_sys::Object| {
            if let Ok(payload) = js_sys::Reflect::get(&evt_obj, &JsValue::from_str("payload")) {
                if let Ok(cfg) = serde_wasm_bindgen::from_value::<AppConfig>(payload) {
                    if let Some(window_cfg) = &cfg.window {
                        apply_window_cfg(window_cfg);
                    }
                    if let Some(font_cfg) = &cfg.font {
                        apply_font_cfg(font_cfg);
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

pub fn init_cmd_list_listener(
    set_cmd_list: WriteSignal<Vec<yal_core::Command>>,
    reset: impl Fn() + 'static,
) {
    leptos::task::spawn_local(async move {
        let cb = Closure::<dyn FnMut(js_sys::Object)>::new(move |evt_obj: js_sys::Object| {
            if let Ok(payload) = js_sys::Reflect::get(&evt_obj, &JsValue::from_str("payload")) {
                if let Ok(cmds) = serde_wasm_bindgen::from_value::<Vec<yal_core::Command>>(payload)
                {
                    reset();
                    set_cmd_list.set(cmds);
                }
            }
        });
        let _unlisten = tauri_listen("commands://updated", cb.as_ref().unchecked_ref()).await;
        cb.forget();
    });
}

pub fn init_api_listener(
    set_prompt: WriteSignal<Option<PromptRequest>>,
    prompt: ReadSignal<Option<PromptRequest>>,
) {
    leptos::task::spawn_local(async move {
        // prompt:show
        let set_prompt_show = set_prompt;
        let cb_show = Closure::<dyn FnMut(js_sys::Object)>::new(move |evt_obj: js_sys::Object| {
            if let Ok(payload) = js_sys::Reflect::get(&evt_obj, &JsValue::from_str("payload")) {
                if let Ok(req) = serde_wasm_bindgen::from_value::<FrontendRequest<Prompt>>(payload)
                {
                    set_prompt_show.set(Some(PromptRequest {
                        id: req.id,
                        prompt: req.data,
                    }));
                }
            }
        });
        let _u_show = tauri_listen("api://prompt:show", cb_show.as_ref().unchecked_ref()).await;
        cb_show.forget();

        // prompt:state
        let prompt_state = prompt;
        let cb_state =
            Closure::<dyn FnMut(js_sys::Object)>::new(move |_evt_obj: js_sys::Object| {
                if let Some(p) = prompt_state.get() {
                    let response = PromptResponse::State {
                        values: serde_json::json!({}),
                    };
                    leptos::task::spawn_local(async move {
                        api_respond(p.id.clone(), response).await;
                    });
                }
            });
        let _u_state = tauri_listen("api://prompt:state", cb_state.as_ref().unchecked_ref()).await;
        cb_state.forget();

        // prompt:close
        let set_prompt_close = set_prompt;
        let cb_close =
            Closure::<dyn FnMut(js_sys::Object)>::new(move |_evt_obj: js_sys::Object| {
                set_prompt_close.set(None);
            });
        let _u_close = tauri_listen("api://prompt:cancel", cb_close.as_ref().unchecked_ref()).await;
        cb_close.forget();
    });
}
