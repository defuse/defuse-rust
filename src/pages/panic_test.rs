//! Test page that intentionally panics during template rendering.
//!
//! Used to verify that CatchPanicLayer properly handles panics and returns 500.

use askama::Template;
use axum::response::IntoResponse;

use crate::context::PageContext;
use crate::handler::{BoxFuture, PageHandler};
use crate::state::AppState;

pub struct Handler;

impl PageHandler for Handler {
    fn get(&self, ctx: PageContext, _state: &AppState) -> BoxFuture {
        Box::pin(async move { PanicTestPage { ctx }.into_response() })
    }
}

#[derive(Template)]
#[template(path = "pages/panic_test.html")]
struct PanicTestPage {
    ctx: PageContext,
}

impl PanicTestPage {
    /// This method panics when called. Used to test panic handling during template render.
    fn trigger_panic(&self) -> &str {
        panic!("Intentional panic during template rendering!");
    }
}
