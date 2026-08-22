//! Adaptive threading for rules-engine background work.

use rayon::{ThreadPool, ThreadPoolBuildError, ThreadPoolBuilder};

use crate::EngineError;

/// A Rayon executor that adapts its scheduling policy to the runtime platform.
///
/// Native builds schedule work on a dedicated parallel pool. On Web, Battlement's
/// page initializer chooses the worker count: desktop browsers get a parallel
/// worker pool, while mobile browsers get a current-thread pool and synchronous
/// execution. The mobile fallback avoids starting nested WebAssembly workers,
/// which can hang or crash Unity players even when `SharedArrayBuffer` exists.
pub struct AdaptiveThreadPool {
    pool: ThreadPool,
    execute_synchronously: bool,
}

impl AdaptiveThreadPool {
    /// Creates an executor using the platform's preferred scheduling policy.
    pub fn new() -> Result<Self, EngineError> {
        self::build_pool()
            .map(|(pool, execute_synchronously)| Self {
                pool,
                execute_synchronously,
            })
            .map_err(|error| EngineError::new(format!("could not create thread pool: {error}")))
    }

    /// Executes immediately on mobile Web or schedules on a background worker.
    pub fn execute(&self, operation: impl FnOnce() + Send + 'static) {
        if self.execute_synchronously {
            self.pool.install(operation);
        } else {
            self.pool.spawn(operation);
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn build_pool() -> Result<(ThreadPool, bool), ThreadPoolBuildError> {
    // Native platforms are not subject to browser worker restrictions. Even a
    // one-core host still schedules asynchronously so gameplay remains responsive.
    ThreadPoolBuilder::new().build().map(|pool| (pool, false))
}

#[cfg(target_arch = "wasm32")]
fn build_pool() -> Result<(ThreadPool, bool), ThreadPoolBuildError> {
    // SAFETY: Unity links this symbol from BattlementWebThreading.jslib. init.js
    // initializes its positive thread count before Unity loads the Wasm module.
    let thread_count = unsafe { self::battlement_web_thread_count() }.max(1) as usize;
    let builder = ThreadPoolBuilder::new().num_threads(thread_count);
    if thread_count == 1 {
        // Rayon normally adapts how many workers it creates, but it cannot know
        // that spawning a nested Wasm worker is unsafe on some mobile browsers.
        // Current-thread mode creates no worker and therefore needs synchronous
        // dispatch so the installed search can make progress before returning.
        builder
            .use_current_thread()
            .build()
            .map(|pool| (pool, true))
    } else {
        // Desktop browsers that passed the SharedArrayBuffer/isolation gate can
        // retain parallel search without requiring a second game implementation.
        builder.build().map(|pool| (pool, false))
    }
}

#[cfg(target_arch = "wasm32")]
unsafe extern "C" {
    fn battlement_web_thread_count() -> i32;
}
