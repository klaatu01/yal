use leptos::prelude::*;
use std::rc::Rc;
use yal_core::{SelectField, SliderField, TextField};

#[component]
pub fn RenderTextField(
    field: TextField,
    set_form_values: WriteSignal<std::collections::HashMap<String, serde_json::Value>>,
) -> impl IntoView {
    let name = field.name.clone();
    let placeholder = field.placeholder.clone().unwrap_or_default();
    let initial_value = "";

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
}

#[component]
pub fn RenderSelectField(
    field: SelectField,
    set_form_values: WriteSignal<std::collections::HashMap<String, serde_json::Value>>,
) -> impl IntoView {
    let name = field.name.clone();
    let options = field.options.clone();
    let len = options.len();
    let (sel, set_sel) = signal(0usize);

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

    let on_keydown = move |e: web_sys::KeyboardEvent| {
        let key = e.key();
        match key.as_str() {
            "j" | "ArrowDown" => {
                e.prevent_default();
                e.stop_propagation();
                if len > 0 {
                    set_sel.update(|i| *i = (*i + 1).min(len - 1));
                }
            }
            "k" | "ArrowUp" => {
                e.prevent_default();
                e.stop_propagation();
                set_sel.update(|i| *i = i.saturating_sub(1));
            }
            _ => {}
        }
    };

    view! {
      <ul class="results yal-form-control" tabindex="0" role="listbox" aria-label=name.clone() on:keydown=on_keydown>
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
                on:mousemove=move |_| { set_sel.set(i); }
                on:click={
                    let name = name.clone();
                    move |_| {
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
}

#[component]
pub fn RenderSlider(
    field: SliderField,
    set_form_values: WriteSignal<std::collections::HashMap<String, serde_json::Value>>,
) -> impl IntoView {
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

    Effect::new({
        let name = name.clone();
        move |_| {
            set_form_values.update(|m| {
                m.entry(name.clone()).or_insert(serde_json::Value::Number(
                    serde_json::Number::from_f64(initial).unwrap(),
                ));
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
}

#[component]
pub fn RenderButton(
    label: String,
    set_form_values: WriteSignal<std::collections::HashMap<String, serde_json::Value>>,
) -> impl IntoView {
    let button_label = label.clone();
    let on_click = Rc::new(move || {
        set_form_values.update(|m| {
            m.insert(
                "button".to_string(),
                serde_json::Value::String(button_label.clone()),
            );
        });
    });

    view! {
      <button
        type="button"
        class="yal-btn yal-form-control"
        on:click=move |_| {
          on_click();
        }
      >
        { label }
      </button>
    }
}
