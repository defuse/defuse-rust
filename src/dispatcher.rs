//! Central dispatcher for all page requests.
//!
//! This module handles routing by looking up pages in the registry and
//! calling the appropriate handler method based on the HTTP method.
//!
//! Hit counting and vote state are fetched here (not in middleware) because
//! they only apply to formally-defined pages. This matches PHP's approach
//! and keeps all page-handling logic in one place.

use axum::{
    body::Body,
    extract::State,
    http::{header, Method, Request, StatusCode},
    response::{IntoResponse, Redirect, Response},
};
use bytes::Bytes;
use tracing::{debug, warn};

use crate::context::PageContext;
use crate::middleware::client_ip::ClientIp;
use crate::middleware::{HitCounts, VoteState};
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

    // Extract all data from request BEFORE any async operations
    // (Request<Body> is not Sync, so can't hold reference across await)
    let client_ip = request
        .extensions()
        .get::<ClientIp>()
        .expect("BUG: ClientIp not in extensions - client_ip_middleware must run first")
        .0
        .clone();

    let dnt_enabled = request
        .headers()
        .get(header::DNT)
        .and_then(|v| v.to_str().ok())
        .map(|v| v == "1")
        .unwrap_or(false);

    let user_agent = request
        .headers()
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    // Extract body for POST requests (consumes request)
    let body = if method == Method::POST {
        let (_parts, body) = request.into_parts();
        match axum::body::to_bytes(body, 100 * 1024 * 1024).await {
            // 100MB limit (matches PHP's post_max_size)
            Ok(bytes) => bytes,
            Err(_) => {
                return (StatusCode::BAD_REQUEST, "Failed to read request body").into_response();
            }
        }
    } else {
        Bytes::new()
    };

    // Now do async operations (hit counting, vote fetching)
    let page_id = page_info.hit_counter_id();
    let hit_counts = record_and_get_hits(&state, page_id, &client_ip, &user_agent).await;

    let vote_state = if let Some(upvote_config) = &page_info.upvote {
        fetch_vote_state(&state, page_info, upvote_config, &client_ip).await
    } else {
        VoteState::default()
    };

    debug!(
        "Page {} - hits: {}, votes: {}",
        page_id,
        hit_counts.page_hits,
        vote_state.total()
    );

    let ctx = PageContext {
        page_info,
        client_ip,
        dnt_enabled,
        hit_counts,
        vote_state,
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

/// Record a hit and get hit counts from the database.
async fn record_and_get_hits(
    state: &AppState,
    page_id: &str,
    client_ip: &str,
    user_agent: &str,
) -> HitCounts {
    // Record the hit (errors logged but don't block page render)
    if let Err(e) = state.phpcount.add_hit(page_id, client_ip, user_agent).await {
        warn!("Failed to record hit for {}: {}", page_id, e);
    }

    // Fetch counts
    let page_hits = state.phpcount.get_hits(page_id, false).await.unwrap_or(0);
    let unique_hits = state.phpcount.get_hits(page_id, true).await.unwrap_or(0);
    let total_hits = state.phpcount.get_total_hits(false).await.unwrap_or(0);
    let total_unique_hits = state.phpcount.get_total_hits(true).await.unwrap_or(0);

    HitCounts {
        page_hits,
        unique_hits,
        total_hits,
        total_unique_hits,
    }
}

/// Fetch vote state for a page with upvoting enabled.
async fn fetch_vote_state(
    state: &AppState,
    page_info: &'static PageInfo,
    upvote_config: &crate::registry::UpvoteConfig,
    client_ip: &str,
) -> VoteState {
    // Get title/description from upvote config override or page defaults
    let title = upvote_config
        .title
        .unwrap_or_else(|| page_info.title_or_default());
    let description = upvote_config
        .description
        .unwrap_or_else(|| page_info.description_or_default());
    let page_url = canonical_url(page_info.slug);

    // Ensure page exists in database (creates or updates metadata)
    if let Err(e) = state
        .upvotes
        .ensure_page(
            upvote_config.id,
            upvote_config.category,
            title,
            description,
            &page_url,
        )
        .await
    {
        warn!(
            "Failed to ensure page {} in upvotes database: {}",
            upvote_config.id, e
        );
    }

    // Fetch vote counts and user's vote
    match state
        .upvotes
        .get_vote_result(upvote_config.id, client_ip)
        .await
    {
        Ok(result) => VoteState {
            upvotes: result.upvotes,
            downvotes: result.downvotes,
            user_vote: result.user_action,
        },
        Err(e) => {
            warn!(
                "Failed to get vote counts for {}: {}",
                upvote_config.id, e
            );
            VoteState::default()
        }
    }
}

/// Render the 404 not found page.
fn render_not_found(request: &Request<Body>) -> Response {
    // For 404 pages, we don't record hits or fetch votes
    let client_ip = request
        .extensions()
        .get::<ClientIp>()
        .expect("BUG: ClientIp not in extensions - client_ip_middleware must run first")
        .0
        .clone();

    let dnt_enabled = request
        .headers()
        .get(header::DNT)
        .and_then(|v| v.to_str().ok())
        .map(|v| v == "1")
        .unwrap_or(false);

    let ctx = PageContext {
        page_info: &NOT_FOUND_PAGE_INFO,
        client_ip,
        dnt_enabled,
        hit_counts: HitCounts::default(),
        vote_state: VoteState::default(),
    };

    (StatusCode::NOT_FOUND, NotFoundPage { ctx }).into_response()
}
