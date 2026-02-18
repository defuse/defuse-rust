//! Expression parser using pest.
//!
//! Defense layer 2: Validates that the expression has valid arithmetic structure.
//! Only accepts expressions matching the grammar - no method calls, no Ruby code.

use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "libs/big_number_calculator/grammar.pest"]
struct ExprParser;

/// A validated expression that is safe to evaluate.
///
/// This type can only be constructed by [`validate`], which ensures the
/// expression has passed AST-level validation. The evaluator requires this
/// type, making it impossible to evaluate an unvalidated expression.
pub struct SafeExpr(String);

impl SafeExpr {
    /// Get the validated expression string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Validate that the expression matches our arithmetic grammar.
///
/// This is called AFTER character filtering and operator transformation.
/// Returns a [`SafeExpr`] if valid, Err with a message if invalid.
pub fn validate(expr: &str) -> Result<SafeExpr, String> {
    // Empty or whitespace-only expressions are invalid
    if expr.trim().is_empty() {
        return Err("Empty expression".to_string());
    }

    match ExprParser::parse(Rule::expr, expr) {
        Ok(_) => Ok(SafeExpr(expr.to_string())),
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
    fn test_scientific_notation() {
        assert!(validate("1e5").is_ok());
        assert!(validate("1E5").is_ok());
        assert!(validate("1e+5").is_ok());
        assert!(validate("1e-5").is_ok());
        assert!(validate("2.5e3").is_ok());
        assert!(validate("2.5e+3").is_ok());
        assert!(validate("2.5e-3").is_ok());
        assert!(validate("1e5 + 2").is_ok());
        assert!(validate("3e-2-1").is_ok()); // (3e-2) - 1
        // Invalid forms
        assert!(validate("1e").is_err());    // no exponent digits
        assert!(validate("e5").is_err());    // no mantissa
        // Ruby rejects 5.e3 (interprets . as method call) — must not parse
        assert!(validate("5.e3").is_err());
        // Ruby rejects .5e3 (no leading digit before .) — must not parse
        assert!(validate(".5e3").is_err());
    }

    #[test]
    fn test_scientific_notation_does_not_break_hex() {
        // Hex numbers containing 'e' must still parse as hex, not scientific
        assert!(validate("0xe5").is_ok());
        assert!(validate("0xfe5").is_ok());
        assert!(validate("0xdeadbeef").is_ok());
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

    // --- Security-focused rejection tests ---
    // These test that the grammar blocks constructs an attacker might try.
    // The character filter is a separate defense layer; these tests verify
    // the grammar provides structural protection independently.

    #[test]
    fn test_rejects_function_calls() {
        assert!(validate("puts(3)").is_err());
        assert!(validate("exit(1)").is_err());
        assert!(validate("exec(1)").is_err());
        assert!(validate("system(1)").is_err());
        assert!(validate("eval(1)").is_err());
        assert!(validate("send(1)").is_err());
        assert!(validate("p(3)").is_err());
    }

    #[test]
    fn test_rejects_hex_char_identifiers() {
        // a-f pass the character filter but must not parse as identifiers
        assert!(validate("abc").is_err());
        assert!(validate("dead").is_err());
        assert!(validate("cafe").is_err());
        assert!(validate("def").is_err());
        assert!(validate("a").is_err());
        assert!(validate("f").is_err());
        assert!(validate("abc(3)").is_err());
        assert!(validate("def(3)").is_err());
        assert!(validate("a(3)").is_err());
        assert!(validate("bad + 1").is_err());
    }

    #[test]
    fn test_rejects_implicit_multiplication() {
        // No implicit multiplication through adjacency
        assert!(validate("1(2)").is_err());
        assert!(validate("(1)(2)").is_err());
        assert!(validate("2(3+4)").is_err());
        assert!(validate("1 2").is_err());
        assert!(validate("0xdeadbeef(1+2)").is_err());
    }

    #[test]
    fn test_rejects_ruby_method_calls_on_numbers() {
        assert!(validate("1.class").is_err());
        assert!(validate("1.send").is_err());
        assert!(validate("1.chr").is_err());
        assert!(validate("1.abs").is_err());
    }

    #[test]
    fn test_rejects_ruby_percent_command() {
        // %x(...) is Ruby shell execution
        assert!(validate("%x(1)").is_err());
    }

    #[test]
    fn test_rejects_statement_separators() {
        assert!(validate("1;2").is_err());
        assert!(validate("1\n2").is_err());
    }

    #[test]
    fn test_rejects_assignment() {
        assert!(validate("a=1").is_err());
        assert!(validate("x=1").is_err());
    }

    #[test]
    fn test_rejects_malformed_numbers() {
        assert!(validate("0x").is_err());      // incomplete hex prefix
        assert!(validate("1rr").is_err());     // double rational suffix
        assert!(validate("r1").is_err());      // r before number
        assert!(validate("1r2").is_err());     // digits after rational suffix
        assert!(validate("1.2.3").is_err());   // multiple decimal points
    }

    #[test]
    fn test_rejects_bare_operators() {
        assert!(validate("+").is_err());
        assert!(validate("*").is_err());
        assert!(validate("**").is_err());
        assert!(validate("/").is_err());
        assert!(validate("%").is_err());
        assert!(validate("|").is_err());
        assert!(validate("&").is_err());
        assert!(validate("<<").is_err());
        assert!(validate(">>").is_err());
        assert!(validate("1+").is_err());
        assert!(validate("*1").is_err());
        assert!(validate("1**").is_err());
        assert!(validate("1+*2").is_err());
    }

    #[test]
    fn test_rejects_empty_parens_nested() {
        assert!(validate("(())").is_err());
        assert!(validate("((()))").is_err());
        assert!(validate("1+()").is_err());
    }

    #[test]
    fn test_rejects_range_operator() {
        assert!(validate("1..10").is_err());
        assert!(validate("1...10").is_err());
    }

    #[test]
    fn test_rejects_ruby_special_variables() {
        assert!(validate("$0").is_err());
        assert!(validate("@a").is_err());
        assert!(validate("@@a").is_err());
    }

    #[test]
    fn test_rejects_string_literals() {
        assert!(validate("\"abc\"").is_err());
        assert!(validate("'abc'").is_err());
        assert!(validate("`cmd`").is_err());
    }

    #[test]
    fn test_rejects_ruby_keywords() {
        assert!(validate("if").is_err());
        assert!(validate("1 if 1").is_err());
        assert!(validate("while").is_err());
        assert!(validate("begin").is_err());
        assert!(validate("end").is_err());
    }

    #[test]
    fn test_rejects_comma_separated() {
        // Can't sneak multiple arguments or array construction
        assert!(validate("1,2").is_err());
        assert!(validate("(1,2)").is_err());
    }

    #[test]
    fn test_rejects_brackets() {
        assert!(validate("[1]").is_err());
        assert!(validate("{1}").is_err());
    }
}
