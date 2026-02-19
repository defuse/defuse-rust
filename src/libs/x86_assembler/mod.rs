//! x86/x64 Assembler and Disassembler.
//!
//! This module provides a safe interface for assembling x86/x64 assembly code
//! and disassembling binary data. It uses GCC and objdump behind the scenes.
//!
//! # Security
//!
//! Assembly input is validated before execution to prevent dangerous GAS
//! directives that could:
//! - Include external files (.include, .incbin)
//! - Create unbounded output (.fill, .space, .rept)
//! - Define macros or symbols (.macro, .set)
//!
//! The public API enforces that all input goes through validation before
//! reaching the execution layer. The executor functions are `pub(super)`
//! to prevent bypassing the filter.
//!
//! # Example
//!
//! ```ignore
//! use crate::libs::x86_assembler::{assemble, disassemble, Arch};
//!
//! // Assemble some code
//! let result = assemble("nop\nmov eax, 0", Arch::X86)?;
//! println!("Hex: {}", result.hex);
//!
//! // Disassemble some bytes
//! let result = disassemble("909090", Arch::X86)?;
//! println!("Disassembly: {}", result.disassembly);
//! ```

mod executor;
mod filter;
mod parser;

pub use executor::{Arch, AssemblerError};
pub use parser::AssemblyResult;

/// Assemble x86/x64 assembly code.
///
/// # Arguments
/// * `code` - The assembly code to compile (Intel syntax)
/// * `arch` - Target architecture (X86 or X64)
///
/// # Returns
/// The assembly result containing hex bytes, literals, and disassembly.
///
/// # Errors
/// - `InputTooLarge` if the input exceeds the size limit
/// - `UnsafeCode` if the input contains unsafe directives
/// - `AssemblyFailure` if GCC fails to compile the code
/// - `Timeout` if GCC or objdump takes too long
pub async fn assemble(code: &str, arch: Arch) -> Result<AssemblyResult, AssemblerError> {
    // Validate input before executing
    let safe_code = filter::check_code_safety(code).map_err(|rejection| match rejection {
        filter::SafetyRejection::InputTooLarge => AssemblerError::InputTooLarge,
        filter::SafetyRejection::UnsafeDirectives => AssemblerError::UnsafeCode,
    })?;

    executor::assemble_unsafe(&safe_code, arch).await
}

/// Disassemble a hex string into x86/x64 instructions.
///
/// The input can be in various formats:
/// - Raw hex: "909090"
/// - With spaces: "90 90 90"
/// - With 0x prefixes: "0x90 0x90"
/// - C string literal: "\x90\x90\x90"
/// - C array: "{ 0x90, 0x90 }"
///
/// # Arguments
/// * `hex_input` - The hex string to disassemble
/// * `arch` - Target architecture (X86 or X64)
///
/// # Returns
/// The disassembly result containing hex bytes, literals, and disassembly.
///
/// # Errors
/// - `AssemblyFailure` if objdump fails or input is invalid
/// - `Timeout` if objdump takes too long
/// Maximum disassembler input size (1MB), checked on the raw hex string before
/// any parsing. This prevents memory-intensive hex parsing on huge inputs.
const MAX_DISASSEMBLER_INPUT_SIZE: usize = 1024 * 1024;

pub async fn disassemble(hex_input: &str, arch: Arch) -> Result<AssemblyResult, AssemblerError> {
    if hex_input.len() > MAX_DISASSEMBLER_INPUT_SIZE {
        return Err(AssemblerError::AssemblyFailure(
            "Input too large. Maximum input size is 1MB.".to_string(),
        ));
    }

    // Parse the hex input into binary data
    let binary = parse_hex_input(hex_input)?;

    if binary.is_empty() {
        return Err(AssemblerError::AssemblyFailure(
            "No valid hex data provided".to_string(),
        ));
    }

    executor::disassemble_unsafe(&binary, arch).await
}

/// Parse various hex input formats into binary data.
///
/// Supports:
/// - Raw hex: "909090"
/// - Spaced: "90 90 90"
/// - 0x prefixed: "0x90 0x90"
/// - C string: "\x90\x90"
/// - C array: "{ 0x90, 0x90 }"
fn parse_hex_input(input: &str) -> Result<Vec<u8>, AssemblerError> {
    // Remove 0x prefixes
    let cleaned = input.replace("0x", "").replace("0X", "");

    // Remove all non-hex characters
    let hex_only: String = cleaned
        .chars()
        .filter(|c| c.is_ascii_hexdigit())
        .collect();

    // Reject odd number of hex digits — likely a typo or truncated input
    if hex_only.len() % 2 != 0 {
        return Err(AssemblerError::AssemblyFailure(
            format!(
                "Odd number of hex digits ({}). Each byte requires exactly two hex digits.",
                hex_only.len()
            ),
        ));
    }

    // Convert pairs of hex digits to bytes
    let mut bytes = Vec::with_capacity(hex_only.len() / 2);
    let chars: Vec<char> = hex_only.chars().collect();

    for chunk in chars.chunks(2) {
        let hex_str: String = chunk.iter().collect();
        let byte = u8::from_str_radix(&hex_str, 16)
            .map_err(|_| AssemblerError::AssemblyFailure("Invalid hex input".to_string()))?;
        bytes.push(byte);
    }

    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hex_raw() {
        let result = parse_hex_input("909090").unwrap();
        assert_eq!(result, vec![0x90, 0x90, 0x90]);
    }

    #[test]
    fn test_parse_hex_spaced() {
        let result = parse_hex_input("90 90 90").unwrap();
        assert_eq!(result, vec![0x90, 0x90, 0x90]);
    }

    #[test]
    fn test_parse_hex_0x_prefix() {
        let result = parse_hex_input("0x90 0x90 0x90").unwrap();
        assert_eq!(result, vec![0x90, 0x90, 0x90]);
    }

    #[test]
    fn test_parse_hex_c_string() {
        // C string literal: "\x90\x90\x90"
        let result = parse_hex_input("\"\\x90\\x90\\x90\"").unwrap();
        assert_eq!(result, vec![0x90, 0x90, 0x90]);
    }

    #[test]
    fn test_parse_hex_c_array() {
        let result = parse_hex_input("{ 0x90, 0x90, 0x90 }").unwrap();
        assert_eq!(result, vec![0x90, 0x90, 0x90]);
    }

    #[test]
    fn test_parse_hex_mixed_case() {
        let result = parse_hex_input("aAbBcC").unwrap();
        assert_eq!(result, vec![0xAA, 0xBB, 0xCC]);
    }

    #[test]
    fn test_parse_hex_odd_length() {
        // Odd number of hex digits should return an error
        let result = parse_hex_input("909");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Odd number"), "Expected odd-digit error, got: {}", err);
    }

    #[tokio::test]
    async fn test_assemble_unsafe_code_rejected() {
        let result = assemble(".include \"/etc/passwd\"", Arch::X86).await;
        assert!(matches!(result, Err(AssemblerError::UnsafeCode)));
    }

    #[tokio::test]
    async fn test_assemble_large_input_rejected() {
        let large = "nop\n".repeat(300000); // > 1MB
        let result = assemble(&large, Arch::X86).await;
        assert!(matches!(result, Err(AssemblerError::InputTooLarge)));
    }
}
