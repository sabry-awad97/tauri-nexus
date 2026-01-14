//! Input Validation Framework
//!
//! This module provides a comprehensive validation framework for RPC input types.
//! It includes the `Validate` trait, field-level error reporting, and a builder
//! pattern for common validation rules.
//!
//! # Example
//!
//! ```rust,ignore
//! use tauri_plugin_rpc::validation::{Validate, ValidationResult, FieldError, ValidationRules};
//!
//! #[derive(Debug)]
//! struct CreateUserInput {
//!     name: String,
//!     email: String,
//!     age: i64,
//! }
//!
//! impl Validate for CreateUserInput {
//!     fn validate(&self) -> ValidationResult {
//!         ValidationRules::new()
//!             .required("name", &self.name)
//!             .min_length("name", &self.name, 2)
//!             .max_length("name", &self.name, 100)
//!             .email("email", &self.email)
//!             .range("age", self.age, 0, 150)
//!             .build()
//!     }
//! }
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, trace, warn};

/// Validation error for a single field.
///
/// Contains the field name, error message, and a code identifying the type of error.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldError {
    /// The name of the field that failed validation
    pub field: String,
    /// Human-readable error message
    pub message: String,
    /// Error code identifying the type of validation failure
    pub code: String,
}

impl FieldError {
    /// Create a new field error
    pub fn new(
        field: impl Into<String>,
        message: impl Into<String>,
        code: impl Into<String>,
    ) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
            code: code.into(),
        }
    }

    /// Create a "required" field error
    pub fn required(field: impl Into<String>) -> Self {
        let field = field.into();
        Self::new(&field, format!("{} is required", field), "required")
    }

    /// Create a "min_length" field error
    pub fn min_length(field: impl Into<String>, min: usize) -> Self {
        let field = field.into();
        Self::new(
            &field,
            format!("{} must be at least {} characters", field, min),
            "min_length",
        )
    }

    /// Create a "max_length" field error
    pub fn max_length(field: impl Into<String>, max: usize) -> Self {
        let field = field.into();
        Self::new(
            &field,
            format!("{} must be at most {} characters", field, max),
            "max_length",
        )
    }

    /// Create a "range" field error
    pub fn range(field: impl Into<String>, min: i64, max: i64) -> Self {
        let field = field.into();
        Self::new(
            &field,
            format!("{} must be between {} and {}", field, min, max),
            "range",
        )
    }

    /// Create a "pattern" field error
    pub fn pattern(field: impl Into<String>, pattern: &str) -> Self {
        let field = field.into();
        Self::new(
            &field,
            format!("{} must match pattern: {}", field, pattern),
            "pattern",
        )
    }

    /// Create an "email" field error
    pub fn email(field: impl Into<String>) -> Self {
        let field = field.into();
        Self::new(
            &field,
            format!("{} must be a valid email address", field),
            "email",
        )
    }

    /// Create a custom field error
    pub fn custom(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(field, message, "custom")
    }
}

/// Result of validating an input.
///
/// Contains a flag indicating whether validation passed and a list of field errors.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Whether the input is valid
    pub valid: bool,
    /// List of field-level errors (empty if valid)
    pub errors: Vec<FieldError>,
}

impl ValidationResult {
    /// Create a successful validation result
    pub fn ok() -> Self {
        trace!("Validation passed");
        Self {
            valid: true,
            errors: Vec::new(),
        }
    }

    /// Create a failed validation result with errors
    pub fn fail(errors: Vec<FieldError>) -> Self {
        let error_count = errors.len();
        let field_names: Vec<_> = errors.iter().map(|e| e.field.as_str()).collect();
        debug!(
            error_count = error_count,
            fields = ?field_names,
            "Validation failed"
        );
        Self {
            valid: errors.is_empty(),
            errors,
        }
    }

    /// Create a validation result from a list of errors.
    /// If the list is empty, the result is valid.
    pub fn from_errors(errors: Vec<FieldError>) -> Self {
        if errors.is_empty() {
            trace!("Validation passed (no errors)");
        } else {
            let field_names: Vec<_> = errors.iter().map(|e| e.field.as_str()).collect();
            debug!(
                error_count = errors.len(),
                fields = ?field_names,
                "Validation failed"
            );
        }
        Self {
            valid: errors.is_empty(),
            errors,
        }
    }

    /// Check if validation passed
    pub fn is_valid(&self) -> bool {
        self.valid
    }

    /// Get the errors
    pub fn errors(&self) -> &[FieldError] {
        &self.errors
    }

    /// Convert to a map of field -> errors for easier lookup
    pub fn errors_by_field(&self) -> HashMap<String, Vec<&FieldError>> {
        let mut map: HashMap<String, Vec<&FieldError>> = HashMap::new();
        for error in &self.errors {
            map.entry(error.field.clone()).or_default().push(error);
        }
        map
    }

    /// Merge another validation result into this one
    pub fn merge(mut self, other: ValidationResult) -> Self {
        trace!(
            current_errors = self.errors.len(),
            other_errors = other.errors.len(),
            "Merging validation results"
        );
        self.errors.extend(other.errors);
        self.valid = self.errors.is_empty();
        self
    }
}

impl Default for ValidationResult {
    fn default() -> Self {
        Self::ok()
    }
}

/// Trait for validatable input types.
///
/// Implement this trait on your input structs to enable automatic validation
/// before handler execution.
///
/// # Example
///
/// ```rust,ignore
/// use tauri_plugin_rpc::validation::{Validate, ValidationResult, ValidationRules};
///
/// struct CreateUserInput {
///     name: String,
///     email: String,
/// }
///
/// impl Validate for CreateUserInput {
///     fn validate(&self) -> ValidationResult {
///         ValidationRules::new()
///             .required("name", &self.name)
///             .email("email", &self.email)
///             .build()
///     }
/// }
/// ```
pub trait Validate {
    /// Validate the input and return a result with any errors
    fn validate(&self) -> ValidationResult;
}

