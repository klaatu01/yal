use std::{path::PathBuf, sync::Arc};

use anyhow::{Context, Result};
use git2::Repository;
use tokio::fs;

use crate::{manager::config::PluginConfig, plugin::Plugin, protocol::PluginExecuteContext};

mod config;

pub fn plugins_config_path() -> PathBuf {
    let mut path = dirs::home_dir().expect("Failed to get home directory");
    path.push(".config/yal/plugins.toml");
    path
}

pub fn plugins_dir() -> PathBuf {
    let mut dir = dirs::home_dir().expect("Failed to get home directory");
    dir.push(".local/share/yal/plugins");
    dir
}

pub struct PluginManager {
    pub config: PluginConfig,
    pub plugins: Vec<Plugin>,
}

impl PluginManager {
    pub fn new() -> Self {
        Self {
            config: PluginConfig::default(),
            plugins: Vec::new(),
        }
    }

    pub async fn init(&self) -> Result<()> {
        let dir = plugins_dir();
        if !dir.exists() {
            fs::create_dir_all(&dir)
                .await
                .with_context(|| format!("Failed creating plugins directory {}", dir.display()))?;
        }
        Ok(())
    }

    pub async fn load_config(&mut self) -> Result<()> {
        let path = plugins_config_path();

        if !path.exists() {
            self.config = PluginConfig::default();
            return Ok(());
        }

        let raw = fs::read_to_string(&path)
            .await
            .with_context(|| format!("Failed reading {}", path.display()))?;

        self.config = config::parse_plugins_toml(&raw)
            .with_context(|| format!("Failed parsing {}", path.display()))?;

        Ok(())
    }

    pub async fn install(&mut self) -> Result<()> {
        self.load_config().await?;
        for plugin in &self.config.plugins {
            log::info!("Installing plugin: {}", plugin.name);
            log::info!("  from: {}", plugin.github_url);
            let plugin_dir = plugins_dir().join(&plugin.name);
            if plugin_dir.exists() {
                log::info!("  already installed, skipping");
                continue;
            }
            let repo = Repository::clone(&plugin.github_url, &plugin_dir)
                .with_context(|| format!("Failed cloning {}", plugin.github_url))?;
            log::info!("  cloned to: {}", repo.path().parent().unwrap().display());
        }
        Ok(())
    }

    pub async fn load_plugins(&mut self) -> Result<()> {
        self.plugins.clear();
        for plugin in &self.config.plugins {
            let plugin_dir = plugins_dir().join(&plugin.name);
            if !plugin_dir.exists() {
                log::warn!("Plugin '{}' is not installed, skipping", plugin.name);
                continue;
            }
            let plugin_ref = crate::plugin::PluginRef {
                name: plugin.name.clone(),
                path: plugin_dir.clone(),
                config: plugin.config.clone(),
            };
            let lua_plugin = crate::plugin::LuaPlugin::new(plugin_ref).unwrap();
            let init_response = lua_plugin.initialize().await?;
            let plugin = Plugin {
                name: plugin.name.clone(),
                commands: init_response
                    .commands
                    .iter()
                    .map(|c| c.name.clone())
                    .collect(),
                lua: lua_plugin,
            };
            log::info!(
                "Plugin '{}' initialized with {} commands",
                plugin.name,
                plugin.commands.len()
            );
            self.plugins.push(plugin);
        }
        Ok(())
    }

    pub async fn run_command(
        &self,
        plugin_name: &str,
        command_name: &str,
        context: PluginExecuteContext,
    ) -> Result<()> {
        let plugin = self
            .plugins
            .iter()
            .find(|p| p.name == plugin_name)
            .ok_or_else(|| anyhow::anyhow!("Plugin '{}' not found", plugin_name))
            .with_context(|| format!("Failed to find plugin '{}'", plugin_name))?;
        if !plugin.commands.contains(&command_name.to_string()) {
            return Err(anyhow::anyhow!(
                "Command '{}' not found in plugin '{}'",
                command_name,
                plugin_name
            ))
            .with_context(|| {
                format!(
                    "Failed to find command '{}' in plugin '{}'",
                    command_name, plugin_name
                )
            });
        }
        let _ = plugin.lua.run(command_name.to_string(), &context).await?;
        Ok(())
    }

    pub async fn commands(&self) -> Vec<(String, Vec<String>)> {
        self.plugins
            .iter()
            .map(|p| (p.name.clone(), p.commands.clone()))
            .collect()
    }
}
