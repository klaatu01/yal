use futures::StreamExt;
use kameo::actor::ActorRef;
use tarpc::context::Context;
use tarpc::server::{BaseChannel, Channel};
use tauri::Manager;
use tokio_serde::formats::Json;
use yal_ipc::{socket_path, YalIPC};

use crate::application_tree::ApplicationTreeActor;

#[derive(Clone)]
pub struct YalIPCServer {
    app_handle: tauri::AppHandle,
}

impl YalIPC for YalIPCServer {
    async fn ping(self, _ctx: Context) -> String {
        log::info!("Received ping request");
        "pong".to_string()
    }
    async fn tree(self, _ctx: Context) -> serde_json::Value {
        log::info!("Received tree request");
        let tree_actor = self.app_handle.state::<ActorRef<ApplicationTreeActor>>();
        let tree = tree_actor
            .ask(crate::application_tree::SearchParam::All)
            .await
            .unwrap_or_default();

        serde_json::to_value(&tree).unwrap_or(serde_json::Value::Null)
    }
}

impl YalIPCServer {
    pub fn new(app_handle: tauri::AppHandle) -> Self {
        YalIPCServer { app_handle }
    }

    pub async fn spawn(self) {
        let path = socket_path();

        let _ = tokio::fs::remove_file(&path).await;

        let mut incoming = tarpc::serde_transport::unix::listen(&path, Json::default)
            .await
            .expect("uds listen failed");

        incoming.config_mut().max_frame_length(usize::MAX);

        log::info!("YalIPC server listening on {:?}", path);

        tokio::spawn(async move {
            incoming
                .filter_map(|r| async { r.ok() })
                .map(BaseChannel::with_defaults)
                .for_each(|channel| {
                    log::info!("YalIPC server accepted connection");
                    let svc = self.clone();
                    async move {
                        channel.execute(svc.serve()).for_each(|fut| fut).await;
                    }
                })
                .await;
        });
    }
}
