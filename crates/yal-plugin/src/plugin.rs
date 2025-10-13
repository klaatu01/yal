use anyhow::{Context, Result, bail};
use mlua::prelude::LuaSerdeExt;
use mlua::{Function, Lua, Table, Value as LuaValue};
use std::path::PathBuf;

#[cfg(debug_assertions)]
use std::time::Instant;

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
    execute: Function,
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

        // Cache `execute`
        let execute = match module.get("execute")? {
            mlua::Value::Function(f) => f,
            _ => bail!("plugin 'execute' is not a function"),
        };

        Ok(Self {
            lua,
            module,
            execute,
        })
    }

    pub async fn initialize(&self) -> Result<PluginInitResponse> {
        let init_v = self.module.get("init")?;
        match init_v {
            mlua::Value::Function(init_fn) => {
                let lua_ret = init_fn.call_async(()).await?;
                // Directly convert Lua -> Rust (no JSON round-trip)
                let response: PluginInitResponse = self.lua.from_value(lua_ret)?;
                Ok(response)
            }
            _ => bail!("plugin 'init' is not a function"),
        }
    }

    pub async fn run(
        &self,
        command: String,
        context: &PluginExecuteContext,
        args: Option<serde_json::Value>,
    ) -> Result<PluginExecuteResponse> {
        #[cfg(debug_assertions)]
        let now = Instant::now();

        #[cfg(debug_assertions)]
        log::info!("Running plugin command: {}", command);

        // Build the request directly; mlua will serialize it without JSON
        let req = PluginExecuteRequest {
            command,
            context,
            args,
        };

        // Rust -> Lua
        let lua_req = self.lua.to_value(&req)?;

        // Call cached execute (async)
        let lua_ret: LuaValue = self.execute.call_async(lua_req).await?;

        // Lua -> Rust (no JSON)
        let response: PluginExecuteResponse = self.lua.from_value(lua_ret)?;

        #[cfg(debug_assertions)]
        log::info!("Plugin command completed in {:.2?}", now.elapsed());

        Ok(response)
    }
}

fn lua_string_literal(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}
