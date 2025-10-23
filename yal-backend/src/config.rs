use anyhow::Result;
use kameo::prelude::Message;
use kameo::Actor;
use std::path::PathBuf;
use yal_core::AppConfig;

#[derive(Actor)]
pub struct ConfigActor {
    config: AppConfig,
}

impl ConfigActor {
    pub fn new() -> Self {
        let config = yal_config::load_config(self::config_path().as_path());
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
        self.config = yal_config::load_config(self::config_path().as_path());
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
    config_base_path().join("config.lua")
}

pub fn themes_path() -> PathBuf {
    config_base_path().join("themes.lua")
}
