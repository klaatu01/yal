use std::path::Path;
use std::time::Duration;

use futures::{SinkExt, StreamExt};
use notify::RecursiveMode;
use notify_debouncer_mini::{
    new_debouncer, DebounceEventResult, DebouncedEvent, DebouncedEventKind,
};

const FILES_TO_WATCH: &[&str] = &["config.toml", "themes.toml"];

pub struct ConfigWatcher {
    event_tx: futures::channel::mpsc::UnboundedSender<crate::common::Events>,
}

impl ConfigWatcher {
    pub fn spawn(event_tx: futures::channel::mpsc::UnboundedSender<crate::common::Events>) {
        tauri::async_runtime::spawn(async move {
            let watcher = Self { event_tx };
            if let Err(e) = watcher.run().await {
                log::error!("ConfigWatcher error: {:?}", e);
            }
        });
    }

    async fn run(mut self) -> notify::Result<()> {
        let (tx, mut rx) = futures::channel::mpsc::unbounded();

        let mut debouncer = new_debouncer(
            Duration::from_millis(250),
            move |res: DebounceEventResult| {
                if let Ok(events) = res {
                    for e in events {
                        if e.kind == DebouncedEventKind::Any {
                            let _ = tx.unbounded_send(e);
                        }
                    }
                }
            },
        )?;

        let file = crate::config::config_path();

        let watcher = debouncer.watcher();
        let dir = file.parent().unwrap_or_else(|| Path::new("."));
        watcher.watch(dir, RecursiveMode::NonRecursive)?;

        while let Some(event) = rx.next().await {
            if self.is_relevant(&event) {
                let _ = self.request_reload().await;
            }
        }

        Ok(())
    }

    fn is_relevant(&self, event: &DebouncedEvent) -> bool {
        event.path == crate::config::config_path()
            && event
                .path
                .file_name()
                .is_some_and(|n| FILES_TO_WATCH.contains(&n.to_str().unwrap_or_default()))
    }

    async fn request_reload(&mut self) -> Result<(), String> {
        self.event_tx
            .send(crate::common::Events::ReloadConfig)
            .await
            .map_err(|e| format!("Failed to send reload event: {}", e))
    }
}
