//! Application state shared across all requests

use crate::libs::{PhpCountService, UpvoteService};

/// Application state containing database services and shared resources
#[derive(Clone)]
pub struct AppState {
    /// Hit counter service
    pub phpcount: PhpCountService,
    /// Upvote/downvote service
    pub upvotes: UpvoteService,
    // Future services will be added here:
    // pub pastebin: PastebinService,
    // pub trent: TrentService,
    // etc.
}

impl AppState {
    pub fn new(phpcount: PhpCountService, upvotes: UpvoteService) -> Self {
        Self { phpcount, upvotes }
    }
}
