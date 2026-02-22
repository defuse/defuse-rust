use askama::Template;
use axum::response::IntoResponse;

use crate::app_state::AppState;
use crate::context::PageContext;
use crate::handler::{BoxFuture, PageHandler, PostBody};
use crate::libs::markdown;

static README_MD: &str = include_str!("../../../static/markdown/wavetool-readme.md");

pub struct Handler;

impl PageHandler for Handler {
    fn get(&self, ctx: PageContext, _state: &AppState) -> BoxFuture {
        Box::pin(async move {
            let content_html = markdown::render_readme(README_MD);
            WavetoolPage { ctx, content_html }.into_response()
        })
    }

    fn post(&self, ctx: PageContext, state: &AppState, _body: PostBody) -> Option<BoxFuture> {
        Some(self.get(ctx, state))
    }
}

#[derive(Template)]
#[template(path = "pages/projects/wavetool.html")]
struct WavetoolPage {
    ctx: PageContext,
    content_html: String,
}
