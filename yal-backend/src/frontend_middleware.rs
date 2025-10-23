use serde::{de::DeserializeOwned, Serialize};
use std::sync::Arc;
use tauri::Emitter;
use yal_core::FrontendRequest;

pub struct FrontendMiddleware {
    app: tauri::AppHandle,
    responders: whirlwind::ShardMap<String, kanal::Sender<serde_json::Value>>,
}

impl FrontendMiddleware {
    pub fn new(app: tauri::AppHandle) -> Self {
        Self {
            app,
            responders: whirlwind::ShardMap::new(),
        }
    }

    pub async fn ask<
        T: Send + Serialize + Clone + 'static,
        R: Send + DeserializeOwned + 'static,
    >(
        &self,
        topic: impl Into<String>,
        id: String,
        data: T,
    ) -> FrontendResponse<R> {
        let topic = topic.into();
        let (tx, rx) = kanal::unbounded::<serde_json::Value>();
        let _ = self.responders.insert(id.clone(), tx).await;
        log::info!("asking frontend {}", &topic);
        let data = FrontendRequest { id, data };
        self.send(topic, data).await;
        FrontendResponse::new(rx)
    }

    pub async fn tell<T: Send + Serialize + Clone + 'static>(
        &self,
        topic: impl Into<String>,
        id: impl Into<String>,
        data: T,
    ) {
        let topic = topic.into();
        let id = id.into();
        let req = FrontendRequest { id, data };
        log::info!("telling frontend {}", &topic);
        self.send(topic, req).await;
    }

    pub async fn respond(
        &self,
        id: impl Into<String>,
        response: serde_json::Value,
    ) -> anyhow::Result<()> {
        let id = id.into();
        if let Some(tx) = self.responders.remove(&id).await {
            tx.send(response)
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("No responder found for id {}", id))
        }
    }

    async fn send<T: Send + Serialize + Clone + 'static>(
        &self,
        topic: String,
        data: FrontendRequest<T>,
    ) {
        self.app.emit(&format!("api://{}", topic), data).unwrap();
    }
}

pub struct FrontendResponse<T: Send + DeserializeOwned + 'static> {
    receiver: kanal::Receiver<serde_json::Value>,
    _marker: std::marker::PhantomData<T>,
}

impl<T: Send + DeserializeOwned + 'static> FrontendResponse<T> {
    pub fn new(receiver: kanal::Receiver<serde_json::Value>) -> Self {
        Self {
            receiver,
            _marker: std::marker::PhantomData,
        }
    }

    pub async fn recv(self) -> anyhow::Result<T> {
        let value = self.receiver.as_async().recv().await?;
        let result: T = serde_json::from_value(value)?;
        Ok(result)
    }
}

#[tauri::command]
pub async fn api_response(
    middleware: tauri::State<'_, Arc<FrontendMiddleware>>,
    id: String,
    response: serde_json::Value,
) -> Result<(), String> {
    middleware
        .respond(id, response)
        .await
        .map_err(|e| e.to_string())
}
