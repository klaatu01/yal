use mlua::{Lua, Result as LuaResult, Table, Value};

use base64::{
    Engine as _,
    engine::general_purpose::{STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD},
};

pub fn install_base64_preload(lua: &Lua) -> LuaResult<()> {
    let pkg: Table = lua.globals().get("package")?;
    let preload: Table = pkg.get("preload")?;

    let loader = lua.create_function(|lua, ()| {
        let m = lua.create_table()?;

        // opts.pad? (defaults to true)
        let get_pad = |opts: Option<Table>| -> bool {
            if let Some(t) = opts
                && let Ok(p) = t.get("pad")
            {
                return p;
            }
            true
        };

        // encode(str|bytes, opts?)
        let encode = lua.create_function(move |_lua, (input, opts): (Value, Option<Table>)| {
            let s = match input {
                Value::String(ls) => ls.as_bytes().to_vec(),
                _ => return Err(mlua::Error::external("encode expects a string")),
            };
            let pad = get_pad(opts);
            let engine = if pad { STANDARD } else { STANDARD_NO_PAD };
            let out = engine.encode(s);
            Ok(out)
        })?;
        m.set("encode", encode)?;

        // encode_url(str|bytes, opts?)
        let encode_url =
            lua.create_function(move |_lua, (input, opts): (Value, Option<Table>)| {
                let s = match input {
                    Value::String(ls) => ls.as_bytes().to_vec(),
                    _ => return Err(mlua::Error::external("encode_url expects a string")),
                };
                let pad = get_pad(opts);
                let engine = if pad { URL_SAFE } else { URL_SAFE_NO_PAD };
                let out = engine.encode(s);
                Ok(out)
            })?;
        m.set("encode_url", encode_url)?;

        // decode(b64, opts?) -> raw bytes (Lua string)
        let decode = lua.create_function(move |lua, (b64s, _opts): (String, Option<Table>)| {
            let bytes = STANDARD.decode(b64s).map_err(mlua::Error::external)?;
            lua.create_string(&bytes)
        })?;
        m.set("decode", decode)?;

        // decode_url(b64, opts?) -> raw bytes (Lua string)
        let decode_url =
            lua.create_function(move |lua, (b64s, _opts): (String, Option<Table>)| {
                let bytes = URL_SAFE.decode(b64s).map_err(mlua::Error::external)?;
                lua.create_string(&bytes)
            })?;
        m.set("decode_url", decode_url)?;

        Ok(m)
    })?;

    preload.set("host.base64", loader)?;
    Ok(())
}
