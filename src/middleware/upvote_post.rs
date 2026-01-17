//! Upvote POST fallback middleware
//!
//! Handles upvote form submissions when JavaScript is disabled.
//! When a POST request contains upvotes_id and upvotes_direction,
//! processes the vote and redirects back to the same page (302).
//!
//! This matches the PHP behavior of Upvote::process_post(true).

use axum::{
    body::Body,
    extract::{ConnectInfo, State},
    http::{header, Method, Request},
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
};
use std::net::SocketAddr;
use tracing::debug;

use crate::app_state::AppState;
use crate::libs::util::client_ip;
use crate::registry::{resolve_path, PathLookupResult};

/// Middleware function that handles upvote POST fallback for non-JS users
pub async fn upvote_post_middleware(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    // Only intercept POST requests
    if request.method() != Method::POST {
        return next.run(request).await;
    }

    // Only process upvotes for known pages (not static files like CSS/JS)
    // This includes pages that don't have upvoting themselves but display
    // upvote forms for other pages (like homepage and all-pages).
    let path = request.uri().path();
    if !matches!(resolve_path(path), PathLookupResult::Canonical(_)) {
        return next.run(request).await;
    }

    // Check Content-Type - we only handle form submissions
    let content_type = request
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if !content_type.starts_with("application/x-www-form-urlencoded") {
        return next.run(request).await;
    }

    // Get client IP from connection info + headers
    let connection_ip = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .expect("BUG: ConnectInfo not available - is into_make_service_with_connect_info set up?")
        .0
        .ip();
    let client_ip = client_ip(connection_ip, request.headers());

    // Get the redirect URL (current page)
    let redirect_url = request.uri().to_string();

    // Collect the body to parse form data
    let (parts, body) = request.into_parts();
    let bytes = match axum::body::to_bytes(body, 1024 * 16).await {
        Ok(b) => b,
        Err(_) => {
            // Can't read body, just continue with empty body
            let request = Request::from_parts(parts, Body::empty());
            return next.run(request).await;
        }
    };

    // Parse form data
    let form_data: Vec<(String, String)> = form_urlencoded::parse(&bytes)
        .into_owned()
        .collect();

    // Look for upvote parameters
    let upvotes_id = form_data.iter().find(|(k, _)| k == "upvotes_id").map(|(_, v)| v.as_str());
    let upvotes_direction = form_data.iter().find(|(k, _)| k == "upvotes_direction").map(|(_, v)| v.as_str());

    match (upvotes_id, upvotes_direction) {
        (Some(id), Some(direction)) => {
            // This is an upvote POST - process it and redirect
            debug!("Processing upvote fallback: id={}, direction={}", id, direction);

            // Process the vote - panic on failure so user doesn't think vote succeeded
            state.upvotes.process_vote(id, &client_ip, direction).await
                .expect("Failed to process upvote");

            // 302 redirect back to the same page to prevent resubmission
            Redirect::to(&redirect_url).into_response()
        }
        _ => {
            // Not an upvote POST - reconstruct request and continue
            let request = Request::from_parts(parts, Body::from(bytes));
            next.run(request).await
        }
    }
}
