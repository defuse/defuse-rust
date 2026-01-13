//! Request context - extracted once per request, shared with all templates.
//!
//! This module provides PageContext which is automatically extracted from
//! each request and made available to all page handlers via Axum's extractor system.

use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{request::Parts, StatusCode, header},
};

use crate::middleware::HitCounts;
use crate::pages::registry::{lookup_page, PageInfo, DEFAULT_TITLE, DEFAULT_META_DESCRIPTION};

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
    /// Whether this is the home page (path == "/")
    /// Used by base template to hide footer on home page
    pub is_home: bool,
    // TODO: Why do we have copies of title and description here, shouldn't we just get a copy of the PageInfo to avoid duplication?
    /// Page title from registry (or default)
    pub title: &'static str,
    /// Page meta description from registry (or default)
    pub description: &'static str,
    /// Client's IP address (from X-Forwarded-For, X-Real-IP, or connection)
    pub client_ip: String,
    /// Whether Do Not Track header is set
    pub dnt_enabled: bool,
    /// Page hit count (from PHPCount middleware)
    pub page_hits: u32,
    /// Unique visitor count (from PHPCount middleware)
    pub unique_hits: u32,
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

        // Look up page info from registry based on path
        let page_info = lookup_page_from_path(path);

        // Get hit counts from middleware (if available)
        let hit_counts = parts
            .extensions
            .get::<HitCounts>()
            .cloned()
            .unwrap_or_default();

        Ok(Self {
            is_home: path == "/",
            title: page_info.map(|p| p.title_or_default()).unwrap_or(DEFAULT_TITLE),
            description: page_info.map(|p| p.description_or_default()).unwrap_or(DEFAULT_META_DESCRIPTION),
            client_ip: extract_client_ip(headers),
            dnt_enabled: headers
                .get(header::DNT)
                .and_then(|v| v.to_str().ok())
                .map(|v| v == "1")
                .unwrap_or(false),
            page_hits: hit_counts.page_hits,
            unique_hits: hit_counts.unique_hits,
        })
    }
}

/// Look up page info from a URL path
/// TODO: it seems like this is duplicate code with the lookup-from-path in pages/registry.rs? Can we DRY this up?
fn lookup_page_from_path(path: &str) -> Option<&'static PageInfo> {
    if path == "/" {
        return lookup_page("");
    }

    // Strip leading slash and .htm extension
    let name = path
        .trim_start_matches('/')
        .trim_end_matches(".htm")
        .trim_end_matches(".html");

    lookup_page(name)
}

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
    
    // TODO: what if there are no IP headers? we need to pull it from the actual TCP connection, no?

    // Fallback - in production this would come from the connection info
    "127.0.0.1".to_string()
}
