//! PageHandler trait for registry-driven routing.
//!
//! All registered pages implement this trait. The registered_page_handler calls
//! the appropriate method based on the HTTP method of the request.

use std::future::Future;
use std::pin::Pin;

use axum::response::Response;
use bytes::Bytes;

use crate::app_state::AppState;
use crate::context::PageContext;

/// Represents a parsed POST body - either form-urlencoded or multipart.
#[derive(Debug)]
pub enum PostBody {
    /// Standard form-urlencoded data
    UrlEncoded(Bytes),
    /// Multipart form data with parsed fields
    Multipart { fields: Vec<FormField> },
}

/// A single field from a multipart form submission.
#[derive(Debug)]
pub struct FormField {
    pub name: String,
    pub filename: Option<String>,
    pub data: Bytes,
}

pub type BoxFuture = Pin<Box<dyn Future<Output = Response> + Send>>;

/// Each registered page implements this trait to define its request handlers.
///
/// Code in registered_page_handler.rs looks up the handler in the registry and
/// calls the appropriate method based on the HTTP request method.
pub trait PageHandler: Send + Sync + 'static {
    /// Handle GET requests.
    fn get(&self, ctx: PageContext, state: &AppState) -> BoxFuture;

    /// Handle POST requests. Returns None if POST is not supported (405 Method Not Allowed).
    /// Override this method to handle POST requests.
    fn post(&self, _ctx: PageContext, _state: &AppState, _body: PostBody) -> Option<BoxFuture> {
        None
    }
}

/// Macro for simple pages that just render a template with PageContext.
///
/// You can also look at this macro as an example of how to define a page
/// with custom handler logic.
///
/// Usage:
/// ```ignore
/// simple_page!(AboutPage, "pages/about.html");
/// ```
#[macro_export]
macro_rules! simple_page {
    ($template:ident, $path:expr) => {
        use askama::Template;
        use axum::response::IntoResponse;

        use $crate::context::PageContext;
        use $crate::handler::{BoxFuture, PageHandler};
        use $crate::app_state::AppState;
        #[allow(unused_imports)]
        use $crate::prelude::*;

        pub struct Handler;

        impl PageHandler for Handler {
            fn get(&self, ctx: PageContext, _state: &AppState) -> BoxFuture {
                Box::pin(async move { $template { ctx }.into_response() })
            }
        }

        #[derive(Template)]
        #[template(path = $path)]
        struct $template {
            ctx: PageContext,
        }
    };
}

/// Macro for markdown-rendered blog post pages.
///
/// Reads the markdown file from disk at runtime and renders it to HTML using
/// `render_post`. This allows editing .md files without recompiling.
///
/// Usage:
/// ```ignore
/// crate::markdown_page!(MilkSadPage, "zecsec/milk-sad.md");
/// ```
#[macro_export]
macro_rules! markdown_page {
    ($template:ident, $md_path:expr) => {
        use askama::Template;
        use axum::response::IntoResponse;

        use $crate::context::PageContext;
        use $crate::handler::{BoxFuture, PageHandler};
        use $crate::app_state::AppState;
        use $crate::libs::markdown;

        pub struct Handler;

        impl PageHandler for Handler {
            fn get(&self, ctx: PageContext, _state: &AppState) -> BoxFuture {
                Box::pin(async move {
                    let path = std::path::Path::new("static/markdown").join($md_path);
                    let md = match std::fs::read_to_string(&path) {
                        Ok(md) => md,
                        Err(e) => {
                            tracing::error!("Failed to read markdown file {}: {}", path.display(), e);
                            return axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
                        }
                    };
                    let content_html = markdown::render_post(&md);
                    $template { ctx, content_html }.into_response()
                })
            }
        }

        #[derive(Template)]
        #[template(path = "pages/zecsec/post.html")]
        struct $template {
            ctx: PageContext,
            content_html: String,
        }
    };
}
