use std::sync::Arc;

use crate::backend::{Backend, RequestId};
use mlua::{Function, Lua, LuaSerdeExt, Result as LuaResult, UserData, Value};

pub struct Visibility<T: Backend> {
    backend: Arc<T>,
}

impl<T: Backend> Visibility<T> {
    pub fn new(backend: Arc<T>) -> Self {
        Self { backend }
    }

    pub async fn hide(&mut self) -> anyhow::Result<()> {
        self.backend.set_visibility(false).await
    }

    pub async fn show(&mut self) -> anyhow::Result<()> {
        self.backend.set_visibility(true).await
    }
}

impl<T: Backend> UserData for Visibility<T> {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_async_method_mut("hide", |lua, mut this, ()| async move {
            let response = this.hide().await;
            match response {
                Ok(values) => Ok(lua.to_value(&values)?),
                Err(e) => Err(mlua::Error::external(e)),
            }
        });

        methods.add_async_method_mut("show", |lua, mut this, ()| async move {
            let response = this.show().await;
            match response {
                Ok(values) => Ok(lua.to_value(&values)?),
                Err(e) => Err(mlua::Error::external(e)),
            }
        });
    }
}

pub fn create_visibility_module<B: Backend>(
    lua: &Lua,
    plugin_backend: Arc<B>,
) -> LuaResult<Function> {
    let prompt = lua.create_async_function(move |lua, _v: Value| {
        let _backend = plugin_backend.clone();
        async move {
            let visibility = Visibility::new(_backend.clone());

            let ud = lua.create_userdata(visibility)?;
            Ok(ud)
        }
    })?;

    Ok(prompt)
}
