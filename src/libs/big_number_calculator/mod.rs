//! Big Number Calculator library.
//!
//! Provides arbitrary-precision arithmetic by safely shelling out to Ruby.
//! Security is enforced through:
//! 1. Character whitelist filter (defense layer 1)
//! 2. AST parser validation (defense layer 2)
//! 3. Process timeout and resource limits

mod evaluator;
mod filter;
mod formatter;
mod parser;


// Note: EvalError, EvalSuccess, and ResultType are intentionally NOT re-exported.
// The evaluator module is internal to this module for security reasons.
use evaluator::{EvalError, EvalSuccess, ResultType};

/// Output base for the calculation result.
#[derive(Debug, Clone, Copy, Default)]
pub enum OutputBase {
    #[default]
    Decimal,
    Hexadecimal,
    Octal,
}

impl OutputBase {
    /// Parse from form value (dec, hex, oct).
    pub fn from_str(s: &str) -> Self {
        match s {
            "hex" => OutputBase::Hexadecimal,
            "oct" => OutputBase::Octal,
            _ => OutputBase::Decimal,
        }
    }

    /// Ruby base number for to_s conversion.
    fn ruby_base(&self) -> u8 {
        match self {
            OutputBase::Decimal => 10,
            OutputBase::Hexadecimal => 16,
            OutputBase::Octal => 8,
        }
    }

    /// Digit grouping interval for this base.
    fn grouping_interval(&self) -> usize {
        match self {
            OutputBase::Decimal => 3,
            OutputBase::Hexadecimal => 4,
            OutputBase::Octal => 2,
        }
    }
}

/// Options for the calculator.
#[derive(Debug, Clone, Default)]
pub struct CalculatorOptions {
    pub base: OutputBase,
    pub add_spaces: bool,
}

/// Result of a calculation.
#[derive(Debug)]
pub struct CalculatorResult {
    /// The formatted output (result or error message).
    pub output: String,
    /// Whether this is an error result.
    pub is_error: bool,
}

/// Main entry point: validate, evaluate, and format an expression.
///
/// Returns a CalculatorResult with either the formatted result or an error message.
pub async fn calculate(expr: &str, options: &CalculatorOptions) -> CalculatorResult {
    // Step 1: Transform operators (lowercase, replace OR/AND/XOR/SHL/SHR/^)
    // This must happen FIRST, matching PHP behavior.
    let transformed = filter::transform_operators(expr);

    // Step 2: Character filter on TRANSFORMED expression (defense layer 1)
    // The whitelist only allows chars that can appear AFTER transformation.
    if !filter::is_safe(&transformed) {
        return CalculatorResult {
            output: "Sorry, what you entered wasn't recognized as a valid mathematical expression.".to_string(),
            is_error: true,
        };
    }

    // Step 3: Parse and validate structure (defense layer 2)
    let safe_expr = match parser::validate(&transformed) {
        Ok(safe) => safe,
        Err(_) => {
            return CalculatorResult {
                output: "Sorry, what you entered wasn't recognized as a valid mathematical expression.".to_string(),
                is_error: true,
            };
        }
    };

    // Step 4: Evaluate with Ruby
    let eval_result = evaluator::evaluate(&safe_expr, options.base).await;

    match eval_result {
        Ok(EvalSuccess { value, result_type }) => {
            // Defense-in-depth: HTML-escape the Ruby output before formatting.
            // The input filter guarantees Ruby output contains only safe characters
            // (digits, hex letters, `-`, `/`, `.`, space, `true`, `false`), so
            // escaping should be a no-op. We assert this to catch any regression
            // in the input filter that could lead to XSS via |safe rendering.
            let escaped_value = crate::libs::util::html_escape(&value);
            assert!(
                escaped_value == value,
                "BUG: Ruby output contains HTML-special characters: {:?}",
                value
            );

            // Step 5: Format output
            let (formatted, is_multiline_rational) = if options.add_spaces && value != "true" && value != "false" {
                // For rationals, only group the numerator and denominator parts, not the " / "
                if result_type == ResultType::Rational {
                    (format_rational_with_grouping(&value, options.base.grouping_interval()), true)
                } else {
                    (formatter::group_digits(&value, options.base.grouping_interval()), false)
                }
            } else {
                // No formatting needed - CSS word-break handles wrapping
                (value, false)
            };

            // Add warning if float result with non-decimal base
            let output = if result_type == ResultType::Float && !matches!(options.base, OutputBase::Decimal) {
                format!(
                    "{}\n\n\n(Note: Floating-point results are always displayed in decimal.)",
                    formatted
                )
            } else {
                formatted
            };

            // Wrap multiline rationals in right-aligned div for proper alignment
            let html_output = if is_multiline_rational {
                format!("<div style=\"text-align: right;\">{}</div>", newlines_to_br(&output))
            } else {
                newlines_to_br(&output)
            };

            CalculatorResult {
                output: html_output,
                is_error: false,
            }
        }
        Err(EvalError::Timeout) => CalculatorResult {
            output: "Sorry, it's taking too long to calculate that number.".to_string(),
            is_error: true,
        },
        Err(EvalError::InvalidExpression) => CalculatorResult {
            output: "Sorry, what you entered wasn't recognized as a valid mathematical expression.".to_string(),
            is_error: true,
        },
        Err(EvalError::TooLarge) => CalculatorResult {
            output: "Sorry, we can't calculate numbers THAT big!".to_string(),
            is_error: true,
        },
    }
}

/// Format a rational result with digit grouping.
/// Input is "num / den", output groups each part on separate lines:
/// numerator
///  /
/// denominator
fn format_rational_with_grouping(value: &str, interval: usize) -> String {
    if let Some((num, den)) = value.split_once(" / ") {
        let num_grouped = formatter::group_digits(num.trim(), interval);
        let den_grouped = formatter::group_digits(den.trim(), interval);
        format!("{}\n / \n{}", num_grouped, den_grouped)
    } else {
        // Fallback if not in expected format
        formatter::group_digits(value, interval)
    }
}

/// Convert newlines to `<br />` for HTML display.
fn newlines_to_br(s: &str) -> String {
    s.replace('\n', "<br />")
}
