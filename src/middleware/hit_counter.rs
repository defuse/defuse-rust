//! Hit counter middleware - records page hits using PHPCount
//!
//! This middleware:
//! 1. Records a hit for each HTML page request
//! 2. Stores hit counts in request extensions for templates to display
//! 3. Skips static files (CSS, JS, images)

use axum::{
    body::Body,
    extract::State,
    http::{header, Request},
    middleware::Next,
    response::Response,
};
use tracing::{debug, warn};

use crate::pages::registry::lookup_page_from_path;
use crate::state::AppState;

/// Hit counts stored in request extensions for templates to read
#[derive(Clone, Debug, Default)]
pub struct HitCounts {
    pub page_hits: u32,
    pub unique_hits: u32,
    pub total_hits: u32,
    pub total_unique_hits: u32,
}

/// Middleware function that records hits and stores counts
pub async fn hit_counter_middleware(
    State(state): State<AppState>,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    let path = request.uri().path().to_string();

    // Skip static files - only count HTML pages
    if should_skip_path(&path) {
        return next.run(request).await;
    }

    // Look up the page in the registry to get the correct page ID
    let page_info = lookup_page_from_path(&path);

    // Skip if page not found in registry (404s, unknown paths)
    let page_id = match page_info {
        Some(info) => info.hit_counter_id().to_string(),
        None => {
            // For unknown pages, use a fallback ID
            path_to_page_id(&path)
        }
    };

    // Extract info needed for hit counting
    let client_ip = extract_client_ip(request.headers());
    let user_agent = request
        .headers()
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    // Record the hit
    if let Err(e) = state
        .phpcount
        .add_hit(&page_id, &client_ip, user_agent)
        .await
    {
        warn!("Failed to record hit for {}: {}", page_id, e);
    }

    // Get hit counts for display
    let hit_counts = match get_hit_counts(&state, &page_id).await {
        Ok(counts) => counts,
        Err(e) => {
            warn!("Failed to get hit counts for {}: {}", page_id, e);
            HitCounts::default()
        }
    };

    debug!(
        "Hit recorded for {} - page: {}, total: {}",
        page_id, hit_counts.page_hits, hit_counts.total_hits
    );

    // Store in request extensions for PageContext to read
    request.extensions_mut().insert(hit_counts);

    next.run(request).await
}

/// Check if path should skip hit counting (static files, etc.)
fn should_skip_path(path: &str) -> bool {
    // Skip static file extensions
    let static_extensions = [".css", ".js", ".png", ".gif", ".jpg", ".jpeg", ".ico", ".svg", ".woff", ".woff2", ".ttf"];
    if static_extensions.iter().any(|ext| path.ends_with(ext)) {
        return true;
    }

    // Skip known static directories
    let static_dirs = ["/images/", "/js/", "/css/", "/fonts/"];
    if static_dirs.iter().any(|dir| path.starts_with(dir)) {
        return true;
    }

    false
}

/// Convert URL path to page ID (matching PHP behavior)
fn path_to_page_id(path: &str) -> String {
    // PHP uses the page name without extension
    // e.g., "/checksums.htm" -> "checksums"
    //       "/" -> "home"
    if path == "/" {
        return "home".to_string();
    }

    path.trim_start_matches('/')
        .trim_end_matches(".htm")
        .trim_end_matches(".html")
        .to_string()
}

/// Extract client IP from headers (X-Forwarded-For, X-Real-IP, or fallback)
fn extract_client_ip(headers: &axum::http::HeaderMap) -> String {
    // Check X-Forwarded-For first (for reverse proxy setups)
    if let Some(forwarded) = headers.get("x-forwarded-for") {
        if let Ok(s) = forwarded.to_str() {
            // Take the first IP if there are multiple
            return s.split(',').next().unwrap_or(s).trim().to_string();
        }
    }

    // Check X-Real-IP
    if let Some(real_ip) = headers.get("x-real-ip") {
        if let Ok(s) = real_ip.to_str() {
            return s.to_string();
        }
    }

    // Fallback - in production the reverse proxy should always set headers
    "127.0.0.1".to_string()
}

/// Get hit counts from database
async fn get_hit_counts(state: &AppState, page_id: &str) -> Result<HitCounts, sqlx::Error> {
    let page_hits = state.phpcount.get_hits(page_id, false).await?;
    let unique_hits = state.phpcount.get_hits(page_id, true).await?;
    let total_hits = state.phpcount.get_total_hits(false).await?;
    let total_unique_hits = state.phpcount.get_total_hits(true).await?;

    Ok(HitCounts {
        page_hits,
        unique_hits,
        total_hits,
        total_unique_hits,
    })
}
