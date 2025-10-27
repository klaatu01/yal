mod fields;
mod form;
mod markdown;
mod render_node;

pub use fields::RenderButton;
pub use form::RenderForm;
pub use markdown::RenderMarkdown;
pub use render_node::RenderNode;

use crate::bridge::invoke::api_respond;
use crate::utils::focus::{
    active_is_range, focus_move, nudge_active_slider, raf_focus_first_form_control,
};
use leptos::prelude::*;
use yal_core::{PromptRequest, PromptResponse};

#[component]
pub fn PromptView(
    prompt: ReadSignal<Option<PromptRequest>>,
    set_prompt: WriteSignal<Option<PromptRequest>>,
    set_form_values: WriteSignal<std::collections::HashMap<String, serde_json::Value>>,
    form_values: ReadSignal<std::collections::HashMap<String, serde_json::Value>>,
) -> impl IntoView {
    let popup_keydown = move |e: web_sys::KeyboardEvent| {
        let key = e.key();
        match key.as_str() {
            "Escape" => {
                e.prevent_default();
                if let Some(p) = prompt.get() {
                    leptos::task::spawn_local(async move {
                        let response = PromptResponse::Cancel;
                        api_respond(p.id.clone(), response).await;
                        set_prompt.set(None);
                    });
                }
            }
            "Enter" => {
                e.prevent_default();
                if let Some(p) = prompt.get() {
                    leptos::task::spawn_local(async move {
                        let values = form_values.get();
                        let response = PromptResponse::Submit {
                            values: serde_json::to_value(&values).unwrap(),
                        };
                        api_respond(p.id.clone(), response).await;
                        set_prompt.set(None);
                        set_form_values.set(std::collections::HashMap::new());
                    });
                }
            }
            "n" if e.ctrl_key() => {
                focus_move(1);
            }
            "p" if e.ctrl_key() => {
                focus_move(-1);
            }
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
            _ => {}
        }
    };

    Effect::new(move |_| {
        raf_focus_first_form_control();
    });
    Effect::new(move |_| {
        let _ = prompt.get();
        raf_focus_first_form_control();
    });

    let p = move || prompt.get().unwrap();

    view! {
      <div class="yal-popup-overlay" on:keydown=popup_keydown tabindex="0">
        <div class="yal-popup"
          style=move || {
            let w = p().prompt.width.unwrap_or(75.0);
            let height_css = if let Some(h) = p().prompt.height {
              format!("height:{}%;", h)
            } else {
              "height:auto;".to_string()
            };
            format!("width:{}%;{}", w, height_css)
          }
        >
          <div class="yal-popup-header">
            { p().prompt.title.clone().unwrap_or_default() }
          </div>

          <div class="yal-popup-body">
            {
              p().prompt.content.iter().cloned()
                .map(|n| view!{ <RenderNode node=n set_form_values=set_form_values /> })
                .collect_view()
            }
            {
              move || {
                if p().prompt.contains_input_fields() {
                    ().into_any()
                } else {
                  view! {
                    <br/>
                    <div class="yal-form">
                      <RenderButton label="close".to_string() set_form_values=set_form_values />
                    </div>
                  }.into_any()
                }
              }
            }
          </div>
        </div>
      </div>
    }
}
