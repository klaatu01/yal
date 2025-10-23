use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use leptos::prelude::*;
use yal_core::{Command, CommandKind};

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

pub fn filter_memoized_commands(
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
