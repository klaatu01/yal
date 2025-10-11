use anyhow::{Context, Result, bail};
use mlua::prelude::LuaSerdeExt;
use mlua::{Function, Lua, Table, Value as LuaValue};
use std::path::PathBuf;

use crate::protocol::{
    PluginExecuteContext, PluginExecuteRequest, PluginExecuteResponse, PluginInitResponse,
};

pub struct PluginRef {
    pub name: String,
    pub path: PathBuf,
    pub config: Option<serde_json::Value>,
}

pub struct Plugin {
    pub name: String,
    pub commands: Vec<String>,
    pub lua: LuaPlugin,
}

pub struct LuaPlugin {
    lua: Lua,
    module: Table,
}

impl LuaPlugin {
    pub fn new(plugin_ref: PluginRef) -> Result<Self> {
        let lua = Lua::new();

        let script_dir = plugin_ref.path;
        if !script_dir.is_dir() {
            bail!("Plugin directory does not exist: {}", script_dir.display());
        }

        let dir_str = lua_string_literal(script_dir.to_string_lossy().as_ref());
        let entry_str = lua_string_literal("init");

        let bootstrap = format!(
            r#"
-- Prepend our plugin dir to the module search path
package.path = "{dir}/?.lua;{dir}/?/init.lua;" .. package.path

-- Load the entry module
local ok, mod = pcall(require, "{entry}")
if not ok then error(mod, 0) end
return mod
"#,
            dir = dir_str,
            entry = entry_str
        );

        // Evaluate the bootstrap and capture the returned module table
        let module: Table = lua
            .load(&bootstrap)
            .set_name(&format!("plugin://{}/{}", plugin_ref.name, "init"))
            .eval()
            .with_context(|| format!("Failed to load plugin '{}'", plugin_ref.name))?;

        Ok(Self { lua, module })
    }

    pub async fn initialize(&self) -> Result<PluginInitResponse> {
        let init_v = self.module.get("init")?;
        match init_v {
            mlua::Value::Function(init_fn) => {
                let lua_ret = init_fn.call_async(()).await?;
                let json: serde_json::Value = self.lua.from_value(lua_ret)?;
                let response: PluginInitResponse = serde_json::from_value(json)
                    .with_context(|| "Failed to parse plugin init response")?;
                Ok(response)
            }
            _ => bail!("plugin 'init' is not a function"),
        }
    }

    pub async fn run(
        &self,
        command: String,
        context: &PluginExecuteContext,
    ) -> Result<PluginExecuteResponse> {
        let run_v = self.module.get("execute")?;
        let run_fn: Function = match run_v {
            mlua::Value::Function(f) => f,
            _ => bail!("plugin 'execute' is not a function"),
        };

        let req = PluginExecuteRequest { command, context };

        let lua_req = self.lua.to_value(&serde_json::to_value(&req)?)?;

        let lua_ret: LuaValue = run_fn.call_async(lua_req).await?;

        let json: serde_json::Value = self.lua.from_value(lua_ret)?;

        let response: PluginExecuteResponse = serde_json::from_value(json)
            .with_context(|| "Failed to parse plugin execute response")?;

        Ok(response)
    }
}

fn lua_string_literal(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}
