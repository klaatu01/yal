pub trait Actor<Protocol> {
    fn start(&self);
    async fn stop(&self);
    async fn handle_message(&self, message: Protocol);
}

pub struct InboxHandle<Protocol> {
    tx: async_channel::Sender<Protocol>,
}

impl<Protocol> InboxHandle<Protocol> {
    pub fn new(tx: async_channel::Sender<Protocol>) -> Self {
        Self { tx }
    }

    pub async fn send(&self, message: Protocol) -> Result<(), async_channel::SendError<Protocol>> {
        self.tx.send(message).await
    }
}

pub struct ActorSystem<T: Actor<Protocol> + Send + Sync, Protocol> {
    _phantom: std::marker::PhantomData<(T, Protocol)>,
}

impl<T: Actor<Protocol> + Send + Sync, Protocol> ActorSystem<T, Protocol> {
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn run(&self, actor: T) ->  {

    }
}
