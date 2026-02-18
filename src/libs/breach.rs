//! BREACH attack mitigation functions.
//!
//! This module provides functions for mitigating the BREACH attack on SSL/TLS
//! by adding randomness to secrets in HTML pages.
//!
//! The BREACH attack exploits HTTP compression to extract secrets from pages
//! delivered over TLS. These functions implement the "Masking Secrets" technique
//! discussed in section 3.4 of the BREACH paper.
//!
//! Reference: https://defuse.ca/mitigating-breach-tls-attack-in-php.htm

use rand::rngs::OsRng;
use rand::RngCore;

/// Encode a string with a random one-time pad to prevent BREACH attacks.
///
/// The output is hex-encoded: pad || (input XOR pad)
/// This ensures the encoded output has no correlation with the input.
///
/// WARNING: Do not re-use the output across requests.
/// 
/// TODO: These are not actually used by the site, but are here anyway in case I
/// want to add the rust code to the page.
#[allow(dead_code)]
pub fn breach_encode(input: &str) -> String {
    let input_bytes = input.as_bytes();
    let len = input_bytes.len();

    // Generate random pad
    let mut pad = vec![0u8; len];
    OsRng.fill_bytes(&mut pad);

    // XOR input with pad
    let mut encoded = Vec::with_capacity(len);
    for i in 0..len {
        encoded.push(input_bytes[i] ^ pad[i]);
    }

    // Return hex(pad || encoded)
    let mut result = Vec::with_capacity(len * 2);
    result.extend_from_slice(&pad);
    result.extend_from_slice(&encoded);
    hex::encode(result)
}

/// Decode a breach_encode'd string back to the original.
///
/// Returns None if the input is invalid (not valid hex or odd length after decoding).
/// TODO: These are not actually used by the site, but are here anyway in case I
/// want to add the rust code to the page.
#[allow(dead_code)]
pub fn breach_decode(encoded: &str) -> Option<String> {
    let bytes = hex::decode(encoded).ok()?;

    if bytes.len() % 2 != 0 {
        return None;
    }

    let length = bytes.len() / 2;
    let pad = &bytes[0..length];
    let encoded_data = &bytes[length..];

    // XOR to decode
    let mut decoded = Vec::with_capacity(length);
    for i in 0..length {
        decoded.push(pad[i] ^ encoded_data[i]);
    }

    String::from_utf8(decoded).ok()
}

/// Encode a string for visual display with BREACH protection.
///
/// This inserts random HTML comments and zero-width spaces between each character,
/// making compression-based attacks ineffective while still displaying correctly
/// in the browser.
///
/// WARNING: This function is EXPERIMENTAL and should not be relied on for
/// high-security applications.
pub fn breach_visual_html(input: &str) -> String {
    let mut result = String::new();

    for ch in input.chars() {
        result.push_str(&breach_comment_string());
        result.push_str(&breach_zws_string());
        result.push(ch);
        result.push_str(&breach_zws_string());
        result.push_str(&breach_comment_string());
    }

    result
}

/// Generate a random string of zero-width space HTML entities.
///
/// Returns 0-15 zero-width spaces, randomly using either &#8203; or &#x200b; format.
fn breach_zws_string() -> String {
    let mut rng = OsRng;
    let mut byte = [0u8; 1];

    rng.fill_bytes(&mut byte);
    let zws_count = (byte[0] % 16) as usize;

    let mut result = String::new();
    for _ in 0..zws_count {
        rng.fill_bytes(&mut byte);
        if byte[0] % 2 == 0 {
            result.push_str("&#8203;");
        } else {
            result.push_str("&#x200b;");
        }
    }

    result
}

/// Generate a random HTML comment string.
///
/// Returns a comment like: <!-- 1a2b3c4d -->
fn breach_comment_string() -> String {
    let mut rng = OsRng;
    let mut bytes = [0u8; 4];
    rng.fill_bytes(&mut bytes);

    format!("<!-- {} -->", hex::encode(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_breach_encode_decode_roundtrip() {
        let original = "Hello, World!";
        let encoded = breach_encode(original);
        let decoded = breach_decode(&encoded);
        assert_eq!(decoded, Some(original.to_string()));
    }

    #[test]
    fn test_breach_encode_different_each_time() {
        let input = "secret";
        let encoded1 = breach_encode(input);
        let encoded2 = breach_encode(input);
        assert_ne!(encoded1, encoded2, "Two encodings of the same input should differ");
    }

    #[test]
    fn test_breach_decode_invalid_hex() {
        assert_eq!(breach_decode("not valid hex!"), None);
    }

    #[test]
    fn test_breach_decode_odd_length() {
        // Valid hex but odd number of decoded bytes
        assert_eq!(breach_decode("abc"), None);
    }

    #[test]
    fn test_breach_visual_html_contains_original() {
        let input = "Test";
        let output = breach_visual_html(input);
        // Should contain each character from input
        for ch in input.chars() {
            assert!(output.contains(ch), "Output should contain '{}'", ch);
        }
    }

    #[test]
    fn test_breach_visual_html_contains_comments() {
        let input = "A";
        let output = breach_visual_html(input);
        // Should contain HTML comments
        assert!(output.contains("<!--"), "Output should contain HTML comments");
        assert!(output.contains("-->"), "Output should contain HTML comments");
    }

    #[test]
    fn test_breach_visual_html_different_each_time() {
        let input = "test";
        let output1 = breach_visual_html(input);
        let output2 = breach_visual_html(input);
        assert_ne!(output1, output2, "Two visual encodings should differ");
    }

    #[test]
    fn test_breach_encode_empty_string() {
        let encoded = breach_encode("");
        let decoded = breach_decode(&encoded);
        assert_eq!(decoded, Some(String::new()));
    }
}
