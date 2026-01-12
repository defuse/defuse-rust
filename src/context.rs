//! Request context - extracted once per request, shared with all templates.
//!
//! This module provides PageContext which is automatically extracted from
//! each request and made available to all page handlers via Axum's extractor system.

use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{request::Parts, StatusCode, header},
};

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
    /// Page title from registry (or default)
    pub title: &'static str,
    /// Page meta description from registry (or default)
    pub description: &'static str,
    /// Client's IP address (from X-Forwarded-For, X-Real-IP, or connection)
    pub client_ip: String,
    /// Whether Do Not Track header is set
    pub dnt_enabled: bool,
    /// Page hit count (TODO: implement PHPCount)
    pub page_hits: u64,
    /// Unique visitor count (TODO: implement PHPCount)
    pub unique_hits: u64,
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
            // TODO: Implement PHPCount database integration
            page_hits: 0,
            unique_hits: 0,
        })
    }
}

/// Look up page info from a URL path
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

    // Fallback - in production this would come from the connection info
    "127.0.0.1".to_string()
}
