//! Snefru hash algorithm implementation.
//! THIS IMPLEMENTATION HAS NOT BEEN AUDITED! DO NOT RELY ON IT FOR SECURITY!
//!
//! Snefru is a cryptographic hash function designed by Ralph Merkle in 1990.
//! This implementation produces a 256-bit (32-byte) digest using 8 rounds,
//! matching PHP's hash('snefru') and hash('snefru256') output exactly.
//!
//! Note: Snefru is considered cryptographically broken and should not be used
//! for security-critical applications. This implementation is provided for
//! compatibility with legacy systems.

mod tables;

use tables::SBOX;

/// Block size in bytes (256 bits = 32 bytes)
const BLOCK_SIZE: usize = 32;

/// Digest size in bytes (256 bits = 32 bytes)
const DIGEST_SIZE: usize = 32;

/// Number of rounds
const ROUNDS: usize = 8;

/// Rotation shifts for each sub-round
const SHIFTS: [u32; 4] = [16, 8, 16, 24];

/// Snefru hash context
struct SnefruContext {
    /// Hash state (16 x 32-bit words)
    state: [u32; 16],
    /// Bit count (low, high)
    count: [u32; 2],
    /// Buffer for partial blocks
    buffer: [u8; BLOCK_SIZE],
    /// Number of bytes in buffer
    buffer_len: usize,
}

impl SnefruContext {
    fn new() -> Self {
        Self {
            state: [0u32; 16],
            count: [0u32; 2],
            buffer: [0u8; BLOCK_SIZE],
            buffer_len: 0,
        }
    }

    fn update(&mut self, data: &[u8]) {
        // Update bit count
        let bit_len = (data.len() as u32).wrapping_mul(8);
        let (new_low, overflow) = self.count[1].overflowing_add(bit_len);
        self.count[1] = new_low;
        if overflow {
            self.count[0] = self.count[0].wrapping_add(1);
        }

        let mut offset = 0;

        // If we have buffered data, try to complete a block
        if self.buffer_len > 0 {
            let needed = BLOCK_SIZE - self.buffer_len;
            if data.len() >= needed {
                self.buffer[self.buffer_len..].copy_from_slice(&data[..needed]);
                self.transform(&self.buffer.clone());
                offset = needed;
                self.buffer_len = 0;
            } else {
                self.buffer[self.buffer_len..self.buffer_len + data.len()]
                    .copy_from_slice(data);
                self.buffer_len += data.len();
                return;
            }
        }

        // Process complete blocks
        while offset + BLOCK_SIZE <= data.len() {
            self.transform(&data[offset..offset + BLOCK_SIZE]);
            offset += BLOCK_SIZE;
        }

        // Buffer remaining data
        let remaining = data.len() - offset;
        if remaining > 0 {
            self.buffer[..remaining].copy_from_slice(&data[offset..]);
            self.buffer_len = remaining;
        }
    }

    fn transform(&mut self, block: &[u8]) {
        // Load block into state[8..16] in big-endian format
        for i in 0..8 {
            let j = i * 4;
            self.state[8 + i] = u32::from_be_bytes([
                block[j],
                block[j + 1],
                block[j + 2],
                block[j + 3],
            ]);
        }

        // Apply Snefru function
        snefru_block(&mut self.state);

        // Clear the input portion
        for i in 8..16 {
            self.state[i] = 0;
        }
    }

    fn finalize(mut self) -> [u8; DIGEST_SIZE] {
        // Process any remaining buffered data
        if self.buffer_len > 0 {
            // Zero-pad the buffer
            for i in self.buffer_len..BLOCK_SIZE {
                self.buffer[i] = 0;
            }
            self.transform(&self.buffer.clone());
        }

        // Append bit count and do final transformation
        self.state[14] = self.count[0];
        self.state[15] = self.count[1];
        snefru_block(&mut self.state);

        // Extract digest from state[0..8] in big-endian format
        let mut digest = [0u8; DIGEST_SIZE];
        for i in 0..8 {
            let bytes = self.state[i].to_be_bytes();
            digest[i * 4..i * 4 + 4].copy_from_slice(&bytes);
        }

        digest
    }
}

/// Core Snefru block function
///
/// Operates on 16 32-bit words (512-bit state).
/// Uses 8 iterations with 4 sub-rounds each.
fn snefru_block(state: &mut [u32; 16]) {
    let mut b = *state;

    for round in 0..ROUNDS {
        let t0 = &SBOX[2 * round];
        let t1 = &SBOX[2 * round + 1];

        for sub_round in 0..4 {
            // Apply S-box lookups and XOR operations
            macro_rules! sbox_round {
                ($l:expr, $c:expr, $n:expr, $table:expr) => {{
                    let sbe = $table[(b[$c] & 0xff) as usize];
                    b[$l] ^= sbe;
                    b[$n] ^= sbe;
                }};
            }

            sbox_round!(15, 0, 1, t0);
            sbox_round!(0, 1, 2, t0);
            sbox_round!(1, 2, 3, t1);
            sbox_round!(2, 3, 4, t1);
            sbox_round!(3, 4, 5, t0);
            sbox_round!(4, 5, 6, t0);
            sbox_round!(5, 6, 7, t1);
            sbox_round!(6, 7, 8, t1);
            sbox_round!(7, 8, 9, t0);
            sbox_round!(8, 9, 10, t0);
            sbox_round!(9, 10, 11, t1);
            sbox_round!(10, 11, 12, t1);
            sbox_round!(11, 12, 13, t0);
            sbox_round!(12, 13, 14, t0);
            sbox_round!(13, 14, 15, t1);
            sbox_round!(14, 15, 0, t1);

            // Rotate all state words
            let shift = SHIFTS[sub_round];
            for word in &mut b {
                *word = word.rotate_right(shift);
            }
        }
    }

    // XOR transformed values back into state
    state[0] ^= b[15];
    state[1] ^= b[14];
    state[2] ^= b[13];
    state[3] ^= b[12];
    state[4] ^= b[11];
    state[5] ^= b[10];
    state[6] ^= b[9];
    state[7] ^= b[8];
}

