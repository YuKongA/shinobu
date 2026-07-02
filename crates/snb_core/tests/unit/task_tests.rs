use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use super::{shutdown, spawn};

// NOTE: these tests share one process-global runtime + shutdown flag, and
// `shutdown()` is terminal (sets the shutting-down flag permanently). Keep them
// in ONE test function so ordering is deterministic and shutdown runs last.
#[test]
fn spawn_runs_then_shutdown_drains_and_blocks_further_spawns() {
    let counter = Arc::new(AtomicUsize::new(0));

    // Spawned async work runs on the managed runtime.
    for _ in 0..5 {
        let c = counter.clone();
        spawn(async move {
            c.fetch_add(1, Ordering::SeqCst);
        });
    }

    // Give the workers a moment (poll until observed, bounded).
    let start = std::time::Instant::now();
    while counter.load(Ordering::SeqCst) < 5 {
        if start.elapsed() > Duration::from_secs(5) {
            panic!("spawned tasks did not run: {}", counter.load(Ordering::SeqCst));
        }
        std::thread::sleep(Duration::from_millis(10));
    }

    // Shutdown drains the runtime.
    shutdown();

    // After shutdown, spawn is a no-op (must not start new work that could
    // outlive dlclose).
    let post = counter.clone();
    spawn(async move {
        post.fetch_add(1, Ordering::SeqCst);
    });
    std::thread::sleep(Duration::from_millis(100));
    assert_eq!(counter.load(Ordering::SeqCst), 5, "spawn after shutdown must be a no-op");

    // shutdown() is idempotent / safe to call again (macro calls it on every unload).
    shutdown();
}
