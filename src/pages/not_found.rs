//! 404 Not Found page.
//!
//! This module provides the NotFoundPage template struct used by the dispatcher.
//! The dispatcher handles rendering 404s directly rather than using a handler.

use askama::Template;

use crate::context::PageContext;

#[derive(Template)]
#[template(path = "pages/404.html")]
pub struct NotFoundPage {
    pub ctx: PageContext,
}
