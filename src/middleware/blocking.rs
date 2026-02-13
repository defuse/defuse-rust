//! Blocking middleware - runs handlers on the blocking thread pool.
//!
//! This middleware wraps all request handlers so they run on Tokio's blocking
//! thread pool instead of the async runtime. This provides OS-level preemption
//! for CPU-bound work, preventing a single slow/CPU-intensive request from
//! blocking the entire async runtime.
//!
//! Without this middleware, N CPU-bound requests (where N = async worker threads)
//! could completely block all other request processing. With this middleware,
//! the OS scheduler can preempt any request, providing defense-in-depth against
//! accidental DoS from CPU-intensive handlers.

use axum::{extract::Request, middleware::Next, response::Response};
use tokio::runtime::Handle;

/// Middleware that runs the inner handler on the blocking thread pool.
///
/// This allows the OS to preempt CPU-bound work, preventing async runtime starvation.
pub async fn blocking_middleware(request: Request, next: Next) -> Response {
    let handle = Handle::current();
    tokio::task::spawn_blocking(move || handle.block_on(next.run(request)))
        .await
        .expect("blocking execution of a handler panicked")
}
