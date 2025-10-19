use mlua::{Lua, Result as LuaResult, Table};

pub fn install_log_preload(lua: &Lua) -> LuaResult<()> {
    let pkg: Table = lua.globals().get("package")?;
    let preload: Table = pkg.get("preload")?;

    let loader = lua.create_function(|lua, ()| {
        let m = lua.create_table()?;

        let debug = lua.create_function(|_, msg: String| {
            log::debug!("[lua] {}", msg);
            Ok(())
        })?;
        let info = lua.create_function(|_, msg: String| {
            log::info!("[lua] {}", msg);
            Ok(())
        })?;
        let warn = lua.create_function(|_, msg: String| {
            log::warn!("[lua] {}", msg);
            Ok(())
        })?;
        let error = lua.create_function(|_, msg: String| {
            log::error!("[lua] {}", msg);
            Ok(())
        })?;

        m.set("debug", debug)?;
        m.set("info", info)?;
        m.set("warn", warn)?;
        m.set("error", error)?;
        Ok(m)
    })?;

    preload.set("yal.log", loader)?;
    Ok(())
}
