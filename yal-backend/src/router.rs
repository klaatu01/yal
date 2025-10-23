use kameo::actor::ActorRef;
use tauri::Emitter;

use crate::{
    application_tree::ApplicationTreeActor,
    cmd::theme::ThemeManagerActor,
    common::Events,
    config::{ConfigActor, GetConfig, ReloadConfig},
    plugin_backend::PluginBackend,
};

pub struct EventRouter {
    app_handle: tauri::AppHandle,
    config_ref: ActorRef<ConfigActor>,
    theme_ref: ActorRef<ThemeManagerActor>,
    application_tree_ref: ActorRef<crate::application_tree::ApplicationTreeActor>,
    ax_ref: ActorRef<crate::ax::AXActor>,
    plugin_manager_ref: ActorRef<crate::plugin::PluginManagerActor<PluginBackend>>,
}

impl EventRouter {
    pub fn new(
        app_handle: tauri::AppHandle,
        config_ref: ActorRef<ConfigActor>,
        theme_ref: ActorRef<ThemeManagerActor>,
        application_tree_ref: ActorRef<ApplicationTreeActor>,
        plugin_manager_ref: ActorRef<crate::plugin::PluginManagerActor<PluginBackend>>,
        ax_ref: ActorRef<crate::ax::AXActor>,
    ) -> Self {
        Self {
            app_handle,
            config_ref,
            theme_ref,
            application_tree_ref,
            plugin_manager_ref,
            ax_ref,
        }
    }

    pub fn spawn(self) -> kanal::Sender<Events> {
        let (event_tx, event_rx) = kanal::unbounded::<Events>();
        tauri::async_runtime::spawn(async move {
            let event_rx = event_rx.as_async();
            while let Ok(event) = event_rx.recv().await {
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
                            .ask(crate::application_tree::RefreshTree)
                            .await;

                        let tree = self
                            .application_tree_ref
                            .ask(crate::application_tree::SearchParam::All)
                            .await
                            .unwrap_or_default();

                        let current_display = self
                            .ax_ref
                            .ask(crate::ax::CurrentDisplaySpace)
                            .await
                            .unwrap();

                        let context = yal_plugin::protocol::PluginExecuteContext {
                            windows: tree
                                .into_iter()
                                .map(|res| yal_plugin::protocol::Window {
                                    app_name: res.app_name,
                                    title: res.title,
                                    pid: res.pid,
                                    window_id: res.window_id.0,
                                    display_id: res.display_id.to_string(),
                                    space_id: res.space_id.0,
                                    is_focused: res.is_focused,
                                    space_index: res.space_index,
                                })
                                .collect(),
                            displays: vec![],
                            current_display: yal_plugin::protocol::Display {
                                display_id: current_display.display_id.to_string(),
                                current_space_id: current_display.space_id.0,
                            },
                        };
                        let _ = self.plugin_manager_ref.tell(context).await;
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
