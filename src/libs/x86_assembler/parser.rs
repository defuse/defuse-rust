//! Parser for objdump output.
//!
//! This module parses the output of `objdump -d` into a structured format
//! containing the raw hex bytes, string/array literals, and disassembly text.

use regex::Regex;
use std::sync::LazyLock;

/// Check if a line is a label line that should be skipped.
///
/// Label lines look like: "00000013 <location1>:"
/// We also need to allow segment:offset like "ds:0x0" or "jmp 0xf000:0xe05b"
fn is_label_line(line: &str) -> bool {
    // Check for pattern like "address <name>:" which indicates a label
    if let Some(colon_pos) = line.rfind(':') {
        // Look at what's before the colon
        let before_colon = &line[..colon_pos];
        // If it ends with '>' it's a label like "<location1>:"
        if before_colon.ends_with('>') {
            return true;
        }
        // If the character before colon is not a hex digit, 's', or part of
        // a segment register, treat as label line
        if let Some(ch) = before_colon.chars().last() {
            // Allow: hex digits, 's' (for ds:, cs:, etc.)
            if !ch.is_ascii_hexdigit() && ch != 's' {
                return true;
            }
        }
    }
    false
}

/// Result of parsing assembly/disassembly output.
#[derive(Debug, Clone)]
pub struct AssemblyResult {
    /// Hex with null bytes wrapped in <b></b> tags
    pub hex_zero_bold: String,
    /// C string literal format (e.g., "\x90\x90")
    pub string_literal: String,
    /// C array literal format (e.g., "{ 0x90, 0x90 }")
    pub array_literal: String,
    /// The disassembly text
    pub disassembly: String,
}

/// Parse objdump output into structured format.
///
/// # Arguments
/// * `objdump_output` - The raw output from objdump
/// * `is_disassembly` - If true, look for `<.data>:`, otherwise `<_main>:`
///
/// # Returns
/// The parsed result, or an error message if parsing failed.
pub fn parse_objdump_output(objdump_output: &str, is_disassembly: bool) -> Result<AssemblyResult, String> {
    // Find where the actual code starts
    let start_marker = if is_disassembly { "<.data>:\n" } else { "<_main>:\n" };

    let code_start = objdump_output
        .find(start_marker)
        .ok_or("Could not find code section in objdump output")?;

    let code_start = code_start + start_marker.len();

    // Extract just the code section
    let code = &objdump_output[code_start..];

    // Normalize whitespace: replace leading whitespace on each line
    let code = normalize_code(code);

    // Regex to extract hex bytes from objdump disassembly lines.
    // Matches runs of hex byte pairs (e.g., "90 90 90") where each pair
    // is followed by whitespace or end of string.
    static HEX_BYTES_REGEX: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"([a-fA-F0-9]{2}(\s+|$))+").unwrap()
    });

    // Extract hex bytes from each line
    let mut hex_bytes = String::new();
    for line in code.lines() {
        if is_label_line(line) {
            continue;
        }

        // Find hex byte sequences in the line
        if let Some(captures) = HEX_BYTES_REGEX.find(line) {
            hex_bytes.push_str(captures.as_str());
        }
    }

    // Clean up: uppercase
    let hex_bytes = hex_bytes.to_uppercase();

    // Replace "00" with placeholder BEFORE removing spaces.
    // This ensures we only match actual 00 bytes, not cross-byte patterns like "F00F".
    // (Matching PHP's approach: replace while bytes are still space-separated)
    let hex_with_marker = hex_bytes.replace("00", "ZERO");

    // Now remove whitespace
    let hex_bytes = hex_bytes.replace(' ', "").replace('\t', "");
    let hex_with_marker = hex_with_marker.replace(' ', "").replace('\t', "");

    // Create bold version by replacing the marker with <b>00</b>
    let hex_zero_bold = hex_with_marker.replace("ZERO", "<b>00</b>");

    // Create string literal: "\x90\x90..."
    let string_literal = build_string_literal(&hex_bytes);

    // Create array literal: "{ 0x90, 0x90, ... }"
    let array_literal = build_array_literal(&hex_bytes);

    Ok(AssemblyResult {
        hex_zero_bold,
        string_literal,
        array_literal,
        disassembly: code,
    })
}

/// Normalize code by removing leading whitespace on each line.
fn normalize_code(code: &str) -> String {
    code.lines()
        .map(|line| line.trim_start())
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

/// Build a C string literal from hex bytes.
///
/// Example: "9090B8" -> "\x90\x90\xB8"
fn build_string_literal(hex: &str) -> String {
    let mut result = String::new();
    let chars: Vec<char> = hex.chars().collect();

    for chunk in chars.chunks(2) {
        if chunk.len() == 2 {
            result.push_str("\\x");
            result.push(chunk[0]);
            result.push(chunk[1]);
        }
    }

    result
}

/// Build a C array literal from hex bytes.
///
/// Example: "9090B8" -> "{ 0x90, 0x90, 0xB8 }"
fn build_array_literal(hex: &str) -> String {
    let mut parts = Vec::new();
    let chars: Vec<char> = hex.chars().collect();

    for chunk in chars.chunks(2) {
        if chunk.len() == 2 {
            parts.push(format!("0x{}{}", chunk[0], chunk[1]));
        }
    }

    if parts.is_empty() {
        "{ }".to_string()
    } else {
        format!("{{ {} }}", parts.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_string_literal() {
        assert_eq!(build_string_literal("90"), "\\x90");
        assert_eq!(build_string_literal("9090"), "\\x90\\x90");
        assert_eq!(build_string_literal("909090"), "\\x90\\x90\\x90");
    }

    #[test]
    fn test_build_array_literal() {
        assert_eq!(build_array_literal("90"), "{ 0x90 }");
        assert_eq!(build_array_literal("9090"), "{ 0x90, 0x90 }");
        assert_eq!(build_array_literal("909090"), "{ 0x90, 0x90, 0x90 }");
        assert_eq!(build_array_literal(""), "{ }");
    }

    #[test]
    fn test_is_label_line() {
        assert!(is_label_line("00000013 <location1>:"));
        assert!(is_label_line("00000000 <_main>:"));
        assert!(!is_label_line("   0:   90                      nop"));
        // Segment overrides should not be treated as labels
        assert!(!is_label_line("mov eax, ds:0x0"));
    }

    #[test]
    fn test_hex_zero_bold() {
        // Helper to simulate the bold logic (must match parse_objdump_output)
        fn make_bold(hex_with_spaces: &str) -> String {
            let upper = hex_with_spaces.to_uppercase();
            let with_marker = upper.replace("00", "ZERO");
            let with_marker = with_marker.replace(' ', "").replace('\t', "");
            with_marker.replace("ZERO", "<b>00</b>")
        }

        // Actual 00 byte gets bolded
        assert_eq!(make_bold("90 00 90 90"), "90<b>00</b>9090");

        // Cross-byte pattern F00F should NOT be bolded (F0 and 0F are separate bytes)
        assert_eq!(make_bold("F0 0F"), "F00F");

        // But F0 00 0F should bold the middle byte
        assert_eq!(make_bold("F0 00 0F"), "F0<b>00</b>0F");

        // Multiple 00 bytes
        assert_eq!(make_bold("00 00 00"), "<b>00</b><b>00</b><b>00</b>");
    }
}
