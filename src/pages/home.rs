use askama::Template;
use askama_axum::IntoResponse;
use axum::extract::State;

use crate::context::PageContext;
use crate::db::upvotes::PageVoteInfo;
use crate::state::AppState;

#[derive(Template)]
#[template(path = "pages/home.html")]
pub struct HomePage {
    pub ctx: PageContext,
    pub top_pages: Vec<PageVoteInfo>,
}

pub async fn get(State(state): State<AppState>, ctx: PageContext) -> impl IntoResponse {
    // Fetch top 8 pages for display
    let top_pages = state
        .upvotes
        .get_top_pages(8, None)
        .await
        .unwrap_or_default();

    HomePage { ctx, top_pages }
}
