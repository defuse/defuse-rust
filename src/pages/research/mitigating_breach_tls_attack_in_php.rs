//! Mitigating BREACH attack in PHP page handler.
//!
//! This page demonstrates PHP code for mitigating the BREACH attack on SSL/TLS.
//! It shows the PHP source code with syntax highlighting and generates sample output.

use std::path::Path;

use askama::Template;
use axum::response::IntoResponse;

use crate::app_state::AppState;
use crate::context::PageContext;
use crate::handler::{BoxFuture, PageHandler, PostBody};
use crate::libs::breach;
use crate::libs::vim_highlight;

pub struct Handler;

impl PageHandler for Handler {
    fn get(&self, ctx: PageContext, _state: &AppState) -> BoxFuture {
        Box::pin(async move {
            // Highlight the PHP source file
            let source_path = Path::new("static/source/breach.php");
            let highlighted_source = vim_highlight::highlight_file(source_path, false)
                .unwrap_or_else(|e| format!("<p>Error highlighting source: {}</p>", e));

            // Generate sample breach_visual_html output
            let sample_header = breach::breach_visual_html("Sample Header");
            let sample_paragraph = breach::breach_visual_html("Sample paragraph text.");

            MitigatingBreachPage {
                ctx,
                highlighted_source,
                sample_header,
                sample_paragraph,
            }
            .into_response()
        })
    }

    fn post(&self, ctx: PageContext, state: &AppState, _body: PostBody) -> Option<BoxFuture> {
        Some(self.get(ctx, state))
    }
}

#[derive(Template)]
#[template(path = "pages/research/mitigating_breach_tls_attack_in_php.html")]
struct MitigatingBreachPage {
    ctx: PageContext,
    highlighted_source: String,
    sample_header: String,
    sample_paragraph: String,
}
