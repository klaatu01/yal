use nanoid::nanoid;
use serde::{Deserialize, Serialize};
use yal_core::Popup;

#[derive(Serialize, Deserialize, Clone)]
pub struct PluginCommand {
    pub name: String,
    pub description: Option<String>,
    #[serde(default)]
    pub hidden: bool,
}

#[derive(Serialize, Deserialize)]
pub struct PluginInitRequest {
    pub config: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize)]
pub struct PluginInitResponse {
    pub name: String,
    pub description: Option<String>,
    pub version: String,
    pub author: Option<String>,
    pub commands: Vec<PluginCommand>,
}

#[derive(Serialize, Deserialize)]
pub struct PluginExecuteResponse {
    pub hide: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub popup: Option<Popup>,
}

#[derive(Serialize)]
pub struct PluginExecuteRequest<'a> {
    pub command: String,
    pub context: &'a PluginExecuteContext,
    pub args: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize)]
pub struct PluginExecuteContext {
    pub windows: Vec<Window>,
    pub displays: Vec<Display>,
    pub current_display: Display,
}

#[derive(Serialize, Deserialize)]
pub struct Display {
    pub display_id: String,
    pub current_space_id: u64,
}

#[derive(Serialize, Deserialize)]
pub struct Window {
    pub display_id: String,
    pub space_id: u64,
    pub space_index: usize,
    pub window_id: u32,
    pub title: Option<String>,
    pub pid: i32,
    pub app_name: String,
    pub is_focused: bool,
}

#[derive(Clone, Debug)]
pub struct PluginAPIRequest {
    pub id: String,
    pub payload: PluginAPIEvent,
    pub responder: kanal::Sender<serde_json::Value>,
}

impl PluginAPIRequest {
    pub fn new(payload: PluginAPIEvent) -> (Self, kanal::Receiver<serde_json::Value>) {
        let (tx, rx) = kanal::bounded::<serde_json::Value>(1);
        (
            PluginAPIRequest {
                id: nanoid!(21),
                payload,
                responder: tx,
            },
            rx,
        )
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum PluginAPIEvent {
    Prompt(Popup),
}
