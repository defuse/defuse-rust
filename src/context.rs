//! Request context - extracted once per request, shared with all templates.
//!
//! This module provides PageContext which is automatically extracted from
//! each request and made available to all page handlers via Axum's extractor system.

use std::path::Path;

use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{header, request::Parts, StatusCode},
};

use crate::vim_highlight;

use crate::middleware::{HitCounts, VoteCounts};
use crate::pages::registry::{lookup_page_from_path, PageInfo, UpvoteConfig};
use crate::utils::extract_client_ip;

/// Common context data available to all page templates.
///
/// This is automatically extracted from each request - handlers just declare
/// it as a parameter and Axum provides it automatically:
///
/// ```rust
/// pub async fn get(ctx: PageContext) -> impl IntoResponse {
///     // ctx is automatically populated from the request
/// }
/// ```
#[derive(Debug, Clone)]
pub struct PageContext {
    /// Page info from registry (always present - uses DEFAULT_PAGE_INFO for unknown pages)
    /// Templates access this directly: ctx.page_info.title_or_default()
    pub page_info: &'static PageInfo,
    /// Client's IP address (from X-Forwarded-For, X-Real-IP, or connection)
    pub client_ip: String,
    /// Whether Do Not Track header is set
    pub dnt_enabled: bool,
    /// Page hit count (from PHPCount middleware)
    pub page_hits: u32,
    /// Unique visitor count (from PHPCount middleware)
    pub unique_hits: u32,
    /// Vote counts (from middleware, only if page has upvoting)
    vote_counts: Option<VoteCounts>,
}

impl PageContext {
    /// Create a PageContext for the 404 page (bypasses registry lookup)
    pub fn for_not_found(page_info: &'static PageInfo, client_ip: String, dnt_enabled: bool) -> Self {
        Self {
            page_info,
            client_ip,
            dnt_enabled,
            page_hits: 0,
            unique_hits: 0,
            vote_counts: None,
        }
    }

    /// Whether this is the home page
    pub fn is_home(&self) -> bool {
        self.page_info.slug.is_empty()
    }

    /// Get upvote config if this page has voting enabled
    pub fn upvote(&self) -> Option<&'static UpvoteConfig> {
        self.page_info.upvote.as_ref()
    }

    /// Check if this page has voting enabled
    pub fn has_upvote(&self) -> bool {
        self.upvote().is_some()
    }

    /// Get the upvote ID (for templates)
    pub fn upvote_id(&self) -> &'static str {
        self.upvote().map(|u| u.id).unwrap_or("")
    }

    /// Get the canonical URL for this page
    pub fn canonical_url(&self) -> String {
        let p = self.page_info;
        if p.slug.is_empty() {
            "https://defuse.ca/".to_string()
        } else if p.is_directory {
            format!("https://defuse.ca/{}/", p.slug.trim_end_matches('/'))
        } else {
            format!("https://defuse.ca/{}.htm", p.slug)
        }
    }

    /// Get vote total (upvotes - downvotes)
    pub fn vote_total(&self) -> i32 {
        self.vote_counts.as_ref().map(|v| v.total()).unwrap_or(0)
    }

    /// Check if user has upvoted this page
    pub fn user_upvoted(&self) -> bool {
        self.vote_counts
            .as_ref()
            .and_then(|v| v.user_vote.as_ref())
            .map(|v| v == "upvote")
            .unwrap_or(false)
    }

    /// Check if user has downvoted this page
    pub fn user_downvoted(&self) -> bool {
        self.vote_counts
            .as_ref()
            .and_then(|v| v.user_vote.as_ref())
            .map(|v| v == "downvote")
            .unwrap_or(false)
    }

    // ===== Syntax Highlighting (matches PHP's printHlString/printSourceFile) =====

    /// Syntax highlight a string. Matches PHP's printHlString($text, $ft, $numbers).
    /// Returns HTML wrapped in <div class="vimhighlight">
    pub fn hl_string(&self, text: &str, file_type: &str, show_lines: bool) -> String {
        vim_highlight::highlight_string(text, file_type, show_lines)
            .unwrap_or_else(|e| format!("<pre>Error highlighting: {}</pre>", html_escape(&e.to_string())))
    }

    /// Syntax highlight a source file. Matches PHP's printSourceFile($path, $numbers).
    /// Returns HTML wrapped in <div class="vimhighlight">
    pub fn hl_file(&self, path: &str, show_lines: bool) -> String {
        vim_highlight::highlight_file(Path::new(path), show_lines)
            .unwrap_or_else(|e| format!("<pre>Error highlighting file: {}</pre>", html_escape(&e.to_string())))
    }
}

/// Escape HTML special characters to prevent XSS
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

/// Axum extractor - automatically creates PageContext from request
#[async_trait]
impl<S> FromRequestParts<S> for PageContext
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let headers = &parts.headers;
        let path = parts.uri.path();

        // Look up page info from registry - FAILS if page not found
        // This ensures every page handler has a corresponding registry entry
        let page_info = lookup_page_from_path(path)
            .ok_or((StatusCode::INTERNAL_SERVER_ERROR, "Page not in registry"))?;

        // Get hit counts from middleware (if available)
        let hit_counts = parts
            .extensions
            .get::<HitCounts>()
            .cloned()
            .unwrap_or_default();

        // Get vote counts from middleware (if page has upvoting)
        let vote_counts = parts.extensions.get::<VoteCounts>().cloned();

        Ok(Self {
            page_info,
            client_ip: extract_client_ip(headers),
            dnt_enabled: headers
                .get(header::DNT)
                .and_then(|v| v.to_str().ok())
                .map(|v| v == "1")
                .unwrap_or(false),
            page_hits: hit_counts.page_hits,
            unique_hits: hit_counts.unique_hits,
            vote_counts,
        })
    }
}
