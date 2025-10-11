use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AlignH {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AlignV {
    Top,
    Center,
    Bottom,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    pub window: Option<WindowConfig>,
    pub theme: Option<String>,
    pub font: Option<FontConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FontConfig {
    pub font: Option<String>,
    pub font_size: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WindowConfig {
    pub w_width: Option<f64>,
    pub w_height: Option<f64>,
    pub align_h: Option<AlignH>,  // "left" | "center" | "right"
    pub align_v: Option<AlignV>,  // "top"  | "center" | "bottom"
    pub margin_x: Option<f64>,    // px inset from left/right edges (default ~12)
    pub margin_y: Option<f64>,    // px inset from top/bottom edges (default ~12)
    pub padding: Option<f64>,     // px padding inside window (default ~6)
    pub line_height: Option<f64>, // line height multiplier (default ~1.2)
    pub w_radius: Option<f64>,    // window corner radius in px (default ~0)
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Theme {
    pub name: Option<String>,
    pub bg_color: Option<String>,      // background color
    pub fg_color: Option<String>,      // foreground color used for highlighting
    pub bg_font_color: Option<String>, // font color used for background items
    pub fg_font_color: Option<String>, // font color used for foreground items
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct AppInfo {
    pub name: String,
    pub path: String,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct WindowTarget {
    pub app_name: String,
    pub title: Option<String>,
    pub pid: i32,
    pub window_id: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Command {
    App(AppInfo),
    Switch(WindowTarget),
    Theme(String),
    Plugin {
        plugin_name: String,
        command_name: String,
    },
}

impl Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
