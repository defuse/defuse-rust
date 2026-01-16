use askama::Template;
use axum::response::IntoResponse;

use crate::app_state::AppState;
use crate::context::PageContext;
use crate::handler::{BoxFuture, PageHandler};

pub struct Handler;

impl PageHandler for Handler {
    fn get(&self, ctx: PageContext, state: &AppState) -> BoxFuture {
        let upvotes = state.upvotes.clone();
        let page_url = ctx.canonical_url();
        let client_ip = ctx.client_ip.clone();
        Box::pin(async move {
            let all_pages = upvotes
                .get_all_pages(None)
                .await
                .expect("BUG: Failed to get all pages from database");

            let user_actions = upvotes
                .get_user_actions_batch(&all_pages, &client_ip)
                .await
                .expect("BUG: Failed to get user actions");

            let all_pages_html = crate::libs::upvotes::UpvoteService::render_list(
                &all_pages,
                &page_url,
                &user_actions,
            );

            AllPagesPage { ctx, all_pages_html }.into_response()
        })
    }
}

#[derive(Template)]
#[template(path = "pages/all_pages.html")]
struct AllPagesPage {
    ctx: PageContext,
    all_pages_html: String,
}
