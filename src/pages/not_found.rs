use askama::Template;
use axum::{
    body::Body,
    http::{header, Request, StatusCode},
    response::{IntoResponse, Response},
};

use crate::context::PageContext;
use crate::middleware::ClientIp;
use crate::registry::NOT_FOUND_PAGE_INFO;

#[derive(Template)]
#[template(path = "pages/404.html")]
pub struct NotFoundPage {
    pub ctx: PageContext,
}

/// 404 handler - does NOT use PageContext extractor (which would fail for unknown pages)
pub async fn handler(request: Request<Body>) -> Response {
    // Get client IP from extensions (always present - set by client_ip_middleware)
    let client_ip = request
        .extensions()
        .get::<ClientIp>()
        .expect("BUG: ClientIp not in extensions - client_ip_middleware not running?")
        .0
        .clone();

    let dnt_enabled = request
        .headers()
        .get(header::DNT)
        .and_then(|v| v.to_str().ok())
        .map(|v| v == "1")
        .unwrap_or(false);

    let ctx = PageContext::for_not_found(&NOT_FOUND_PAGE_INFO, client_ip, dnt_enabled);
    let page = NotFoundPage { ctx };
    (StatusCode::NOT_FOUND, page).into_response()
}
