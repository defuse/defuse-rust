//! Common imports for page templates.
//!
//! This module re-exports types that templates frequently need.
//! The simple_page!() macro automatically imports everything from here.

pub use std::path::Path;

pub use crate::libs::bibliography::Bibliography;
pub use crate::libs::vim_highlight;
