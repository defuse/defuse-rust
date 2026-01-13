//! Request context - extracted once per request, shared with all templates.
//!
//! This module provides PageContext which is automatically extracted from
//! each request and made available to all page handlers via Axum's extractor system.

use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{header, request::Parts, StatusCode},
};

use crate::middleware::HitCounts;
use crate::pages::registry::{lookup_page_from_path, PageInfo, DEFAULT_META_DESCRIPTION, DEFAULT_TITLE};
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
    /// Page info from registry (None if page not found)
    /// This is the single source of truth for title, description, etc.
    page_info: Option<&'static PageInfo>,
    /// Client's IP address (from X-Forwarded-For, X-Real-IP, or connection)
    pub client_ip: String,
    /// Whether Do Not Track header is set
    pub dnt_enabled: bool,
    /// Page hit count (from PHPCount middleware)
    pub page_hits: u32,
    /// Unique visitor count (from PHPCount middleware)
    pub unique_hits: u32,
}

impl PageContext {
    /// Whether this is the home page
    pub fn is_home(&self) -> bool {
        self.page_info.map(|p| p.slug.is_empty()).unwrap_or(false)
    }

    /// Page title (from registry or default)
    pub fn title(&self) -> &'static str {
        self.page_info
            .map(|p| p.title_or_default())
            .unwrap_or(DEFAULT_TITLE)
    }

    /// Page meta description (from registry or default)
    pub fn description(&self) -> &'static str {
        self.page_info
            .map(|p| p.description_or_default())
            .unwrap_or(DEFAULT_META_DESCRIPTION)
    }
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

        // Look up page info from registry (single source of truth)
        let page_info = lookup_page_from_path(path);

        // Get hit counts from middleware (if available)
        let hit_counts = parts
            .extensions
            .get::<HitCounts>()
            .cloned()
            .unwrap_or_default();

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
        })
    }
}
