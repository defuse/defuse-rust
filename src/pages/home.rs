use askama::Template;
use axum::response::IntoResponse;

use crate::app_state::AppState;
use crate::context::PageContext;
use crate::handler::{BoxFuture, PageHandler};

pub struct Handler;

impl PageHandler for Handler {
    fn get(&self, ctx: PageContext, state: &AppState) -> BoxFuture {
        let upvotes = state.upvotes.clone();
        let page_url = ctx.page_info.relative_url();
        let client_ip = ctx.client_ip.clone();
        Box::pin(async move {
            // TODO: this stuff should just be moved into render_list
            let top_pages = upvotes
                .get_top_pages(8, None)
                .await
                .expect("BUG: Failed to get top pages from database");

            let user_actions = upvotes
                .get_user_actions_batch(&top_pages, &client_ip)
                .await
                .expect("BUG: Failed to get user actions");

            let top_pages_html = crate::libs::upvotes::UpvoteService::render_list(
                &top_pages,
                &page_url,
                &user_actions,
            );

            HomePage { ctx, top_pages_html }.into_response()
        })
    }
}

#[derive(Template)]
#[template(path = "pages/home.html")]
struct HomePage {
    ctx: PageContext,
    top_pages_html: String,
}
