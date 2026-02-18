//! Ruby expression evaluator with timeout protection.
//!
//! **SECURITY WARNING**: The function in this module executes arbitrary Ruby code.
//! It must ONLY be called after the expression has been validated by both:
//! 1. The character whitelist filter (filter::is_safe)
//! 2. The parser (parser::validate)
//!
//! This module is intentionally NOT re-exported from the parent module.
//! Only the parent module's calculate() function should call into here.

use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

use super::OutputBase;
use super::parser::SafeExpr;

/// Timeout for the Ruby process (wall-clock time).
const PROCESS_TIMEOUT: Duration = Duration::from_secs(8);

/// CPU time limit for ulimit (slightly longer than wall-clock to allow for overhead).
const ULIMIT_CPU_SECONDS: u32 = 10;

/// Virtual memory limit in KB (256 MB).
/// Prevents DoS attacks that try to exhaust memory faster than CPU limits can catch.
const ULIMIT_VMEM_KB: u32 = 262144;

/// Evaluation error types.
#[derive(Debug)]
pub(super) enum EvalError {
    /// The calculation took too long.
    Timeout,
    /// The expression was invalid (Ruby couldn't evaluate it).
    InvalidExpression,
    /// The result was too large (Infinity, error, or warning in output).
    TooLarge,
}

/// Type of the evaluation result.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum ResultType {
    Integer,
    Float,
    Rational,
}

/// Successful evaluation result.
#[derive(Debug)]
pub(super) struct EvalSuccess {
    /// The computed result as a string.
    pub value: String,
    /// The type of the result.
    pub result_type: ResultType,
}

/// Execute a validated expression in Ruby.
///
/// # Security
///
/// This function executes the expression as Ruby code. The [`SafeExpr`]
/// parameter ensures the expression has been validated by the parser before
/// it can be passed here.
///
/// The `pub(super)` visibility ensures this function cannot be called from
/// outside the `big_number_calculator` module.
pub(super) async fn evaluate(
    expr: &SafeExpr,
    base: OutputBase,
) -> Result<EvalSuccess, EvalError> {
    let ruby_base = base.ruby_base();

    // Build Ruby code to evaluate the expression.
    // In Ruby 3.x, Fixnum and Bignum are unified into Integer.
    // We detect Float, Rational, and Integer types and format accordingly:
    // - Float: prefixed with FLOAT:, always decimal (base conversion not supported)
    // - Rational: prefixed with RATIONAL:, formatted as "num / den" with base conversion
    // - Integer: converted to the specified base
    let ruby_code = format!(
        r#"x = ({});
if x.is_a?(Float)
  puts "FLOAT:" + x.to_s
elsif x.is_a?(Rational)
  puts "RATIONAL:" + x.numerator.to_s({}) + " / " + x.denominator.to_s({})
else
  puts x.to_s({})
end"#,
        expr.as_str(), ruby_base, ruby_base, ruby_base
    );

    // Execute Ruby with ulimit for CPU time and memory protection.
    // We use sh -c to get ulimit support.
    let shell_command = format!(
        "ulimit -t {} -v {}; ruby -e {}",
        ULIMIT_CPU_SECONDS,
        ULIMIT_VMEM_KB,
        shell_escape(&ruby_code)
    );

    // kill_on_drop ensures the child is killed immediately when the timeout
    // fires. ulimits remain as a kernel-enforced backstop for defense in depth.
    let result = timeout(
        PROCESS_TIMEOUT,
        Command::new("sh")
            .arg("-c")
            .arg(&shell_command)
            .kill_on_drop(true)
            .output(),
    )
    .await;

    match result {
        // Timeout waiting for the process
        Err(_) => Err(EvalError::Timeout),

        // Process completed (may have succeeded or failed)
        Ok(output_result) => {
            match output_result {
                Err(_) => Err(EvalError::InvalidExpression),
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let result = stdout.trim();

                    // Check for error conditions
                    if result.is_empty() {
                        // Empty output means Ruby couldn't evaluate (e.g., syntax error, div by zero)
                        return Err(EvalError::InvalidExpression);
                    }

                    // Check for overflow/error indicators in output
                    let result_lower = result.to_lowercase();
                    if result_lower.contains("infinity")
                        || result_lower.contains("error")
                        || result_lower.contains("warning")
                    {
                        return Err(EvalError::TooLarge);
                    }

                    // Check for type prefixes
                    if let Some(float_value) = result.strip_prefix("FLOAT:") {
                        Ok(EvalSuccess {
                            value: float_value.to_string(),
                            result_type: ResultType::Float,
                        })
                    } else if let Some(rational_value) = result.strip_prefix("RATIONAL:") {
                        Ok(EvalSuccess {
                            value: rational_value.to_string(),
                            result_type: ResultType::Rational,
                        })
                    } else {
                        Ok(EvalSuccess {
                            value: result.to_string(),
                            result_type: ResultType::Integer,
                        })
                    }
                }
            }
        }
    }
}

