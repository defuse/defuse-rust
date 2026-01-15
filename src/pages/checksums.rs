use askama::Template;
use axum::response::IntoResponse;
use bytes::Bytes;
use serde::Deserialize;

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

// CRC
use crc::{Crc, CRC_32_BZIP2, CRC_32_ISCSI, CRC_32_ISO_HDLC};

// DES for LM hash
use des::cipher::{BlockEncrypt, KeyInit};

use crate::app_state::AppState;
use crate::context::PageContext;
use crate::handler::{BoxFuture, PageHandler};

// Order matches PHP's hash_algos() output
const SUPPORTED_ALGORITHMS: &[&str] = &[
    "md5", "LM", "NTLM", "sha1", "sha256", "sha384", "sha512",
    "md5(md5())", "MySQL4.1+", "ripemd160", "whirlpool",
    "adler32", "crc32", "crc32b", "crc32c",
    "fnv1a32", "fnv1a64", "fnv132", "fnv164",
    "gost", "gost-crypto",
    // haval hashes not implemented
    "joaat", "md2", "md4",
    "ripemd128", "ripemd256", "ripemd320",
    "sha3-224", "sha3-256", "sha3-384", "sha3-512",
    "sha224", "sha512/224", "sha512/256",
    // snefru, tiger hashes not implemented
];

pub struct Handler;

impl PageHandler for Handler {
    fn get(&self, ctx: PageContext, _state: &AppState) -> BoxFuture {
        Box::pin(async move {
            ChecksumsPage {
                ctx,
                input: String::new(),
                normalize: false,
                results: Vec::new(),
                supported_algorithms: SUPPORTED_ALGORITHMS,
            }
            .into_response()
        })
    }

    fn post(&self, ctx: PageContext, _state: &AppState, body: Bytes) -> Option<BoxFuture> {
        Some(Box::pin(async move {
            let form: ChecksumsForm =
                serde_urlencoded::from_bytes(&body).expect("Failed to parse checksums form");

            let normalize = form.normalize.as_deref() == Some("yes");

            let data = if normalize {
                form.data.trim().to_string()
            } else {
                form.data.clone()
            };

            let results = compute_hashes(&data);

            ChecksumsPage {
                ctx,
                input: form.data,
                normalize,
                results,
                supported_algorithms: SUPPORTED_ALGORITHMS,
            }
            .into_response()
        }))
    }
}

