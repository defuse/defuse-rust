//! Additional hash algorithm implementations.
//!
//! These implementations match PHP's hash() function output exactly.
//! Implemented from algorithm specifications, verified against PHP test vectors.
//! 
//! THIS CODE HAS NOT BEEN AUDITED! DO NOT RELY ON IT FOR SECURITY!

pub mod snefru;
pub mod haval;
pub mod tiger;

pub use snefru::snefru256;
pub use haval::{
    haval128_3, haval128_4, haval128_5,
    haval160_3, haval160_4, haval160_5,
    haval192_3, haval192_4, haval192_5,
    haval224_3, haval224_4, haval224_5,
    haval256_3, haval256_4, haval256_5,
};
pub use tiger::{
    tiger128_3, tiger128_4,
    tiger160_3, tiger160_4,
    tiger192_3, tiger192_4,
};
