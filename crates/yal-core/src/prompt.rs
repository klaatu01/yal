use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Hotkey {
    pub key: String,
    pub label: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Prompt {
    pub title: Option<String>,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub content: Vec<Node>,
    pub ui_schema_version: Option<u32>,
    pub hotkeys: Option<Vec<Hotkey>>,
}

impl Prompt {
    pub fn contains_input_fields(&self) -> bool {
        fn node_contains_input_fields(node: &Node) -> bool {
            match node {
                Node::Form(form) => !form.fields.is_empty(),
                Node::VStack { children, .. }
                | Node::HStack { children, .. }
                | Node::Grid { children, .. } => {
                    children.iter().any(node_contains_input_fields)
                }
                _ => false,
            }
        }

        self.content.iter().any(node_contains_input_fields)
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
    VStack { gap: Option<f32>, children: Vec<Node> },
    HStack { gap: Option<f32>, children: Vec<Node> },
    Grid { cols: u16, gap: Option<f32>, children: Vec<Node> },
    Markdown { md: String },
    Html { html: String },
    Text { text: String, variant: Option<TextVariant> },
    Image { src: String, alt: Option<String>, w: Option<u32>, h: Option<u32> },
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
