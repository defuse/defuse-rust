//! Cryptographically secure password generator.
//!
//! This module provides unbiased random password generation using:
//! - OS-provided CSPRNG (via `rand::rngs::OsRng`)
//! - Rejection sampling for uniform distribution
//! - Constant-time array indexing to prevent timing side-channels
//!
//! This is a direct port of the PHP PasswordGenerator class from defuse.ca.

use rand::rngs::OsRng;
use rand::RngCore;
use subtle::{ConditionallySelectable, ConstantTimeEq};

/// Printable ASCII characters (codes 33-126)
const ASCII_CHARS: &[u8] = b"!\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~";

/// Alphanumeric characters (A-Z, a-z, 0-9)
const ALPHANUM_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

/// Hexadecimal characters (0-9, A-F)
const HEX_CHARS: &[u8] = b"0123456789ABCDEF";

/// Generate a password using printable ASCII characters.
///
/// Characters range from '!' (33) to '~' (126).
pub fn generate_ascii_password(length: usize) -> String {
    generate_password(ASCII_CHARS, length)
}

/// Generate a password using alphanumeric characters.
///
/// Characters are A-Z, a-z, and 0-9.
pub fn generate_alphanumeric_password(length: usize) -> String {
    generate_password(ALPHANUM_CHARS, length)
}

/// Generate a password using hexadecimal characters.
///
/// Characters are 0-9 and A-F (uppercase).
pub fn generate_hex_password(length: usize) -> String {
    generate_password(HEX_CHARS, length)
}

/// Generate a random password from a custom character set.
///
/// Uses rejection sampling to ensure uniform distribution across all
/// characters in the set, regardless of the set size.
fn generate_password(charset: &[u8], length: usize) -> String {
    if length == 0 || charset.is_empty() {
        return String::new();
    }

    let charset_len = charset.len();
    assert!(charset_len <= 255);
    let mask = get_minimal_bit_mask(charset_len - 1);

    let mut password = Vec::with_capacity(length);
    let mut rng = OsRng;

    // Buffer for random bytes - we'll need roughly length bytes on average,
    // but rejection sampling may require more
    let mut random_bytes = vec![0u8; length * 2];
    rng.fill_bytes(&mut random_bytes);

    // Iteration limit to prevent infinite loops from malicious/broken RNG
    // It's astronomically unlikely to need more than length * 128 attempts
    let iter_limit = length.saturating_mul(128).max(length);
    let mut iterations = 0;
    let mut byte_idx = 0;

    while password.len() < length {
        // Refill random buffer if exhausted
        if byte_idx >= random_bytes.len() {
            let needed = (length - password.len()) * 2;
            random_bytes.resize(needed, 0);
            rng.fill_bytes(&mut random_bytes);
            byte_idx = 0;
        }

        // Apply mask and check if within range (rejection sampling)
        let masked = (random_bytes[byte_idx] as usize) & mask;
        byte_idx += 1;

        if masked < charset_len {
            // Use constant-time indexing to prevent timing side-channels
            let char_byte = constant_time_index(charset, masked);
            password.push(char_byte);
        }

        iterations += 1;
        if iterations >= iter_limit {
            // This should never happen with a proper RNG
            panic!("There's something seriously wrong with the random number generator!");
        }
    }

    // SAFETY: All characters come from ASCII charset, so this is valid UTF-8
    String::from_utf8(password).expect("password should be valid UTF-8")
}

/// Get the smallest bit mask of all 1s such that (value & mask) == value.
///
/// For example:
/// - get_minimal_bit_mask(5) returns 0b111 (7)
/// - get_minimal_bit_mask(15) returns 0b1111 (15)
/// - get_minimal_bit_mask(16) returns 0b11111 (31)
fn get_minimal_bit_mask(max_value: usize) -> usize {
    if max_value == 0 {
        return 0;
    }

    let mut mask: usize = 1;
    while mask < max_value {
        mask = (mask << 1) | 1;
    }
    mask
}

/// Index into an array in constant time to prevent timing side-channels.
///
/// Uses the `subtle` crate's `ConditionallySelectable` and `ConstantTimeEq`
/// to stay entirely within `subtle`'s type system, ensuring the compiler
/// cannot optimize the selection into a branch.
fn constant_time_index(array: &[u8], index: usize) -> u8 {
    let mut result: u8 = 0;
    let index_byte = index as u8;

    for (i, &byte) in array.iter().enumerate() {
        let choice = (i as u8).ct_eq(&index_byte);
        result.conditional_assign(&byte, choice);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_minimal_bit_mask() {
        assert_eq!(get_minimal_bit_mask(0), 0);
        assert_eq!(get_minimal_bit_mask(1), 1);
        assert_eq!(get_minimal_bit_mask(2), 3);
        assert_eq!(get_minimal_bit_mask(3), 3);
        assert_eq!(get_minimal_bit_mask(4), 7);
        assert_eq!(get_minimal_bit_mask(5), 7);
        assert_eq!(get_minimal_bit_mask(7), 7);
        assert_eq!(get_minimal_bit_mask(8), 15);
        assert_eq!(get_minimal_bit_mask(15), 15);
        assert_eq!(get_minimal_bit_mask(16), 31);
        assert_eq!(get_minimal_bit_mask(93), 127); // ASCII charset has 94 chars
    }

    #[test]
    fn test_constant_time_index() {
        let array = b"ABCDEFGHIJ";
        assert_eq!(constant_time_index(array, 0), b'A');
        assert_eq!(constant_time_index(array, 1), b'B');
        assert_eq!(constant_time_index(array, 5), b'F');
        assert_eq!(constant_time_index(array, 9), b'J');
    }

    #[test]
    fn test_ascii_password_length() {
        let password = generate_ascii_password(64);
        assert_eq!(password.len(), 64);
    }

    #[test]
    fn test_ascii_password_charset() {
        let password = generate_ascii_password(1000);
        for c in password.chars() {
            assert!(c.is_ascii());
            assert!(!c.is_ascii_control());
            assert!(c as u8 >= 33 && c as u8 <= 126);
        }
    }

    #[test]
    fn test_alphanumeric_password_length() {
        let password = generate_alphanumeric_password(64);
        assert_eq!(password.len(), 64);
    }

    #[test]
    fn test_alphanumeric_password_charset() {
        let password = generate_alphanumeric_password(1000);
        for c in password.chars() {
            assert!(c.is_ascii_alphanumeric());
        }
    }

    #[test]
    fn test_hex_password_length() {
        let password = generate_hex_password(64);
        assert_eq!(password.len(), 64);
    }

    #[test]
    fn test_hex_password_charset() {
        let password = generate_hex_password(1000);
        for c in password.chars() {
            assert!(c.is_ascii_hexdigit());
            // Should be uppercase
            assert!(c.is_ascii_digit() || c.is_ascii_uppercase());
        }
    }

    #[test]
    fn test_passwords_are_different() {
        let p1 = generate_ascii_password(64);
        let p2 = generate_ascii_password(64);
        assert_ne!(p1, p2, "Two generated passwords should be different");
    }

    #[test]
    fn test_empty_length() {
        assert_eq!(generate_ascii_password(0), "");
        assert_eq!(generate_alphanumeric_password(0), "");
        assert_eq!(generate_hex_password(0), "");
    }
}
