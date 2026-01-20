//! Expression parser using pest.
//!
//! Defense layer 2: Validates that the expression has valid arithmetic structure.
//! Only accepts expressions matching the grammar - no method calls, no Ruby code.

use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "libs/big_number_calculator/grammar.pest"]
struct ExprParser;

/// Validate that the expression matches our arithmetic grammar.
///
/// This is called AFTER character filtering and operator transformation.
/// Returns Ok(()) if valid, Err with a message if invalid.
pub fn validate(expr: &str) -> Result<(), String> {
    // Empty or whitespace-only expressions are invalid
    if expr.trim().is_empty() {
        return Err("Empty expression".to_string());
    }

    match ExprParser::parse(Rule::expr, expr) {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("Parse error: {}", e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_arithmetic() {
        assert!(validate("2+2").is_ok());
        assert!(validate("10-3").is_ok());
        assert!(validate("6*7").is_ok());
        assert!(validate("10/3").is_ok());
        assert!(validate("10%3").is_ok());
    }

    #[test]
    fn test_exponentiation() {
        assert!(validate("2**10").is_ok());
        assert!(validate("2**3**2").is_ok()); // Right-associative
    }

    #[test]
    fn test_bitwise() {
        assert!(validate("5|4").is_ok());
        assert!(validate("5&4").is_ok());
        assert!(validate("5^4").is_ok());
        assert!(validate("1<<8").is_ok());
        assert!(validate("256>>4").is_ok());
    }

    #[test]
    fn test_parentheses() {
        assert!(validate("(2+3)*4").is_ok());
        assert!(validate("((1+2))").is_ok());
        assert!(validate("(((1)))").is_ok());
    }

    #[test]
    fn test_unary() {
        assert!(validate("-5").is_ok());
        assert!(validate("--5").is_ok());
        assert!(validate("+-5").is_ok());
        assert!(validate("2*-3").is_ok());
    }

    #[test]
    fn test_hex() {
        assert!(validate("0xff").is_ok());
        assert!(validate("0xFF").is_ok());
        assert!(validate("0x10*0x10").is_ok());
    }

    #[test]
    fn test_floats() {
        assert!(validate("3.14").is_ok());
        assert!(validate(".5").is_ok());
        assert!(validate("5.").is_ok());
        assert!(validate("0.5").is_ok());
    }

    #[test]
    fn test_rationals() {
        assert!(validate("1r").is_ok());
        assert!(validate("1r/2").is_ok());
        assert!(validate("2r**-3").is_ok());
        assert!(validate("(1r/2)*3").is_ok());
        assert!(validate("1r/2 + 1r/4").is_ok());
    }

    #[test]
    fn test_whitespace() {
        assert!(validate(" 2 + 2 ").is_ok());
        assert!(validate("2  +  2").is_ok());
    }

    #[test]
    fn test_complex() {
        assert!(validate("(2**8-1)*3+10").is_ok());
        assert!(validate("2+3*4**2").is_ok());
    }

    #[test]
    fn test_invalid_empty() {
        assert!(validate("").is_err());
        assert!(validate("   ").is_err());
    }

    #[test]
    fn test_invalid_trailing_operator() {
        assert!(validate("2+").is_err());
        assert!(validate("*2").is_err());
    }

    #[test]
    fn test_invalid_unmatched_parens() {
        assert!(validate("(2+3").is_err());
        assert!(validate("2+3)").is_err());
        assert!(validate("()").is_err());
    }
}
