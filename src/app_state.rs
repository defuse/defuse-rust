use crate::libs::{PastebinService, PhpCountService, UpvoteService};

/// Holds instances of database-connected libraries common to most pages.
#[derive(Clone)]
pub struct AppState {
    pub phpcount: PhpCountService,
    pub upvotes: UpvoteService,
    // AUDIT: pastebin does not need to be scoped to AppState, can't the pastebin object just construct it the same way e.g. TRENT constructs its instance of the library? Or how the syntax highlighting pages get vim highlight?
    pub pastebin: PastebinService,
}

impl AppState {
    pub fn new(phpcount: PhpCountService, upvotes: UpvoteService, pastebin: PastebinService) -> Self {
        Self { phpcount, upvotes, pastebin }
    }
}
