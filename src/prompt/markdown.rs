use leptos::prelude::*;

#[component]
pub fn RenderMarkdown(md: String) -> impl IntoView {
    view! {
        <div class="yal-md">{ md }</div>
    }
}
