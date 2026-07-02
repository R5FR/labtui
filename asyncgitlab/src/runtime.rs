//! A process-wide tokio runtime so the synchronous `AsyncJob::run` (which
//! executes on labtui's threadpool) can drive the async reqwest client.
//!
//! labtui's job model is "blocking work on a worker thread"; reqwest is async.
//! We bridge the two by `block_on`-ing on a shared multi-thread runtime instead
//! of spinning up a runtime per request.

use std::sync::OnceLock;
use tokio::runtime::{Builder, Runtime};

static RUNTIME: OnceLock<Runtime> = OnceLock::new();

/// Access the shared runtime, building it on first use.
pub fn runtime() -> &'static Runtime {
	RUNTIME.get_or_init(|| {
		Builder::new_multi_thread()
			.worker_threads(2)
			.enable_all()
			.thread_name("labtui-gitlab")
			.build()
			.expect("failed to build gitlab tokio runtime")
	})
}

/// Run an async future to completion from synchronous code.
pub fn block_on<F: std::future::Future>(fut: F) -> F::Output {
	runtime().block_on(fut)
}
