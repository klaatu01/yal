use std::sync::Arc;

use crate::backend::{Backend, RequestId};
use mlua::{Function, Lua, LuaSerdeExt, Result as LuaResult, UserData, Value};

pub struct Prompt<T: Backend> {
    backend: Arc<T>,
    pub prompt_id: RequestId,
    result: Option<yal_core::PromptResponse>,
}

impl<T: Backend> Prompt<T> {
    pub fn new(prompt_id: RequestId, backend: Arc<T>) -> Self {
        Self {
            backend,
            prompt_id,
            result: None,
        }
    }

    pub async fn submission(&mut self) -> anyhow::Result<serde_json::Value> {
        if let Some(yal_core::PromptResponse::Submit { values }) = &self.result {
            return Ok(values.clone());
        }
        let resp = self.backend.prompt_submission(self.prompt_id.clone()).await;
        let resp = loop {
            match resp {
                Ok(ref response) => match response {
                    yal_core::PromptResponse::Submit { values } => {
                        break Ok(values.clone());
                    }
                    yal_core::PromptResponse::Cancel => {
                        break Err(anyhow::anyhow!("Prompt was cancelled by user"));
                    }
                    _ => {
                        continue;
                    }
                },
                Err(e) => {
                    break Err(anyhow::anyhow!(
                        "Failed to receive prompt submission: {}",
                        e
                    ));
                }
            };
        };
        if resp.is_err() {
            self.cancel().await?;
        }
        resp
    }

    pub async fn state(&mut self) -> anyhow::Result<Option<serde_json::Value>> {
        let resp = self.backend.prompt_state(self.prompt_id.clone()).await;
        let resp = match resp {
            Ok(response) => match response {
                yal_core::PromptResponse::State { values } => Ok(Some(values)),
                yal_core::PromptResponse::Submit { .. } => {
                    self.result = Some(response);
                    Ok(None)
                }
                yal_core::PromptResponse::Cancel => {
                    Err(anyhow::anyhow!("Prompt was cancelled by user"))
                }
            },
            Err(e) => Err(anyhow::anyhow!("Failed to receive prompt state: {}", e)),
        };
        if resp.is_err() {
            self.cancel().await?;
        }
        resp
    }

    pub async fn cancel(&mut self) -> anyhow::Result<()> {
        self.backend.prompt_cancel(self.prompt_id.clone()).await
    }
}

impl<T: Backend> UserData for Prompt<T> {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_async_method_mut("submission", |lua, mut this, ()| async move {
            let response = this.submission().await;
            match response {
                Ok(values) => Ok(lua.to_value(&values)?),
                Err(e) => Err(mlua::Error::external(e)),
            }
        });

        methods.add_async_method_mut("state", |lua, mut this, ()| async move {
            let response = this.state().await;
            match response {
                Ok(opt_values) => {
                    if let Some(values) = opt_values {
                        Ok(lua.to_value(&values)?)
                    } else {
                        Ok(Value::Nil)
                    }
                }
                Err(e) => Err(mlua::Error::external(e)),
            }
        });

        methods.add_async_method_mut("cancel", |_, mut this, ()| async move {
            let response = this.cancel().await;
            match response {
                Ok(_) => Ok(()),
                Err(e) => Err(mlua::Error::external(e)),
            }
        });
    }
}

pub fn create_prompt_module<B: Backend>(lua: &Lua, plugin_backend: Arc<B>) -> LuaResult<Function> {
    let prompt = lua.create_async_function(move |lua, v: Value| {
        let _backend = plugin_backend.clone();
        async move {
            let prompt_request = lua.from_value(v)?;

            let request_id = _backend
                .prompt(prompt_request)
                .await
                .map_err(mlua::Error::external)?;

            let prompt = Prompt::new(request_id, _backend.clone());

            let ud = lua.create_userdata(prompt)?;
            Ok(ud)
        }
    })?;

    Ok(prompt)
}
