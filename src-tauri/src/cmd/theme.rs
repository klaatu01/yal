use crate::config;
use tauri::Emitter;
use yal_theme::ALL;

pub struct ThemeManager {
    pub current: Option<String>,
}

impl ThemeManager {
    pub fn new() -> Self {
        Self { current: None }
    }

    pub fn set_current(&mut self, theme_name: &str) {
        self.current = Some(theme_name.to_string());
    }

    pub fn load_themes(&self) -> Vec<yal_core::Theme> {
        let user_themes = config::load_themes();
        let default_themes = ALL
            .iter()
            .copied()
            .map(|theme_ref| theme_ref.to_owned())
            .collect::<Vec<_>>();
        [user_themes, default_themes].concat()
    }

    pub fn apply_theme(&mut self, app: &tauri::AppHandle, theme_name: &str) {
        let themes = self.load_themes();
        if let Some(theme) = themes
            .iter()
            .find(|t| t.name.as_deref() == Some(theme_name))
        {
            log::info!("Applying theme: {}", theme_name);
            let _ = app.emit("theme://applied", theme.clone());
        }
        self.current = Some(theme_name.to_string());
    }
}
