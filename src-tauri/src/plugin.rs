use kameo::{prelude::Message, Actor};
use yal_plugin::{
    protocol::{PluginExecuteContext, PluginExecuteResponse},
    PluginManager,
};

#[derive(Actor)]
pub struct PluginManagerActor {
    pub manager: PluginManager,
}

impl PluginManagerActor {
    pub fn new() -> Self {
        let manager = PluginManager::new();
        Self { manager }
    }
}

pub struct InstallPlugins;

impl Message<InstallPlugins> for PluginManagerActor {
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

pub struct PluginCommand {
    pub plugin_name: String,
    pub commands: Vec<String>,
}

impl Message<LoadPlugins> for PluginManagerActor {
    type Reply = Vec<PluginCommand>;

    async fn handle(
        &mut self,
        _msg: LoadPlugins,
        _ctx: &mut kameo::prelude::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        log::info!("Loading plugins...");
        self.manager.load_config().await.unwrap();
        log::info!("Plugin config loaded: {:#?}", self.manager.config);
        self.manager.load_plugins().await.unwrap();
        log::info!("Plugins loaded: {}", self.manager.plugins.len());
        self.manager
            .commands()
            .await
            .iter()
            .map(|c| PluginCommand {
                plugin_name: c.0.clone(),
                commands: c.1.clone(),
            })
            .collect()
    }
}

pub struct ExecutePluginCommand {
    pub plugin_name: String,
    pub command_name: String,
    pub args: Option<serde_json::Value>,
}

impl Message<ExecutePluginCommand> for PluginManagerActor {
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
            Ok(res) => Ok(res),
            Err(e) => Err(format!(
                "Failed to execute command '{}::{}': {}",
                msg.plugin_name, msg.command_name, e
            )),
        }
    }
}

impl Message<PluginExecuteContext> for PluginManagerActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: PluginExecuteContext,
        _ctx: &mut kameo::prelude::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.manager.set_execution_context(msg);
    }
}
