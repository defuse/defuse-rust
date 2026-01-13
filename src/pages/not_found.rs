use askama::Template;
use axum::{
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};

use crate::context::PageContext;
use crate::registry::NOT_FOUND_PAGE_INFO;
use crate::utils::extract_client_ip;

#[derive(Template)]
#[template(path = "pages/404.html")]
pub struct NotFoundPage {
    pub ctx: PageContext,
}

/// 404 handler - does NOT use PageContext extractor (which would fail for unknown pages)
pub async fn handler(headers: HeaderMap) -> Response {
    let client_ip = extract_client_ip(&headers);
    let dnt_enabled = headers
        .get(header::DNT)
        .and_then(|v| v.to_str().ok())
        .map(|v| v == "1")
        .unwrap_or(false);

    let ctx = PageContext::for_not_found(&NOT_FOUND_PAGE_INFO, client_ip, dnt_enabled);
    let page = NotFoundPage { ctx };
    (StatusCode::NOT_FOUND, page).into_response()
}
