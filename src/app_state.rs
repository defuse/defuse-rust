use crate::libs::{PastebinService, PhpCountService, UpvoteService};

#[derive(Clone)]
pub struct AppState {
    pub phpcount: PhpCountService,
    pub upvotes: UpvoteService,
    pub pastebin: PastebinService,
}

impl AppState {
    pub fn new(phpcount: PhpCountService, upvotes: UpvoteService, pastebin: PastebinService) -> Self {
        Self { phpcount, upvotes, pastebin }
    }
}
