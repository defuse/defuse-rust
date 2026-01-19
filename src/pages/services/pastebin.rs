//! Main pastebin page handler.
//!
//! Renders the pastebin form at /pastebin.htm

use askama::Template;
use axum::response::IntoResponse;
use rand::RngCore;

use crate::app_state::AppState;
use crate::context::PageContext;
use crate::handler::{BoxFuture, PageHandler, PostBody};
#[allow(unused_imports)]
use crate::prelude::*;

pub struct Handler;

impl PageHandler for Handler {
    fn get(&self, ctx: PageContext, _state: &AppState) -> BoxFuture {
        Box::pin(async move {
            // Generate 64 hex chars of random entropy for SJCL
            let mut entropy_bytes = [0u8; 32];
            rand::rngs::OsRng.fill_bytes(&mut entropy_bytes);
            let entropy = hex::encode(entropy_bytes);

            PastebinPage { ctx, entropy }.into_response()
        })
    }

    fn post(&self, _ctx: PageContext, _state: &AppState, _body: PostBody) -> Option<BoxFuture> {
        // No POST handler for main page - posts go to /bin/add.php
        None
    }
}

#[derive(Template)]
#[template(path = "pages/services/pastebin.html")]
struct PastebinPage {
    ctx: PageContext,
    entropy: String,
}
