//! Password Generator page handler.
//!
//! Generates cryptographically secure random passwords using the OS CSPRNG.
//! This page should never be cached since it generates sensitive data.

use askama::Template;
use axum::response::IntoResponse;

use crate::app_state::AppState;
use crate::context::PageContext;
use crate::handler::{BoxFuture, PageHandler, PostBody};
use crate::libs::markdown;
use crate::libs::passgen;

static README_MD: &str = include_str!("../../../static/markdown/passgenr-readme.md");

pub struct Handler;

impl PageHandler for Handler {
    fn get(&self, ctx: PageContext, _state: &AppState) -> BoxFuture {
        Box::pin(async move {
            // Generate 64-character passwords of each type
            let ascii = passgen::generate_ascii_password(64);
            let alpha = passgen::generate_alphanumeric_password(64);
            let hex = passgen::generate_hex_password(64);
            let readme_html = markdown::render_readme(README_MD);

            PassgenPage {
                ctx,
                ascii,
                alpha,
                hex,
                readme_html,
            }
            .into_response()
        })
    }

    fn post(&self, ctx: PageContext, state: &AppState, _body: PostBody) -> Option<BoxFuture> {
        // POST behaves the same as GET - just generate new passwords
        Some(self.get(ctx, state))
    }
}

#[derive(Template)]
#[template(path = "pages/software/passgen.html")]
struct PassgenPage {
    ctx: PageContext,
    ascii: String,
    alpha: String,
    hex: String,
    readme_html: String,
}
