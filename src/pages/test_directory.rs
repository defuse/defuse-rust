//! Test directory page for unit testing directory-style URL handling.

use askama::Template;
use axum::response::IntoResponse;

use crate::context::PageContext;
use crate::handler::{BoxFuture, PageHandler};
use crate::app_state::AppState;

pub struct Handler;

impl PageHandler for Handler {
    fn get(&self, ctx: PageContext, _state: &AppState) -> BoxFuture {
        Box::pin(async move { TestDirectoryPage { ctx }.into_response() })
    }
}

#[derive(Template)]
#[template(path = "pages/test_directory.html")]
struct TestDirectoryPage {
    ctx: PageContext,
}
