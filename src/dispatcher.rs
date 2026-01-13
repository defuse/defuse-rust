//! Central dispatcher for all page requests.
//!
//! This module handles routing by looking up pages in the registry and
//! calling the appropriate handler method based on the HTTP method.

use axum::{
    body::Body,
    extract::State,
    http::{header, Method, Request, StatusCode},
    response::{IntoResponse, Redirect, Response},
};
use bytes::Bytes;

use crate::context::PageContext;
use crate::middleware::client_ip::ClientIp;
use crate::middleware::hit_counter::{HitCounts, VoteState};
use crate::pages::not_found::NotFoundPage;
use crate::registry::{canonical_url, lookup_page_from_path, PageInfo, NOT_FOUND_PAGE_INFO};
use crate::state::AppState;

/// Main dispatcher - handles all page requests via the registry.
///
/// This is the fallback handler that processes any request not matched
/// by explicit routes (like /upvote or static files).
pub async fn handle(State(state): State<AppState>, request: Request<Body>) -> Response {
    let method = request.method().clone();
    let path = request.uri().path().to_string();

    // Look up page in registry
    let Some(page_info) = lookup_page_from_path(&path) else {
        // Page not in registry - return 404
        return render_not_found(&request);
    };

    // Handle aliases/redirects
    if let Some(target) = page_info.redirect {
        let canonical = canonical_url(target);
        return Redirect::permanent(&canonical).into_response();
    }

    // Check if page has a handler implemented
    let Some(handler) = page_info.handler else {
        // Page is in registry but not yet implemented - return 404
        // (stub_page! entries have handler: None)
        return render_not_found(&request);
    };

    // Extract PageContext from request (before consuming body)
    let ctx = extract_page_context(&request, page_info);

    // Extract body for POST requests
    let body = if method == Method::POST {
        // Consume request body
        let (_parts, body) = request.into_parts();
        match axum::body::to_bytes(body, 100 * 1024 * 1024).await {
            // 100MB limit (matches PHP's post_max_size)
            Ok(bytes) => bytes,
            Err(_) => {
                return (StatusCode::BAD_REQUEST, "Failed to read request body").into_response();
            }
        }
    } else {
        // For non-POST, we don't need the body
        Bytes::new()
    };

    // Dispatch based on HTTP method
    match method {
        Method::GET | Method::HEAD => handler.get(ctx, &state).await,
        Method::POST => match handler.post(ctx, &state, body) {
            Some(future) => future.await,
            None => {
                // Handler doesn't support POST - return 405
                (StatusCode::METHOD_NOT_ALLOWED, "Method Not Allowed").into_response()
            }
        },
        _ => {
            // Unsupported method
            (StatusCode::METHOD_NOT_ALLOWED, "Method Not Allowed").into_response()
        }
    }
}

/// Extract PageContext from the request.
fn extract_page_context(request: &Request<Body>, page_info: &'static PageInfo) -> PageContext {
    // Get client IP from extensions (set by client_ip_middleware)
    let client_ip = request
        .extensions()
        .get::<ClientIp>()
        .map(|ip| ip.0.clone())
        .unwrap_or_else(|| "unknown".to_string());

    // Get DNT header
    let dnt_enabled = request
        .headers()
        .get(header::DNT)
        .and_then(|v| v.to_str().ok())
        .map(|v| v == "1")
        .unwrap_or(false);

    // Get hit counts from extensions (set by hit_counter_middleware)
    let hit_counts = request
        .extensions()
        .get::<HitCounts>()
        .cloned()
        .unwrap_or_default();

    // Get vote state from extensions (set by hit_counter_middleware)
    let vote_state = request
        .extensions()
        .get::<VoteState>()
        .cloned()
        .unwrap_or_default();

    PageContext {
        page_info,
        client_ip,
        dnt_enabled,
        hit_counts,
        vote_state,
    }
}

/// Render the 404 not found page.
fn render_not_found(request: &Request<Body>) -> Response {
    let ctx = extract_page_context(request, &NOT_FOUND_PAGE_INFO);
    (StatusCode::NOT_FOUND, NotFoundPage { ctx }).into_response()
}
