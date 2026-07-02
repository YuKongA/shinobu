//! A per-plugin managed runtime for fire-and-forget background async work.
//!
//! Plugins must spawn background work here instead of detaching raw OS threads.
//! Why: the host's unload drain ([`snb_runtime`]'s `unregister_plugin`) waits
//! only on the plugin's component `Arc`s and its `PluginCell` before running
//! `on_unload` and `dlclose`-ing the dylib. A detached thread holds none of
//! those, so the drain can't see it and would `dlclose` while it still runs
//! cdylib code — a use-after-free (observed as `STATUS_ACCESS_VIOLATION` on
//! Windows). Tasks spawned here live on a runtime that [`shutdown`] drains
//! before the library unmaps.
//!
//! The runtime is a per-cdylib static (each plugin statically links its own
//! `snb_core`), so every plugin owns exactly one. The `#[plugin]` macro injects
//! a [`shutdown`] call into the generated `destroy_plugin`, which runs after
//! `on_unload` and before the `Library` unmaps on the clean unload path (the
//! leak path never `dlclose`s, so tasks are safe there too) — so plugin authors
//! get this protection automatically, without a manual teardown call.

use std::future::Future;
use std::sync::RwLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use tokio::runtime::Runtime;

/// Max time [`shutdown`] waits for tasks to finish before detaching remaining
/// workers. Async tasks cancel at their next await (instant during a network
/// teardown, the common unload case); this only bounds a genuinely blocking
/// task so unload can never hang the host.
const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);

static RUNTIME: RwLock<Option<Runtime>> = RwLock::new(None);
static SHUTTING_DOWN: AtomicBool = AtomicBool::new(false);

fn build_runtime() -> std::io::Result<Runtime> {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .thread_name("snb-task")
        .build()
}

/// Spawn fire-and-forget async work on this plugin's managed runtime.
///
/// A no-op once [`shutdown`] has started, so an unload in progress can't start
/// new work that would outlive `dlclose`. Task panics are contained by tokio at
/// the task boundary (inside this cdylib) and do not reach the host.
pub fn spawn<F>(future: F)
where
    F: Future<Output = ()> + Send + 'static,
{
    if SHUTTING_DOWN.load(Ordering::Acquire) {
        return;
    }
    // Fast path: runtime already built.
    {
        let guard = RUNTIME.read().unwrap();
        if let Some(rt) = guard.as_ref() {
            rt.spawn(future);
            return;
        }
    }
    // Slow path: create it under the write lock (double-checked).
    let mut guard = RUNTIME.write().unwrap();
    if SHUTTING_DOWN.load(Ordering::Acquire) {
        return;
    }
    if guard.is_none() {
        match build_runtime() {
            Ok(rt) => *guard = Some(rt),
            Err(e) => {
                log::error!("snb_core::task: failed to create runtime: {e}");
                return;
            }
        }
    }
    if let Some(rt) = guard.as_ref() {
        rt.spawn(future);
    }
}

/// Drain and drop this plugin's managed runtime. Called automatically from the
/// `#[plugin]`-generated `destroy_plugin` before the library unmaps.
///
/// Sets the shutting-down flag first (so concurrent/late `spawn`s become
/// no-ops), then `shutdown_timeout`s the runtime: async tasks are cancelled at
/// their next await and worker threads are joined, bounded by
/// [`SHUTDOWN_TIMEOUT`] so unload can never hang. Idempotent and safe to call
/// when no runtime was ever created (cheap no-op).
pub fn shutdown() {
    SHUTTING_DOWN.store(true, Ordering::Release);
    let runtime = RUNTIME.write().unwrap().take();
    if let Some(runtime) = runtime {
        runtime.shutdown_timeout(SHUTDOWN_TIMEOUT);
    }
}

#[cfg(test)]
#[path = "../tests/unit/task_tests.rs"]
mod task_tests;
