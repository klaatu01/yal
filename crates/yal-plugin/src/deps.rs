use anyhow::Result;
use mlua::Lua;

pub mod base64;
pub mod http;
pub mod json;
pub mod log;
pub mod vendor;

/// Options for installing builtins for a given Lua state (plugin VM).
pub struct InstallOptions<'a> {
    /// Optional vendor folder (pure-Lua modules live here).
    pub vendor_dir: Option<&'a std::path::Path>,
    /// Concurrency / timeout / size limits for HTTP.
    pub http_limits: Option<http::HttpLimits>,
}

/// Install host modules + vendor searcher into a Lua state.
pub fn install_all(lua: &Lua, opts: InstallOptions) -> Result<()> {
    // host.json
    json::install_json_preload(lua)?;

    // host.http
    let limits = opts.http_limits.unwrap_or_default();
    let env = http::HttpEnv::new(limits)?;
    http::install_http_preload(lua, env)?;

    // host.base64
    base64::install_base64_preload(lua)?;

    // host.log
    log::install_log_preload(lua)?;

    // vendor searcher (pure-Lua deps bundled with the plugin)
    if let Some(vendor_dir) = opts.vendor_dir {
        vendor::add_vendor_searcher(lua, vendor_dir)?;
    }

    Ok(())
}
