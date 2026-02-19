//! Character filter and operator transformation.
//!
//! Defense layer 1: Ensures only safe characters are present in the TRANSFORMED expression.
//! This matches the PHP implementation's approach:
//! 1. Transform operators (lowercase, replace word operators)
//! 2. THEN check the whitelist on the transformed result
//!
//! The whitelist is restrictive enough that arbitrary Ruby code cannot be injected.

/// Character whitelist for TRANSFORMED expressions.
/// Based on PHP: "1234567890abcdefABCDEF()*^|&%/+-<>. x"
/// Extended with 'r' for Ruby's Rational suffix (e.g., 1r/2)
///
/// After transformation:
/// - Input is lowercased, so A-F rarely appear (but included for PHP parity)
/// - 'x' is for hex prefix (0xff)
/// - 'r' is for rational suffix (1r, 2r, etc.)
/// - Operators: ( ) * ^ | & % / + - < > .
/// - Space for separating tokens
const WHITELIST: &str = "1234567890abcdefABCDEFr()*^|&%/+-<>. x";

/// Check if the expression contains only allowed characters.
///
/// Also rejects expressions containing ".." (Ruby range operator).
pub fn is_safe(expr: &str) -> bool {
    // Reject range operator
    if expr.contains("..") {
        return false;
    }

    // Check all characters against whitelist
    for ch in expr.chars() {
        if !WHITELIST.contains(ch) {
            return false;
        }
    }

    true
}

/// Transform operators to Ruby syntax.
///
/// 1. Lowercase the expression
/// 2. Replace operators:
///    - ^ → ** (exponentiation)
///    - xor → ^ (bitwise XOR)
///    - or → | (bitwise OR)
///    - and → & (bitwise AND)
///    - shl → << (shift left)
///    - shr → >> (shift right)
///
/// Note: Order matters! We must replace ^ before xor, since xor→^ would
/// then be affected by ^→**.
pub fn transform_operators(expr: &str) -> String {
    let mut result = expr.to_lowercase();

    // Replace ^ with ** first (user's exponentiation operator)
    result = result.replace("^", "**");

    // Then replace word operators (which become Ruby operators)
    // xor becomes ^ (Ruby's XOR), but we already converted ^ to **
    // so this is safe
    result = result.replace("xor", "^");
    // xor has to be done first so we don't replace the "or" in xor!
    result = result.replace("or", "|");
    result = result.replace("and", "&");
    result = result.replace("shl", "<<");
    result = result.replace("shr", ">>");

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: is_safe() is designed to be called on TRANSFORMED input
    // (after transform_operators has been applied)

    #[test]
    fn test_safe_basic() {
        // These are already in transformed form (lowercase, operators replaced)
        assert!(is_safe("2+2"));
        assert!(is_safe("0xff * 0xff"));
        assert!(is_safe("(1+2)*3"));
        assert!(is_safe("5 | 3"));   // OR transformed to |
        assert!(is_safe("1 << 8"));  // SHL transformed to <<
        assert!(is_safe("5 ^ 3"));   // XOR transformed to ^
        assert!(is_safe("5 & 3"));   // AND transformed to &
        assert!(is_safe("2**10"));   // ^ transformed to **
        assert!(is_safe("1r/2"));    // Rational suffix
        assert!(is_safe("2r**-3"));  // Rational with exponent
    }

    #[test]
    fn test_unsafe_characters() {
        // These contain characters not in the whitelist
        assert!(!is_safe("system('ls')"));  // 's', 'y', 't', 'm', quotes
        assert!(!is_safe("`ls`"));          // backticks, 'l', 's'
        assert!(!is_safe("$safe"));         // $ not in whitelist
        assert!(!is_safe("eval"));          // 'v', 'l' not in whitelist
        assert!(!is_safe("1;2"));           // ; not in whitelist
        assert!(!is_safe("1\n2"));          // newline not in whitelist
    }

    #[test]
    fn test_range_operator_rejected() {
        assert!(!is_safe("1..10"));
        assert!(!is_safe("1...10"));
    }

    #[test]
    fn test_transform_operators() {
        assert_eq!(transform_operators("2^3"), "2**3");
        assert_eq!(transform_operators("5 XOR 3"), "5 ^ 3");
        assert_eq!(transform_operators("5 OR 3"), "5 | 3");
        assert_eq!(transform_operators("5 AND 3"), "5 & 3");
        assert_eq!(transform_operators("1 SHL 8"), "1 << 8");
        assert_eq!(transform_operators("256 SHR 4"), "256 >> 4");
    }

    #[test]
    fn test_transform_combined() {
        // 2^3 XOR 1 → 2**3 ^ 1
        assert_eq!(transform_operators("2^3 XOR 1"), "2**3 ^ 1");
    }

    #[test]
    fn test_full_pipeline() {
        // Test the full transform -> is_safe pipeline
        let input = "5 OR 3";
        let transformed = transform_operators(input);
        assert_eq!(transformed, "5 | 3");
        assert!(is_safe(&transformed));

        let input2 = "2^3 XOR 1 AND 7";
        let transformed2 = transform_operators(input2);
        assert_eq!(transformed2, "2**3 ^ 1 & 7");
        assert!(is_safe(&transformed2));
    }
}
