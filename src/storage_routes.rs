//! Storage directory routes
//!
//! Serves files from the STORAGE_PATH directory:
//! - /files -> STORAGE_PATH/extras/files (force download)
//! - /files2 -> STORAGE_PATH/extras/files2 (viewable in browser)
//! - /mirrors -> STORAGE_PATH/extras/mirrors (force download)
//! - /upload -> STORAGE_PATH/extras/upload (force download)
//!
//! Download headers are applied by SecurityHeadersLayer based on path.

use axum::Router;
use std::path::Path;
use tower_http::services::ServeDir;

/// Build a router for storage directory routes.
///
/// The storage_path should point to the root storage directory
/// (containing extras/files, extras/files2, etc.)
///
/// Generic over state type S so it can be merged with any Router<S>.
pub fn storage_router<S>(storage_path: &Path) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    let extras_path = storage_path.join("extras");

    Router::new()
        .nest_service("/files", ServeDir::new(extras_path.join("files")))
        .nest_service("/files2", ServeDir::new(extras_path.join("files2")))
        .nest_service("/mirrors", ServeDir::new(extras_path.join("mirrors")))
        .nest_service("/upload", ServeDir::new(extras_path.join("upload")))
}
