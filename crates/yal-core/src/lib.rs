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
        args: Option<serde_json::Value>,
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Popup {
    pub id: Option<String>,
    pub title: Option<String>,
    pub width: Option<f32>,             // %; default 75%
    pub height: Option<f32>,            // %; default 75%
    pub content: Vec<Node>,             // layout + widgets
    pub hotkeys: Option<Vec<Hotkey>>,   // optional hotkeys
    pub ui_schema_version: Option<u32>, // default 1
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Node {
    // Layout
    VStack {
        gap: Option<f32>,
        children: Vec<Node>,
    },
    HStack {
        gap: Option<f32>,
        children: Vec<Node>,
    },
    Grid {
        cols: u16,
        gap: Option<f32>,
        children: Vec<Node>,
    },

    // Content
    Markdown {
        md: String,
    },
    Html {
        html: String,
    }, // render with sanitization
    Text {
        text: String,
        variant: Option<TextVariant>,
    },
    Image {
        src: String,
        alt: Option<String>,
        w: Option<u32>,
        h: Option<u32>,
    },

    // Form (single submit)
    Form(Form),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum TextVariant {
    Muted,
    Caption,
    Code,
    Emphasis,
    Heading,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Form {
    pub name: Option<String>,
    #[serde(default)]
    pub fields: Vec<Field>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TextField {
    pub name: String,
    pub label: Option<String>,
    pub placeholder: Option<String>,
    pub max_length: Option<u32>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SelectField {
    pub name: String,
    pub label: Option<String>,
    pub options: Vec<OptionKV>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SliderField {
    pub name: String,
    pub label: Option<String>,
    pub min: f64,
    pub max: f64,
    pub step: f64,
    pub value: Option<f64>,
    pub show_value: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Field {
    Text(TextField),
    Select(SelectField),
    Slider(SliderField),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OptionKV {
    pub label: String,
    pub value: serde_json::Value,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Hotkey {
    pub combo: String, // e.g., "ctrl+enter", "esc"
    pub value: OptionKV,
}
