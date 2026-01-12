//! Validation rule IR definitions.
//!
//! This module defines validation rule structures that represent
//! constraints applied to fields. These rules are transformed into
//! validation method calls in the generated schema code.

use serde::{Deserialize, Serialize};

/// Validation rule for fields.
///
/// Represents a single validation constraint that can be applied to a field.
/// Multiple rules can be combined and will be chained in the generated output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "rule", content = "value")]
pub enum ValidationRule {
    // ==========================================================================
    // String Validations
    // ==========================================================================
    /// Minimum string length: `.min(n)`
    MinLength(usize),

    /// Maximum string length: `.max(n)`
    MaxLength(usize),

    /// Exact string length: `.length(n)`
    Length(usize),

    /// Email format: `.email()`
    Email,

    /// URL format: `.url()`
    Url,

    /// UUID format: `.uuid()`
    Uuid,

    /// CUID format: `.cuid()`
    Cuid,

    /// CUID2 format: `.cuid2()`
    Cuid2,

    /// ULID format: `.ulid()`
    Ulid,

    /// Regex pattern: `.regex(/pattern/)`
    Regex(String),

    /// Starts with prefix: `.startsWith("prefix")`
    StartsWith(String),

    /// Ends with suffix: `.endsWith("suffix")`
    EndsWith(String),

    /// Contains substring: `.includes("substring")`
    Includes(String),

    /// ISO datetime format: `.datetime()`
    Datetime,

    /// IP address format: `.ip()`
    Ip,

    /// IPv4 address format: `.ip({ version: "v4" })`
    Ipv4,

    /// IPv6 address format: `.ip({ version: "v6" })`
    Ipv6,

    /// Emoji validation: `.emoji()`
    Emoji,

    /// Trim whitespace: `.trim()`
    Trim,

    /// Convert to lowercase: `.toLowerCase()`
    ToLowerCase,

    /// Convert to uppercase: `.toUpperCase()`
    ToUpperCase,

    // ==========================================================================
    // Number Validations
    // ==========================================================================
    /// Minimum value: `.min(n)` or `.gte(n)`
    Min(f64),

    /// Maximum value: `.max(n)` or `.lte(n)`
    Max(f64),

    /// Greater than: `.gt(n)`
    GreaterThan(f64),

    /// Less than: `.lt(n)`
    LessThan(f64),

    /// Positive number (> 0): `.positive()`
    Positive,

    /// Negative number (< 0): `.negative()`
    Negative,

    /// Non-negative number (>= 0): `.nonnegative()`
    NonNegative,

    /// Non-positive number (<= 0): `.nonpositive()`
    NonPositive,

    /// Integer validation: `.int()`
    Int,

    /// Finite number validation: `.finite()`
    Finite,

    /// Safe integer validation: `.safe()`
    Safe,

    /// Multiple of: `.multipleOf(n)`
    MultipleOf(f64),

    // ==========================================================================
    // Array Validations
    // ==========================================================================
    /// Minimum array length: `.min(n)`
    MinItems(usize),

    /// Maximum array length: `.max(n)`
    MaxItems(usize),

    /// Exact array length: `.length(n)`
    ItemsLength(usize),

    /// Non-empty array: `.nonempty()`
    Nonempty,

    // ==========================================================================
    // Custom Validations
    // ==========================================================================
    /// Custom validation expression: `.refine(fn)`
    Custom(String),

    /// Refinement with custom function: `.refine(fn, { message })`
    Refine {
        /// The refinement function expression
        expression: String,
        /// Optional error message
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
    },

    /// Transform the value: `.transform(fn)`
    Transform(String),

    /// Superrefine for complex validations: `.superRefine(fn)`
    SuperRefine(String),
}

impl ValidationRule {
    /// Check if this is a string validation rule.
    pub fn is_string_validation(&self) -> bool {
        matches!(
            self,
            ValidationRule::MinLength(_)
                | ValidationRule::MaxLength(_)
                | ValidationRule::Length(_)
                | ValidationRule::Email
                | ValidationRule::Url
                | ValidationRule::Uuid
                | ValidationRule::Cuid
                | ValidationRule::Cuid2
                | ValidationRule::Ulid
                | ValidationRule::Regex(_)
                | ValidationRule::StartsWith(_)
                | ValidationRule::EndsWith(_)
                | ValidationRule::Includes(_)
                | ValidationRule::Datetime
                | ValidationRule::Ip
                | ValidationRule::Ipv4
                | ValidationRule::Ipv6
                | ValidationRule::Emoji
                | ValidationRule::Trim
                | ValidationRule::ToLowerCase
                | ValidationRule::ToUpperCase
        )
    }

