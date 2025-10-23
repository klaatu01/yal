use super::fields::{RenderSelectField, RenderSlider, RenderTextField};
use leptos::prelude::*;
use yal_core::{Field, Form};

#[component]
pub fn RenderForm(
    form: Form,
    set_form_values: WriteSignal<std::collections::HashMap<String, serde_json::Value>>,
) -> impl IntoView {
    view! {
      <form class="yal-form">
        {
          form.fields.into_iter().map(|field| {
            match field {
              Field::Text(f) => view! { <RenderTextField field=f set_form_values=set_form_values /> }.into_any(),
              Field::Select(f) => view! { <RenderSelectField field=f set_form_values=set_form_values /> }.into_any(),
              Field::Slider(f) => view! { <RenderSlider field=f set_form_values=set_form_values /> }.into_any(),
            }
          }).collect_view()
        }
      </form>
    }
}
