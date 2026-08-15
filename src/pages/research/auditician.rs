use askama::Template;
use axum::response::IntoResponse;

use crate::app_state::AppState;
use crate::context::PageContext;
use crate::handler::{BoxFuture, PageHandler, PostBody};
use crate::libs::markdown;

static README_MD: &str = include_str!("../../../static/markdown/auditician-readme.md");

pub struct Handler;

impl PageHandler for Handler {
    fn get(&self, ctx: PageContext, _state: &AppState) -> BoxFuture {
        Box::pin(async move {
            let content_html = markdown::render_readme(README_MD);
            AuditicianPage { ctx, content_html }.into_response()
        })
    }

    fn post(&self, ctx: PageContext, state: &AppState, _body: PostBody) -> Option<BoxFuture> {
        Some(self.get(ctx, state))
    }
}

#[derive(Template)]
#[template(path = "pages/research/auditician.html")]
struct AuditicianPage {
    ctx: PageContext,
    content_html: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The README is downloaded by scripts/update-readmes.sh and embedded at
    /// compile time. A failed download would embed an error page instead, so
    /// check that what we embedded is really the auditician README.
    #[test]
    fn test_embedded_readme_is_the_auditician_readme() {
        assert!(README_MD.starts_with("# auditician\n"));
        assert!(README_MD.contains("**Automated security auditing for Claude Code.**"));
    }

    /// The page template supplies its own <h1>, so the README's title heading
    /// must be stripped and the remaining headings demoted one level.
    #[test]
    fn test_rendered_readme_drops_title_and_demotes_headings() {
        let html = markdown::render_readme(README_MD);
        assert!(!html.contains("<h1"));
        assert!(html.contains(
            "<h3><a href=\"#getting-started\" aria-hidden=\"true\" class=\"anchor\" \
             id=\"sec-getting-started\"></a>Getting Started</h3>"
        ));
    }
}
