use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PluginConfigEntry {
    /// Human/plugin key (the key under [plugins] if present), or fallback
    pub name: String,
    /// Full git URL (e.g. https://github.com/owner/repo.git) or shorthand "owner/repo"
    pub github_url: String,
    /// Free-form plugin config handed to the plugin
    pub config: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Default)]
pub struct PluginConfig {
    pub plugins: Vec<PluginConfigEntry>,
}

/// The top-level TOML layout: a single table [plugins]
#[derive(Deserialize)]
struct RawConfig {
    #[serde(default)]
    plugins: HashMap<String, RawPlugin>,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum RawPlugin {
    Shorthand(String),
    Table {
        #[serde(default)]
        name: Option<String>,
        #[serde(default)]
        git: Option<String>,
        /// Accept "github" or "github_url" for convenience
        #[serde(default)]
        github: Option<String>,
        #[serde(default)]
        github_url: Option<String>,
        #[serde(default)]
        config: Option<serde_json::Value>,
    },
}

pub fn parse_plugins_toml(toml_str: &str) -> Result<PluginConfig> {
    let raw: RawConfig = toml::from_str(toml_str).context("invalid TOML")?;
    let mut plugins = Vec::with_capacity(raw.plugins.len());

    for (key, val) in raw.plugins {
        let entry = match val {
            RawPlugin::Shorthand(s) => PluginConfigEntry {
                name: key.clone(),
                github_url: normalize_github_url(&s)?,
                config: None,
            },
            RawPlugin::Table {
                name,
                git,
                github,
                github_url,
                config,
            } => {
                // Derive a git/github_url in priority order
                let gh = github_url.or(github).or(git).ok_or_else(|| {
                    anyhow!("plugin '{}' missing 'git'/'github'/'github_url'", key)
                })?;

                PluginConfigEntry {
                    name: name.unwrap_or_else(|| key.clone()),
                    github_url: normalize_github_url(&gh)?,
                    config,
                }
            }
        };

        plugins.push(entry);
    }

    Ok(PluginConfig { plugins })
}

fn normalize_github_url(input: &str) -> Result<String> {
    // already a URL?
    if input.starts_with("http://")
        || input.starts_with("https://")
        || input.starts_with("git@")
        || input.ends_with(".git")
    {
        return Ok(input.to_string());
    }

    // Looks like "owner/repo"
    if input.split('/').count() == 2 {
        return Ok(format!("https://github.com/{}.git", input.trim()));
    }

    Err(anyhow!(
        "Unrecognized GitHub reference: '{}'. Use 'owner/repo' or a full git URL.",
        input
    ))
}
