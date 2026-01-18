//! Central dispatcher for all registered page requests.
//!
//! This module handles routing by looking up pages in the registry and
//! calling the appropriate handler method based on the HTTP method.
//!
//! Hit counting and vote state are fetched here (not in middleware) because
//! they only apply to formally-defined pages. This matches the PHP version's
//! approach and keeps all page-handling logic in one place.

use axum::{
    body::Body,
    extract::{ConnectInfo, State},
    http::{header, Method, Request, StatusCode},
    response::{IntoResponse, Response},
};
use bytes::Bytes;
use multer::Multipart;
use std::net::SocketAddr;
use tracing::{debug, error};

use crate::app_state::AppState;
use crate::context::PageContext;
use crate::handler::{FormField, PostBody};
use crate::libs::{phpcount::HitCounts, upvotes::VoteState, util::client_ip};
use crate::pages::not_found::NotFoundPage;
use crate::registry::{resolve_path, PageInfo, PathLookupResult, NOT_FOUND_PAGE_INFO};

/// Processes any request not matched by explicit routes (like /upvote or static
/// files). If the request is not for a registered page or what we expect to be
/// a 404, that's a bug in main.rs.
pub async fn handle(State(state): State<AppState>, request: Request<Body>) -> Response {
    let method = request.method().clone();
    let path = request.uri().path().to_string();

    // Extract all data from request BEFORE any async operations
    // (Request<Body> is not Sync, so we can't hold reference across await)
    let connection_ip = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .expect("BUG: ConnectInfo not available - is into_make_service_with_connect_info set up?")
        .0
        .ip();
    let client_ip = client_ip(connection_ip, request.headers());

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

    let captcha_bypass_header = request
        .headers()
        .get("X-Captcha-Bypass")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    // Resolve path to page (middleware should have already handled all redirects)
    let page_info = match resolve_path(&path) {
        PathLookupResult::Canonical(page) => page,
        PathLookupResult::NotFound => {
            return render_not_found(client_ip, dnt_enabled);
        }
        PathLookupResult::Redirect { canonical_path } => {
            // Middleware should have already redirected - this is a bug
            panic!(
                "BUG: Redirect reached dispatcher - middleware failed to redirect {} -> {}",
                path, canonical_path
            );
        }
    };

    // All non-redirect registry entries MUST have a handler, if not, fail loud.
    let handler = page_info.handler.unwrap();

    // Extract body for POST requests (consumes request)
    let post_body = if method == Method::POST {
        // Get Content-Type header before consuming request
        let content_type = request
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let (_parts, body) = request.into_parts();
        // 100MB limit (matches PHP's post_max_size)
        let bytes = match axum::body::to_bytes(body, 100 * 1024 * 1024).await {
            Ok(bytes) => bytes,
            Err(_) => {
                return (StatusCode::BAD_REQUEST, "Failed to read request body").into_response();
            }
        };

        // Check if multipart and parse accordingly
        if let Some(boundary) = content_type.as_ref().and_then(|ct| multer::parse_boundary(ct).ok())
        {
            match parse_multipart(bytes, &boundary).await {
                Ok(fields) => PostBody::Multipart { fields },
                Err(e) => {
                    error!("Failed to parse multipart: {}", e);
                    return (StatusCode::BAD_REQUEST, "Failed to parse multipart data")
                        .into_response();
                }
            }
        } else {
            PostBody::UrlEncoded(bytes)
        }
    } else {
        PostBody::UrlEncoded(Bytes::new())
    };

    // Now do async operations

    let page_id = page_info.hit_counter_id();
    let hit_counts = record_and_get_hits(&state, page_id, &client_ip, &user_agent).await;

    let vote_state = if let Some(upvote_config) = &page_info.upvote {
        fetch_vote_state(&state, page_info, upvote_config, &client_ip).await
    } else {
        VoteState::default()
    };

    debug!("{} {} (ip: {})", method, path, client_ip);

    let ctx = PageContext {
        page_info,
        client_ip,
        dnt_enabled,
        hit_counts,
        vote_state,
        captcha_bypass_header,
    };

    // Dispatch based on HTTP method
    match method {
        // Axum takes care of not returning the body for HEAD requests
        Method::GET | Method::HEAD => handler.get(ctx, &state).await,
        Method::POST => match handler.post(ctx, &state, post_body) {
            Some(future) => future.await,
            None => {
                // Handler doesn't support POST - return 405
                (StatusCode::METHOD_NOT_ALLOWED, "Method Not Allowed").into_response()
            }
        },
        _ => {
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
        error!("Failed to record hit for {}: {}", page_id, e);
    }

    // Fetch counts
    state.phpcount.get_hit_counts(page_id).await
        .unwrap_or_else(|e| {
            error!("Failed to get hit counts for {}: {}", page_id, e);
            HitCounts::default()
        })
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

    // In the PHP code, each of these were hard-coded, but we can automatically generate them.
    let page_url = if page_info.slug.is_empty() {
        "https://defuse.ca/".to_string()
    } else if page_info.is_directory() {
        format!("https://defuse.ca/{}/", page_info.slug.trim_end_matches('/'))
    } else {
        format!("https://defuse.ca/{}.htm", page_info.slug)
    };

    // Ensure page exists in database (this is how entries for new pages are added)
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
        error!(
            "Failed to ensure page {} in upvotes database: {}",
            upvote_config.id, e
        );
    }

    // Fetch vote counts and user's vote
    state
        .upvotes
        .get_vote_state(upvote_config.id, client_ip)
        .await
        .unwrap_or_else(|e| {
            error!(
                "Failed to get vote counts for {}: {}",
                upvote_config.id, e
            );
            VoteState::default()
        })
}

/// Render the 404 not found page.
fn render_not_found(client_ip: String, dnt_enabled: bool) -> Response {
    let ctx = PageContext {
        page_info: &NOT_FOUND_PAGE_INFO,
        client_ip,
        dnt_enabled,
        hit_counts: HitCounts::default(),
        vote_state: VoteState::default(),
        captcha_bypass_header: None,
    };

    (StatusCode::NOT_FOUND, NotFoundPage { ctx }).into_response()
}

/// Parse multipart form data into fields.
async fn parse_multipart(body: Bytes, boundary: &str) -> Result<Vec<FormField>, multer::Error> {
    let stream = futures_util::stream::once(async move { Ok::<_, std::io::Error>(body) });
    let mut multipart = Multipart::new(stream, boundary);

    let mut fields = Vec::new();

    while let Some(field) = multipart.next_field().await? {
        let name = field.name().unwrap_or("").to_string();
        let filename = field.file_name().map(|s| s.to_string());
        let data = Bytes::from(field.bytes().await?);

        fields.push(FormField {
            name,
            filename,
            data,
        });
    }

    Ok(fields)
}
