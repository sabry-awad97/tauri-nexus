//! Property-based tests for input validation
//!
//! These tests validate the correctness properties of input validation:
//! - Property 6: Input Validation Rejection
//! - Property 1: Validation Rules Correctness

use crate::validation::{FieldError, ValidationRules};
use crate::validation::{validate_input_size, validate_path, validate_subscription_id};
use crate::{RpcConfig, RpcErrorCode};
use proptest::prelude::*;

// =============================================================================
// Property 6: Input Validation Rejection
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Property 6: Input Validation Rejection**
    /// *For any* input that violates validation rules (invalid path characters,
    /// exceeds size limit, invalid subscription ID format), the system SHALL
    /// reject the request with an appropriate error before processing.
    /// **Feature: tauri-rpc-plugin-optimization, Property 6: Input Validation Rejection**

    // --- Path Validation Tests ---

    /// Valid paths should be accepted
    #[test]
    fn prop_valid_paths_accepted(
        segments in prop::collection::vec("[a-z][a-z0-9_]{0,10}", 1..5)
    ) {
        let path = segments.join(".");
        let result = validate_path(&path);
        prop_assert!(result.is_ok(), "Valid path '{}' should be accepted", path);
    }

    /// Paths with invalid characters should be rejected
    #[test]
    fn prop_invalid_char_paths_rejected(
        prefix in "[a-z]{1,5}",
        invalid_char in "[^a-zA-Z0-9_.]",
        suffix in "[a-z]{1,5}"
    ) {
        let path = format!("{}{}{}", prefix, invalid_char, suffix);
        let result = validate_path(&path);
        prop_assert!(
            result.is_err(),
            "Path '{}' with invalid char should be rejected", path
        );
        if let Err(e) = result {
            prop_assert_eq!(e.code, RpcErrorCode::ValidationError);
        }
    }

    /// Empty paths should be rejected
    #[test]
    fn prop_empty_path_rejected(_dummy in 0..10) {
        let result = validate_path("");
        prop_assert!(result.is_err(), "Empty path should be rejected");
        if let Err(e) = result {
            prop_assert_eq!(e.code, RpcErrorCode::ValidationError);
        }
    }

    /// Paths starting with dot should be rejected
    #[test]
    fn prop_leading_dot_path_rejected(suffix in "[a-z]{1,10}") {
        let path = format!(".{}", suffix);
        let result = validate_path(&path);
        prop_assert!(result.is_err(), "Path '{}' starting with dot should be rejected", path);
        if let Err(e) = result {
            prop_assert_eq!(e.code, RpcErrorCode::ValidationError);
        }
    }

    /// Paths ending with dot should be rejected
    #[test]
    fn prop_trailing_dot_path_rejected(prefix in "[a-z]{1,10}") {
        let path = format!("{}.", prefix);
        let result = validate_path(&path);
        prop_assert!(result.is_err(), "Path '{}' ending with dot should be rejected", path);
        if let Err(e) = result {
            prop_assert_eq!(e.code, RpcErrorCode::ValidationError);
        }
    }

    /// Paths with consecutive dots should be rejected
    #[test]
    fn prop_consecutive_dots_rejected(
        prefix in "[a-z]{1,5}",
        suffix in "[a-z]{1,5}"
    ) {
        let path = format!("{}..{}", prefix, suffix);
        let result = validate_path(&path);
        prop_assert!(result.is_err(), "Path '{}' with consecutive dots should be rejected", path);
        if let Err(e) = result {
            prop_assert_eq!(e.code, RpcErrorCode::ValidationError);
        }
    }

    // --- Input Size Validation Tests ---

    /// Inputs within size limit should be accepted
    #[test]
    fn prop_small_inputs_accepted(data in "[a-z]{0,100}") {
        let config = RpcConfig::default(); // 1MB limit
        let input = serde_json::json!({"data": data});
        let result = validate_input_size(&input, &config);
        prop_assert!(result.is_ok(), "Small input should be accepted");
    }

    /// Inputs exceeding size limit should be rejected
    #[test]
    fn prop_oversized_inputs_rejected(size_multiplier in 2usize..10) {
        let config = RpcConfig::new().with_max_input_size(100); // 100 bytes limit
        // Create input larger than limit
        let large_data: String = "x".repeat(100 * size_multiplier);
        let input = serde_json::json!({"data": large_data});
        let result = validate_input_size(&input, &config);
        prop_assert!(result.is_err(), "Oversized input should be rejected");
        if let Err(e) = result {
            prop_assert_eq!(e.code, RpcErrorCode::PayloadTooLarge);
        }
    }

    // --- Subscription ID Validation Tests ---

    /// Valid subscription IDs should be accepted
    #[test]
    fn prop_valid_subscription_ids_accepted(_dummy in 0..100) {
        // Generate a valid UUID v7 and test both formats
        let id = crate::SubscriptionId::new();

        // With prefix
        let with_prefix = id.to_string();
        let result = validate_subscription_id(&with_prefix);
        prop_assert!(result.is_ok(), "Valid ID '{}' should be accepted", with_prefix);

        // Without prefix (just UUID)
        let without_prefix = &with_prefix[4..]; // Remove "sub_"
        let result = validate_subscription_id(without_prefix);
        prop_assert!(result.is_ok(), "Valid UUID '{}' should be accepted", without_prefix);
    }

    /// Invalid subscription IDs should be rejected
    #[test]
    fn prop_invalid_subscription_ids_rejected(invalid_id in "[^0-9a-f-]{5,20}") {
        // Skip if it accidentally generates a valid UUID
        if uuid::Uuid::parse_str(&invalid_id).is_ok() {
            return Ok(());
        }

        let result = validate_subscription_id(&invalid_id);
        prop_assert!(result.is_err(), "Invalid ID '{}' should be rejected", invalid_id);
        if let Err(e) = result {
            prop_assert_eq!(e.code, RpcErrorCode::ValidationError);
        }
    }

    /// Empty subscription IDs should be rejected
    #[test]
    fn prop_empty_subscription_id_rejected(_dummy in 0..10) {
        let result = validate_subscription_id("");
        prop_assert!(result.is_err(), "Empty subscription ID should be rejected");
        if let Err(e) = result {
            prop_assert_eq!(e.code, RpcErrorCode::ValidationError);
        }
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_valid_simple_path() {
        assert!(validate_path("health").is_ok());
        assert!(validate_path("users").is_ok());
        assert!(validate_path("api_v1").is_ok());
    }

    #[test]
    fn test_valid_nested_path() {
        assert!(validate_path("users.get").is_ok());
        assert!(validate_path("api.v1.users.list").is_ok());
        assert!(validate_path("health_check.status").is_ok());
    }

    #[test]
    fn test_invalid_path_characters() {
        assert!(validate_path("users/get").is_err());
        assert!(validate_path("users:get").is_err());
        assert!(validate_path("users get").is_err());
        assert!(validate_path("users@get").is_err());
    }

    #[test]
    fn test_input_size_at_boundary() {
        let config = RpcConfig::new().with_max_input_size(50);

        // Just under limit
        let small = serde_json::json!({"a": "b"});
        assert!(validate_input_size(&small, &config).is_ok());

        // Over limit
        let large = serde_json::json!({"data": "x".repeat(100)});
        assert!(validate_input_size(&large, &config).is_err());
    }
}

