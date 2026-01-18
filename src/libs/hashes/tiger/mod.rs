//! Tiger hash algorithm implementation.
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
}
