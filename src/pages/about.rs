use askama::Template;
use askama_axum::IntoResponse;

use crate::context::PageContext;

#[derive(Template)]
#[template(path = "pages/about.html")]
pub struct AboutPage {
    pub ctx: PageContext,
}

pub async fn get(ctx: PageContext) -> impl IntoResponse {
    AboutPage { ctx }
}
