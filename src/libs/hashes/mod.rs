//! Additional hash algorithm implementations.
//!
//! These implementations match PHP's hash() function output exactly.
//! Implemented from algorithm specifications, verified against PHP test vectors.
//! 
//! THIS CODE HAS NOT BEEN AUDITED! DO NOT RELY ON IT FOR SECURITY!

pub mod snefru;
pub mod haval;
pub mod tiger;
