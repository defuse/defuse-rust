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

// Note: EvalError is intentionally NOT re-exported.
// The evaluator module is internal to this module for security reasons.
use evaluator::EvalError;

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
    if parser::validate(&transformed).is_err() {
        return CalculatorResult {
            output: "Sorry, what you entered wasn't recognized as a valid mathematical expression.".to_string(),
            is_error: true,
        };
    }

    // Step 4: Evaluate with Ruby
    // SAFETY: Expression has passed both character filter and parser validation above.
    let eval_result =
        evaluator::evaluate_unsafe_requires_prior_validation(&transformed, options.base).await;

    match eval_result {
        Ok(result) => {
            // Step 5: Format output
            let formatted = if options.add_spaces && result != "true" && result != "false" {
                formatter::group_digits(&result, options.base.grouping_interval())
            } else {
                formatter::break_lines(&result, 60)
            };

            CalculatorResult {
                output: formatted,
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