/// Escape a string for use as a shell argument.
/// This wraps the string in single quotes and escapes any single quotes within.
fn shell_escape(s: &str) -> String {
    // Replace ' with '\'' (end quote, escaped quote, start quote)
    let escaped = s.replace("'", "'\\''");
    format!("'{}'", escaped)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::parser;

    // Note: These tests require Ruby to be installed.

    fn safe(expr: &str) -> SafeExpr {
        parser::validate(expr).unwrap()
    }

    #[tokio::test]

    async fn test_basic_arithmetic() {
        assert_eq!(
            evaluate(&safe("2+2"), OutputBase::Decimal).await.unwrap().value,
            "4"
        );
        assert_eq!(
            evaluate(&safe("10-3"), OutputBase::Decimal).await.unwrap().value,
            "7"
        );
        assert_eq!(
            evaluate(&safe("6*7"), OutputBase::Decimal).await.unwrap().value,
            "42"
        );
    }

    #[tokio::test]

    async fn test_floor_division() {
        // Ruby uses floor division
        assert_eq!(
            evaluate(&safe("10/3"), OutputBase::Decimal).await.unwrap().value,
            "3"
        );
        assert_eq!(
            evaluate(&safe("-10/3"), OutputBase::Decimal).await.unwrap().value,
            "-4"
        );
    }

    #[tokio::test]

    async fn test_modulo() {
        // Ruby modulo: result has same sign as divisor
        assert_eq!(
            evaluate(&safe("-7%3"), OutputBase::Decimal).await.unwrap().value,
            "2"
        );
    }

    #[tokio::test]

    async fn test_hex_output() {
        assert_eq!(
            evaluate(&safe("255"), OutputBase::Hexadecimal).await.unwrap().value,
            "ff"
        );
    }

    #[tokio::test]

    async fn test_big_numbers() {
        let result = evaluate(&safe("2**100"), OutputBase::Decimal).await.unwrap();
        assert!(result.value.starts_with("1267650600228229401496703205376"));
    }

    #[tokio::test]

    async fn test_division_by_zero() {
        assert!(matches!(
            evaluate(&safe("1/0"), OutputBase::Decimal).await,
            Err(EvalError::InvalidExpression)
        ));
    }

    #[tokio::test]

    async fn test_all_arithmetic_ops_combined() {
        // 2^10 + 0xff*3 - (100%7) | 0xf = 1791
        assert_eq!(
            evaluate(&safe("2**10 + 0xff * 3 - (100 % 7) | 0xf"), OutputBase::Decimal).await.unwrap().value,
            "1791"
        );
    }

    #[tokio::test]

    async fn test_shifts_and_bitwise_combined() {
        // Build 0x01ABCDEF from shifts and ORs
        assert_eq!(
            evaluate(&safe("(1 << 24) + (0xab << 16) + (0xcd << 8) + 0xef"), OutputBase::Decimal).await.unwrap().value,
            "28036591"
        );
    }

    #[tokio::test]

    async fn test_alternating_powers_of_two() {
        // 2^64 - 2^32 + 2^16 - 2^8 + 2^4 - 1
        assert_eq!(
            evaluate(&safe("2**64 - 2**32 + 2**16 - 2**8 + 2**4 - 1"), OutputBase::Decimal).await.unwrap().value,
            "18446744069414649615"
        );
    }

    #[tokio::test]

    async fn test_chained_unary_operators() {
        // --3 + -(-5) * --2 = 3 + 5*2 = 13
        assert_eq!(
            evaluate(&safe("--3 + -(-5) * --2"), OutputBase::Decimal).await.unwrap().value,
            "13"
        );
    }

    #[tokio::test]

    async fn test_nested_exponentiation() {
        // (2^3)^2 + (3^2)^3 = 64 + 729 = 793
        assert_eq!(
            evaluate(&safe("(2**3)**2 + (3**2)**3"), OutputBase::Decimal).await.unwrap().value,
            "793"
        );
    }

    #[tokio::test]

    async fn test_right_associative_exponentiation() {
        // 2**2**2**2 = 2**(2**(2**2)) = 2**(2**4) = 2**16 = 65536
        assert_eq!(
            evaluate(&safe("2**2**2**2"), OutputBase::Decimal).await.unwrap().value,
            "65536"
        );
    }

    #[tokio::test]

    async fn test_shift_and_mask_chain() {
        // 0xff >> 4 << 2 & 0x3f | 0x80
        assert_eq!(
            evaluate(&safe("0xff >> 4 << 2 & 0x3f | 0x80"), OutputBase::Decimal).await.unwrap().value,
            "188"
        );
    }

    #[tokio::test]

    async fn test_hex_output_complex() {
        // (0xdead << 16) | 0xbeef in hex = deadbeef
        assert_eq!(
            evaluate(&safe("(0xdead << 16) | 0xbeef"), OutputBase::Hexadecimal).await.unwrap().value,
            "deadbeef"
        );
    }

    #[tokio::test]

    async fn test_big_modular_arithmetic() {
        // 2^128 % (10^9 + 7) — common competitive programming pattern
        assert_eq!(
            evaluate(&safe("2**128 % (10**9 + 7)"), OutputBase::Decimal).await.unwrap().value,
            "279632277"
        );
    }

    #[tokio::test]

    async fn test_deeply_nested_parens() {
        // ((1+2)*(3+4)*(5+6)) = 3*7*11 = 231
        assert_eq!(
            evaluate(&safe("((1+2)*(3+4)*(5+6))"), OutputBase::Decimal).await.unwrap().value,
            "231"
        );
    }

    #[tokio::test]

    async fn test_rational_exact_fraction() {
        // 1r/7 * 7 = 1/1 (exact — no floating point error)
        let result = evaluate(&safe("1r/7 * 7"), OutputBase::Decimal).await.unwrap();
        assert_eq!(result.result_type, ResultType::Rational);
        assert_eq!(result.value, "1 / 1");
    }

    #[tokio::test]

    async fn test_rational_sum() {
        // 1r/2 + 1r/3 + 1r/6 = 1/1
        let result = evaluate(&safe("1r/2 + 1r/3 + 1r/6"), OutputBase::Decimal).await.unwrap();
        assert_eq!(result.result_type, ResultType::Rational);
        assert_eq!(result.value, "1 / 1");
    }

    #[tokio::test]

    async fn test_float_mixed_arithmetic() {
        // 3.14 + 2.86 + 0.5 * 2 = 7.0
        let result = evaluate(&safe("3.14 + 2.86 + 0.5 * 2"), OutputBase::Decimal).await.unwrap();
        assert_eq!(result.result_type, ResultType::Float);
        assert_eq!(result.value, "7.0");
    }

    #[tokio::test]
    async fn test_scientific_notation() {
        let result = evaluate(&safe("1e5"), OutputBase::Decimal).await.unwrap();
        assert_eq!(result.result_type, ResultType::Float);
        assert_eq!(result.value, "100000.0");
    }

    #[tokio::test]
    async fn test_scientific_notation_negative_exponent() {
        let result = evaluate(&safe("1e-5"), OutputBase::Decimal).await.unwrap();
        assert_eq!(result.result_type, ResultType::Float);
        assert_eq!(result.value, "1.0e-05");
    }

    #[tokio::test]
    async fn test_scientific_notation_float_mantissa() {
        let result = evaluate(&safe("2.5e3"), OutputBase::Decimal).await.unwrap();
        assert_eq!(result.result_type, ResultType::Float);
        assert_eq!(result.value, "2500.0");
    }
}
