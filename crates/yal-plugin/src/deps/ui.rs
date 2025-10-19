use mlua::{Lua, Result as LuaResult, Table};

use crate::protocol::PluginAPIRequest;

pub mod prompt;

pub fn install_ui_preload(lua: &Lua, event_tx: kanal::Sender<PluginAPIRequest>) -> LuaResult<()> {
    let pkg: Table = lua.globals().get("package")?;
    let preload: Table = pkg.get("preload")?;

    let loader = lua.create_function(move |lua, ()| {
        let m = lua.create_table()?;

        let prompt_module = prompt::create_prompt_module(lua, event_tx.clone())?;
        m.set("prompt", prompt_module)?;

        Ok(m)
    })?;

    preload.set("host.ui", loader)?;
    Ok(())
}
