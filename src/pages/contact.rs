use askama::Template;
use askama_axum::IntoResponse;

use crate::context::PageContext;

#[derive(Template)]
#[template(path = "pages/contact.html")]
pub struct ContactPage {
    pub ctx: PageContext,
}

pub async fn get(ctx: PageContext) -> impl IntoResponse {
    ContactPage { ctx }
}
