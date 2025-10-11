use futures::StreamExt;
use kameo::actor::ActorRef;
use tauri::Emitter;

use crate::{
    application_tree::ApplicationTreeActor,
    cmd::theme::ThemeManagerActor,
    common::Events,
    config::{ConfigActor, GetConfig, ReloadConfig},
};

pub struct EventRouter {
    app_handle: tauri::AppHandle,
    config_ref: ActorRef<ConfigActor>,
    theme_ref: ActorRef<ThemeManagerActor>,
    application_tree_ref: ActorRef<crate::application_tree::ApplicationTreeActor>,
    plugin_manager_ref: ActorRef<crate::plugin::PluginManagerActor>,
}

impl EventRouter {
    pub fn new(
        app_handle: tauri::AppHandle,
        config_ref: ActorRef<ConfigActor>,
        theme_ref: ActorRef<ThemeManagerActor>,
        application_tree_ref: ActorRef<ApplicationTreeActor>,
        plugin_manager_ref: ActorRef<crate::plugin::PluginManagerActor>,
    ) -> Self {
        Self {
            app_handle,
            config_ref,
            theme_ref,
            application_tree_ref,
            plugin_manager_ref,
        }
    }

    pub fn spawn(self) -> futures::channel::mpsc::UnboundedSender<crate::common::Events> {
        let (event_tx, mut event_rx) = futures::channel::mpsc::unbounded();
        tauri::async_runtime::spawn(async move {
            while let Some(event) = event_rx.next().await {
                match event {
                    Events::ReloadConfig => {
                        log::info!("EventRouter: ReloadConfig event received");
                        let _ = self.config_ref.tell(ReloadConfig).await;
                        let config = self.config_ref.ask(GetConfig).await;
                        if let Ok(cfg) = config {
                            let _ = self
                                .theme_ref
                                .tell(crate::cmd::theme::ApplyTheme {
                                    theme_name: cfg.theme.clone().unwrap_or_default(),
                                })
                                .await;

                            crate::window::apply_window_size(&self.app_handle, &cfg);

                            crate::window::position_main_window_on_mouse_display(
                                &self.app_handle,
                                &cfg,
                            );

                            let _ = self.app_handle.emit("config://updated", cfg);
                        }
                    }
                    Events::RefreshTree => {
                        log::info!("EventRouter: RefreshTree event received");
                        let _ = self
                            .application_tree_ref
                            .tell(crate::application_tree::RefreshTree)
                            .await;
                    }
                    Events::ReloadPlugins => {
                        log::info!("EventRouter: ReloadPlugins event received");
                        let _ = self
                            .plugin_manager_ref
                            .ask(crate::plugin::InstallPlugins)
                            .await;
                    }
                }
            }
        });
        event_tx
    }
}
