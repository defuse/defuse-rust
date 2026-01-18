//! HTML Sanitize/Escape page handler.
//!
//! A tool for escaping text so that it looks and behaves exactly
//! like it does in a text editor when displayed in HTML.

use std::path::Path;

use askama::Template;
use axum::response::IntoResponse;
use serde::Deserialize;

use crate::app_state::AppState;
use crate::context::PageContext;
use crate::handler::{BoxFuture, PageHandler, PostBody};
use crate::libs::html_escape;
use crate::libs::vim_highlight;

pub struct Handler;

impl PageHandler for Handler {
    fn get(&self, ctx: PageContext, _state: &AppState) -> BoxFuture {
        Box::pin(async move {
            let source_html = get_source_html();

            HtmlSanitizePage {
                ctx,
                data: String::new(),
                tab_width: 8,
                br_checked: true,
                submitted: false,
                error: None,
                source_html,
            }
            .into_response()
        })
    }

    fn post(&self, ctx: PageContext, _state: &AppState, body: PostBody) -> Option<BoxFuture> {
        Some(Box::pin(async move {
            match body {
                PostBody::UrlEncoded(bytes) => {
                    let form: SanitizeForm =
                        serde_urlencoded::from_bytes(&bytes).unwrap_or_default();

                    let source_html = get_source_html();

                    // Parse tab width
                    let tab_width: i32 = form.tw.parse().unwrap_or(0);

                    if tab_width < 1 {
                        // Invalid tab width
                        return HtmlSanitizePage {
                            ctx,
                            data: "Invalid tab width.".to_string(),
                            tab_width: 8,
                            br_checked: form.br.is_some(),
                            submitted: form.sanitize.is_some(),
                            error: None,
                            source_html,
                        }
                        .into_response();
                    }

                    let br_tags = form.br.as_deref() == Some("yes");

                    // Escape the text
                    let escaped = html_escape::escape_text(&form.data, br_tags, tab_width as usize);

                    HtmlSanitizePage {
                        ctx,
                        data: escaped,
                        tab_width: tab_width as u32,
                        br_checked: br_tags,
                        submitted: form.sanitize.is_some(),
                        error: None,
                        source_html,
                    }
                    .into_response()
                }
                PostBody::Multipart { .. } => {
                    let source_html = get_source_html();

                    HtmlSanitizePage {
                        ctx,
                        data: String::new(),
                        tab_width: 8,
                        br_checked: true,
                        submitted: false,
                        error: None,
                        source_html,
                    }
                    .into_response()
                }
            }
        }))
    }
}

/// Get the syntax-highlighted PHP source code
fn get_source_html() -> String {
    let source_path = Path::new("static/source/HtmlEscape.php");
    vim_highlight::highlight_file(source_path, true).unwrap_or_else(|e| {
        format!("<pre>Error loading source: {}</pre>", e)
    })
}

#[derive(Template)]
#[template(path = "pages/services/html_sanitize.html")]
struct HtmlSanitizePage {
    ctx: PageContext,
    data: String,
    tab_width: u32,
    br_checked: bool,
    submitted: bool,
    error: Option<String>,
    source_html: String,
}

#[derive(Deserialize, Default)]
struct SanitizeForm {
    #[serde(default)]
    data: String,
    #[serde(default)]
    tw: String,
    #[serde(default)]
    br: Option<String>,
    #[serde(default)]
    sanitize: Option<String>,
}
