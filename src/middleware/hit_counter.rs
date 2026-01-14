//! Hit count and vote state types.
//!
//! These structs are populated by the dispatcher and stored in PageContext.

use crate::db::upvotes::VoteAction;

/// Hit counts for display in templates.
#[derive(Clone, Debug, Default)]
pub struct HitCounts {
    pub page_hits: u32,
    pub unique_hits: u32,
    pub total_hits: u32,
    pub total_unique_hits: u32,
}

/// Vote state for display in templates.
/// Contains both aggregate counts and the current user's vote.
#[derive(Clone, Debug, Default)]
pub struct VoteState {
    pub upvotes: i32,
    pub downvotes: i32,
    pub user_vote: Option<VoteAction>,
}

impl VoteState {
    /// Net vote total (upvotes - downvotes)
    pub fn total(&self) -> i32 {
        self.upvotes - self.downvotes
    }

    /// Whether the current user has upvoted
    pub fn user_upvoted(&self) -> bool {
        self.user_vote == Some(VoteAction::Upvote)
    }

    /// Whether the current user has downvoted
    pub fn user_downvoted(&self) -> bool {
        self.user_vote == Some(VoteAction::Downvote)
    }
}
