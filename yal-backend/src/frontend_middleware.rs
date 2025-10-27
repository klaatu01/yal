use anyhow::Result;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tauri::Emitter;
use yal_core::FrontendRequest;

pub struct FrontendMiddleware {
    app: tauri::AppHandle,
    responders: tokio::sync::RwLock<HashMap<String, kanal::Sender<FrontendPromise>>>,
}

impl FrontendMiddleware {
    pub fn new(app: tauri::AppHandle) -> Self {
        Self {
            app,
            responders: tokio::sync::RwLock::new(HashMap::new()),
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
        let (tx, rx) = kanal::unbounded();
        let _ = self.responders.write().await.insert(id.clone(), tx);
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
        response: FrontendPromise,
    ) -> anyhow::Result<()> {
        let id = id.into();
        if let Some(tx) = self.responders.write().await.remove(&id) {
            tx.send(response)
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("No responder found for id {}", id))
        }
    }

    pub async fn respond_all(&self, response: Result<serde_json::Value>) -> anyhow::Result<()> {
        let response: FrontendPromise = response.into();
        let mut responders_guard = self.responders.write().await;

        let responders = responders_guard.values();

        for responder in responders {
            let _ = responder.as_async().send(response.clone()).await;
        }

        responders_guard.clear();
        Ok(())
    }

    async fn send<T: Send + Serialize + Clone + 'static>(
        &self,
        topic: String,
        data: FrontendRequest<T>,
    ) {
        self.app.emit(&format!("api://{}", topic), data).unwrap();
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub enum FrontendPromise {
    Fulfilled(serde_json::Value),
    Rejected(String),
}

impl From<anyhow::Result<serde_json::Value>> for FrontendPromise {
    fn from(result: anyhow::Result<serde_json::Value>) -> Self {
        match result {
            Ok(value) => FrontendPromise::Fulfilled(value),
            Err(e) => FrontendPromise::Rejected(e.to_string()),
        }
    }
}

pub struct FrontendResponse<T: Send + DeserializeOwned + 'static> {
    receiver: kanal::Receiver<FrontendPromise>,
    _marker: std::marker::PhantomData<T>,
}

impl<T: Send + DeserializeOwned + 'static> FrontendResponse<T> {
    pub fn new(receiver: kanal::Receiver<FrontendPromise>) -> Self {
        Self {
            receiver,
            _marker: std::marker::PhantomData,
        }
    }

    pub async fn recv(self) -> anyhow::Result<T> {
        let value = self.receiver.as_async().recv().await?;
        match value {
            FrontendPromise::Rejected(e) => Err(anyhow::anyhow!(e)),
            FrontendPromise::Fulfilled(v) => {
                let result =
                    serde_json::from_value::<T>(v).map_err(|e| anyhow::anyhow!(e.to_string()))?;
                Ok(result)
            }
        }
    }
}

#[tauri::command]
pub async fn api_response(
    middleware: tauri::State<'_, Arc<FrontendMiddleware>>,
    id: String,
    response: serde_json::Value,
) -> Result<(), String> {
    middleware
        .respond(id, Ok(response).into())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn api_error(
    middleware: tauri::State<'_, Arc<FrontendMiddleware>>,
    id: String,
    error: String,
) -> Result<(), String> {
    middleware
        .respond(id, Err(anyhow::anyhow!(error)).into())
        .await
        .map_err(|e| e.to_string())
}
