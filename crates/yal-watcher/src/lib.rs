use futures::FutureExt;

pub trait Watcher: Send + 'static {
    fn watch(&mut self) -> impl std::future::Future<Output = ()> + Send;
    fn spawn(self) -> WatcherRef
    where
        Self: Sized,
    {
        WatcherInstance::spawn(self)
    }
}

pub struct WatcherRef {
    terminate_tx: futures::channel::oneshot::Sender<()>,
}

impl WatcherRef {
    pub fn terminate(self) {
        let _ = self.terminate_tx.send(());
    }
}

pub struct WatcherInstance<W: Watcher> {
    terminate_rx: futures::channel::oneshot::Receiver<()>,
    watcher: W,
}

impl<W: Watcher> WatcherInstance<W> {
    pub fn spawn(watcher: W) -> WatcherRef {
        let (terminate_tx, terminate_rx) = futures::channel::oneshot::channel();
        let watcher_ref = WatcherRef { terminate_tx };
        tokio::spawn(async move {
            let instance = WatcherInstance {
                terminate_rx,
                watcher,
            };
            instance.run().await;
        });
        watcher_ref
    }

    pub async fn run(mut self) {
        let terminate_fut = &mut self.terminate_rx;

        loop {
            let watch_fut = self.watcher.watch();
            futures::select! {
                _ = terminate_fut.fuse() => {
                    break;
                },
                _ = watch_fut.fuse() => {},
            }
        }
    }
}
