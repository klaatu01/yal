use serde::de::{self, MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
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
    pub keys: Option<KeysConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct KeysConfig {
    pub shortcuts: Option<Vec<Shortcut>>,
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
pub struct Prompt {
    pub title: Option<String>,
    pub width: Option<f32>,             // %; default 75%
    pub height: Option<f32>,            // %; default 75%
    pub content: Vec<Node>,             // layout + widgets
    pub ui_schema_version: Option<u32>, // default 1
}

impl Prompt {
    pub fn contains_input_fields(&self) -> bool {
        fn node_contains_input_fields(node: &Node) -> bool {
            match node {
                Node::Form(form) => !form.fields.is_empty(),
                Node::VStack { children, .. }
                | Node::HStack { children, .. }
                | Node::Grid { children, .. } => {
                    for child in children {
                        if node_contains_input_fields(child) {
                            return true;
                        }
                    }
                    false
                }
                _ => false,
            }
        }

        for node in &self.content {
            if node_contains_input_fields(node) {
                return true;
            }
        }
        false
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PromptRequest {
    pub id: String,
    pub prompt: Prompt,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FrontendResult {
    pub id: String,
    pub response: serde_json::Value,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum PromptResponse {
    Submit { values: serde_json::Value },
    State { values: serde_json::Value },
    Cancel,
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ShortcutCommand {
    pub plugin: String,
    pub command: String,
}

#[derive(Debug, Clone)]
pub struct Shortcut {
    pub combination: String,
    pub command: ShortcutCommand,
}

// ----- ShortcutCommand as { plugin: "...", command: "..." } -----

impl Serialize for ShortcutCommand {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("plugin", &self.plugin)?;
        map.serialize_entry("command", &self.command)?;
        map.end()
    }
}

impl<'de> Deserialize<'de> for ShortcutCommand {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct SCVisitor;

        impl<'de> Visitor<'de> for SCVisitor {
            type Value = ShortcutCommand;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str(r#"a map with keys "plugin" and "command""#)
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut plugin: Option<String> = None;
                let mut command: Option<String> = None;

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "plugin" => plugin = Some(map.next_value()?),
                        "command" => command = Some(map.next_value()?),
                        _ => {
                            // consume unknown
                            let _: de::IgnoredAny = map.next_value()?;
                        }
                    }
                }

                let plugin = plugin.ok_or_else(|| de::Error::missing_field("plugin"))?;
                let command = command.ok_or_else(|| de::Error::missing_field("command"))?;

                Ok(ShortcutCommand { plugin, command })
            }
        }

        deserializer.deserialize_map(SCVisitor)
    }
}

impl Serialize for Shortcut {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(1))?;
        map.serialize_entry(&self.combination, &self.command)?;
        map.end()
    }
}

impl<'de> Deserialize<'de> for Shortcut {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ShortcutVisitor;

        impl<'de> Visitor<'de> for ShortcutVisitor {
            type Value = Shortcut;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str(r#"a single-entry map like { "<combo>": { plugin, command } }"#)
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                // Read the first (and only) entry
                let (combination, command): (String, ShortcutCommand) = match map.next_entry()? {
                    Some(pair) => pair,
                    None => return Err(de::Error::invalid_length(0, &self)),
                };

                // Ensure there are no extra entries
                if map
                    .next_entry::<de::IgnoredAny, de::IgnoredAny>()?
                    .is_some()
                {
                    return Err(de::Error::custom(
                        "expected a single-entry map for Shortcut, but found multiple entries",
                    ));
                }

                log::info!(
                    "Deserialized Shortcut: combination='{}', command=plugin:'{}', command:'{}'",
                    combination,
                    command.plugin,
                    command.command
                );

                Ok(Shortcut {
                    combination,
                    command,
                })
            }
        }

        deserializer.deserialize_map(ShortcutVisitor)
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct FrontendRequest<T: Send + Serialize + Clone + 'static> {
    pub id: String,
    pub data: T,
}
