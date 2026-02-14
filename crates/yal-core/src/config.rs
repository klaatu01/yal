use crate::align::{AlignH, AlignV};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    pub window: Option<WindowConfig>,
    pub theme: Option<String>,
    pub font: Option<FontConfig>,
    pub keys: Option<KeysConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct KeysConfig {
    pub shortcuts: Option<Vec<crate::shortcut::Shortcut>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FontConfig {
    pub font: Option<String>,
    pub font_size: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WindowConfig {
    pub w_width: Option<f64>,
    pub w_height: Option<f64>,
    pub align_h: Option<AlignH>,
    pub align_v: Option<AlignV>,
    pub margin_x: Option<f64>,
    pub margin_y: Option<f64>,
    pub padding: Option<f64>,
    pub line_height: Option<f64>,
    pub w_radius: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Theme {
    pub name: Option<String>,
    pub bg_color: Option<String>,
    pub fg_color: Option<String>,
    pub bg_font_color: Option<String>,
    pub fg_font_color: Option<String>,
}
