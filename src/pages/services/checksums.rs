//! WARNING: This source file contains AI-generated implementations of hash
//! functions which HAVE NOT BEEN REVIEWED FOR SECURITY.

use askama::Template;
use axum::response::IntoResponse;
use serde::Deserialize;

// Maximum file size: 5MB (matching PHP)
const MAX_FILE_SIZE: usize = 5 * 1024 * 1024;

// RustCrypto hashes - all use the Digest trait
use md2::Md2;
use md4::Md4;
use md5::Md5;
use sha1::Sha1;
use sha2::{Digest, Sha224, Sha256, Sha384, Sha512, Sha512_224, Sha512_256};
use sha3::{Sha3_224, Sha3_256, Sha3_384, Sha3_512};
use ripemd::{Ripemd128, Ripemd160, Ripemd256, Ripemd320};
use whirlpool::Whirlpool;
use gost94::{Gost94Test, Gost94CryptoPro};

// Custom hash implementations
use crate::libs::hashes::{
    snefru::snefru256,
    tiger::{tiger128_3, tiger128_4, tiger160_3, tiger160_4, tiger192_3, tiger192_4},
    haval::{
        haval128_3, haval128_4, haval128_5,
        haval160_3, haval160_4, haval160_5,
        haval192_3, haval192_4, haval192_5,
        haval224_3, haval224_4, haval224_5,
        haval256_3, haval256_4, haval256_5,
    },
};

// CRC
use crc::{Crc, CRC_32_BZIP2, CRC_32_ISCSI, CRC_32_ISO_HDLC};

// DES for LM hash
use des::cipher::{BlockEncrypt, KeyInit};

use crate::app_state::AppState;
use crate::context::PageContext;
use crate::handler::{BoxFuture, FormField, PageHandler, PostBody};

// Order matches defuse.ca exactly (57 algorithms)
const SUPPORTED_ALGORITHMS: &[&str] = &[
    "md5", "LM", "NTLM", "sha1", "sha256", "sha384", "sha512",
    "md5(md5())", "MySQL4.1+", "ripemd160", "whirlpool",
    "adler32", "crc32", "crc32b", "crc32c",
    "fnv1a32", "fnv1a64", "fnv132", "fnv164",
    "gost", "gost-crypto",
    "haval128,3", "haval128,4", "haval128,5",
    "haval160,3", "haval160,4", "haval160,5",
    "haval192,3", "haval192,4", "haval192,5",
    "haval224,3", "haval224,4", "haval224,5",
    "haval256,3", "haval256,4", "haval256,5",
    "joaat", "md2", "md4",
    "ripemd128", "ripemd256", "ripemd320",
    "sha3-224", "sha3-256", "sha3-384", "sha3-512",
    "sha224", "sha512/224", "sha512/256",
    "snefru", "snefru256",
    "tiger128,3", "tiger128,4",
    "tiger160,3", "tiger160,4",
    "tiger192,3", "tiger192,4",
];

#[derive(Template)]
#[template(path = "pages/services/checksums.html")]
struct ChecksumsPage {
    ctx: PageContext,
    input: String,
    normalize: bool,
    results: Vec<HashResult>,
    error: Option<String>,
    supported_algorithms: &'static [&'static str],
}

pub struct HashResult {
    pub algorithm: &'static str,
    pub hash: String,
}

#[derive(Deserialize, Default)]
struct ChecksumsForm {
    #[serde(default)]
    data: String,
    #[serde(default)]
    normalize: Option<String>,
}

pub struct Handler;

impl PageHandler for Handler {
    fn get(&self, ctx: PageContext, _state: &AppState) -> BoxFuture {
        Box::pin(async move {
            ChecksumsPage {
                ctx,
                input: String::new(),
                normalize: false,
                results: Vec::new(),
                error: None,
                supported_algorithms: SUPPORTED_ALGORITHMS,
            }
            .into_response()
        })
    }

