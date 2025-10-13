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
    pub id: Option<String>, // for refresh/replace semantics
    pub title: Option<String>,
    pub width: Option<f32>,  // %; default 75%
    pub height: Option<f32>, // %; default 75%
    pub modal: Option<bool>, // default false
    pub content: Vec<Node>,  // layout + widgets
    #[serde(default)]
    pub actions: Vec<Action>, // e.g., footer buttons
    #[serde(default)]
    pub hotkeys: Vec<Hotkey>, // global popup shortcuts
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
    pub submit: Action,
    pub submit_label: Option<String>,  // default "Submit"
    pub submit_on_enter: Option<bool>, // default true
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Field {
    Text {
        name: String,
        label: Option<String>,
        placeholder: Option<String>,
        multiline: Option<bool>,
        rows: Option<u8>,
        required: Option<bool>,
        max_length: Option<u32>,
    },
    Number {
        name: String,
        label: Option<String>,
        min: Option<f64>,
        max: Option<f64>,
        step: Option<f64>,
        required: Option<bool>,
    },
    Select {
        name: String,
        label: Option<String>,
        options: Vec<OptionKV>,
        multiple: Option<bool>,
        required: Option<bool>,
    },
    Checkbox {
        name: String,
        label: String,
        checked: Option<bool>,
    },
    RadioGroup {
        name: String,
        label: Option<String>,
        options: Vec<OptionKV>,
        required: Option<bool>,
    },
    Slider {
        name: String,
        label: Option<String>,
        min: f64,
        max: f64,
        step: f64,
        value: Option<f64>,
        show_value: Option<bool>,
    },
    Hidden {
        name: String,
        value: serde_json::Value,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OptionKV {
    pub value: String,
    pub label: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Action {
    /// Execute a plugin command. If `plugin` is None, use the current plugin.
    Command {
        plugin: String,
        command: String,
        /// Arbitrary payload; for forms, YAL injects {"fields": {name: value, ...}} and merges with this.
        #[serde(default)]
        args: serde_json::Value,
        /// What to do with the UI after the action runs.
        #[serde(default)]
        presentation: Presentation, // default: ReplacePopup
    },

    // Optional extra actions if you want:
    OpenUrl {
        url: String,
        in_app: Option<bool>,
    },
    CopyToClipboard {
        text: String,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum Presentation {
    KeepPopup,    // leave as-is
    ClosePopup,   // close on success
    ReplacePopup, // replace with the next popup the command returns (default)
}

impl Default for Presentation {
    fn default() -> Self {
        Presentation::ReplacePopup
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Hotkey {
    pub combo: String, // e.g., "ctrl+enter", "esc"
    pub action: Action,
}