    /// Check if this is a number validation rule.
    pub fn is_number_validation(&self) -> bool {
        matches!(
            self,
            ValidationRule::Min(_)
                | ValidationRule::Max(_)
                | ValidationRule::GreaterThan(_)
                | ValidationRule::LessThan(_)
                | ValidationRule::Positive
                | ValidationRule::Negative
                | ValidationRule::NonNegative
                | ValidationRule::NonPositive
                | ValidationRule::Int
                | ValidationRule::Finite
                | ValidationRule::Safe
                | ValidationRule::MultipleOf(_)
        )
    }

    /// Check if this is an array validation rule.
    pub fn is_array_validation(&self) -> bool {
        matches!(
            self,
            ValidationRule::MinItems(_)
                | ValidationRule::MaxItems(_)
                | ValidationRule::ItemsLength(_)
                | ValidationRule::Nonempty
        )
    }

    /// Check if this is a custom validation rule.
    pub fn is_custom_validation(&self) -> bool {
        matches!(
            self,
            ValidationRule::Custom(_)
                | ValidationRule::Refine { .. }
                | ValidationRule::Transform(_)
                | ValidationRule::SuperRefine(_)
        )
    }

    /// Create a refinement with a message.
    pub fn refine_with_message(expression: impl Into<String>, message: impl Into<String>) -> Self {
        ValidationRule::Refine {
            expression: expression.into(),
            message: Some(message.into()),
        }
    }

    /// Create a refinement without a message.
    pub fn refine(expression: impl Into<String>) -> Self {
        ValidationRule::Refine {
            expression: expression.into(),
            message: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_validations() {
        assert!(ValidationRule::Email.is_string_validation());
        assert!(ValidationRule::Url.is_string_validation());
        assert!(ValidationRule::Uuid.is_string_validation());
        assert!(ValidationRule::MinLength(1).is_string_validation());
        assert!(ValidationRule::Regex(".*".to_string()).is_string_validation());
    }

    #[test]
    fn test_number_validations() {
        assert!(ValidationRule::Min(0.0).is_number_validation());
        assert!(ValidationRule::Max(100.0).is_number_validation());
        assert!(ValidationRule::Positive.is_number_validation());
        assert!(ValidationRule::Int.is_number_validation());
        assert!(ValidationRule::MultipleOf(2.0).is_number_validation());
    }

    #[test]
    fn test_array_validations() {
        assert!(ValidationRule::MinItems(1).is_array_validation());
        assert!(ValidationRule::MaxItems(10).is_array_validation());
        assert!(ValidationRule::Nonempty.is_array_validation());
    }

    #[test]
    fn test_custom_validations() {
        assert!(ValidationRule::Custom("x => x > 0".to_string()).is_custom_validation());
        assert!(ValidationRule::refine("x => x > 0").is_custom_validation());
        assert!(ValidationRule::Transform("x => x.trim()".to_string()).is_custom_validation());
    }

    #[test]
    fn test_refine_with_message() {
        let rule = ValidationRule::refine_with_message("x => x > 0", "Must be positive");
        match rule {
            ValidationRule::Refine {
                expression,
                message,
            } => {
                assert_eq!(expression, "x => x > 0");
                assert_eq!(message, Some("Must be positive".to_string()));
            }
            _ => panic!("Expected Refine variant"),
        }
    }

    #[test]
    fn test_validation_categories_are_exclusive() {
        // String validations should not be number or array validations
        assert!(!ValidationRule::Email.is_number_validation());
        assert!(!ValidationRule::Email.is_array_validation());

        // Number validations should not be string or array validations
        assert!(!ValidationRule::Positive.is_string_validation());
        assert!(!ValidationRule::Positive.is_array_validation());

        // Array validations should not be string or number validations
        assert!(!ValidationRule::Nonempty.is_string_validation());
        assert!(!ValidationRule::Nonempty.is_number_validation());
    }
}
