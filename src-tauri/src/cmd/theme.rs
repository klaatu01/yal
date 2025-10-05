use crate::config;
use kameo::{prelude::Message, Actor};
use tauri::Emitter;
use yal_theme::ALL;

#[derive(Actor)]
pub struct ThemeManagerActor {
    pub app_handle: tauri::AppHandle,
    pub current: Option<String>,
}

impl ThemeManagerActor {
    pub fn new(app_handle: tauri::AppHandle) -> Self {
        Self {
            current: None,
            app_handle,
        }
    }

    #[allow(dead_code)]
    fn set_current(&mut self, theme_name: &str) {
        self.current = Some(theme_name.to_string());
    }

    fn load_themes(&self) -> Vec<yal_core::Theme> {
        let user_themes = config::load_themes();
        let default_themes = ALL
            .iter()
            .copied()
            .map(|theme_ref| theme_ref.to_owned())
            .collect::<Vec<_>>();
        [user_themes, default_themes].concat()
    }

    fn apply_theme(&mut self, theme_name: &str) {
        let themes = self.load_themes();
        if let Some(theme) = themes
            .iter()
            .find(|t| t.name.as_deref() == Some(theme_name))
        {
            log::info!("Applying theme: {}", theme_name);
            let _ = self.app_handle.emit("theme://applied", theme.clone());
        }
        self.current = Some(theme_name.to_string());
    }
}

pub struct ApplyTheme {
    pub theme_name: String,
}

impl Message<ApplyTheme> for ThemeManagerActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: ApplyTheme,
        _ctx: &mut kameo::prelude::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.apply_theme(&msg.theme_name);
    }
}

pub struct LoadThemes;

impl Message<LoadThemes> for ThemeManagerActor {
    type Reply = Vec<yal_core::Theme>;

    async fn handle(
        &mut self,
        _msg: LoadThemes,
        _ctx: &mut kameo::prelude::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.load_themes()
    }
}

pub struct GetCurrentTheme;

impl Message<GetCurrentTheme> for ThemeManagerActor {
    type Reply = Option<String>;

    async fn handle(
        &mut self,
        _msg: GetCurrentTheme,
        _ctx: &mut kameo::prelude::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.current.clone()
    }
}
