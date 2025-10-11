// src-tauri/src/ns_watcher.rs
#![allow(clippy::needless_lifetimes)]

use futures::channel::mpsc;

#[cfg(target_os = "macos")]
mod macos {
    use super::*;
    use block2::{Block, RcBlock, StackBlock};
    use core::ptr::NonNull;
    use core_graphics::display::{
        CGDisplayRegisterReconfigurationCallback, CGDisplayRemoveReconfigurationCallback,
    };
    use futures::StreamExt;
    use log::{error, info};
    use objc2::rc::Retained;
    use objc2::runtime::{AnyObject, ProtocolObject};
    use objc2_app_kit::{NSRunningApplication, NSWorkspace};
    use objc2_core_foundation::{kCFRunLoopDefaultMode, CFRunLoop, CFType, Type};
    use objc2_foundation::{
        ns_string, NSNotification, NSNotificationCenter, NSObjectProtocol, NSOperationQueue,
        NSString,
    };
    use once_cell::sync::OnceCell;
    use std::cell::RefCell;
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};
    use tauri::async_runtime;
    use tokio::time::sleep;

    pub struct SystemWatcher {
        event_tx: mpsc::UnboundedSender<crate::common::Events>,
    }

    impl SystemWatcher {
        /// Call this **after** the app’s main runloop is alive (e.g. in `tauri::Builder::setup`).
        pub fn spawn(event_tx: mpsc::UnboundedSender<crate::common::Events>) {
            async_runtime::spawn(async move {
                let watcher = Self { event_tx };
                if let Err(e) = watcher.run().await {
                    error!("SystemWatcher error: {e}");
                }
            });
        }

        async fn run(self) -> Result<(), String> {
            info!("Starting SystemWatcher (installing on main runloop)");

            let (raw_tx, mut raw_rx) = mpsc::unbounded::<()>();

            let (ready_tx, mut ready_rx) = mpsc::unbounded::<Result<(), String>>();
            std::thread::Builder::new()
                .name("mac-system-watcher-installer".into())
                .spawn(move || unsafe {
                    if let Some(main_loop) = CFRunLoop::main() {
                        let blk = StackBlock::new(move || {
                            match install_observers_on_main(raw_tx.clone()) {
                                Ok(guard) => {
                                    MAIN_GUARD.with(|cell| {
                                        *cell.borrow_mut() = Some(guard);
                                    });
                                    let _ = ready_tx.unbounded_send(Ok(()));
                                }
                                Err(e) => {
                                    let _ = ready_tx.unbounded_send(Err(e));
                                }
                            }
                        });

                        let cfstr =
                            kCFRunLoopDefaultMode.expect("kCFRunLoopDefaultMode unavailable?");
                        let mode: &CFType = cfstr;
                        main_loop.perform_block(Some(mode), Some(&*blk as &Block<_>));
                        main_loop.wake_up();
                    } else {
                        let _ = ready_tx.unbounded_send(Err(
                            "CFRunLoop::main() returned None; AppKit not initialized?".into(),
                        ));
                    }
                })
                .map_err(|e| format!("spawn error: {e}"))?;

            match ready_rx.next().await {
                Some(Ok(())) => info!("SystemWatcher observers installed on main"),
                Some(Err(e)) => return Err(e),
                None => return Err("failed to install system watchers".into()),
            }

            // Debounce -> RefreshTree
            let debounce_ms = 200u64;
            let mut last: Instant = Instant::now();

            while let Some(()) = raw_rx.next().await {
                if last.elapsed() >= Duration::from_millis(debounce_ms) {
                    last = Instant::now();
                    sleep(Duration::from_millis(1000)).await;
                    if !is_self_frontmost() {
                        let _ = self
                            .event_tx
                            .unbounded_send(crate::common::Events::RefreshTree);
                    }
                } else {
                    // trailing edge send so a burst coalesces
                    let tx = self.event_tx.clone();
                    async_runtime::spawn(async move {
                        sleep(Duration::from_millis(debounce_ms + 20)).await;
                        if !is_self_frontmost() {
                            let _ = tx.unbounded_send(crate::common::Events::RefreshTree);
                        }
                    });
                }
            }

            Ok(())
        }
    }

    fn is_self_frontmost() -> bool {
        use objc2::rc::autoreleasepool;

        unsafe {
            let ws: Retained<NSWorkspace> = NSWorkspace::sharedWorkspace();
            if let Some(front) = ws.frontmostApplication() {
                if let Some(name) = front.localizedName() {
                    return autoreleasepool(|pool| name.to_str(pool).eq_ignore_ascii_case("yal"));
                }
            }
        }
        false
    }

    struct SystemObserverGuard {
        center: Retained<NSNotificationCenter>,
        tokens: Vec<Retained<ProtocolObject<dyn NSObjectProtocol>>>,
        cg_registered: bool,
    }

    impl Drop for SystemObserverGuard {
        fn drop(&mut self) {
            unsafe {
                for token in self.tokens.drain(..) {
                    let any: &objc2::runtime::AnyObject = &*((&*token)
                        as *const ProtocolObject<dyn NSObjectProtocol>
                        as *const objc2::runtime::AnyObject);
                    self.center.removeObserver(any);
                }
                if self.cg_registered {
                    CGDisplayRemoveReconfigurationCallback(display_cb, std::ptr::null());
                }
            }
        }
    }

    thread_local! {
        static MAIN_GUARD: RefCell<Option<SystemObserverGuard>> = const { RefCell::new(None) };
    }

    static SINK: OnceCell<Arc<Mutex<mpsc::UnboundedSender<()>>>> = OnceCell::new();

    unsafe fn install_observers_on_main(
        raw_tx: mpsc::UnboundedSender<()>,
    ) -> Result<SystemObserverGuard, String> {
        let _ = SINK.set(Arc::new(Mutex::new(raw_tx)));

        let ws: Retained<NSWorkspace> = NSWorkspace::sharedWorkspace();
        let center: Retained<NSNotificationCenter> = ws.notificationCenter();

        let main_queue = NSOperationQueue::mainQueue();
        let queue: Option<&NSOperationQueue> = Some(&main_queue);

        let mut tokens: Vec<Retained<ProtocolObject<dyn NSObjectProtocol>>> = Vec::new();

        let mut add = |name: &NSString| {
            let sink = SINK.get().expect("SINK initialized").clone();
            let block = move |_note: NonNull<NSNotification>| {
                let _ = sink.lock().unwrap().unbounded_send(());
            };
            let blk: RcBlock<dyn Fn(NonNull<NSNotification>) + 'static> =
                StackBlock::new(block).copy();

            let token: Retained<ProtocolObject<dyn NSObjectProtocol>> = center
                .addObserverForName_object_queue_usingBlock(
                    Some(name),
                    None,
                    queue,
                    &*blk as &Block<_>,
                );
            tokens.push(token);
        };

        let names: &[&NSString] = &[
            ns_string!("NSWorkspaceActiveSpaceDidChangeNotification"),
            ns_string!("NSWorkspaceDidLaunchApplicationNotification"),
            ns_string!("NSWorkspaceDidTerminateApplicationNotification"),
            ns_string!("NSWorkspaceDidActivateApplicationNotification"),
            ns_string!("NSWorkspaceDidHideApplicationNotification"),
            ns_string!("NSWorkspaceDidUnhideApplicationNotification"),
        ];
        for &n in names {
            add(n);
        }

        let mut cg_registered = false;
        let err = CGDisplayRegisterReconfigurationCallback(display_cb, std::ptr::null());
        if err == 0 {
            cg_registered = true;
        } else {
            error!("CGDisplayRegisterReconfigurationCallback error: {}", err);
        }

        Ok(SystemObserverGuard {
            center,
            tokens,
            cg_registered,
        })
    }

    // Signature must be unsafe extern "C" and *const c_void
    unsafe extern "C" fn display_cb(_display: u32, _flags: u32, _user: *const std::ffi::c_void) {
        if let Some(sink) = SINK.get() {
            let _ = sink.lock().unwrap().unbounded_send(());
        }
    }

    pub use SystemWatcher as PlatformSystemWatcher;
}

pub use macos::PlatformSystemWatcher as SystemWatcher;
