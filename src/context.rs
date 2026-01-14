//! Request context - extracted once per request, shared with all templates.
//!
//! PageContext contains all per-request data needed by templates: page metadata,
//! client info, hit counts, and vote state. It is constructed by the dispatcher.

use std::path::Path;

use crate::libs::phpcount::HitCounts;
use crate::libs::upvotes::VoteState;
use crate::libs::{util::html_escape, vim_highlight};
use crate::registry::PageInfo;

/// Common context data available to all page templates.
///
/// Constructed by the dispatcher for each request.
#[derive(Debug, Clone)]
pub struct PageContext {
    /// Page info from registry (always present for known pages; 404 handler uses NOT_FOUND_PAGE_INFO)
    pub page_info: &'static PageInfo,
    /// Client's IP address (from X-Forwarded-For, X-Real-IP, or connection)
    pub client_ip: String,
    /// Whether Do Not Track header is set
    pub dnt_enabled: bool,
    /// Hit counts for this page and site totals
    pub hit_counts: HitCounts,
    /// Vote state (counts + user's vote). Always present, defaults to zeros.
    pub vote_state: VoteState,
}

impl PageContext {
    /// Whether this is the home page
    pub fn is_home(&self) -> bool {
        self.page_info.slug.is_empty()
    }

    /// Get the canonical URL for this page
    pub fn canonical_url(&self) -> String {
        let p = self.page_info;
        if p.slug.is_empty() {
            "https://defuse.ca/".to_string()
        } else if p.is_directory() {
            format!("https://defuse.ca/{}/", p.slug.trim_end_matches('/'))
        } else {
            format!("https://defuse.ca/{}.htm", p.slug)
        }
    }

    // ===== Syntax Highlighting (matches PHP's printHlString/printSourceFile) =====

    /// Syntax highlight a string. Matches PHP's printHlString($text, $ft, $numbers).
    /// Returns HTML wrapped in <div class="vimhighlight">
    pub fn hl_string(&self, text: &str, file_type: &str, show_lines: bool) -> String {
        vim_highlight::highlight_string(text, file_type, show_lines)
            .unwrap_or_else(|e| format!("<pre>Error highlighting: {}</pre>", html_escape(&e.to_string())))
    }

    /// Syntax highlight a source file. Matches PHP's printSourceFile($path, $numbers).
    /// Returns HTML wrapped in <div class="vimhighlight">
    pub fn hl_file(&self, path: &str, show_lines: bool) -> String {
        vim_highlight::highlight_file(Path::new(path), show_lines)
            .unwrap_or_else(|e| format!("<pre>Error highlighting file: {}</pre>", html_escape(&e.to_string())))
    }
}
