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

// =============================================================================
// RPC-Specific Validation Functions
// =============================================================================

use crate::{RpcConfig, RpcError, subscription::SubscriptionId};

/// Validate procedure path format.
///
/// A valid procedure path:
/// - Cannot be empty
/// - Cannot start or end with a dot
/// - Cannot contain consecutive dots (..)
/// - Can only contain alphanumeric characters, underscores, and dots
///
/// # Errors
///
/// Returns `RpcError::validation` if the path is invalid.
///
/// # Examples
///
/// ```rust,ignore
/// validate_path("user.get")?;  // OK
/// validate_path("user.list")?; // OK
/// validate_path("")?;          // Error: empty
/// validate_path(".user")?;     // Error: starts with dot
/// validate_path("user..get")?; // Error: consecutive dots
/// ```
pub fn validate_path(path: &str) -> Result<(), RpcError> {
    if path.is_empty() {
        return Err(RpcError::validation(format!(
            "Procedure path cannot be empty (got: '{}')",
            path
        )));
    }
    if path.starts_with('.') || path.ends_with('.') {
        return Err(RpcError::validation(format!(
            "Procedure path cannot start or end with a dot (got: '{}')",
            path
        )));
    }
    if path.contains("..") {
        return Err(RpcError::validation(format!(
            "Procedure path cannot contain consecutive dots (got: '{}')",
            path
        )));
    }

    // Use iterator methods for cleaner validation
    if let Some(invalid_char) = path
        .chars()
        .find(|&ch| !ch.is_ascii_alphanumeric() && ch != '_' && ch != '.')
    {
        return Err(RpcError::validation(format!(
            "Procedure path contains invalid character: '{}' in path '{}'",
            invalid_char, path
        )));
    }

    Ok(())
}

/// Validate input size against configuration limit.
///
/// Uses heuristics to avoid unnecessary serialization for small inputs:
/// - Null: 4 bytes
/// - Boolean: 5 bytes  
/// - Number: ~20 bytes (conservative estimate)
/// - String: length + 2 (quotes)
/// - Small arrays/objects: skip serialization if obviously small
///
/// # Errors
///
/// Returns `RpcError::payload_too_large` if the input exceeds the configured maximum size.
///
/// # Examples
///
/// ```rust,ignore
/// let config = RpcConfig::default();
/// validate_input_size(&json!({"name": "test"}), &config)?;  // OK
/// validate_input_size(&json!(null), &config)?;              // OK (4 bytes)
/// ```
pub fn validate_input_size(input: &serde_json::Value, config: &RpcConfig) -> Result<(), RpcError> {
    use serde_json::Value;

    // Fast path: estimate size for simple types
    let estimated_size = match input {
        Value::Null => 4,
        Value::Bool(_) => 5,
        Value::Number(_) => 20,                    // Conservative estimate
        Value::String(s) => s.len() + 2,           // Add quotes
        Value::Array(arr) if arr.is_empty() => 2,  // "[]"
        Value::Object(obj) if obj.is_empty() => 2, // "{}"
        _ => {
            // Complex type: need actual serialization
            let size = serde_json::to_vec(input).map(|v| v.len()).unwrap_or(0);

            if size > config.max_input_size {
                return Err(RpcError::payload_too_large(format!(
                    "Input size {} bytes exceeds maximum {} bytes",
                    size, config.max_input_size
                )));
            }
            return Ok(());
        }
    };

    // Early return for small inputs
    if estimated_size > config.max_input_size {
        return Err(RpcError::payload_too_large(format!(
            "Input size ~{} bytes exceeds maximum {} bytes",
            estimated_size, config.max_input_size
        )));
    }

    Ok(())
}

/// Validate subscription ID format when provided by client.
///
/// This function accepts both formats for backward compatibility:
/// - With prefix: "sub_01234567-89ab-7cde-8f01-234567890abc"
/// - Without prefix: "01234567-89ab-7cde-8f01-234567890abc"
///
/// # Errors
///
/// Returns `RpcError::validation` if the ID is invalid.
///
/// # Examples
///
/// ```rust,ignore
/// validate_subscription_id("sub_01234567-89ab-7cde-8f01-234567890abc")?;  // OK
/// validate_subscription_id("01234567-89ab-7cde-8f01-234567890abc")?;      // OK
/// validate_subscription_id("")?;                                           // Error
/// validate_subscription_id("invalid")?;                                    // Error
/// ```
pub fn validate_subscription_id(id: &str) -> Result<SubscriptionId, RpcError> {
    if id.is_empty() {
        return Err(RpcError::validation(format!(
            "Subscription ID cannot be empty (got: '{}')",
            id
        )));
    }
    SubscriptionId::parse_lenient(id)
        .map_err(|e| RpcError::validation(format!("Invalid subscription ID '{}': {}", id, e)))
}

/// Validate all inputs for an RPC call.
///
/// This is a convenience function that combines path and input size validation.
///
/// # Errors
///
/// Returns an error if either the path or input size validation fails.
///
/// # Examples
///
/// ```rust,ignore
/// let config = RpcConfig::default();
/// validate_rpc_input("user.get", &json!({"id": 1}), &config)?;
/// ```
pub fn validate_rpc_input(
    path: &str,
    input: &serde_json::Value,
    config: &RpcConfig,
) -> Result<(), RpcError> {
    validate_path(path)?;
    validate_input_size(input, config)?;
    Ok(())
}

// =============================================================================
// Property-Based Tests
// =============================================================================

#[cfg(test)]
mod rpc_validation_tests {
    use super::*;
    use proptest::prelude::*;
    use serde_json::json;