// Implement Validate for common types that don't need validation
impl Validate for () {
    fn validate(&self) -> ValidationResult {
        ValidationResult::ok()
    }
}

impl Validate for serde_json::Value {
    fn validate(&self) -> ValidationResult {
        ValidationResult::ok()
    }
}

impl<T: Validate> Validate for Option<T> {
    fn validate(&self) -> ValidationResult {
        match self {
            Some(value) => value.validate(),
            None => ValidationResult::ok(),
        }
    }
}

/// Builder for validation rules.
///
/// Provides a fluent API for building validation rules with common validators.
///
/// # Example
///
/// ```rust,ignore
/// let result = ValidationRules::new()
///     .required("name", &input.name)
///     .min_length("name", &input.name, 2)
///     .max_length("name", &input.name, 100)
///     .email("email", &input.email)
///     .range("age", input.age, 0, 150)
///     .pattern("phone", &input.phone, r"^\+?[0-9]{10,15}$")
///     .build();
/// ```
#[derive(Debug, Default)]
pub struct ValidationRules {
    errors: Vec<FieldError>,
}

impl ValidationRules {
    /// Create a new validation rules builder
    pub fn new() -> Self {
        trace!("Creating new ValidationRules builder");
        Self { errors: Vec::new() }
    }

    /// Add a custom error
    pub fn add_error(mut self, error: FieldError) -> Self {
        trace!(field = %error.field, code = %error.code, "Adding custom validation error");
        self.errors.push(error);
        self
    }

    /// Validate that a string field is not empty (required)
    pub fn required(mut self, field: &str, value: &str) -> Self {
        if value.trim().is_empty() {
            trace!(field = %field, "Required field is empty");
            self.errors.push(FieldError::required(field));
        }
        self
    }

    /// Validate that an optional string field is not empty if present
    pub fn required_if_present(mut self, field: &str, value: &Option<String>) -> Self {
        if let Some(v) = value
            && v.trim().is_empty()
        {
            trace!(field = %field, "Optional field present but empty");
            self.errors.push(FieldError::required(field));
        }
        self
    }

    /// Validate minimum string length
    pub fn min_length(mut self, field: &str, value: &str, min: usize) -> Self {
        if value.len() < min {
            trace!(field = %field, length = value.len(), min = min, "Field below minimum length");
            self.errors.push(FieldError::min_length(field, min));
        }
        self
    }

    /// Validate maximum string length
    pub fn max_length(mut self, field: &str, value: &str, max: usize) -> Self {
        if value.len() > max {
            trace!(field = %field, length = value.len(), max = max, "Field exceeds maximum length");
            self.errors.push(FieldError::max_length(field, max));
        }
        self
    }

    /// Validate that a number is within a range (inclusive)
    pub fn range(mut self, field: &str, value: i64, min: i64, max: i64) -> Self {
        if value < min || value > max {
            trace!(field = %field, value = value, min = min, max = max, "Field outside valid range");
            self.errors.push(FieldError::range(field, min, max));
        }
        self
    }

    /// Validate that a float is within a range (inclusive)
    pub fn range_f64(mut self, field: &str, value: f64, min: f64, max: f64) -> Self {
        if value < min || value > max {
            trace!(field = %field, value = %value, min = %min, max = %max, "Float field outside valid range");
            self.errors.push(FieldError::new(
                field,
                format!("{} must be between {} and {}", field, min, max),
                "range",
            ));
        }
        self
    }

    /// Validate that a string matches a regex pattern
    pub fn pattern(mut self, field: &str, value: &str, pattern: &str) -> Self {
        match regex::Regex::new(pattern) {
            Ok(re) => {
                if !re.is_match(value) {
                    trace!(field = %field, pattern = %pattern, "Field does not match pattern");
                    self.errors.push(FieldError::pattern(field, pattern));
                }
            }
            Err(e) => {
                // Invalid regex pattern - this is a programming error
                warn!(field = %field, pattern = %pattern, error = %e, "Invalid validation regex pattern");
                self.errors.push(FieldError::new(
                    field,
                    format!("Invalid validation pattern: {}", pattern),
                    "invalid_pattern",
                ));
            }
        }
        self
    }

    /// Validate that a string is a valid email address
    pub fn email(mut self, field: &str, value: &str) -> Self {
        // Simple email validation - checks for @ and at least one dot after @
        let is_valid = value.contains('@')
            && value.split('@').count() == 2
            && value
                .split('@')
                .next_back()
                .map(|domain| domain.contains('.'))
                .unwrap_or(false)
            && !value.starts_with('@')
            && !value.ends_with('@')
            && !value.ends_with('.');

        if !is_valid {
            trace!(field = %field, "Invalid email format");
            self.errors.push(FieldError::email(field));
        }
        self
    }

    /// Add a custom validation with a predicate
    pub fn custom<F>(mut self, field: &str, predicate: F, message: &str) -> Self
    where
        F: FnOnce() -> bool,
    {
        if !predicate() {
            trace!(field = %field, message = %message, "Custom validation failed");
            self.errors.push(FieldError::custom(field, message));
        }
        self
    }

    /// Build the validation result
    pub fn build(self) -> ValidationResult {
        let error_count = self.errors.len();
        if error_count == 0 {
            trace!("Validation rules passed");
            ValidationResult::ok()
        } else {
            let field_names: Vec<_> = self.errors.iter().map(|e| e.field.as_str()).collect();
            debug!(
                error_count = error_count,
                fields = ?field_names,
                "Validation rules failed"
            );
            ValidationResult::fail(self.errors)
        }
    }
}
