//! PageHandler trait for registry-driven routing.
//!
//! All page handlers implement this trait. The dispatcher calls the appropriate
//! method based on the HTTP method of the request.

use std::future::Future;
use std::pin::Pin;

use axum::response::Response;
use bytes::Bytes;

use crate::context::PageContext;
use crate::app_state::AppState;

/// A boxed future that returns a Response. Used for trait object compatibility.
pub type BoxFuture = Pin<Box<dyn Future<Output = Response> + Send>>;

/// Trait for page handlers. Implement this for each page.
///
/// The dispatcher looks up the handler in the registry and calls the appropriate
/// method based on the HTTP request method.
pub trait PageHandler: Send + Sync + 'static {
    /// Handle GET requests.
    fn get(&self, ctx: PageContext, state: &AppState) -> BoxFuture;

    /// Handle POST requests. Returns None if POST is not supported (405 Method Not Allowed).
    /// Override this method to handle POST requests.
    fn post(&self, _ctx: PageContext, _state: &AppState, _body: Bytes) -> Option<BoxFuture> {
        None
    }
}

/// Macro for simple pages that just render a template with PageContext.
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
