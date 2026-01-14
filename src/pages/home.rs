use askama::Template;
use axum::response::IntoResponse;

use crate::context::PageContext;
use crate::app_state::AppState;
use crate::handler::{BoxFuture, PageHandler};
use crate::libs::upvotes::PageVoteInfo;

pub struct Handler;

impl PageHandler for Handler {
    fn get(&self, ctx: PageContext, state: &AppState) -> BoxFuture {
        let upvotes = state.upvotes.clone();
        Box::pin(async move {
            let top_pages = upvotes
                .get_top_pages(8, None)
                .await
                .expect("BUG: Failed to get top pages from database");
            HomePage { ctx, top_pages }.into_response()
        })
    }
}

#[derive(Template)]
#[template(path = "pages/home.html")]
struct HomePage {
    ctx: PageContext,
    top_pages: Vec<PageVoteInfo>,
}
