use std::fs;

use anyhow::Result;
use mlua::{Lua, LuaSerdeExt};
use mlua::{Table, Value};
use serde::de::DeserializeOwned;
use std::path::Path;

pub fn load_config<ConfigType: DeserializeOwned + Default>(path: &Path) -> ConfigType {
    let lua = Lua::new();
    if path.exists()
        && let Ok(table) = eval_lua_file(path, &lua)
    {
        return lua
            .from_value::<ConfigType>(Value::Table(table))
            .unwrap_or_else(|e| {
                eprintln!("Failed to parse config.lua: {}", e);
                ConfigType::default()
            });
    }

    ConfigType::default()
}

fn eval_lua_file(path: &Path, lua: &Lua) -> Result<Table> {
    let src = fs::read_to_string(path)?;
    let value = lua
        .load(&src)
        .set_name(path.to_string_lossy())
        .eval::<Value>()?;

    match value {
        Value::Table(table) => Ok(table),
        _ => Err(anyhow::anyhow!(
            "Lua file did not return a table: {}",
            path.display()
        )),
    }
}
