use std::path::PathBuf;

use crate::libs::{PhpCountService, UpvoteService};

/// Holds instances of database-connected libraries common to most pages.
#[derive(Clone)]
pub struct AppState {
    pub phpcount: PhpCountService,
    pub upvotes: UpvoteService,
    pub static_dir: PathBuf,
}

impl AppState {
    pub fn new(phpcount: PhpCountService, upvotes: UpvoteService) -> Self {
        Self {
            phpcount,
            upvotes,
            static_dir: PathBuf::from("static"),
        }
    }
}
