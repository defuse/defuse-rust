//! Input validation for x86 assembly code.
//!
//! This module implements a whitelist-based filter that ensures only safe
//! GAS (GNU Assembler) directives are allowed. This is critical for security
//! since the code will be passed to GCC for compilation.

use regex::Regex;
use std::sync::LazyLock;

/// Maximum allowed input size in bytes (10KB)
const MAX_INPUT_SIZE: usize = 10 * 1024;

/// Directives that are considered safe to use.
///
/// These are whitelisted because they:
/// - Cannot include external files (.include, .incbin)
/// - Cannot create unbounded output (.fill, .space, .skip, .rept)
/// - Cannot define macros or symbols (.macro, .set, .equ)
/// - Cannot change sections or addresses (.section, .org)
///
/// Note: ". " (dot followed by space) allows relative jumps like `jmp . + 5`
const SAFE_DIRECTIVES: &[&str] = &[
    ".ascii", ".asciz", ".align", ".balign",
    ".byte", ".int", ".double", ".quad", ".octa", ".word", ". ",
];

/// Check if the given assembly code is safe to compile.
///
/// Returns `true` if the code passes all safety checks, `false` otherwise.
///
/// # Security Model
///
/// 1. Size limit: Input must be under 10KB
/// 2. Remove all safe directives from the code
/// 3. Remove decimal floating-point literals (e.g., 1.0, 3.14159)
/// 4. Reject if `#APP` or `#NO_APP` appears (GAS special comments) -- TODO: check if this is a complete list
/// 5. Reject if any `.` character remains after filtering
///
/// This ensures only basic instructions and whitelisted directives are allowed.
///
/// ## Security Note on Decimal Float Removal
///
/// The decimal float regex `\d+\.\d+` is safe because it requires digits on
/// BOTH sides of the period. Dangerous directives like `.include` start with
/// `.` followed by letters, not digits, so they cannot be hidden within a
/// decimal pattern.
pub fn is_safe_code(code: &str) -> bool {
    // Check size limit
    if code.len() >= MAX_INPUT_SIZE {
        return false;
    }

    // Remove all safe directives from the code
    let mut filtered = code.to_string();
    for directive in SAFE_DIRECTIVES {
        filtered = filtered.replace(directive, "");
    }

    // Remove decimal floating-point literals (e.g., 1.0, 3.14159)
    // This allows .double directive to take floating-point arguments.
    // Safe because the pattern requires digits on both sides of the period.
    static DECIMAL_FLOAT_REGEX: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\d+\.\d+").unwrap());
    filtered = DECIMAL_FLOAT_REGEX.replace_all(&filtered, "").to_string();

    // Reject if GAS special comments are present.
    // These have special meaning to the assembler and could be dangerous.
    if filtered.contains("#NO_APP") || filtered.contains("#APP") {
        return false;
    }

    // If any period remains, reject the input.
    // This catches any directives not in the whitelist.
    //
    // NOTE: This is overly conservative - it will reject periods in string
    // constants like `.ascii "hello.world"` after the directive is removed.
    // The PHP implementation has the same limitation. To properly handle this
    // would require actual parsing, not just string matching.
    !filtered.contains('.')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_instructions() {
        assert!(is_safe_code("nop"));
        assert!(is_safe_code("xor eax, eax"));
        assert!(is_safe_code("push eax\npop ebx"));
        assert!(is_safe_code("mov rax, 0"));
    }

    #[test]
    fn test_safe_directives() {
        assert!(is_safe_code(".byte 0x41, 0x42"));
        assert!(is_safe_code(".ascii \"HELLO\""));
        assert!(is_safe_code(".asciz \"HI\""));
        assert!(is_safe_code(".align 4"));
        assert!(is_safe_code(".balign 4"));
        assert!(is_safe_code(".int 0x12345678"));
        assert!(is_safe_code(".word 0x1234"));
        assert!(is_safe_code(".quad 0x0102030405060708"));
        assert!(is_safe_code(".octa 0x0102030405060708090a0b0c0d0e0f10"));
        assert!(is_safe_code(".double 1")); // Integer works
    }

    #[test]
    fn test_relative_jump_dot_space() {
        // ". " is safe for relative jumps
        assert!(is_safe_code("jmp . + 5"));
        assert!(is_safe_code("jmp . + 2\nnop"));
    }

    #[test]
    fn test_unsafe_gas_comments() {
        assert!(!is_safe_code("#APP\nnop"));
        assert!(!is_safe_code("#NO_APP\nnop"));
        assert!(!is_safe_code("#APP\nmov eax, 0\n#NO_APP"));
    }

    #[test]
    fn test_unsafe_directives() {
        assert!(!is_safe_code(".fill 1000000, 1, 0x90"));
        assert!(!is_safe_code(".org 0x1000\nnop"));
        assert!(!is_safe_code(".section .text\nnop"));
        assert!(!is_safe_code(".include \"/etc/passwd\""));
        assert!(!is_safe_code(".incbin \"/etc/passwd\""));
        assert!(!is_safe_code(".macro mymacro\nnop\n.endm"));
        assert!(!is_safe_code(".rept 1000000\nnop\n.endr"));
        assert!(!is_safe_code(".space 1000000"));
        assert!(!is_safe_code(".skip 1000000"));
        assert!(!is_safe_code(".set foo, 42"));
        assert!(!is_safe_code(".equ foo, 42"));
        assert!(!is_safe_code(".global _start"));
        assert!(!is_safe_code(".extern printf"));
    }

    #[test]
    fn test_size_limit() {
        let large_input = "nop\n".repeat(300000); // > 1MB
        assert!(!is_safe_code(&large_input));

        let small_input = "nop\n".repeat(1000);
        assert!(is_safe_code(&small_input));
    }

    #[test]
    fn test_double_with_decimal() {
        // .double with floating-point literals should work
        assert!(is_safe_code(".double 1.0"));
        assert!(is_safe_code(".double 3.14159"));
        assert!(is_safe_code(".double 1.5, 2.5, 3.5"));
        assert!(is_safe_code(".double 1.0e10")); // Scientific notation
        assert!(is_safe_code(".double -1.0")); // Negative
    }

    #[test]
    fn test_decimal_removal_security() {
        // Ensure decimal removal can't be exploited to hide directives
        // These should all be rejected because the directive remains visible
        assert!(!is_safe_code(".in1.0clude \"/etc/passwd\""));
        assert!(!is_safe_code("1.0.fill 1000"));
        assert!(!is_safe_code(".1.0include \"x\"")); // .1 doesn't match \d+\.\d+
    }
}
