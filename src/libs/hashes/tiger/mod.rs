//! Tiger hash algorithm implementation.
//! THIS IMPLEMENTATION HAS NOT BEEN AUDITED! DO NOT RELY ON IT FOR SECURITY!
//!
//! Tiger is a cryptographic hash function designed by Ross Anderson and Eli Biham
//! in 1995. It produces a 192-bit hash value and was designed for 64-bit platforms.
//!
//! This module supports:
//! - Tiger with 3 passes (standard): tiger192,3, tiger160,3, tiger128,3
//! - Tiger with 4 passes: tiger192,4, tiger160,4, tiger128,4
//!
//! The output lengths (128, 160, 192) are truncations of the full 192-bit hash.
//!
//! This implementation matches PHP's hash() function output exactly.
//!
//! Note: Tiger is considered weak by modern standards and should not be used
//! for security-critical applications.

mod tiger4;

// Re-export the 192-bit implementations
pub use tiger4::{tiger192_3, tiger192_4};

/// Tiger-160 with 3 passes.
/// Returns first 160 bits (20 bytes) of Tiger-192.
pub fn tiger160_3(data: &[u8]) -> [u8; 20] {
    let full = tiger192_3(data);
    let mut output = [0u8; 20];
    output.copy_from_slice(&full[..20]);
    output
}

/// Tiger-128 with 3 passes.
/// Returns first 128 bits (16 bytes) of Tiger-192.
pub fn tiger128_3(data: &[u8]) -> [u8; 16] {
    let full = tiger192_3(data);
    let mut output = [0u8; 16];
    output.copy_from_slice(&full[..16]);
    output
}

/// Tiger-160 with 4 passes.
/// Returns first 160 bits (20 bytes) of Tiger-192.
pub fn tiger160_4(data: &[u8]) -> [u8; 20] {
    let full = tiger192_4(data);
    let mut output = [0u8; 20];
    output.copy_from_slice(&full[..20]);
    output
}