// =============================================================================
// Property 1: Validation Rules Correctness
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Property 1: Validation Rules Correctness**
    /// *For any* input value, validation rules SHALL correctly identify valid and invalid inputs.
    /// **Feature: tauri-rpc-framework, Property 1: Validation Rules Correctness**
    /// **Validates: Requirements 6.4**

    // --- Required Rule Tests ---

    /// Non-empty strings should pass required validation
    #[test]
    fn prop_required_accepts_non_empty(value in "[a-zA-Z0-9]{1,100}") {
        let result = ValidationRules::new()
            .required("field", &value)
            .build();
        prop_assert!(result.is_valid(), "Non-empty string '{}' should pass required", value);
    }

    /// Empty or whitespace-only strings should fail required validation
    #[test]
    fn prop_required_rejects_empty_or_whitespace(spaces in " {0,10}") {
        let result = ValidationRules::new()
            .required("field", &spaces)
            .build();
        prop_assert!(!result.is_valid(), "Empty/whitespace string should fail required");
        prop_assert_eq!(result.errors.len(), 1);
        prop_assert_eq!(result.errors[0].code.as_str(), "required");
    }

    // --- Min Length Rule Tests ---

    /// Strings meeting minimum length should pass
    #[test]
    fn prop_min_length_accepts_valid(
        min in 1usize..20,
        extra in 0usize..50
    ) {
        let value: String = "x".repeat(min + extra);
        let result = ValidationRules::new()
            .min_length("field", &value, min)
            .build();
        prop_assert!(result.is_valid(), "String of length {} should pass min_length {}", value.len(), min);
    }

    /// Strings below minimum length should fail
    #[test]
    fn prop_min_length_rejects_short(
        min in 2usize..20,
        deficit in 1usize..10
    ) {
        let actual_len = min.saturating_sub(deficit).max(0);
        let value: String = "x".repeat(actual_len);
        if value.len() < min {
            let result = ValidationRules::new()
                .min_length("field", &value, min)
                .build();
            prop_assert!(!result.is_valid(), "String of length {} should fail min_length {}", value.len(), min);
            prop_assert_eq!(result.errors[0].code.as_str(), "min_length");
        }
    }

    // --- Max Length Rule Tests ---

    /// Strings within maximum length should pass
    #[test]
    fn prop_max_length_accepts_valid(
        max in 1usize..100,
        len in 0usize..100
    ) {
        let actual_len = len.min(max);
        let value: String = "x".repeat(actual_len);
        let result = ValidationRules::new()
            .max_length("field", &value, max)
            .build();
        prop_assert!(result.is_valid(), "String of length {} should pass max_length {}", value.len(), max);
    }

    /// Strings exceeding maximum length should fail
    #[test]
    fn prop_max_length_rejects_long(
        max in 1usize..50,
        excess in 1usize..50
    ) {
        let value: String = "x".repeat(max + excess);
        let result = ValidationRules::new()
            .max_length("field", &value, max)
            .build();
        prop_assert!(!result.is_valid(), "String of length {} should fail max_length {}", value.len(), max);
        prop_assert_eq!(result.errors[0].code.as_str(), "max_length");
    }

    // --- Range Rule Tests ---

    /// Numbers within range should pass
    #[test]
    fn prop_range_accepts_valid(
        min in -1000i64..0,
        max in 0i64..1000,
        offset in 0i64..1000
    ) {
        let value = min + (offset % (max - min + 1));
        let result = ValidationRules::new()
            .range("field", value, min, max)
            .build();
        prop_assert!(result.is_valid(), "Value {} should pass range [{}, {}]", value, min, max);
    }

    /// Numbers below range should fail
    #[test]
    fn prop_range_rejects_below(
        min in 0i64..100,
        max in 100i64..200,
        deficit in 1i64..100
    ) {
        let value = min - deficit;
        let result = ValidationRules::new()
            .range("field", value, min, max)
            .build();
        prop_assert!(!result.is_valid(), "Value {} should fail range [{}, {}]", value, min, max);
        prop_assert_eq!(result.errors[0].code.as_str(), "range");
    }

    /// Numbers above range should fail
    #[test]
    fn prop_range_rejects_above(
        min in 0i64..100,
        max in 100i64..200,
        excess in 1i64..100
    ) {
        let value = max + excess;
        let result = ValidationRules::new()
            .range("field", value, min, max)
            .build();
        prop_assert!(!result.is_valid(), "Value {} should fail range [{}, {}]", value, min, max);
        prop_assert_eq!(result.errors[0].code.as_str(), "range");
    }

    // --- Pattern Rule Tests ---

    /// Strings matching pattern should pass
    #[test]
    fn prop_pattern_accepts_matching(digits in "[0-9]{3,10}") {
        let result = ValidationRules::new()
            .pattern("field", &digits, r"^[0-9]+$")
            .build();
        prop_assert!(result.is_valid(), "Digits '{}' should match pattern ^[0-9]+$", digits);
    }

    /// Strings not matching pattern should fail
    #[test]
    fn prop_pattern_rejects_non_matching(letters in "[a-zA-Z]{1,10}") {
        let result = ValidationRules::new()
            .pattern("field", &letters, r"^[0-9]+$")
            .build();
        prop_assert!(!result.is_valid(), "Letters '{}' should not match pattern ^[0-9]+$", letters);
        prop_assert_eq!(result.errors[0].code.as_str(), "pattern");
    }

    // --- Email Rule Tests ---

    /// Valid email formats should pass
    #[test]
    fn prop_email_accepts_valid(
        local in "[a-z]{1,10}",
        domain in "[a-z]{1,10}",
        tld in "[a-z]{2,4}"
    ) {
        let email = format!("{}@{}.{}", local, domain, tld);
        let result = ValidationRules::new()
            .email("field", &email)
            .build();
        prop_assert!(result.is_valid(), "Email '{}' should be valid", email);
    }

    /// Strings without @ should fail email validation
    #[test]
    fn prop_email_rejects_no_at(value in "[a-z]{1,20}") {
        if !value.contains('@') {
            let result = ValidationRules::new()
                .email("field", &value)
                .build();
            prop_assert!(!result.is_valid(), "String '{}' without @ should fail email", value);
            prop_assert_eq!(result.errors[0].code.as_str(), "email");
        }
    }

    /// Strings with @ but no domain dot should fail
    #[test]
    fn prop_email_rejects_no_domain_dot(
        local in "[a-z]{1,10}",
        domain in "[a-z]{1,10}"
    ) {
        let email = format!("{}@{}", local, domain);
        let result = ValidationRules::new()
            .email("field", &email)
            .build();
        prop_assert!(!result.is_valid(), "Email '{}' without domain dot should fail", email);
        prop_assert_eq!(result.errors[0].code.as_str(), "email");
    }

    // --- Multiple Rules Tests ---

    /// Multiple validation errors should be aggregated
    #[test]
    fn prop_multiple_errors_aggregated(
        short_name in "[a-z]{0,1}",
        invalid_email in "[a-z]{1,5}",
        out_of_range in 200i64..500
    ) {
        let result = ValidationRules::new()
            .min_length("name", &short_name, 2)
            .email("email", &invalid_email)
            .range("age", out_of_range, 0, 150)
            .build();

        // Count expected errors
        let mut expected_errors = 0;
        if short_name.len() < 2 { expected_errors += 1; }
        if !invalid_email.contains('@') { expected_errors += 1; }
        if !(0..=150).contains(&out_of_range) { expected_errors += 1; }

        prop_assert_eq!(
            result.errors.len(),
            expected_errors,
            "Should have {} errors for invalid inputs",
            expected_errors
        );
    }
}

