use std::sync::Arc;

use mlua::{Lua, Result as LuaResult, Table};

use crate::backend::Backend;

pub mod prompt;

pub fn install_ui_preload<B: Backend>(lua: &Lua, plugin_backend: Arc<B>) -> LuaResult<()> {
    let pkg: Table = lua.globals().get("package")?;
    let preload: Table = pkg.get("preload")?;

    let loader = lua.create_function(move |lua, ()| {
        let m = lua.create_table()?;

        let prompt_module = prompt::create_prompt_module(lua, plugin_backend.clone())?;
        m.set("prompt", prompt_module)?;

        Ok(m)
    })?;

    preload.set("yal.ui", loader)?;
    Ok(())
}
