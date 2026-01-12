use askama::Template;
use askama_axum::IntoResponse;
use axum::http::HeaderMap;

use crate::context::PageContext;

#[derive(Template)]
#[template(path = "pages/about.html")]
pub struct AboutPage {
    pub title: &'static str,
    // Base template context
    pub is_home: bool,
    pub client_ip: String,
    pub dnt_enabled: bool,
    pub page_hits: u64,
    pub unique_hits: u64,
}

pub async fn get(headers: HeaderMap) -> impl IntoResponse {
    let ctx = PageContext::from_headers(&headers);
    AboutPage {
        title: "About - Defuse Security",
        is_home: ctx.is_home,
        client_ip: ctx.client_ip,
        dnt_enabled: ctx.dnt_enabled,
        page_hits: ctx.page_hits,
        unique_hits: ctx.unique_hits,
    }
}
