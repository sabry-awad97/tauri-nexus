//! Property-based tests for input validation
//!
//! These tests validate the correctness properties of input validation:
//! - Property 6: Input Validation Rejection

use crate::plugin::{validate_input_size, validate_path, validate_subscription_id};
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
    /// **Validates: Requirements 5.1, 5.2, 5.3**
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
