use crate::{app::AppInfo, window_target::WindowTarget};
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Command {
    App(AppInfo),
    Switch(WindowTarget),
    Theme(String),
    Plugin {
        plugin_name: String,
        command_name: String,
        args: Option<serde_json::Value>,
    },
}

impl Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.prefix(), self.name())
    }
}

impl Command {
    pub fn name(&self) -> String {
        match self {
            Command::App(app) => app.name.clone(),
            Command::Switch(t) => {
                if let Some(title) = &t.title {
                    format!("{} - {}", t.app_name, title)
                } else {
                    t.app_name.clone()
                }
            }
            Command::Theme(name) => name.clone(),
            Command::Plugin {
                plugin_name,
                command_name,
                ..
            } => format!("{} - {}", plugin_name, command_name),
        }
    }

    pub fn prefix(&self) -> &str {
        match self {
            Command::App(_) => "app",
            Command::Switch(_) => "switch",
            Command::Theme(_) => "theme",
            Command::Plugin { .. } => "plugin",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CommandKind {
    App,
    Switch,
    Theme,
    Plugin,
}

impl CommandKind {
    pub fn is_kind(&self, cmd: &Command) -> bool {
        matches!(
            (self, cmd),
            (CommandKind::App, Command::App(_))
                | (CommandKind::Switch, Command::Switch(_))
                | (CommandKind::Theme, Command::Theme(_))
                | (CommandKind::Plugin, Command::Plugin { .. })
        )
    }
}
