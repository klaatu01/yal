use std::{any::Any, future::Future, marker::PhantomData, pin::Pin};

type BoxFutureAny = Pin<Box<dyn Future<Output = Box<dyn Any + Send + Sync + 'static>> + Send>>;

pub struct APIResponder<T> {
    pub id: String,
    responder: Box<dyn FnMut() -> BoxFutureAny + Send>,
    pub _marker: PhantomData<T>,
}

impl<T> APIResponder<T> {
    /// Create from an async `FnOnce() -> impl Future<Output = Box<dyn Any + Send + Sync>>`.
    pub fn new<F, Fut>(id: String, f: F) -> Self
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = Box<dyn Any + Send + Sync + 'static>> + Send + 'static,
    {
        // Wrap `FnOnce` in an `Option` and expose as `FnMut` to make it object-safe.
        let mut once = Some(f);
        let wrapper = move || -> BoxFutureAny {
            let fut = (once.take().expect("APIResponder called more than once"))();
            Box::pin(fut)
        };

        Self {
            id,
            responder: Box::new(wrapper),
            _marker: PhantomData,
        }
    }

    /// Convenience: invoke and downcast to `T`.
    pub async fn call(mut self) -> Result<T, Box<dyn Any + Send + Sync + 'static>>
    where
        T: Send + Sync + 'static,
    {
        match (self.responder)().await.downcast::<T>() {
            Ok(b) => Ok(*b),
            Err(raw) => Err(raw),
        }
    }
}

pub trait PluginAPI {
    fn gen_id(&self) -> String;
    fn handle<T, R>(&self, request: T) -> APIResponder<R>
    where
        R: Send + Sync + 'static;
}