/// Tiger-128 with 4 passes.
/// Returns first 128 bits (16 bytes) of Tiger-192.
pub fn tiger128_4(data: &[u8]) -> [u8; 16] {
    let full = tiger192_4(data);
    let mut output = [0u8; 16];
    output.copy_from_slice(&full[..16]);
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    // PHP test vectors generated with php -r 'echo hash(...);'
    // All vectors verified against PHP 8.x

    // Tiger-192,3 tests
    #[test]
    fn test_tiger192_3_empty() {
        let result = tiger192_3(b"");
        let expected = hex::decode("3293ac630c13f0245f92bbb1766e16167a4e58492dde73f3").unwrap();
        assert_eq!(&result[..], &expected[..]);
    }

    #[test]
    fn test_tiger192_3_test() {
        let result = tiger192_3(b"test");
        let expected = hex::decode("7ab383fc29d81f8d0d68e87c69bae5f1f18266d730c48b1d").unwrap();
        assert_eq!(&result[..], &expected[..]);
    }

    #[test]
    fn test_tiger192_3_abc() {
        let result = tiger192_3(b"abc");
        let expected = hex::decode("2aab1484e8c158f2bfb8c5ff41b57a525129131c957b5f93").unwrap();
        assert_eq!(&result[..], &expected[..]);
    }

    #[test]
    fn test_tiger192_3_quick_brown_fox() {
        let result = tiger192_3(b"The quick brown fox jumps over the lazy dog");
        let expected = hex::decode("6d12a41e72e644f017b6f0e2f7b44c6285f06dd5d2c5b075").unwrap();
        assert_eq!(&result[..], &expected[..]);
    }

    // Tiger-160,3 tests (truncated)
    #[test]
    fn test_tiger160_3_empty() {
        let result = tiger160_3(b"");
        let expected = hex::decode("3293ac630c13f0245f92bbb1766e16167a4e5849").unwrap();
        assert_eq!(&result[..], &expected[..]);
    }

    // Tiger-128,3 tests (truncated)
    #[test]
    fn test_tiger128_3_empty() {
        let result = tiger128_3(b"");
        let expected = hex::decode("3293ac630c13f0245f92bbb1766e1616").unwrap();
        assert_eq!(&result[..], &expected[..]);
    }

    // Tiger-192,4 tests
    #[test]
    fn test_tiger192_4_empty() {
        let result = tiger192_4(b"");
        let expected = hex::decode("24cc78a7f6ff3546e7984e59695ca13d804e0b686e255194").unwrap();
        assert_eq!(&result[..], &expected[..]);
    }

    #[test]
    fn test_tiger192_4_test() {
        let result = tiger192_4(b"test");
        let expected = hex::decode("14b5375c7b29cbf5f9e70a199a40e59dd4d5f1df218b5249").unwrap();
        assert_eq!(&result[..], &expected[..]);
    }

    #[test]
    fn test_tiger192_4_abc() {
        let result = tiger192_4(b"abc");
        let expected = hex::decode("538883c8fc5f28250299018e66bdf4fdb5ef7b65f2e91753").unwrap();
        assert_eq!(&result[..], &expected[..]);
    }

    #[test]
    fn test_tiger192_4_quick_brown_fox() {
        let result = tiger192_4(b"The quick brown fox jumps over the lazy dog");
        let expected = hex::decode("c1f3a704e9f6267e9f75fa47191f83c354100a04c4f1dc6f").unwrap();
        assert_eq!(&result[..], &expected[..]);
    }

    // Tiger-160,4 tests (truncated)
    #[test]
    fn test_tiger160_4_empty() {
        let result = tiger160_4(b"");
        let expected = hex::decode("24cc78a7f6ff3546e7984e59695ca13d804e0b68").unwrap();
        assert_eq!(&result[..], &expected[..]);
    }

    // Tiger-128,4 tests (truncated)
    #[test]
    fn test_tiger128_4_empty() {
        let result = tiger128_4(b"");
        let expected = hex::decode("24cc78a7f6ff3546e7984e59695ca13d").unwrap();
        assert_eq!(&result[..], &expected[..]);
    }

    // Multi-block tests (Tiger-3)
    #[test]
    fn test_tiger192_3_63_bytes() {
        let data = vec![b'a'; 63];
        let result = tiger192_3(&data);
        let expected = hex::decode("9366604ea109e48ed763caabb2d5633b4946eb295ef5781a").unwrap();
        assert_eq!(&result[..], &expected[..]);
    }

    #[test]
    fn test_tiger192_3_64_bytes() {
        let data = vec![b'a'; 64];
        let result = tiger192_3(&data);
        let expected = hex::decode("7503f313bbea92eddca90c5d3fcc4368237457df366fb76e").unwrap();
        assert_eq!(&result[..], &expected[..]);
    }

    #[test]
    fn test_tiger192_3_65_bytes() {
        let data = vec![b'a'; 65];
        let result = tiger192_3(&data);
        let expected = hex::decode("cbda40c307784ada92118d491e32b87bbb8ddc8b4f465682").unwrap();
        assert_eq!(&result[..], &expected[..]);
    }

    #[test]
    fn test_tiger192_3_1000_bytes() {
        let data = vec![b'a'; 1000];
        let result = tiger192_3(&data);
        let expected = hex::decode("42c18814a47b257c40160a80fbe604d949613ee029b31fd9").unwrap();
        assert_eq!(&result[..], &expected[..]);
    }

    // Multi-block tests (Tiger-4)
    #[test]
    fn test_tiger192_4_63_bytes() {
        let data = vec![b'a'; 63];
        let result = tiger192_4(&data);
        let expected = hex::decode("fe897ca63f7389d73c025b32f4bdce503a48d310a20f7211").unwrap();
        assert_eq!(&result[..], &expected[..]);
    }

    #[test]
    fn test_tiger192_4_1000_bytes() {
        let data = vec![b'a'; 1000];
        let result = tiger192_4(&data);
        let expected = hex::decode("63533e5d476a781949e58b25e67bb182d556a52241f6c3e4").unwrap();
        assert_eq!(&result[..], &expected[..]);
    }

    // === Signedness tests (0xFF bytes stress u64 arithmetic) ===

    #[test]
    fn test_tiger192_3_32_0xff() {
        let result = tiger192_3(&vec![0xFFu8; 32]);
        let expected = hex::decode("486ddd22a8ae20b9fa10ba43cc0e0f185fd8ba287142c919").unwrap();
        assert_eq!(&result[..], &expected[..]);
    }

    #[test]
    fn test_tiger192_4_32_0xff() {
        let result = tiger192_4(&vec![0xFFu8; 32]);
        let expected = hex::decode("ab624b399387c292d2fbd416d67091ed299783f2f16525f2").unwrap();
        assert_eq!(&result[..], &expected[..]);
    }

    #[test]
    fn test_tiger192_3_64_0xff() {
        let result = tiger192_3(&vec![0xFFu8; 64]);
        let expected = hex::decode("19622a5aebe86f646b8e06d33a120a368bb7381a60371c8d").unwrap();
        assert_eq!(&result[..], &expected[..]);
    }

    #[test]
    fn test_tiger192_4_64_0xff() {
        let result = tiger192_4(&vec![0xFFu8; 64]);
        let expected = hex::decode("abed1a9c3e17b5b1b03488ab0c4d8ae82953828b8c504991").unwrap();
        assert_eq!(&result[..], &expected[..]);
    }

    #[test]
    fn test_tiger192_3_128_0xff() {
        let result = tiger192_3(&vec![0xFFu8; 128]);
        let expected = hex::decode("53cf15e44ae146c1c4be3155664be98057159304c3354b81").unwrap();
        assert_eq!(&result[..], &expected[..]);
    }

    #[test]
    fn test_tiger192_4_128_0xff() {
        let result = tiger192_4(&vec![0xFFu8; 128]);
        let expected = hex::decode("cbca6a5a7873edc33b953b65158e2f73900f6435c34b051f").unwrap();
        assert_eq!(&result[..], &expected[..]);
    }

    // === Single byte edge cases ===

    #[test]
    fn test_tiger192_3_single_0x00() {
        let result = tiger192_3(&[0x00]);
        let expected = hex::decode("5d9ed00a030e638bdb753a6a24fb900e5a63b8e73e6c25b6").unwrap();
        assert_eq!(&result[..], &expected[..]);
    }

    #[test]
    fn test_tiger192_4_single_0x00() {
        let result = tiger192_4(&[0x00]);
        let expected = hex::decode("24d29ffa7cfaa3fc2ee3136c79d936b5ea4360d7597a2313").unwrap();
        assert_eq!(&result[..], &expected[..]);
    }

    #[test]
    fn test_tiger192_3_single_0x80() {
        let result = tiger192_3(&[0x80]);
        let expected = hex::decode("d82dbb57383c5914b83d782ab8ec094ce6bfa417350d985e").unwrap();
        assert_eq!(&result[..], &expected[..]);
    }

    #[test]
    fn test_tiger192_4_single_0x80() {
        let result = tiger192_4(&[0x80]);
        let expected = hex::decode("054ee66b2733509f623786acaebb0a64549c36dd3f3226d2").unwrap();
        assert_eq!(&result[..], &expected[..]);
    }

    #[test]
    fn test_tiger192_3_single_0xff() {
        let result = tiger192_3(&[0xFF]);
        let expected = hex::decode("ebace53f62b69672952d5dc7858dc79f83466a4b06acd4c8").unwrap();
        assert_eq!(&result[..], &expected[..]);
    }

    #[test]
    fn test_tiger192_4_single_0xff() {
        let result = tiger192_4(&[0xFF]);
        let expected = hex::decode("4c40c1d6f0cc43ee75516fb800be0363a07e3b3c383a86d8").unwrap();
        assert_eq!(&result[..], &expected[..]);
    }

    // === Alternating bit patterns ===

    #[test]
    fn test_tiger192_3_alternating_0x55() {
        let result = tiger192_3(&vec![0x55u8; 64]);
        let expected = hex::decode("e129338017ed2b0b33bec64295cb5f6a8b0c6bd695b252f9").unwrap();
        assert_eq!(&result[..], &expected[..]);
    }

    #[test]
    fn test_tiger192_3_alternating_0xaa() {
        let result = tiger192_3(&vec![0xAAu8; 64]);
        let expected = hex::decode("e6b401a93be789143d8e74ce96f7d4db3ed655e17a7cc442").unwrap();
        assert_eq!(&result[..], &expected[..]);
    }

    // === Truncated variant tests with binary data ===

    #[test]
    fn test_tiger128_3_64_0xff() {
        let result = tiger128_3(&vec![0xFFu8; 64]);
        let expected = hex::decode("19622a5aebe86f646b8e06d33a120a36").unwrap();
        assert_eq!(&result[..], &expected[..]);
    }

    #[test]
    fn test_tiger160_3_64_0xff() {
        let result = tiger160_3(&vec![0xFFu8; 64]);
        let expected = hex::decode("19622a5aebe86f646b8e06d33a120a368bb7381a").unwrap();
        assert_eq!(&result[..], &expected[..]);
    }

    #[test]
    fn test_tiger128_4_64_0xff() {
        let result = tiger128_4(&vec![0xFFu8; 64]);
        let expected = hex::decode("abed1a9c3e17b5b1b03488ab0c4d8ae8").unwrap();
        assert_eq!(&result[..], &expected[..]);
    }

    #[test]
    fn test_tiger160_4_64_0xff() {
        let result = tiger160_4(&vec![0xFFu8; 64]);
        let expected = hex::decode("abed1a9c3e17b5b1b03488ab0c4d8ae82953828b").unwrap();
        assert_eq!(&result[..], &expected[..]);
    }

    // === Large input: 100003 bytes from SHA256(counter) ===
    // Input is SHA256(0_u64_le) || SHA256(1_u64_le) || ... truncated to 100003 bytes.
    // 100003 is prime, ensuring non-aligned multi-block processing.

    #[test]
    fn test_tiger192_3_sha256_counter_100003() {
        use sha2::{Sha256, Digest};
        let mut data = Vec::with_capacity(100003);
        let mut counter: u64 = 0;
        while data.len() < 100003 {
            let mut hasher = Sha256::new();
            hasher.update(counter.to_le_bytes());
            data.extend_from_slice(&hasher.finalize());
            counter += 1;
        }
        data.truncate(100003);
        let result = tiger192_3(&data);
        let expected = hex::decode("a7ae25d0df3bac3e96369133a5b1de2d75878eef5b3fb420").unwrap();
        assert_eq!(&result[..], &expected[..]);
    }

    #[test]
    fn test_tiger192_4_sha256_counter_100003() {
        use sha2::{Sha256, Digest};
        let mut data = Vec::with_capacity(100003);
        let mut counter: u64 = 0;
        while data.len() < 100003 {
            let mut hasher = Sha256::new();
            hasher.update(counter.to_le_bytes());
            data.extend_from_slice(&hasher.finalize());
            counter += 1;
        }
        data.truncate(100003);
        let result = tiger192_4(&data);
        let expected = hex::decode("4727eb171bfac1dbeb8bf9f4274b15ce2a22888f81eedf7f").unwrap();
        assert_eq!(&result[..], &expected[..]);
    }
}
