//! Application state shared across all requests

use crate::db::PhpCountService;

/// Application state containing database services and shared resources
#[derive(Clone)]
pub struct AppState {
    /// Hit counter service
    pub phpcount: PhpCountService,
    // Future services will be added here:
    // pub upvotes: UpvoteService,
    // pub pastebin: PastebinService,
    // pub trent: TrentService,
    // etc.
}

impl AppState {
    pub fn new(phpcount: PhpCountService) -> Self {
        Self { phpcount }
    }
}
