//! Cryptographic operations for the pastebin service.
//! 
//! This is code intended to be backwards-compatible with old PHP code.
//! Do not copy/paste this code in a new pastebin implementation, as there are
//! several things a new version should fix:
//!     - There is no authentication (which is fine for this use case, since the
//!       database and this code are running on the same server, without
//!       isolation.
//!     - It uses null-byte padding (which is fine for this use case since we 
//!       only officially support text inputs, not files.)
//! A modern pastebin implementation should use a library like libsodium.
//!
//! This module provides encryption/decryption compatible with the PHP pastebin implementation:
//! - AES-256-CBC encryption
//! - Zero-byte padding (mcrypt style, NOT PKCS7)
//! - HMAC-SHA256 for key derivation
//!
//! The key derivation uses HMAC with the URL key as the HMAC key and fixed strings
//! as the message. This must match the PHP version exactly.

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use hmac::{Hmac, Mac};
use rand::RngCore;
use sha2::Sha256;

/// AES block size in bytes
const AES_BLOCK_SIZE: usize = 16;

/// IV size for AES-256-CBC (same as block size)
const IV_SIZE: usize = 16;

/// Error type for crypto operations
#[derive(Debug, Clone)]
pub enum CryptoError {
    InvalidBase64,
    InvalidCiphertext,
    DecryptionFailed,
    InvalidUtf8,
}

impl std::fmt::Display for CryptoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CryptoError::InvalidBase64 => write!(f, "Invalid base64 encoding"),
            CryptoError::InvalidCiphertext => write!(f, "Invalid ciphertext format"),
            CryptoError::DecryptionFailed => write!(f, "Decryption failed"),
            CryptoError::InvalidUtf8 => write!(f, "Decrypted data is not valid UTF-8"),
        }
    }
}

impl std::error::Error for CryptoError {}

/// Derive the database token from a URL key.
///
/// This computes HMAC-SHA256 with the URL key as the HMAC key and "database_identity"
/// as the message, returning the result as a 64-character lowercase hex string.
///
/// PHP equivalent: hash_hmac("SHA256", "database_identity", $urlKey, false)
pub fn get_database_id(url_key: &str) -> String {
    let mut mac =
        Hmac::<Sha256>::new_from_slice(url_key.as_bytes()).expect("HMAC can take key of any size");
    mac.update(b"database_identity");
    hex::encode(mac.finalize().into_bytes())
}

/// Derive the encryption key from a URL key.
///
/// This computes HMAC-SHA256 with the URL key as the HMAC key and "encryption_key"
/// as the message, returning the raw 32 bytes for AES-256.
///
/// PHP equivalent: hash_hmac("SHA256", "encryption_key", $urlKey, true)
fn get_encryption_key(url_key: &str) -> [u8; 32] {
    let mut mac =
        Hmac::<Sha256>::new_from_slice(url_key.as_bytes()).expect("HMAC can take key of any size");
    mac.update(b"encryption_key");
    mac.finalize().into_bytes().into()
}

/// Encrypt plaintext using AES-256-CBC with zero-byte padding.
/// Provides NO authentication.
///
/// Returns base64-encoded string of: IV (16 bytes) || ciphertext
pub fn encrypt(url_key: &str, plaintext: &str) -> String {
    use aes::cipher::{BlockEncrypt, KeyInit};

    let key = get_encryption_key(url_key);

    // Generate random IV
    let mut iv = [0u8; IV_SIZE];
    rand::rngs::OsRng.fill_bytes(&mut iv);

    // Pad plaintext with zeros to block boundary (mcrypt style)
    let plaintext_bytes = plaintext.as_bytes();
    let padded_len = if plaintext_bytes.is_empty() {
        AES_BLOCK_SIZE
    } else {
        ((plaintext_bytes.len() + AES_BLOCK_SIZE - 1) / AES_BLOCK_SIZE) * AES_BLOCK_SIZE
    };
    let mut data = vec![0u8; padded_len];
    data[..plaintext_bytes.len()].copy_from_slice(plaintext_bytes);

    // Encrypt in place using CBC mode manually
    let cipher = aes::Aes256::new((&key).into());
    let mut prev_block = iv;

    for chunk in data.chunks_exact_mut(AES_BLOCK_SIZE) {
        // XOR with previous ciphertext block (or IV for first block)
        for (byte, &prev) in chunk.iter_mut().zip(prev_block.iter()) {
            *byte ^= prev;
        }
        // Encrypt the block
        let block = aes::cipher::generic_array::GenericArray::from_mut_slice(chunk);
        cipher.encrypt_block(block);
        // Save for next iteration
        prev_block.copy_from_slice(chunk);
    }

    // Prepend IV to ciphertext and base64 encode
    let mut output = Vec::with_capacity(IV_SIZE + data.len());
    output.extend_from_slice(&iv);
    output.extend_from_slice(&data);

    BASE64.encode(&output)
}