    fn post(&self, ctx: PageContext, _state: &AppState, body: PostBody) -> Option<BoxFuture> {
        Some(Box::pin(async move {
            match body {
                PostBody::UrlEncoded(bytes) => {
                    // Regular form-urlencoded POST
                    let form: ChecksumsForm =
                        serde_urlencoded::from_bytes(&bytes).unwrap_or_default();

                    let normalize = form.normalize.as_deref() == Some("yes");

                    // PHP removes ALL \r and \n characters, not just leading/trailing whitespace
                    let data = if normalize {
                        form.data.replace('\r', "").replace('\n', "")
                    } else {
                        form.data.clone()
                    };

                    // Run CPU-intensive hashing in blocking thread to not block async runtime
                    let results = tokio::task::spawn_blocking(move || compute_hashes(&data))
                        .await
                        .unwrap_or_default();

                    ChecksumsPage {
                        ctx,
                        input: form.data,
                        normalize,
                        results,
                        error: None,
                        supported_algorithms: SUPPORTED_ALGORITHMS,
                    }
                    .into_response()
                }
                PostBody::Multipart { fields } => {
                    handle_multipart_post(ctx, fields).await
                }
            }
        }))
    }
}

/// Handle multipart form data (file upload)
async fn handle_multipart_post(ctx: PageContext, fields: Vec<FormField>) -> axum::response::Response {
    // Find the file field
    let file_field = fields.iter().find(|f| f.name == "filetohash");

    let file_bytes = match file_field {
        Some(field) if field.filename.is_some() => field.data.clone(),
        _ => {
            // No file uploaded, return empty results
            return ChecksumsPage {
                ctx,
                input: String::new(),
                normalize: false,
                results: Vec::new(),
                error: None,
                supported_algorithms: SUPPORTED_ALGORITHMS,
            }
            .into_response();
        }
    };

    // Check file size limit
    if file_bytes.len() > MAX_FILE_SIZE {
        return ChecksumsPage {
            ctx,
            input: String::new(),
            normalize: false,
            results: Vec::new(),
            error: Some("File is too big. Max: 5MB.".to_string()),
            supported_algorithms: SUPPORTED_ALGORITHMS,
        }
        .into_response();
    }

    // Hash the file - run in blocking thread to not block async runtime
    let results = tokio::task::spawn_blocking(move || compute_hashes_bytes(&file_bytes))
        .await
        .unwrap_or_default();

    ChecksumsPage {
        ctx,
        input: String::new(),
        normalize: false,
        results,
        error: None,
        supported_algorithms: SUPPORTED_ALGORITHMS,
    }
    .into_response()
}


// =============================================================================
// Hash computation
// =============================================================================

fn compute_hashes(input: &str) -> Vec<HashResult> {
    compute_hashes_bytes(input.as_bytes())
}

