use std::{path::PathBuf, sync::Arc};

use anyhow::{Context, Result};
use git2::Repository;
use tokio::fs;

use crate::{
    backend,
    manager::config::PluginConfig,
    plugin::{Plugin, PluginManifest},
    protocol::{PluginExecuteContext, PluginExecuteResponse},
};

mod config;

pub fn plugins_config_path() -> PathBuf {
    let mut path = dirs::home_dir().expect("Failed to get home directory");
    path.push(".config/yal/plugins.lua");
    path
}

pub fn plugins_dir() -> PathBuf {
    let mut dir = dirs::home_dir().expect("Failed to get home directory");
    dir.push(".local/share/yal/plugins");
    dir
}

pub struct PluginManager<T: backend::Backend> {
    pub config: PluginConfig,
    pub plugins: Vec<Plugin>,
    pub execution_context: Option<PluginExecuteContext>,
    pub backend: Arc<T>,
}

impl<T: backend::Backend> PluginManager<T> {
    pub fn new(backend: T) -> Self {
        Self {
            config: PluginConfig::default(),
            plugins: Vec::new(),
            execution_context: None,
            backend: Arc::new(backend),
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
        let config = yal_config::load_config::<PluginConfig>(&path);
        self.config = config;
        Ok(())
    }

    pub async fn install(&mut self) -> Result<()> {
        self.load_config().await?;
        for plugin in &self.config {
            log::info!("Installing plugin: {}", plugin.name);
            log::info!("  from: {}", plugin.git);
            let plugin_dir = plugins_dir().join(&plugin.name);
            if plugin_dir.exists() {
                log::info!("  already installed, skipping");
                continue;
            }

            let giturl = if plugin.git.starts_with("http://")
                || plugin.git.starts_with("https://")
                || plugin.git.starts_with("git@")
            {
                plugin.git.clone()
            } else {
                format!("https://github.com/{}.git", plugin.git)
            };

            let repo = Repository::clone(&giturl, &plugin_dir)
                .with_context(|| format!("Failed cloning {}", plugin.git))?;
            log::info!("  cloned to: {}", repo.path().parent().unwrap().display());
        }
        Ok(())
    }

    pub async fn load_plugins(&mut self) -> Result<()> {
        self.plugins.clear();
        for plugin in &self.config {
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
            let lua_plugin = crate::plugin::LuaPlugin::new(plugin_ref, self.backend.clone())
                .with_context(|| format!("Failed loading plugin '{}'", plugin.name))?;
            let init_response = lua_plugin.initialize().await?;
            let plugin = Plugin {
                name: plugin.name.clone(),
                commands: init_response.commands,
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
        args: Option<serde_json::Value>,
    ) -> Result<PluginExecuteResponse> {
        let plugin = self
            .plugins
            .iter()
            .find(|p| p.name == plugin_name)
            .with_context(|| format!("Plugin '{}' not found", plugin_name))?;

        if !plugin
            .commands
            .iter()
            .cloned()
            .any(|c| c.name == command_name)
        {
            return Err(anyhow::anyhow!(
                "Command '{}' not found in plugin '{}'",
                command_name,
                plugin_name
            ));
        }

        if let Some(ctx) = &self.execution_context {
            log::info!(
                "Executing command '{}' of plugin '{}'",
                command_name,
                plugin_name,
            );
            let resp = plugin.lua.run(command_name.to_string(), ctx, args).await?;

            Ok(resp)
        } else {
            log::info!(
                "Executing command '{}' of plugin '{}' with no context",
                command_name,
                plugin_name
            );
            Err(anyhow::anyhow!(
                "No execution context set for plugin command"
            ))
        }
    }

    pub fn set_execution_context(&mut self, context: PluginExecuteContext) {
        log::info!("Setting execution context");
        self.execution_context = Some(context);
    }

    pub async fn commands(&self) -> Vec<PluginManifest> {
        self.plugins
            .iter()
            .map(|p| PluginManifest {
                plugin_name: p.name.clone(),
                commands: p.commands.clone(),
            })
            .collect()
    }
}
