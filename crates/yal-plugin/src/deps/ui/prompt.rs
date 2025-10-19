use crate::protocol::{PluginAPIEvent, PluginAPIRequest};
use mlua::{Function, Lua, LuaSerdeExt, Result as LuaResult, Value};

pub fn create_prompt_module(
    lua: &Lua,
    event_tx: kanal::Sender<PluginAPIRequest>,
) -> LuaResult<Function> {
    let event_tx = event_tx.clone();

    // Async Lua function: can be awaited from Rust (and cooperatively from Lua when driven async)
    let prompt = lua.create_async_function(move |lua, v: Value| {
        // Capture everything we need *before* await
        let event_tx = event_tx.clone();

        // Deserialize outside of await to avoid borrowing `lua` across await points
        async move {
            let form = lua.from_value(v).unwrap();
            // Create a request/response pair (use an async receiver)
            let (request, rx) = PluginAPIRequest::new(PluginAPIEvent::Prompt(form));

            event_tx.as_async().send(request).await.map_err(|e| {
                mlua::Error::external(format!("Failed to send prompt request: {}", e))
            })?;

            // Await the response asynchronously
            let response = rx.as_async().recv().await.map_err(|e| {
                mlua::Error::external(format!("Failed to receive prompt response: {}", e))
            })?;

            let response = lua.to_value(&response)?;
            Ok(response)
        }
    })?;

    Ok(prompt)
}
