use anyhow::Result;
use kameo::prelude::Message;
use kameo::Actor;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::{fs, path::PathBuf};
use yal_core::AppConfig;

#[derive(Actor)]
pub struct ConfigActor {
    config: AppConfig,
}

impl ConfigActor {
    pub fn new() -> Self {
        let config = load_config();
        Self { config }
    }
}

pub struct ReloadConfig;

impl Message<ReloadConfig> for ConfigActor {
    type Reply = Result<()>;

    async fn handle(
        &mut self,
        _msg: ReloadConfig,
        _ctx: &mut kameo::prelude::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.config = load_config();
        Ok(())
    }
}

pub struct GetConfig;

impl Message<GetConfig> for ConfigActor {
    type Reply = Result<AppConfig>;

    async fn handle(
        &mut self,
        _msg: GetConfig,
        _ctx: &mut kameo::prelude::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        Ok(self.config.clone())
    }
}

pub fn config_base_path() -> PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .map(PathBuf::from)
                .map(|h| h.join(".config"))
        })
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("yal")
}

pub fn config_path() -> PathBuf {
    config_base_path().join("config.toml")
}

pub fn themes_path() -> PathBuf {
    config_base_path().join("themes.toml")
}

pub fn load_config() -> AppConfig {
    let path = config_path();
    match fs::read_to_string(&path) {
        Ok(s) => toml::from_str::<AppConfig>(&s).unwrap_or_default(),
        Err(_) => AppConfig::default(),
    }
}

pub fn load_themes() -> Vec<yal_core::Theme> {
    let path = themes_path();
    let Ok(s) = fs::read_to_string(&path) else {
        log::info!("themes file not found at {:?}; using defaults", path);
        return vec![];
    };
    parse_themes(&s)
}

pub fn parse_themes(s: &str) -> Vec<yal_core::Theme> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct ArraySingular {
        #[serde(default)]
        theme: Vec<yal_core::Theme>,
    }
    if let Ok(ArraySingular { theme }) = toml::from_str::<ArraySingular>(s) {
        if !theme.is_empty() {
            return theme;
        }
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct ArrayPlural {
        #[serde(default)]
        themes: Vec<yal_core::Theme>,
    }
    if let Ok(ArrayPlural { themes }) = toml::from_str::<ArrayPlural>(s) {
        if !themes.is_empty() {
            return themes;
        }
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Namespaced {
        themes: BTreeMap<String, yal_core::Theme>,
    }
    if let Ok(Namespaced { themes }) = toml::from_str::<Namespaced>(s) {
        if !themes.is_empty() {
            return themes
                .into_iter()
                .map(|(name, mut t)| {
                    if t.name.is_none() {
                        t.name = Some(name);
                    }
                    t
                })
                .collect();
        }
    }

    if let Ok(map) = toml::from_str::<BTreeMap<String, yal_core::Theme>>(s) {
        if !map.is_empty() {
            return map
                .into_iter() // consume the map
                .map(|(name, mut t)| {
                    if t.name.is_none() {
                        t.name = Some(name);
                    }
                    t
                })
                .collect();
        }
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Inline {
        #[serde(default)]
        themes: Vec<yal_core::Theme>,
    }
    if let Ok(Inline { themes }) = toml::from_str::<Inline>(s) {
        if !themes.is_empty() {
            return themes;
        }
    }

    vec![]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_theme_named() {
        let theme_str = r##"
            [themes.custom]
            fg_color = "#ffffff"
            bg_color = "#000000"
            fg_font_color = "#ff0000"
            bg_font_color = "#00ff00"
        "##;

        let themes = parse_themes(theme_str);
        assert_eq!(themes.len(), 1);

        let theme = &themes[0];
        assert_eq!(theme.name.as_deref(), Some("custom"));
        assert_eq!(theme.fg_color.as_deref(), Some("#ffffff"));
        assert_eq!(theme.bg_color.as_deref(), Some("#000000"));
        assert_eq!(theme.fg_font_color.as_deref(), Some("#ff0000"));
        assert_eq!(theme.bg_font_color.as_deref(), Some("#00ff00"));
    }

    #[test]
    fn parse_theme_array() {
        let theme_str = r##"
            [[theme]]
            name = "custom"
            fg_color = "#ffffff"
            bg_color = "#000000"
            fg_font_color = "#ff0000"
            bg_font_color = "#00ff00"
        "##;

        let themes = parse_themes(theme_str);
        assert_eq!(themes.len(), 1);

        let theme = &themes[0];
        assert_eq!(theme.name.as_deref(), Some("custom"));
        assert_eq!(theme.fg_color.as_deref(), Some("#ffffff"));
        assert_eq!(theme.bg_color.as_deref(), Some("#000000"));
        assert_eq!(theme.fg_font_color.as_deref(), Some("#ff0000"));
        assert_eq!(theme.bg_font_color.as_deref(), Some("#00ff00"));
    }

    #[test]
    fn parse_theme_inline_array() {
        let theme_str = r##"
            themes = [
                { name = "custom", fg_color = "#ffffff", bg_color = "#000000", fg_font_color = "#ff0000", bg_font_color = "#00ff00" }
            ]
        "##;
        let themes = parse_themes(theme_str);
        assert_eq!(themes.len(), 1);
        let theme = &themes[0];
        assert_eq!(theme.name.as_deref(), Some("custom"));
        assert_eq!(theme.fg_color.as_deref(), Some("#ffffff"));
        assert_eq!(theme.bg_color.as_deref(), Some("#000000"));
        assert_eq!(theme.fg_font_color.as_deref(), Some("#ff0000"));
        assert_eq!(theme.bg_font_color.as_deref(), Some("#00ff00"));
    }
}
