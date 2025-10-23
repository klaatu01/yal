pub mod filtering;
pub mod list;

use crate::app::filtering::filter_memoized_commands;
use crate::bridge::events::{
    init_api_listener, init_cmd_list_listener, init_config_listener, init_theme_listener,
    prime_config, prime_theme,
};
use crate::bridge::invoke::{hide_window, run_cmd};
use crate::prompt::PromptView;
use crate::utils::focus::raf_focus_search;
use crate::utils::keys::normalize_combo_string;
use leptos::ev::KeyboardEvent;
use leptos::prelude::*;
use std::collections::HashMap;
use yal_core::{Command, CommandKind, PromptRequest, Shortcut, ShortcutCommand};

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

    let (prompt, set_prompt) = signal::<Option<PromptRequest>>(None);

    // Prime state from backend
    prime_config(set_shortcuts);
    prime_theme();

    // Event listeners
    init_config_listener(set_shortcuts);
    init_theme_listener();
    init_cmd_list_listener(set_cmd_list, reset);
    init_api_listener(set_prompt, prompt);

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
            leptos::task::spawn_local(async move {
                run_cmd(cmd).await;
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
                leptos::task::spawn_local(async move {
                    hide_window().await;
                });
            }
            _ => {
                if let Some(combo) = crate::utils::keys::combo_from_event(&ev) {
                    if let Some(sc) = shortcut_map.get().get(&combo).cloned() {
                        ev.prevent_default();
                        let cmd = yal_core::Command::Plugin {
                            plugin_name: sc.plugin,
                            command_name: sc.command,
                            args: None,
                        };
                        leptos::task::spawn_local(async move {
                            run_cmd(cmd).await;
                        });
                    }
                }
            }
        }
    };

    let on_key = move |ev: KeyboardEvent| {
        if prompt.get().is_none() {
            pallet_keys(ev);
        }
    };

    let on_input = move |ev| set_query.set(event_target_value(&ev));

    Effect::new(move |_| {
        if prompt.get().is_none() {
            raf_focus_search();
        }
    });

    view! {
      <div id="bar">
        <Show when=move || !prefix_text.get().is_empty()>
          <span class="input-prefix">{ move || format!("{} ", prefix_text.get()) }</span>
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

      <list::ResultsList selected=selected filtered=filtered filter=filter />

      <Show when=move || prompt.get().is_some()>
        <PromptView prompt=prompt set_prompt=set_prompt />
      </Show>
    }
}
