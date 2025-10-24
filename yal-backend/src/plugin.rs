use kameo::{prelude::Message, Actor};
use yal_plugin::{
    backend::Backend,
    plugin::PluginManifest,
    protocol::{PluginExecuteContext, PluginExecuteResponse},
    PluginManager,
};

#[derive(Actor)]
pub struct PluginManagerActor<T: Backend> {
    pub manager: PluginManager<T>,
}

impl<T: Backend> PluginManagerActor<T> {
    pub fn new(backend: T) -> Self {
        let manager = PluginManager::new(backend);
        Self { manager }
    }
}

pub struct InstallPlugins;

impl<T: Backend> Message<InstallPlugins> for PluginManagerActor<T> {
    type Reply = Result<(), String>;

    async fn handle(
        &mut self,
        _msg: InstallPlugins,
        _ctx: &mut kameo::prelude::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        log::info!("Installing plugins...");
        match self.manager.install().await {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Failed to install plugins: {}", e)),
        }
    }
}

pub struct LoadPlugins;

impl<T: Backend> Message<LoadPlugins> for PluginManagerActor<T> {
    type Reply = Vec<PluginManifest>;

    async fn handle(
        &mut self,
        _msg: LoadPlugins,
        _ctx: &mut kameo::prelude::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        log::debug!("Loading plugins...");
        self.manager.load_config().await.unwrap();
        log::debug!("Plugin config loaded: {:#?}", self.manager.config);
        self.manager.load_plugins().await.unwrap();
        log::debug!("Plugins loaded: {}", self.manager.plugins.len());
        self.manager.commands().await
    }
}

pub struct ExecutePluginCommand {
    pub plugin_name: String,
    pub command_name: String,
    pub args: Option<serde_json::Value>,
}

impl<T: Backend> Message<ExecutePluginCommand> for PluginManagerActor<T> {
    type Reply = Result<PluginExecuteResponse, String>;

    async fn handle(
        &mut self,
        msg: ExecutePluginCommand,
        _ctx: &mut kameo::prelude::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        log::info!(
            "Executing plugin command: {}::{}",
            msg.plugin_name,
            msg.command_name
        );
        match self
            .manager
            .run_command(&msg.plugin_name, &msg.command_name, msg.args)
            .await
        {
            Ok(res) => {
                log::info!(
                    "Command {}::{} executed successfully",
                    msg.plugin_name,
                    msg.command_name
                );
                log::info!("{}", serde_json::to_string_pretty(&res).unwrap());
                Ok(res)
            }
            Err(e) => Err(format!(
                "Failed to execute command '{}::{}': {}",
                msg.plugin_name, msg.command_name, e
            )),
        }
    }
}

impl<T: Backend> Message<PluginExecuteContext> for PluginManagerActor<T> {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: PluginExecuteContext,
        _ctx: &mut kameo::prelude::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.manager.set_execution_context(msg);
    }
}
