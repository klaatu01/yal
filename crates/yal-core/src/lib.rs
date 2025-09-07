use serde::{Deserialize, Serialize};
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
    // UI
    pub font: Option<String>,
    pub font_size: Option<f32>,
    pub bg_color: Option<String>,
    pub fg_color: Option<String>,
    pub bg_font_color: Option<String>,
    pub fg_font_color: Option<String>,

    // Window size
    pub w_width: Option<f64>,
    pub w_height: Option<f64>,

    // Alignment + margins (optional; defaults used if omitted)
    pub align_h: Option<AlignH>, // "left" | "center" | "right"
    pub align_v: Option<AlignV>, // "top"  | "center" | "bottom"
    pub margin_x: Option<f64>,   // px inset from left/right edges (default ~12)
    pub margin_y: Option<f64>,   // px inset from top/bottom edges (default ~12)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct AppInfo {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Command {
    App(AppInfo),
}

impl Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.prefix(), self.name())
    }
}

impl Command {
    pub fn name(&self) -> &str {
        match self {
            Command::App(app) => &app.name,
        }
    }

    pub fn prefix(&self) -> &str {
        match self {
            Command::App(_) => "app",
        }
    }
}
