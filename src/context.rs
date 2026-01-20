//! Request context - extracted once per request, shared with all templates.
//!
//! PageContext contains all per-request data needed by templates: page metadata,
//! client info, hit counts, and vote state. It is constructed by the dispatcher.

use crate::libs::phpcount::HitCounts;
use crate::libs::upvotes::VoteState;
use crate::registry::PageInfo;

#[derive(Debug, Clone)]
pub struct PageContext {
    pub page_info: &'static PageInfo,
    pub client_ip: String,
    pub dnt_enabled: bool,
    pub hit_counts: HitCounts,
    pub vote_state: VoteState,
    /// CAPTCHA bypass header for automated testing
    pub captcha_bypass_header: Option<String>,
    /// Query string from the URL (without leading ?)
    pub query_string: Option<String>,
    /// URL prefix for building absolute URLs (e.g., "https://defuse.ca" or "http://localhost:3000")
    pub url_prefix: String,
}

impl PageContext {
    pub fn is_home(&self) -> bool {
        self.page_info.slug.is_empty()
    }
}