#[cfg(test)]
mod validation_rules_unit_tests {
    use super::*;

    #[test]
    fn test_required_empty_string() {
        let result = ValidationRules::new().required("name", "").build();
        assert!(!result.is_valid());
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].field, "name");
        assert_eq!(result.errors[0].code, "required");
    }

    #[test]
    fn test_required_whitespace_only() {
        let result = ValidationRules::new().required("name", "   ").build();
        assert!(!result.is_valid());
        assert_eq!(result.errors[0].code, "required");
    }

    #[test]
    fn test_required_valid() {
        let result = ValidationRules::new().required("name", "John").build();
        assert!(result.is_valid());
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_min_length_boundary() {
        // Exactly at minimum
        let result = ValidationRules::new().min_length("name", "ab", 2).build();
        assert!(result.is_valid());

        // One below minimum
        let result = ValidationRules::new().min_length("name", "a", 2).build();
        assert!(!result.is_valid());
    }

    #[test]
    fn test_max_length_boundary() {
        // Exactly at maximum
        let result = ValidationRules::new().max_length("name", "abc", 3).build();
        assert!(result.is_valid());

        // One above maximum
        let result = ValidationRules::new().max_length("name", "abcd", 3).build();
        assert!(!result.is_valid());
    }

    #[test]
    fn test_range_boundaries() {
        // At minimum
        let result = ValidationRules::new().range("age", 0, 0, 100).build();
        assert!(result.is_valid());

        // At maximum
        let result = ValidationRules::new().range("age", 100, 0, 100).build();
        assert!(result.is_valid());

        // Below minimum
        let result = ValidationRules::new().range("age", -1, 0, 100).build();
        assert!(!result.is_valid());

        // Above maximum
        let result = ValidationRules::new().range("age", 101, 0, 100).build();
        assert!(!result.is_valid());
    }

    #[test]
    fn test_email_valid_formats() {
        let valid_emails = vec!["test@example.com", "user.name@domain.org", "a@b.co"];
        for email in valid_emails {
            let result = ValidationRules::new().email("email", email).build();
            assert!(result.is_valid(), "Email '{}' should be valid", email);
        }
    }

    #[test]
    fn test_email_invalid_formats() {
        let invalid_emails = vec![
            "notanemail",
            "@nodomain.com",
            "noat.com",
            "missing@domain",
            "trailing@dot.",
        ];
        for email in invalid_emails {
            let result = ValidationRules::new().email("email", email).build();
            assert!(!result.is_valid(), "Email '{}' should be invalid", email);
        }
    }

    #[test]
    fn test_pattern_valid() {
        let result = ValidationRules::new()
            .pattern("phone", "1234567890", r"^[0-9]{10}$")
            .build();
        assert!(result.is_valid());
    }

    #[test]
    fn test_pattern_invalid() {
        let result = ValidationRules::new()
            .pattern("phone", "123-456-7890", r"^[0-9]{10}$")
            .build();
        assert!(!result.is_valid());
        assert_eq!(result.errors[0].code, "pattern");
    }

    #[test]
    fn test_multiple_errors() {
        let result = ValidationRules::new()
            .required("name", "")
            .email("email", "invalid")
            .range("age", 200, 0, 150)
            .build();

        assert!(!result.is_valid());
        assert_eq!(result.errors.len(), 3);

        let fields: Vec<&str> = result.errors.iter().map(|e| e.field.as_str()).collect();
        assert!(fields.contains(&"name"));
        assert!(fields.contains(&"email"));
        assert!(fields.contains(&"age"));
    }

    #[test]
    fn test_custom_validation() {
        let value = 5;
        let result = ValidationRules::new()
            .custom("value", || value % 2 == 0, "Value must be even")
            .build();
        assert!(!result.is_valid());
        assert_eq!(result.errors[0].code, "custom");
    }

    #[test]
    fn test_validation_result_merge() {
        let result1 = ValidationRules::new().required("name", "").build();
        let result2 = ValidationRules::new().email("email", "invalid").build();

        let merged = result1.merge(result2);
        assert!(!merged.is_valid());
        assert_eq!(merged.errors.len(), 2);
    }

    #[test]
    fn test_field_error_constructors() {
        let err = FieldError::required("name");
        assert_eq!(err.field, "name");
        assert_eq!(err.code, "required");

        let err = FieldError::min_length("name", 5);
        assert_eq!(err.code, "min_length");
        assert!(err.message.contains("5"));

        let err = FieldError::email("email");
        assert_eq!(err.code, "email");
    }
}
