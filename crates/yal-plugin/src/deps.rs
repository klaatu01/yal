use anyhow::Result;
use mlua::Lua;

use crate::protocol::PluginAPIRequest;

pub mod base64;
pub mod db;
pub mod http;
pub mod json;
pub mod log;
pub mod socket;
pub mod ui;
pub mod vendor;

pub struct InstallOptions<'a> {
    pub vendor_dir: Option<&'a std::path::Path>,
    pub http_limits: Option<http::HttpLimits>,
    pub event_tx: kanal::Sender<PluginAPIRequest>,
}

pub fn install_all(lua: &Lua, opts: InstallOptions) -> Result<()> {
    json::install_json_preload(lua)?;

    let limits = opts.http_limits.unwrap_or_default();
    let env = http::HttpEnv::new(limits)?;
    http::install_http_preload(lua, env)?;
    socket::install_socket_preload(lua)?;

    base64::install_base64_preload(lua)?;

    log::install_log_preload(lua)?;

    ui::install_ui_preload(lua, opts.event_tx.clone())?;

    db::install_db_preload(lua)?;

    if let Some(vendor_dir) = opts.vendor_dir {
        vendor::add_vendor_searcher(lua, vendor_dir)?;
    }

    Ok(())
}