fn compute_hashes_bytes(bytes: &[u8]) -> Vec<HashResult> {
    let snefru_hash = hex::encode(snefru256(bytes));

    // Order matches defuse.ca exactly (57 algorithms)
    vec![
        HashResult { algorithm: "md5", hash: hex::encode(Md5::digest(bytes)) },
        HashResult { algorithm: "LM", hash: lm_hash(bytes) },
        HashResult { algorithm: "NTLM", hash: ntlm_hash(bytes) },
        HashResult { algorithm: "sha1", hash: hex::encode(Sha1::digest(bytes)) },
        HashResult { algorithm: "sha256", hash: hex::encode(Sha256::digest(bytes)) },
        HashResult { algorithm: "sha384", hash: hex::encode(Sha384::digest(bytes)) },
        HashResult { algorithm: "sha512", hash: hex::encode(Sha512::digest(bytes)) },
        HashResult { algorithm: "md5(md5())", hash: hex::encode(Md5::digest(hex::encode(Md5::digest(bytes)).as_bytes())) },
        HashResult { algorithm: "MySQL4.1+", hash: mysql41_hash(bytes) },
        HashResult { algorithm: "ripemd160", hash: hex::encode(Ripemd160::digest(bytes)) },
        HashResult { algorithm: "whirlpool", hash: hex::encode(Whirlpool::digest(bytes)) },
        HashResult { algorithm: "adler32", hash: format!("{:08x}", adler32(bytes)) },
        HashResult { algorithm: "crc32", hash: format!("{:08x}", crc32(bytes)) },
        HashResult { algorithm: "crc32b", hash: format!("{:08x}", crc32b(bytes)) },
        HashResult { algorithm: "crc32c", hash: format!("{:08x}", crc32c(bytes)) },
        HashResult { algorithm: "fnv1a32", hash: format!("{:08x}", fnv1a_32(bytes)) },
        HashResult { algorithm: "fnv1a64", hash: format!("{:016x}", fnv1a_64(bytes)) },
        HashResult { algorithm: "fnv132", hash: format!("{:08x}", fnv1_32(bytes)) },
        HashResult { algorithm: "fnv164", hash: format!("{:016x}", fnv1_64(bytes)) },
        HashResult { algorithm: "gost", hash: hex::encode(Gost94Test::digest(bytes)) },
        HashResult { algorithm: "gost-crypto", hash: hex::encode(Gost94CryptoPro::digest(bytes)) },
        HashResult { algorithm: "haval128,3", hash: hex::encode(haval128_3(bytes)) },
        HashResult { algorithm: "haval128,4", hash: hex::encode(haval128_4(bytes)) },
        HashResult { algorithm: "haval128,5", hash: hex::encode(haval128_5(bytes)) },
        HashResult { algorithm: "haval160,3", hash: hex::encode(haval160_3(bytes)) },
        HashResult { algorithm: "haval160,4", hash: hex::encode(haval160_4(bytes)) },
        HashResult { algorithm: "haval160,5", hash: hex::encode(haval160_5(bytes)) },
        HashResult { algorithm: "haval192,3", hash: hex::encode(haval192_3(bytes)) },
        HashResult { algorithm: "haval192,4", hash: hex::encode(haval192_4(bytes)) },
        HashResult { algorithm: "haval192,5", hash: hex::encode(haval192_5(bytes)) },
        HashResult { algorithm: "haval224,3", hash: hex::encode(haval224_3(bytes)) },
        HashResult { algorithm: "haval224,4", hash: hex::encode(haval224_4(bytes)) },
        HashResult { algorithm: "haval224,5", hash: hex::encode(haval224_5(bytes)) },
        HashResult { algorithm: "haval256,3", hash: hex::encode(haval256_3(bytes)) },
        HashResult { algorithm: "haval256,4", hash: hex::encode(haval256_4(bytes)) },
        HashResult { algorithm: "haval256,5", hash: hex::encode(haval256_5(bytes)) },
        HashResult { algorithm: "joaat", hash: format!("{:08x}", joaat(bytes)) },
        HashResult { algorithm: "md2", hash: hex::encode(Md2::digest(bytes)) },
        HashResult { algorithm: "md4", hash: hex::encode(Md4::digest(bytes)) },
        HashResult { algorithm: "ripemd128", hash: hex::encode(Ripemd128::digest(bytes)) },
        HashResult { algorithm: "ripemd256", hash: hex::encode(Ripemd256::digest(bytes)) },
        HashResult { algorithm: "ripemd320", hash: hex::encode(Ripemd320::digest(bytes)) },
        HashResult { algorithm: "sha3-224", hash: hex::encode(Sha3_224::digest(bytes)) },
        HashResult { algorithm: "sha3-256", hash: hex::encode(Sha3_256::digest(bytes)) },
        HashResult { algorithm: "sha3-384", hash: hex::encode(Sha3_384::digest(bytes)) },
        HashResult { algorithm: "sha3-512", hash: hex::encode(Sha3_512::digest(bytes)) },
        HashResult { algorithm: "sha224", hash: hex::encode(Sha224::digest(bytes)) },
        HashResult { algorithm: "sha512/224", hash: hex::encode(Sha512_224::digest(bytes)) },
        HashResult { algorithm: "sha512/256", hash: hex::encode(Sha512_256::digest(bytes)) },
        HashResult { algorithm: "snefru", hash: snefru_hash.clone() },
        HashResult { algorithm: "snefru256", hash: snefru_hash },
        HashResult { algorithm: "tiger128,3", hash: hex::encode(tiger128_3(bytes)) },
        HashResult { algorithm: "tiger128,4", hash: hex::encode(tiger128_4(bytes)) },
        HashResult { algorithm: "tiger160,3", hash: hex::encode(tiger160_3(bytes)) },
        HashResult { algorithm: "tiger160,4", hash: hex::encode(tiger160_4(bytes)) },
        HashResult { algorithm: "tiger192,3", hash: hex::encode(tiger192_3(bytes)) },
        HashResult { algorithm: "tiger192,4", hash: hex::encode(tiger192_4(bytes)) },
    ]
}

