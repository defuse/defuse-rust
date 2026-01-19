//! Storage directory routes
//!
//! Serves files from the STORAGE_PATH directory:
//! - /files -> STORAGE_PATH/extras/files (force download)
//! - /files2 -> STORAGE_PATH/extras/files2 (viewable in browser)
//! - /mirrors -> STORAGE_PATH/extras/mirrors (force download)
//! - /upload -> STORAGE_PATH/extras/upload (force download)
//!
//! Download headers are applied by SecurityHeadersLayer based on path.

use axum::{routing::any, Router};
use std::path::Path;
use tower_http::services::ServeDir;

use crate::app_state::AppState;
use crate::dispatcher;

/// Build a router for storage directory routes.
///
/// The storage_path should point to the root storage directory
/// (containing extras/files, extras/files2, etc.)
///
/// Uses the dispatcher as a fallback for 404s so that missing files
/// show the proper site 404 page instead of a bare error.
pub fn storage_router(storage_path: &Path, state: AppState) -> Router<AppState> {
    let extras_path = storage_path.join("extras");

    // Create a fallback service that renders our 404 page
    let not_found = any(dispatcher::handle).with_state(state);

    Router::new()
        .nest_service(
            "/files",
            ServeDir::new(extras_path.join("files")).not_found_service(not_found.clone()),
        )
        .nest_service(
            "/files2",
            ServeDir::new(extras_path.join("files2")).not_found_service(not_found.clone()),
        )
        .nest_service(
            "/mirrors",
            ServeDir::new(extras_path.join("mirrors")).not_found_service(not_found.clone()),
        )
        .nest_service(
            "/upload",
            ServeDir::new(extras_path.join("upload")).not_found_service(not_found),
        )
}
