use tarpc::{service, tokio_serde::formats::Json};

mod utils;
pub use utils::*;

#[service]
pub trait YalIPC {
    async fn ping() -> String;
    async fn tree() -> serde_json::Value;
}

pub async fn connect() -> Result<YalIPCClient, ()> {
    let transport = tarpc::serde_transport::unix::connect(socket_path(), Json::default)
        .await
        .map_err(|_| ())?;
    Ok(YalIPCClient::new(tarpc::client::Config::default(), transport).spawn())
}
