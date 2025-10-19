use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use mlua::{
    Lua, LuaSerdeExt, Result as LuaResult, Table, UserData, UserDataMethods, Value as LuaValue,
};
use parking_lot::Mutex;
use serde_json::{self as json, Value as JValue};

#[derive(Debug)]
struct KVInner {
    path: PathBuf,
    map: json::Map<String, JValue>,
}

#[derive(Clone, Debug)]
struct KV(Arc<Mutex<KVInner>>);

/* -------------------------- Path resolution -------------------------- */

fn env_path(var: &str) -> Option<PathBuf> {
    std::env::var_os(var)
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
}

fn home_dir() -> Option<PathBuf> {
    // Unix/macOS: respect HOME if present
    env_path("HOME")
}

fn xdg_state_home() -> Option<PathBuf> {
    if let Some(p) = env_path("XDG_STATE_HOME") {
        Some(p)
    } else {
        // Fallback: ~/.local/state
        home_dir().map(|h| h.join(".local/state"))
    }
}

fn xdg_config_home() -> Option<PathBuf> {
    if let Some(p) = env_path("XDG_CONFIG_HOME") {
        Some(p)
    } else {
        // Fallback: ~/.config
        home_dir().map(|h| h.join(".config"))
    }
}

fn fallback_home() -> Option<PathBuf> {
    home_dir().map(|h| h.join(".yal"))
}

fn db_path_for(namespace: &str) -> PathBuf {
    // 1) XDG_STATE_HOME
    if let Some(root) = xdg_state_home() {
        return root.join("yal/plugins").join(format!("{namespace}.json"));
    }
    // 2) XDG_CONFIG_HOME
    if let Some(root) = xdg_config_home() {
        return root.join("yal/plugins").join(format!("{namespace}.json"));
    }
    // 3) ~/.yal (last resort)
    if let Some(root) = fallback_home() {
        return root.join("plugins").join(format!("{namespace}.json"));
    }
    // 4) cwd/plugins
    PathBuf::from("plugins").join(format!("{namespace}.json"))
}

/* ----------------------------- IO helpers ---------------------------- */

fn ensure_parent_dir(p: &Path) -> std::io::Result<()> {
    if let Some(dir) = p.parent() {
        fs::create_dir_all(dir)?;
    }
    Ok(())
}

fn load_map_from(path: &Path) -> json::Map<String, JValue> {
    match fs::File::open(path) {
        Ok(mut f) => {
            let mut s = String::new();
            if f.read_to_string(&mut s).is_ok() {
                if s.trim().is_empty() {
                    return json::Map::new();
                }
                match json::from_str::<JValue>(&s) {
                    Ok(JValue::Object(m)) => m,
                    _ => json::Map::new(),
                }
            } else {
                json::Map::new()
            }
        }
        Err(_) => json::Map::new(),
    }
}

fn atomic_write_json(path: &Path, map: &json::Map<String, JValue>) -> std::io::Result<()> {
    ensure_parent_dir(path)?;
    let tmp = path.with_extension("json.tmp");
    {
        let mut f = fs::File::create(&tmp)?;
        let data =
            json::to_vec_pretty(&JValue::Object(map.clone())).unwrap_or_else(|_| b"{}".to_vec());
        f.write_all(&data)?;
        f.flush()?;
    }
    // POSIX rename is atomic
    fs::rename(&tmp, path)?;
    Ok(())
}

/* ----------------------------- UserData ------------------------------ */

impl UserData for KV {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        // get(key) -> any|nil
        methods.add_method("get", |lua, this, key: String| {
            let inner = this.0.lock();
            if let Some(v) = inner.map.get(&key) {
                let lv = lua.to_value(v)?;
                Ok(Some(lv))
            } else {
                Ok(None)
            }
        });

        // set(key, value) -> true (write-through)
        methods.add_method("set", |lua, this, (key, val): (String, LuaValue)| {
            let mut inner = this.0.lock();
            let jv: JValue = lua.from_value(val)?;
            inner.map.insert(key, jv);
            atomic_write_json(&inner.path, &inner.map).map_err(mlua::Error::external)?;
            Ok(true)
        });

        // set_many(table) -> true
        methods.add_method("set_many", |lua, this, tbl: Table| {
            let mut inner = this.0.lock();
            for pair in tbl.pairs::<LuaValue, LuaValue>() {
                let (k, v) = pair?;
                let key = match k {
                    LuaValue::String(s) => s.to_str()?.to_string(),
                    LuaValue::Integer(i) => i.to_string(),
                    LuaValue::Number(n) => {
                        if n.fract() == 0.0 {
                            (n as i64).to_string()
                        } else {
                            n.to_string()
                        }
                    }
                    _ => {
                        return Err(mlua::Error::external(
                            "set_many: keys must be string/number",
                        ));
                    }
                };
                let jv: JValue = lua.from_value(v)?;
                inner.map.insert(key, jv);
            }
            atomic_write_json(&inner.path, &inner.map).map_err(mlua::Error::external)?;
            Ok(true)
        });

        // delete(key) -> true
        methods.add_method("delete", |_, this, key: String| {
            let mut inner = this.0.lock();
            inner.map.remove(&key);
            atomic_write_json(&inner.path, &inner.map).map_err(mlua::Error::external)?;
            Ok(true)
        });

        // keys() -> { "k1", "k2", ... }
        methods.add_method("keys", |lua, this, ()| {
            let inner = this.0.lock();
            let arr = lua.create_table()?;
            let mut i = 1;
            for k in inner.map.keys() {
                arr.set(i, k.as_str())?;
                i += 1;
            }
            Ok(arr)
        });

        // all() -> table (deep copy)
        methods.add_method("all", |lua, this, ()| {
            let inner = this.0.lock();
            let table = lua.to_value(&JValue::Object(inner.map.clone()))?;
            Ok(table)
        });

        // path() -> string
        methods.add_method("path", |lua, this, ()| {
            let inner = this.0.lock();
            lua.create_string(inner.path.to_string_lossy().as_ref())
        });

        // flush() -> true
        methods.add_method("flush", |_, this, ()| {
            let inner = this.0.lock();
            atomic_write_json(&inner.path, &inner.map).map_err(mlua::Error::external)?;
            Ok(true)
        });

        // reload() -> true
        methods.add_method("reload", |_, this, ()| {
            let mut inner = this.0.lock();
            inner.map = load_map_from(&inner.path);
            Ok(true)
        });
    }
}

/* --------------------------- Module preload -------------------------- */

pub fn install_db_preload(lua: &Lua) -> LuaResult<()> {
    let pkg: Table = lua.globals().get("package")?;
    let preload: Table = pkg.get("preload")?;

    let loader = lua.create_function(|lua, ()| {
        let m = lua.create_table()?;

        let open_fn = lua.create_function(|lua, namespace: String| {
            let path = db_path_for(&namespace);
            if let Err(e) = ensure_parent_dir(&path) {
                return Err(mlua::Error::external(e));
            }
            let map = load_map_from(&path);
            let kv = KV(Arc::new(Mutex::new(KVInner { path, map })));
            lua.create_userdata(kv)
        })?;

        m.set("open", open_fn)?;
        Ok(m)
    })?;

    preload.set("yal.db", loader)?;
    Ok(())
}
