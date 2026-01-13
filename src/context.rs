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

use crate::middleware::{ClientIp, HitCounts, VoteState};
use crate::registry::{lookup_page_from_path, PageInfo};

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
    /// Page info from registry (always present for known pages; 404 handler uses NOT_FOUND_PAGE_INFO)
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
    /// Vote state (counts + user's vote). Always present, defaults to zeros.
    pub vote_state: VoteState,
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
            vote_state: VoteState::default(),
        }
    }

    /// Whether this is the home page
    pub fn is_home(&self) -> bool {
        self.page_info.slug.is_empty()
    }

    /// Get the canonical URL for this page
    pub fn canonical_url(&self) -> String {
        let p = self.page_info;
        if p.slug.is_empty() {
            "https://defuse.ca/".to_string()
        } else if p.is_directory() {
            format!("https://defuse.ca/{}/", p.slug.trim_end_matches('/'))
        } else {
            format!("https://defuse.ca/{}.htm", p.slug)
        }
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

        // Get vote state from middleware (defaults to zeros if not set)
        let vote_state = parts
            .extensions
            .get::<VoteState>()
            .cloned()
            .unwrap_or_default();

        // Get client IP from middleware (always present - set by client_ip_middleware)
        let client_ip = parts
            .extensions
            .get::<ClientIp>()
            .expect("BUG: ClientIp not in extensions - client_ip_middleware not running?")
            .0
            .clone();

        Ok(Self {
            page_info,
            client_ip,
            dnt_enabled: headers
                .get(header::DNT)
                .and_then(|v| v.to_str().ok())
                .map(|v| v == "1")
                .unwrap_or(false),
            page_hits: hit_counts.page_hits,
            unique_hits: hit_counts.unique_hits,
            vote_state,
        })
    }
}
