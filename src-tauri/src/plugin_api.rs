use tauri::{Emitter, Manager};
use yal_plugin::protocol::{PluginAPIEvent, PluginAPIRequest};

pub struct PluginAPIResponse {
    id: String,
    value: serde_json::Value,
}

pub struct PluginAPIResponder(pub kanal::Sender<PluginAPIResponse>);

pub struct PluginAPI {
    pub app_handle: tauri::AppHandle,
    pub responders: std::collections::HashMap<String, kanal::Sender<serde_json::Value>>,
}

impl PluginAPIResponder {
    pub fn send(&self, response: PluginAPIResponse) {
        let _ = self.0.send(response);
    }
}

impl PluginAPI {
    pub fn new(app_handle: tauri::AppHandle) -> Self {
        Self {
            app_handle,
            responders: std::collections::HashMap::new(),
        }
    }

    pub fn handle_plugin_event(&self, id: String, event: PluginAPIEvent) {
        log::info!("Handling plugin event({}): {:?}", id, event);
        match event {
            PluginAPIEvent::Prompt(mut popup) => {
                popup.id = Some(id);
                self.app_handle.emit("popup://show", popup).unwrap();
            }
        }
    }

    pub fn spawn(mut self) -> (kanal::Sender<PluginAPIRequest>, PluginAPIResponder) {
        let (request_tx, request_rx) = kanal::unbounded::<PluginAPIRequest>();
        let (response_tx, response_rx) = kanal::unbounded::<PluginAPIResponse>();
        log::info!("Spawning PluginAPI event loop");
        tauri::async_runtime::spawn(async move {
            loop {
                tokio::select! {
                    response = response_rx.as_async().recv() => {
                        if let Ok(PluginAPIResponse { id, value}) = response {
                            log::info!("Received plugin API response for id {}: {:?}", id, value);
                            let responder = self.responders.remove(&id);
                            if let Some(tx) = responder {
                                let _ = tx.send(value);
                            }
                        } else {
                            log::error!("Plugin API response channel closed");
                            break;
                        }
                    }

                    request = request_rx.as_async().recv() => {
                        if let Ok(request) = request {
                            log::info!("Received plugin API request: {:?}", request);
                            let PluginAPIRequest { id, payload, responder } = request;
                            self.responders.insert(id.clone(), responder);
                            self.handle_plugin_event(id, payload);
                        } else {
                            log::error!("Plugin API request channel closed");
                            break;
                        }
                    }
                }
            }
        });
        (request_tx, PluginAPIResponder(response_tx))
    }
}

#[tauri::command]
pub async fn plugin_api_response_handler(
    app_handle: tauri::AppHandle,
    id: String,
    value: serde_json::Value,
) -> Result<(), String> {
    let plugin_api_ref = app_handle.state::<PluginAPIResponder>();
    let response = PluginAPIResponse { id, value };
    plugin_api_ref.send(response);
    Ok(())
}
