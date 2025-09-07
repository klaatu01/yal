// src/config.rs
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

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
    // UI
    pub font: Option<String>,
    pub font_size: Option<f32>,
    pub bg_color: Option<String>,
    pub fg_color: Option<String>,
    pub font_color: Option<String>,

    // Window size
    pub w_width: Option<f64>,
    pub w_height: Option<f64>,

    // Alignment + margins (optional; defaults used if omitted)
    pub align_h: Option<AlignH>, // "left" | "center" | "right"
    pub align_v: Option<AlignV>, // "top"  | "center" | "bottom"
    pub margin_x: Option<f64>,   // px inset from left/right edges (default ~12)
    pub margin_y: Option<f64>,   // px inset from top/bottom edges (default ~12)
}

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
