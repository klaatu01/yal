use leptos::prelude::*;
use yal_core::Node;

#[component]
pub fn RenderNode(
    node: Node,
    set_form_values: WriteSignal<std::collections::HashMap<String, serde_json::Value>>,
) -> impl IntoView {
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

        Node::Form(form) => view! { <super::RenderForm form=form set_form_values=set_form_values /> }.into_any(),

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