#[derive(Template)]
#[template(path = "pages/checksums.html")]
struct ChecksumsPage {
    ctx: PageContext,
    input: String,
    normalize: bool,
    results: Vec<HashResult>,
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

// =============================================================================
// Hash computation
// =============================================================================

fn compute_hashes(input: &str) -> Vec<HashResult> {
    let bytes = input.as_bytes();

    vec![
        // Checksums
        HashResult {
            algorithm: "adler32",
            hash: format!("{:08x}", adler32(bytes)),
        },
        HashResult {
            algorithm: "crc32",
            hash: format!("{:08x}", crc32(bytes)),
        },
        HashResult {
            algorithm: "crc32b",
            hash: format!("{:08x}", crc32b(bytes)),
        },
        HashResult {
            algorithm: "crc32c",
            hash: format!("{:08x}", crc32c(bytes)),
        },
        // FNV hashes
        HashResult {
            algorithm: "fnv132",
            hash: format!("{:08x}", fnv1_32(bytes)),
        },
        HashResult {
            algorithm: "fnv164",
            hash: format!("{:016x}", fnv1_64(bytes)),
        },
        HashResult {
            algorithm: "fnv1a32",
            hash: format!("{:08x}", fnv1a_32(bytes)),
        },
        HashResult {
            algorithm: "fnv1a64",
            hash: format!("{:016x}", fnv1a_64(bytes)),
        },
        // GOST
        HashResult {
            algorithm: "gost",
            hash: hex::encode(Gost94Test::digest(bytes)),
        },
        HashResult {
            algorithm: "gost-crypto",
            hash: hex::encode(Gost94CryptoPro::digest(bytes)),
        },
        // joaat
        HashResult {
            algorithm: "joaat",
            hash: format!("{:08x}", joaat(bytes)),
        },
        // LM
        HashResult {
            algorithm: "LM",
            hash: lm_hash(input),
        },
        // MD family
        HashResult {
            algorithm: "md2",
            hash: hex::encode(Md2::digest(bytes)),
        },
        HashResult {
            algorithm: "md4",
            hash: hex::encode(Md4::digest(bytes)),
        },
        HashResult {
            algorithm: "md5",
            hash: hex::encode(Md5::digest(bytes)),
        },
        HashResult {
            algorithm: "md5(md5())",
            hash: hex::encode(Md5::digest(hex::encode(Md5::digest(bytes)).as_bytes())),
        },
        // MySQL4.1+
        HashResult {
            algorithm: "MySQL4.1+",
            hash: mysql41_hash(bytes),
        },
        // NTLM
        HashResult {
            algorithm: "NTLM",
            hash: ntlm_hash(input),
        },
        // RIPEMD family
        HashResult {
            algorithm: "ripemd128",
            hash: hex::encode(Ripemd128::digest(bytes)),
        },
        HashResult {
            algorithm: "ripemd160",
            hash: hex::encode(Ripemd160::digest(bytes)),
        },
        HashResult {
            algorithm: "ripemd256",
            hash: hex::encode(Ripemd256::digest(bytes)),
        },
        HashResult {
            algorithm: "ripemd320",
            hash: hex::encode(Ripemd320::digest(bytes)),
        },
        // SHA-1
        HashResult {
            algorithm: "sha1",
            hash: hex::encode(Sha1::digest(bytes)),
        },
        // SHA-2 family
        HashResult {
            algorithm: "sha224",
            hash: hex::encode(Sha224::digest(bytes)),
        },
        HashResult {
            algorithm: "sha256",
            hash: hex::encode(Sha256::digest(bytes)),
        },
        // SHA-3 family
        HashResult {
            algorithm: "sha3-224",
            hash: hex::encode(Sha3_224::digest(bytes)),
        },
        HashResult {
            algorithm: "sha3-256",
            hash: hex::encode(Sha3_256::digest(bytes)),
        },
        HashResult {
            algorithm: "sha3-384",
            hash: hex::encode(Sha3_384::digest(bytes)),
        },
        HashResult {
            algorithm: "sha3-512",
            hash: hex::encode(Sha3_512::digest(bytes)),
        },
        // More SHA-2
        HashResult {
            algorithm: "sha384",
            hash: hex::encode(Sha384::digest(bytes)),
        },
        HashResult {
            algorithm: "sha512",
            hash: hex::encode(Sha512::digest(bytes)),
        },
        HashResult {
            algorithm: "sha512/224",
            hash: hex::encode(Sha512_224::digest(bytes)),
        },
        HashResult {
            algorithm: "sha512/256",
            hash: hex::encode(Sha512_256::digest(bytes)),
        },
        // Whirlpool
        HashResult {
            algorithm: "whirlpool",
            hash: hex::encode(Whirlpool::digest(bytes)),
        },
    ]
}

// =============================================================================
// Checksum implementations
// =============================================================================

/// Adler-32 checksum
fn adler32(data: &[u8]) -> u32 {
    adler::adler32_slice(data)
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
fn lm_hash(password: &str) -> String {
    const LM_MAGIC: &[u8; 8] = b"KGS!@#$%";

    // Uppercase and convert to bytes, truncate or pad to 14 bytes
    let upper = password.to_uppercase();
    let bytes = upper.as_bytes();
    let mut padded = [0u8; 14];
    let len = std::cmp::min(bytes.len(), 14);
    padded[..len].copy_from_slice(&bytes[..len]);

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
fn ntlm_hash(password: &str) -> String {
    // Convert to UTF-16LE
    let utf16: Vec<u8> = password
        .encode_utf16()
        .flat_map(|c| c.to_le_bytes())
        .collect();

    hex::encode(Md4::digest(&utf16))
}