/// Decrypt base64-encoded ciphertext using AES-256-CBC.
/// Provides NO authentication.
///
/// The input format is: base64(IV || ciphertext)
/// After decryption, trailing null bytes are stripped (mcrypt zero-byte padding).
pub fn decrypt(url_key: &str, encoded: &str) -> Result<String, CryptoError> {
    use aes::cipher::{BlockDecrypt, KeyInit};

    let key = get_encryption_key(url_key);

    // Decode base64
    let data = BASE64
        .decode(encoded)
        .map_err(|_| CryptoError::InvalidBase64)?;

    // Must have at least IV + one block
    if data.len() < IV_SIZE + AES_BLOCK_SIZE {
        return Err(CryptoError::InvalidCiphertext);
    }

    // Split IV and ciphertext
    let (iv, ciphertext) = data.split_at(IV_SIZE);

    // Ciphertext must be a multiple of block size
    if ciphertext.len() % AES_BLOCK_SIZE != 0 {
        return Err(CryptoError::InvalidCiphertext);
    }

    // Decrypt using CBC mode manually
    let cipher = aes::Aes256::new((&key).into());
    let mut decrypted = Vec::with_capacity(ciphertext.len());
    let mut prev_block: &[u8] = iv;

    for chunk in ciphertext.chunks_exact(AES_BLOCK_SIZE) {
        let mut block = *aes::cipher::generic_array::GenericArray::from_slice(chunk);
        cipher.decrypt_block(&mut block);
        // XOR with previous ciphertext block (or IV for first block)
        for (byte, &prev) in block.iter_mut().zip(prev_block.iter()) {
            *byte ^= prev;
        }
        decrypted.extend_from_slice(&block);
        prev_block = chunk;
    }

    // Strip trailing null bytes (mcrypt zero-byte padding)
    let end = decrypted.iter().rposition(|&b| b != 0).map_or(0, |i| i + 1);
    decrypted.truncate(end);

    String::from_utf8(decrypted).map_err(|_| CryptoError::InvalidUtf8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_database_id() {
        // Test with a known URL key
        let url_key = "testkey123";
        let db_id = get_database_id(url_key);

        // Should be 64 hex chars (256 bits)
        assert_eq!(db_id.len(), 64);
        assert!(db_id.chars().all(|c| c.is_ascii_hexdigit()));

        // Same key should produce same result
        assert_eq!(get_database_id(url_key), db_id);

        // Different key should produce different result
        assert_ne!(get_database_id("otherkey"), db_id);
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let url_key = "mysecretkey";
        let plaintext = "Hello, World!";

        let ciphertext = encrypt(url_key, plaintext);
        let decrypted = decrypt(url_key, &ciphertext).unwrap();

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encrypt_decrypt_unicode() {
        let url_key = "unicodekey";
        let plaintext = "Hello 世界! 🎉";

        let ciphertext = encrypt(url_key, plaintext);
        let decrypted = decrypt(url_key, &ciphertext).unwrap();

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encrypt_decrypt_multiline() {
        let url_key = "multilinekey";
        let plaintext = "Line 1\nLine 2\nLine 3";

        let ciphertext = encrypt(url_key, plaintext);
        let decrypted = decrypt(url_key, &ciphertext).unwrap();

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encrypt_decrypt_empty() {
        let url_key = "emptykey";
        let plaintext = "";

        let ciphertext = encrypt(url_key, plaintext);
        let decrypted = decrypt(url_key, &ciphertext).unwrap();

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encrypt_produces_different_ciphertext() {
        // Due to random IV, same plaintext should produce different ciphertext
        let url_key = "randomivkey";
        let plaintext = "Same message";

        let ct1 = encrypt(url_key, plaintext);
        let ct2 = encrypt(url_key, plaintext);

        assert_ne!(ct1, ct2);

        // But both should decrypt to the same plaintext
        assert_eq!(decrypt(url_key, &ct1).unwrap(), plaintext);
        assert_eq!(decrypt(url_key, &ct2).unwrap(), plaintext);
    }

    #[test]
    fn test_wrong_key_fails() {
        let plaintext = "Secret data";
        let ciphertext = encrypt("correctkey", plaintext);

        // Decrypting with wrong key should fail or produce garbage
        let result = decrypt("wrongkey", &ciphertext);
        // Due to zero-padding removal, wrong key might produce garbage UTF-8
        // or fail validation - either way it shouldn't match original
        match result {
            Ok(decrypted) => assert_ne!(decrypted, plaintext),
            Err(_) => {} // Also acceptable
        }
    }

    #[test]
    fn test_invalid_base64() {
        let result = decrypt("anykey", "not valid base64!!!");
        assert!(matches!(result, Err(CryptoError::InvalidBase64)));
    }

    #[test]
    fn test_too_short_ciphertext() {
        // Valid base64 but too short
        let result = decrypt("anykey", "AAAA");
        assert!(matches!(result, Err(CryptoError::InvalidCiphertext)));
    }
}
