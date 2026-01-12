use askama::Template;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::context::PageContext;

#[derive(Template)]
#[template(path = "pages/404.html")]
pub struct NotFoundPage {
    pub ctx: PageContext,
}

pub async fn handler(ctx: PageContext) -> Response {
    let page = NotFoundPage { ctx };
    (StatusCode::NOT_FOUND, page).into_response()
}