// =============================================================================
// Checksum implementations
// =============================================================================

/// Adler-32 checksum
fn adler32(data: &[u8]) -> u32 {
    adler2::adler32_slice(data)
}

/// CRC-32 - PHP's hash('crc32') uses CRC-32/BZIP2 (non-reflected), output in big-endian
fn crc32(data: &[u8]) -> u32 {
    let crc = Crc::<u32>::new(&CRC_32_BZIP2);
    crc.checksum(data).swap_bytes()
}

/// CRC-32B - PHP's hash('crc32b') uses CRC-32/ISO-HDLC (reflected, same as zlib)
fn crc32b(data: &[u8]) -> u32 {
    let crc = Crc::<u32>::new(&CRC_32_ISO_HDLC);
    crc.checksum(data)
}

/// CRC-32C - PHP's hash('crc32c') uses CRC-32/ISCSI (Castagnoli polynomial)
fn crc32c(data: &[u8]) -> u32 {
    let crc = Crc::<u32>::new(&CRC_32_ISCSI);
    crc.checksum(data)
}

// =============================================================================
// FNV hash implementations
// =============================================================================

const FNV1_32_INIT: u32 = 0x811c9dc5;
const FNV1_32_PRIME: u32 = 0x01000193;
const FNV1_64_INIT: u64 = 0xcbf29ce484222325;
const FNV1_64_PRIME: u64 = 0x00000100000001B3;

/// FNV-1 32-bit hash
fn fnv1_32(data: &[u8]) -> u32 {
    let mut hash = FNV1_32_INIT;
    for &byte in data {
        hash = hash.wrapping_mul(FNV1_32_PRIME);
        hash ^= byte as u32;
    }
    hash
}

/// FNV-1 64-bit hash
fn fnv1_64(data: &[u8]) -> u64 {
    let mut hash = FNV1_64_INIT;
    for &byte in data {
        hash = hash.wrapping_mul(FNV1_64_PRIME);
        hash ^= byte as u64;
    }
    hash
}

/// FNV-1a 32-bit hash (XOR before multiply)
fn fnv1a_32(data: &[u8]) -> u32 {
    let mut hash = FNV1_32_INIT;
    for &byte in data {
        hash ^= byte as u32;
        hash = hash.wrapping_mul(FNV1_32_PRIME);
    }
    hash
}

/// FNV-1a 64-bit hash (XOR before multiply)
fn fnv1a_64(data: &[u8]) -> u64 {
    let mut hash = FNV1_64_INIT;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(FNV1_64_PRIME);
    }
    hash
}

// =============================================================================
// Jenkins One-at-a-time hash
// =============================================================================

/// Jenkins One-at-a-time hash
fn joaat(data: &[u8]) -> u32 {
    let mut hash: u32 = 0;
    for &byte in data {
        hash = hash.wrapping_add(byte as u32);
        hash = hash.wrapping_add(hash << 10);
        hash ^= hash >> 6;
    }
    hash = hash.wrapping_add(hash << 3);
    hash ^= hash >> 11;
    hash = hash.wrapping_add(hash << 15);
    hash
}

