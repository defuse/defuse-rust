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

/// Assembly code that has passed the safety filter.
///
/// This type cannot be constructed outside of this module — the only way to
/// obtain one is by passing code through [`check_code_safety`]. This ensures
/// at the type level that `assemble_unsafe` can never be called with
/// unvalidated input.
pub struct SafeAsm<'a>(&'a str);

impl<'a> SafeAsm<'a> {
    /// Access the validated assembly code.
    pub(super) fn as_str(&self) -> &str {
        self.0
    }
}

/// Reason why code was rejected by the safety filter.
pub enum SafetyRejection {
    /// Input exceeded the maximum size limit.
    InputTooLarge,
    /// Input contained unsafe directives or characters.
    UnsafeDirectives,
}

/// Check if the given assembly code is safe to compile.
///
/// Returns `Ok(())` if the code passes all safety checks, or a `SafetyRejection`
/// describing why it was rejected.
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
pub fn check_code_safety(code: &str) -> Result<SafeAsm<'_>, SafetyRejection> {
    // Check size limit
    if code.len() >= MAX_INPUT_SIZE {
        return Err(SafetyRejection::InputTooLarge);
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
        return Err(SafetyRejection::UnsafeDirectives);
    }

    // If any period remains, reject the input.
    // This catches any directives not in the whitelist.
    //
    // NOTE: This is overly conservative - it will reject periods in string
    // constants like `.ascii "hello.world"` after the directive is removed.
    // The PHP implementation has the same limitation. To properly handle this
    // would require actual parsing, not just string matching.
    if filtered.contains('.') {
        return Err(SafetyRejection::UnsafeDirectives);
    }

    Ok(SafeAsm(code))
}

/// Build the user-facing error message for unsafe directives.
pub fn unsafe_directives_message() -> String {
    let directives: Vec<&str> = SAFE_DIRECTIVES
        .iter()
        .filter(|d| d.trim() != ".")
        .copied()
        .collect();
    format!(
        "Sorry, your input contains unsafe directives! \n\
         The period (.) character must not appear anywhere in your source code \
         except in the following allowed directives: {}. \
         Decimal floating-point values (e.g. 1.0, 3.14) are also allowed.",
        directives.join(", ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn is_safe(code: &str) -> bool {
        check_code_safety(code).is_ok()
    }

    #[test]
    fn test_safe_instructions() {
        assert!(is_safe("nop"));
        assert!(is_safe("xor eax, eax"));
        assert!(is_safe("push eax\npop ebx"));
        assert!(is_safe("mov rax, 0"));
    }

    #[test]
    fn test_safe_directives() {
        assert!(is_safe(".byte 0x41, 0x42"));
        assert!(is_safe(".ascii \"HELLO\""));
        assert!(is_safe(".asciz \"HI\""));
        assert!(is_safe(".align 4"));
        assert!(is_safe(".balign 4"));
        assert!(is_safe(".int 0x12345678"));
        assert!(is_safe(".word 0x1234"));
        assert!(is_safe(".quad 0x0102030405060708"));
        assert!(is_safe(".octa 0x0102030405060708090a0b0c0d0e0f10"));
        assert!(is_safe(".double 1")); // Integer works
    }

    #[test]
    fn test_relative_jump_dot_space() {
        // ". " is safe for relative jumps
        assert!(is_safe("jmp . + 5"));
        assert!(is_safe("jmp . + 2\nnop"));
    }

    #[test]
    fn test_unsafe_gas_comments() {
        assert!(!is_safe("#APP\nnop"));
        assert!(!is_safe("#NO_APP\nnop"));
        assert!(!is_safe("#APP\nmov eax, 0\n#NO_APP"));
    }

    #[test]
    fn test_unsafe_directives() {
        assert!(!is_safe(".fill 1000000, 1, 0x90"));
        assert!(!is_safe(".org 0x1000\nnop"));
        assert!(!is_safe(".section .text\nnop"));
        assert!(!is_safe(".include \"/etc/passwd\""));
        assert!(!is_safe(".incbin \"/etc/passwd\""));
        assert!(!is_safe(".macro mymacro\nnop\n.endm"));
        assert!(!is_safe(".rept 1000000\nnop\n.endr"));
        assert!(!is_safe(".space 1000000"));
        assert!(!is_safe(".skip 1000000"));
        assert!(!is_safe(".set foo, 42"));
        assert!(!is_safe(".equ foo, 42"));
        assert!(!is_safe(".global _start"));
        assert!(!is_safe(".extern printf"));
    }

    #[test]
    fn test_size_limit() {
        let large_input = "nop\n".repeat(300000); // > 1MB
        assert!(matches!(check_code_safety(&large_input), Err(SafetyRejection::InputTooLarge)));

        let small_input = "nop\n".repeat(1000);
        assert!(is_safe(&small_input));
    }

    #[test]
    fn test_double_with_decimal() {
        // .double with floating-point literals should work
        assert!(is_safe(".double 1.0"));
        assert!(is_safe(".double 3.14159"));
        assert!(is_safe(".double 1.5, 2.5, 3.5"));
        assert!(is_safe(".double 1.0e10")); // Scientific notation
        assert!(is_safe(".double -1.0")); // Negative
    }

    #[test]
    fn test_decimal_removal_security() {
        // Ensure decimal removal can't be exploited to hide directives
        // These should all be rejected because the directive remains visible
        assert!(!is_safe(".in1.0clude \"/etc/passwd\""));
        assert!(!is_safe("1.0.fill 1000"));
        assert!(!is_safe(".1.0include \"x\"")); // .1 doesn't match \d+\.\d+
    }
}
