use crate::libs::{PhpCountService, UpvoteService};

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