// =============================================================================
// LM Hash (LAN Manager)
// =============================================================================

/// LM Hash - the legacy Windows password hash
/// Takes a password, uppercases it, pads/truncates to 14 bytes,
/// splits into two 7-byte halves, and DES-encrypts a magic string with each.
fn lm_hash(password: &[u8]) -> String {
    const LM_MAGIC: &[u8; 8] = b"KGS!@#$%";

    // Uppercase ASCII letters only (matching PHP's strtoupper on binary data)
    // Non-ASCII bytes (0x80-0xFF) pass through unchanged
    let upper: Vec<u8> = password.iter().map(|&b| b.to_ascii_uppercase()).collect();
    let mut padded = [0u8; 14];
    let len = std::cmp::min(upper.len(), 14);
    padded[..len].copy_from_slice(&upper[..len]);

    // Split into two 7-byte halves
    let (first_half, second_half) = padded.split_at(7);

    // Convert 7 bytes to 8-byte DES key (with parity bits)
    let key1 = seven_to_eight_bytes(first_half);
    let key2 = seven_to_eight_bytes(second_half);

    // DES encrypt the magic string with each key
    let cipher1 = des::Des::new_from_slice(&key1).unwrap();
    let cipher2 = des::Des::new_from_slice(&key2).unwrap();

    let mut block1 = *LM_MAGIC;
    let mut block2 = *LM_MAGIC;

    cipher1.encrypt_block((&mut block1).into());
    cipher2.encrypt_block((&mut block2).into());

    // Concatenate the two encrypted blocks
    let mut result = [0u8; 16];
    result[..8].copy_from_slice(&block1);
    result[8..].copy_from_slice(&block2);

    hex::encode(result)
}

/// Convert 7 bytes to 8-byte DES key by inserting parity bits
fn seven_to_eight_bytes(seven: &[u8]) -> [u8; 8] {
    let mut eight = [0u8; 8];
    eight[0] = seven[0] >> 1;
    eight[1] = ((seven[0] & 0x01) << 6) | (seven[1] >> 2);
    eight[2] = ((seven[1] & 0x03) << 5) | (seven[2] >> 3);
    eight[3] = ((seven[2] & 0x07) << 4) | (seven[3] >> 4);
    eight[4] = ((seven[3] & 0x0F) << 3) | (seven[4] >> 5);
    eight[5] = ((seven[4] & 0x1F) << 2) | (seven[5] >> 6);
    eight[6] = ((seven[5] & 0x3F) << 1) | (seven[6] >> 7);
    eight[7] = seven[6] & 0x7F;

    // Set odd parity on each byte
    for byte in &mut eight {
        *byte = (*byte << 1) | (1 - (*byte).count_ones() as u8 % 2);
    }

    eight
}

// =============================================================================
// MySQL 4.1+ password hash
// =============================================================================

/// MySQL 4.1+ password hash: SHA1(SHA1(password))
fn mysql41_hash(data: &[u8]) -> String {
    let first = Sha1::digest(data);
    let second = Sha1::digest(&first);
    hex::encode(second)
}

// =============================================================================
// NTLM Hash
// =============================================================================

/// NTLM Hash: MD4 of the password encoded as UTF-16LE
/// PHP uses iconv('UTF-8', 'UTF-16LE', $password) which fails silently on
/// invalid UTF-8, producing an empty string. We match this behavior.
fn ntlm_hash(password: &[u8]) -> String {
    // NTLM requires valid text - decode as UTF-8, then convert to UTF-16LE
    // This matches Windows behavior where passwords are always text (UTF-16LE native)
    match std::str::from_utf8(password) {
        Ok(s) => {
            let utf16: Vec<u8> = s.encode_utf16().flat_map(|c| c.to_le_bytes()).collect();
            hex::encode(Md4::digest(&utf16))
        }
        Err(_) => "[error: NTLM requires valid text]".to_string(),
    }
}
