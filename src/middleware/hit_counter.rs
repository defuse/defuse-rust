//! Hit counter middleware - records page hits and fetches vote counts
//!
//! This middleware:
//! 1. Records a hit for each HTML page request
//! 2. Stores hit counts in request extensions for templates to display
//! 3. Fetches vote counts for pages with upvoting enabled
//! 4. Skips static files (CSS, JS, images)

use axum::{
    body::Body,
    extract::State,
    http::{header, Request},
    middleware::Next,
    response::Response,
};
use tracing::{debug, warn};

use crate::registry::{canonical_url, lookup_page_from_path};
use crate::state::AppState;
use super::ClientIp;

/// Hit counts stored in request extensions for templates to read
#[derive(Clone, Debug, Default)]
pub struct HitCounts {
    pub page_hits: u32,
    pub unique_hits: u32,
    pub total_hits: u32,
    pub total_unique_hits: u32,
}

/// Vote state stored in request extensions for templates to read.
/// Contains both aggregate counts and the current user's vote.
#[derive(Clone, Debug, Default)]
pub struct VoteState {
    pub upvotes: i32,
    pub downvotes: i32,
    pub user_vote: Option<String>, // "upvote", "downvote", or None
}

impl VoteState {
    /// Net vote total (upvotes - downvotes)
    pub fn total(&self) -> i32 {
        self.upvotes - self.downvotes
    }

    /// Whether the current user has upvoted
    pub fn user_upvoted(&self) -> bool {
        self.user_vote.as_deref() == Some("upvote")
    }

    /// Whether the current user has downvoted
    pub fn user_downvoted(&self) -> bool {
        self.user_vote.as_deref() == Some("downvote")
    }
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
    // Skip if page not found (404s shouldn't be counted)
    let page_info = match lookup_page_from_path(&path) {
        Some(info) => info,
        None => return next.run(request).await,
    };

    let page_id = page_info.hit_counter_id().to_string();

    // Get client IP from extensions (always present - set by client_ip_middleware)
    let client_ip = request
        .extensions()
        .get::<ClientIp>()
        .expect("BUG: ClientIp not in extensions - client_ip_middleware not running?")
        .0
        .clone();

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

    // Fetch vote counts if page has upvoting enabled
    // Also ensure the page exists in the database (like PHP's render_arrows -> add_counter)
    if let Some(upvote_config) = &page_info.upvote {
        // Get title/description from upvote config override or page defaults
        let title = upvote_config.title.unwrap_or_else(|| page_info.title_or_default());
        let description = upvote_config.description.unwrap_or_else(|| page_info.description_or_default());
        let page_url = canonical_url(page_info.slug);

        // Ensure page exists in database (creates or updates metadata)
        if let Err(e) = state.upvotes.ensure_page(
            upvote_config.id,
            upvote_config.category,
            title,
            description,
            &page_url,
        ).await {
            warn!("Failed to ensure page {} in upvotes database: {}", upvote_config.id, e);
        }

        let vote_counts = match get_vote_counts(&state, upvote_config.id, &client_ip).await {
            Ok(counts) => counts,
            Err(e) => {
                warn!("Failed to get vote counts for {}: {}", upvote_config.id, e);
                VoteState::default()
            }
        };
        request.extensions_mut().insert(vote_counts);
    }

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

/// Get vote counts from database
async fn get_vote_counts(state: &AppState, upvote_id: &str, client_ip: &str) -> Result<VoteState, sqlx::Error> {
    let result = state.upvotes.get_vote_result(upvote_id, client_ip).await?;

    Ok(VoteState {
        upvotes: result.upvotes,
        downvotes: result.downvotes,
        user_vote: result.user_action.map(|a| match a {
            crate::db::upvotes::VoteAction::Upvote => "upvote".to_string(),
            crate::db::upvotes::VoteAction::Downvote => "downvote".to_string(),
        }),
    })
}
