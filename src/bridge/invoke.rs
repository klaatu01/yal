use serde::{Deserialize, Serialize};
use serde_json::json;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;
use yal_core::{AppConfig, Theme};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;
}

#[derive(Serialize, Deserialize)]
pub struct Empty {}

#[derive(Serialize, Deserialize)]
pub struct RunCmdArgs<T> {
    pub cmd: T,
}

pub async fn run_cmd<T: Serialize>(cmd: T) {
    let args = serde_wasm_bindgen::to_value(&RunCmdArgs { cmd }).unwrap();
    let _ = invoke("run_cmd", args).await;
}

pub async fn hide_window() {
    let _ = invoke(
        "hide_window",
        serde_wasm_bindgen::to_value(&Empty {}).unwrap(),
    )
    .await;
}

pub async fn get_config() -> Option<AppConfig> {
    let v = invoke(
        "get_config",
        serde_wasm_bindgen::to_value(&Empty {}).unwrap(),
    )
    .await;
    serde_wasm_bindgen::from_value::<AppConfig>(v).ok()
}

pub async fn get_theme() -> Option<Theme> {
    let v = invoke(
        "get_theme",
        serde_wasm_bindgen::to_value(&Empty {}).unwrap(),
    )
    .await;
    serde_wasm_bindgen::from_value::<Theme>(v).ok()
}

pub async fn api_respond<T: Serialize>(id: String, response: T) {
    let resp_json: serde_json::Value = serde_json::to_value(response).unwrap();
    let args = serde_wasm_bindgen::to_value(&json!({
        "id": id,
        "response": resp_json
    }))
    .unwrap();
    let _ = invoke("api_response", args).await;
}
