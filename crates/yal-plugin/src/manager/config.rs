use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PluginConfigEntry {
    /// Human/plugin key (the key under [plugins] if present), or fallback
    pub name: String,
    /// Full git URL (e.g. https://github.com/owner/repo.git) or shorthand "owner/repo"
    pub git: String,
    /// Free-form plugin config handed to the plugin
    pub config: Option<serde_json::Value>,
}

pub type PluginConfig = Vec<PluginConfigEntry>;