/// Compute Snefru-256 hash of the input data.
///
/// Returns a 32-byte (256-bit) digest.
///
/// # Example
/// ```
/// let hash = snefru256(b"test");
/// assert_eq!(hex::encode(hash), "8d25dd0b5715f7e4c799ade3a34b5f6148d0ce416992b5c2eaf614d35d5b3d30");
/// ```
pub fn snefru256(data: &[u8]) -> [u8; DIGEST_SIZE] {
    let mut ctx = SnefruContext::new();
    ctx.update(data);
    ctx.finalize()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snefru_empty() {
        let hash = snefru256(b"");
        assert_eq!(
            hex::encode(hash),
            "8617f366566a011837f4fb4ba5bedea2b892f3ed8b894023d16ae344b2be5881"
        );
    }

    #[test]
    fn test_snefru_test() {
        let hash = snefru256(b"test");
        assert_eq!(
            hex::encode(hash),
            "8d25dd0b5715f7e4c799ade3a34b5f6148d0ce416992b5c2eaf614d35d5b3d30"
        );
    }

    #[test]
    fn test_snefru_quick_brown_fox() {
        let hash = snefru256(b"The quick brown fox jumps over the lazy dog");
        assert_eq!(
            hex::encode(hash),
            "674caa75f9d8fd2089856b95e93a4fb42fa6c8702f8980e11d97a142d76cb358"
        );
    }

    #[test]
    fn test_snefru_long_input() {
        let input = "a".repeat(1000);
        let hash = snefru256(input.as_bytes());
        assert_eq!(
            hex::encode(hash),
            "c5795bac1192bdea5a9dbe735211f890aef23b92687b6002d1938a7876e049c3"
        );
    }

    // === Block boundary tests (block = 32 bytes) ===

    #[test]
    fn test_snefru_31_bytes() {
        let hash = snefru256(&vec![b'a'; 31]);
        assert_eq!(
            hex::encode(hash),
            "96bb2b81b3aff11a4d672b23f600f6965c138276ead7d089369deaa9258988e7"
        );
    }

    #[test]
    fn test_snefru_32_bytes() {
        let hash = snefru256(&vec![b'a'; 32]);
        assert_eq!(
            hex::encode(hash),
            "dbc6238cc321aecba8f057213c3a605d74f21ec352e2183bc3b3853064ffa732"
        );
    }

    #[test]
    fn test_snefru_33_bytes() {
        let hash = snefru256(&vec![b'a'; 33]);
        assert_eq!(
            hex::encode(hash),
            "7a1133846080dd68d6842df39c86f961925605679bad4ffae07118482b6031fa"
        );
    }

    // === Signedness tests (0xFF bytes stress u32 arithmetic) ===

    #[test]
    fn test_snefru_32_0xff() {
        let hash = snefru256(&vec![0xFFu8; 32]);
        assert_eq!(
            hex::encode(hash),
            "d18a2d2d8aa0f831d3f339442e1b8ec8965039405459bc9fb6277fd7b636d9f7"
        );
    }

    #[test]
    fn test_snefru_64_0xff() {
        let hash = snefru256(&vec![0xFFu8; 64]);
        assert_eq!(
            hex::encode(hash),
            "a85110ae4dffe3765c7fadc0579d640c5675004fa3819a48e92d3bd1746d8785"
        );
    }

    // === Single byte edge cases ===

    #[test]
    fn test_snefru_single_0x00() {
        let hash = snefru256(&[0x00]);
        assert_eq!(
            hex::encode(hash),
            "d40c2a1ac28b11a875157ccb3bb2e75fbac5138ca354005381080f67bca0093b"
        );
    }

    #[test]
    fn test_snefru_single_0x80() {
        let hash = snefru256(&[0x80]);
        assert_eq!(
            hex::encode(hash),
            "176b92f680ccacf8d270d9e5cba4d9b762eecdd59aa3726177af65928fe71272"
        );
    }

    #[test]
    fn test_snefru_single_0xff() {
        let hash = snefru256(&[0xFF]);
        assert_eq!(
            hex::encode(hash),
            "8513c4f80aeefde2a48d0790071002420eff7a3611ef6dcdc9fcf4ec2db4eb5b"
        );
    }

    // === Alternating bit patterns ===

    #[test]
    fn test_snefru_alternating_0x55() {
        let hash = snefru256(&vec![0x55u8; 64]);
        assert_eq!(
            hex::encode(hash),
            "5f27a2cc9db4c9f8ef5072f5fe39fc1e8a33b3f2672f231a7f8604a2f729fa32"
        );
    }

    #[test]
    fn test_snefru_alternating_0xaa() {
        let hash = snefru256(&vec![0xAAu8; 64]);
        assert_eq!(
            hex::encode(hash),
            "1d159cf273660ec461a98fb745f60eede2d7125dd82a4b675fbfb801ac79c5b3"
        );
    }

    // === Large input: 100003 bytes from SHA256(counter) ===
    // Input is SHA256(0_u64_le) || SHA256(1_u64_le) || ... truncated to 100003 bytes.
    // 100003 is prime, ensuring non-aligned multi-block processing.

    #[test]
    fn test_snefru_sha256_counter_100003() {
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
        let hash = snefru256(&data);
        assert_eq!(
            hex::encode(hash),
            "b77a6b885d44622487e6dee09c48dfb984ffcca71dff64672df4fe2e179c3934"
        );
    }
}
