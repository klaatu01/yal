use mlua::{Lua, LuaSerdeExt, Result as LuaResult, Table, Value};
use serde_json;

pub fn install_json_preload(lua: &Lua) -> LuaResult<()> {
    let pkg: Table = lua.globals().get("package")?;
    let preload: Table = pkg.get("preload")?;

    let loader = lua.create_function(|lua, ()| {
        let m = lua.create_table()?;

        // json.encode(lua_value) -> string
        let enc = lua.create_function(|lua, v: Value| {
            let sv: serde_json::Value = lua.from_value(v)?;
            let s = serde_json::to_string(&sv).map_err(mlua::Error::external)?;
            Ok(s)
        })?;
        m.set("encode", enc)?;

        // json.decode(string) -> lua_value
        let dec = lua.create_function(|lua, s: String| {
            let v: serde_json::Value = serde_json::from_str(&s).map_err(mlua::Error::external)?;
            let lv = lua.to_value(&v)?;
            Ok(lv)
        })?;
        m.set("decode", dec)?;

        Ok(m)
    })?;

    preload.set("yal.json", loader)?;
    Ok(())
}