    // Property 3: Path validation rejects invalid patterns
    proptest! {
        #[test]
        fn prop_path_validation_rejects_invalid_patterns(
            s in ".*[^a-zA-Z0-9_.].*"
        ) {
            // Paths with invalid characters should be rejected
            if !s.is_empty() && (s.contains(|c: char| !c.is_ascii_alphanumeric() && c != '_' && c != '.')) {
                assert!(validate_path(&s).is_err());
            }
        }

        #[test]
        fn prop_path_validation_accepts_valid_patterns(
            s in "[a-zA-Z0-9_]+([.][a-zA-Z0-9_]+)*"
        ) {
            // Valid paths should be accepted
            assert!(validate_path(&s).is_ok());
        }

        #[test]
        fn prop_path_validation_rejects_empty(
            _unit in Just(())
        ) {
            assert!(validate_path("").is_err());
        }

        #[test]
        fn prop_path_validation_rejects_leading_dot(
            s in "[.][a-zA-Z0-9_.]+"
        ) {
            assert!(validate_path(&s).is_err());
        }

        #[test]
        fn prop_path_validation_rejects_trailing_dot(
            s in "[a-zA-Z0-9_.]+[.]"
        ) {
            assert!(validate_path(&s).is_err());
        }

        #[test]
        fn prop_path_validation_rejects_consecutive_dots(
            prefix in "[a-zA-Z0-9_]+",
            suffix in "[a-zA-Z0-9_]+"
        ) {
            let path = format!("{}..{}", prefix, suffix);
            assert!(validate_path(&path).is_err());
        }
    }

    // Property 4: Input size validation rejects oversized inputs
    proptest! {
        #[test]
        fn prop_input_size_validation_rejects_oversized(
            size in 1000usize..10000usize
        ) {
            let config = RpcConfig::default().with_max_input_size(100);
            let large_string = "a".repeat(size);
            let input = json!(large_string);
            assert!(validate_input_size(&input, &config).is_err());
        }

        #[test]
        fn prop_input_size_validation_accepts_small_inputs(
            size in 1usize..50usize
        ) {
            let config = RpcConfig::default().with_max_input_size(1000);
            let small_string = "a".repeat(size);
            let input = json!(small_string);
            assert!(validate_input_size(&input, &config).is_ok());
        }
    }

    // Property 5: Size validation error messages include sizes
    #[test]
    fn test_size_validation_error_includes_sizes() {
        let config = RpcConfig::default().with_max_input_size(10);
        let large_input = json!("a".repeat(100));
        let result = validate_input_size(&large_input, &config);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("bytes"));
        assert!(err.message.contains("exceeds"));
    }

    // Property 6: Subscription ID validation accepts both formats
    proptest! {
        #[test]
        fn prop_subscription_id_accepts_uuid_format(
            a in "[0-9a-f]{8}",
            b in "[0-9a-f]{4}",
            c in "[0-9a-f]{4}",
            d in "[0-9a-f]{4}",
            e in "[0-9a-f]{12}"
        ) {
            let uuid = format!("{}-{}-{}-{}-{}", a, b, c, d, e);
            assert!(validate_subscription_id(&uuid).is_ok());
        }

        #[test]
        fn prop_subscription_id_accepts_prefixed_format(
            a in "[0-9a-f]{8}",
            b in "[0-9a-f]{4}",
            c in "[0-9a-f]{4}",
            d in "[0-9a-f]{4}",
            e in "[0-9a-f]{12}"
        ) {
            let uuid = format!("sub_{}-{}-{}-{}-{}", a, b, c, d, e);
            assert!(validate_subscription_id(&uuid).is_ok());
        }
    }

    // Property 7: Malformed UUIDs are rejected with descriptive errors
    #[test]
    fn test_malformed_uuid_rejected() {
        let invalid_ids = vec![
            "not-a-uuid",
            "12345",
            "sub_invalid",
            "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx",
        ];
        for id in invalid_ids {
            let result = validate_subscription_id(id);
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(err.message.contains("Invalid subscription ID"));
            assert!(err.message.contains(id));
        }
    }

    // Property 8: Subscription ID normalization
    #[test]
    fn test_subscription_id_normalization() {
        let uuid = "01234567-89ab-7cde-8f01-234567890abc";
        let prefixed = format!("sub_{}", uuid);

        let id1 = validate_subscription_id(uuid).unwrap();
        let id2 = validate_subscription_id(&prefixed).unwrap();

        // Both should parse successfully (normalization happens in SubscriptionId)
        assert_eq!(id1.to_string(), id2.to_string());
    }

    // Property 2: Complete RPC request validation
    proptest! {
        #[test]
        fn prop_complete_rpc_validation(
            path in "[a-zA-Z0-9_]+([.][a-zA-Z0-9_]+)*",
            value in prop::option::of("[a-zA-Z0-9 ]{1,50}")
        ) {
            let config = RpcConfig::default();
            let input = json!(value);
            let result = validate_rpc_input(&path, &input, &config);
            assert!(result.is_ok());
        }

        #[test]
        fn prop_complete_rpc_validation_rejects_invalid_path(
            invalid_char in "[^a-zA-Z0-9_.]",
            suffix in "[a-zA-Z0-9_]+"
        ) {
            let config = RpcConfig::default();
            let path = format!("test{}path{}", invalid_char, suffix);
            let input = json!(null);
            let result = validate_rpc_input(&path, &input, &config);
            assert!(result.is_err());
        }
    }

    // Property 31: Validation error includes invalid value
    #[test]
    fn test_validation_error_includes_invalid_value() {
        let invalid_path = "invalid..path";
        let result = validate_path(invalid_path);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains(invalid_path) || err.message.contains("consecutive dots"));
    }

    #[test]
    fn test_validation_error_includes_invalid_char() {
        let invalid_path = "test@path";
        let result = validate_path(invalid_path);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("@") || err.message.contains("invalid character"));
    }
}
