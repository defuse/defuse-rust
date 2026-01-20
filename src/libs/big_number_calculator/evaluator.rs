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

/// Timeout for the Ruby process (wall-clock time).
const PROCESS_TIMEOUT: Duration = Duration::from_secs(8);

/// CPU time limit for ulimit (slightly longer than wall-clock to allow for overhead).
const ULIMIT_CPU_SECONDS: u32 = 10;

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

/// Execute a pre-validated expression in Ruby. **DANGEROUS - DO NOT CALL DIRECTLY.**
///
/// # Security
///
/// This function executes the expression as Ruby code. Calling this with
/// unvalidated input is a **remote code execution vulnerability**.
///
/// This function should ONLY be called from `calculate()` in the parent module,
/// after the expression has passed both character filtering and parser validation.
///
/// The `pub(super)` visibility ensures this function cannot be called from
/// outside the `big_number_calculator` module.
pub(super) async fn evaluate_unsafe_requires_prior_validation(
    expr: &str,
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
        expr, ruby_base, ruby_base, ruby_base
    );

    // Execute Ruby with ulimit for CPU time protection.
    // We use sh -c to get ulimit support.
    let shell_command = format!(
        "ulimit -t {}; ruby -e {}",
        ULIMIT_CPU_SECONDS,
        shell_escape(&ruby_code)
    );

    let result = timeout(
        PROCESS_TIMEOUT,
        Command::new("sh")
            .arg("-c")
            .arg(&shell_command)
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

    // Note: These tests require Ruby to be installed.
    // They're marked as ignore by default for CI environments without Ruby.

    #[tokio::test]
    #[ignore]
    async fn test_basic_arithmetic() {
        assert_eq!(
            evaluate_unsafe_requires_prior_validation("2+2", OutputBase::Decimal).await.unwrap().value,
            "4"
        );
        assert_eq!(
            evaluate_unsafe_requires_prior_validation("10-3", OutputBase::Decimal).await.unwrap().value,
            "7"
        );
        assert_eq!(
            evaluate_unsafe_requires_prior_validation("6*7", OutputBase::Decimal).await.unwrap().value,
            "42"
        );
    }

    #[tokio::test]
    #[ignore]
    async fn test_floor_division() {
        // Ruby uses floor division
        assert_eq!(
            evaluate_unsafe_requires_prior_validation("10/3", OutputBase::Decimal).await.unwrap().value,
            "3"
        );
        assert_eq!(
            evaluate_unsafe_requires_prior_validation("-10/3", OutputBase::Decimal).await.unwrap().value,
            "-4"
        );
    }

    #[tokio::test]
    #[ignore]
    async fn test_modulo() {
        // Ruby modulo: result has same sign as divisor
        assert_eq!(
            evaluate_unsafe_requires_prior_validation("-7%3", OutputBase::Decimal).await.unwrap().value,
            "2"
        );
    }

    #[tokio::test]
    #[ignore]
    async fn test_hex_output() {
        assert_eq!(
            evaluate_unsafe_requires_prior_validation("255", OutputBase::Hexadecimal).await.unwrap().value,
            "ff"
        );
    }

    #[tokio::test]
    #[ignore]
    async fn test_big_numbers() {
        let result = evaluate_unsafe_requires_prior_validation("2**100", OutputBase::Decimal).await.unwrap();
        assert!(result.value.starts_with("1267650600228229401496703205376"));
    }

    #[tokio::test]
    #[ignore]
    async fn test_division_by_zero() {
        assert!(matches!(
            evaluate_unsafe_requires_prior_validation("1/0", OutputBase::Decimal).await,
            Err(EvalError::InvalidExpression)
        ));
    }
}
