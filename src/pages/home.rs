use askama::Template;
use askama_axum::IntoResponse;

use crate::context::PageContext;

#[derive(Template)]
#[template(path = "pages/home.html")]
pub struct HomePage {
    pub ctx: PageContext,
}

pub async fn get(ctx: PageContext) -> impl IntoResponse {
    HomePage { ctx }
}
