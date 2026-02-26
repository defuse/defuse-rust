use crate::libs::{PhpCountService, UpvoteService};

/// Holds instances of database-connected libraries common to most pages.
#[derive(Clone)]
pub struct AppState {
    pub phpcount: PhpCountService,
    pub upvotes: UpvoteService,
}

impl AppState {
    pub fn new(phpcount: PhpCountService, upvotes: UpvoteService) -> Self {
        Self { phpcount, upvotes }
    }
}
