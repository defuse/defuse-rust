//! Hit count types.
//!
//! These structs are populated by the dispatcher and stored in PageContext.

/// Hit counts for display in templates.
#[derive(Clone, Debug, Default)]
pub struct HitCounts {
    pub page_hits: u32,
    pub unique_hits: u32,
    pub total_hits: u32,
    pub total_unique_hits: u32,
}
