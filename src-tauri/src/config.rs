use std::{fs, path::PathBuf};
use yal_core::AppConfig;

pub fn config_path() -> PathBuf {
    // Respect $XDG_CONFIG_HOME if present, otherwise use ~/.config/yal/config.toml
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .map(PathBuf::from)
                .map(|h| h.join(".config"))
        })
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("yal").join("config.toml")
}

pub fn load_config() -> AppConfig {
    let path = config_path();
    match fs::read_to_string(&path) {
        Ok(s) => toml::from_str::<AppConfig>(&s).unwrap_or_default(),
        Err(_) => AppConfig::default(),
    }
}
