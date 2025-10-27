use anyhow::Result;
use std::sync::Arc;
use tauri::AppHandle;
use yal_plugin::backend::{Backend, RequestId};

#[derive(Clone)]
pub struct PluginBackend {
    app: AppHandle,
    middleware: Arc<crate::frontend_middleware::FrontendMiddleware>,
}

impl PluginBackend {
    pub fn new(
        app: AppHandle,
        middleware: Arc<crate::frontend_middleware::FrontendMiddleware>,
    ) -> Self {
        Self { middleware, app }
    }

    pub fn generate_request_id(&self) -> RequestId {
        nanoid::nanoid!(21)
    }
}

impl Backend for PluginBackend {
    async fn prompt(&self, _prompt: yal_core::Prompt) -> Result<RequestId> {
        let request_id = self.generate_request_id();
        self.middleware
            .tell("prompt:show", request_id.clone(), _prompt)
            .await;
        Ok(request_id)
    }
    async fn prompt_state(&self, id: RequestId) -> Result<yal_core::PromptResponse> {
        self.middleware
            .ask("prompt:state", id.clone(), serde_json::json!({}))
            .await
            .recv()
            .await
            .map_err(|e| anyhow::anyhow!(e.to_string()))
    }
    async fn prompt_submission(&self, id: RequestId) -> Result<yal_core::PromptResponse> {
        self.middleware
            .ask("prompt:submit", id.clone(), serde_json::json!({}))
            .await
            .recv()
            .await
            .map_err(|e| anyhow::anyhow!(e.to_string()))
    }
    async fn prompt_cancel(&self, _id: RequestId) -> Result<()> {
        self.middleware
            .tell("prompt:cancel", _id.clone(), serde_json::json!({}))
            .await;
        Ok(())
    }
    async fn set_visibility(&self, visible: bool) -> Result<()> {
        if visible {
            self.app.show().unwrap();
        } else {
            self.app.hide().unwrap();
        }
        Ok(())
    }
}
