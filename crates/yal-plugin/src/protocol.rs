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
