use leptos::prelude::*;
use yal_core::{Command, CommandKind};

#[component]
pub fn ResultsList(
    selected: ReadSignal<usize>,
    filtered: Memo<Vec<Command>>,
    filter: ReadSignal<Option<CommandKind>>,
) -> impl IntoView {
    view! {
      <ul class="results">
        { move || {
          let sel = selected.get();
          filtered.get().into_iter().enumerate().map(|(i, cmd)| {
            let is_sel = i == sel;
            view! {
              <li class:is-selected=is_sel>
                { if filter.get().is_none() { cmd.to_string() } else { cmd.name().to_string() }.to_lowercase() }
              </li>
            }
          }).collect_view()
        }}
      </ul>
    }
}
