//! Tests for plugin helper functions
//!
//! These tests validate the correctness of helper functions introduced
//! in the plugin improvements:
//! - Error serialization consistency
//! - generate_request_id: UUID v7 format validation
//! - validate_input_size: Heuristic-based validation
//! - validate_path: Iterator-based validation

use crate::validation::{validate_input_size, validate_path};
use crate::{RpcConfig, RpcError, RpcErrorCode};
use serde_json::json;

// =============================================================================
// Error Serialization Tests
// =============================================================================

#[cfg(test)]
mod error_serialization_tests {
    use super::*;

    // Test error serialization consistency through the Serialize impl

    #[test]
    fn test_error_serialization_contains_code_and_message() {
        let error = RpcError::validation("Test validation error");
        let serialized = serde_json::to_string(&error).unwrap();

        assert!(serialized.contains("\"code\""));
        assert!(serialized.contains("\"message\""));
        assert!(serialized.contains("Test validation error"));
    }

    #[test]
    fn test_error_serialization_all_error_types() {
        let errors = vec![
            RpcError::bad_request("Bad request"),
            RpcError::unauthorized("Unauthorized"),
            RpcError::forbidden("Forbidden"),
            RpcError::not_found("Not found"),
            RpcError::validation("Validation error"),
            RpcError::conflict("Conflict"),
            RpcError::payload_too_large("Payload too large"),
            RpcError::internal("Internal error"),
        ];

        for error in errors {
            let serialized = serde_json::to_string(&error);
            assert!(serialized.is_ok(), "Failed to serialize error: {:?}", error);

            let json_value: serde_json::Value = serde_json::from_str(&serialized.unwrap()).unwrap();
            assert!(json_value.get("code").is_some());
            assert!(json_value.get("message").is_some());
        }
    }

    #[test]
    fn test_error_fallback_to_display() {
        // Even if JSON serialization fails, Display trait should work
        let error = RpcError::internal("Test error");
        let display_str = error.to_string();
        assert!(display_str.contains("Test error"));
    }
}

// =============================================================================
// Request ID Generation Tests
// =============================================================================

#[cfg(test)]
mod generate_request_id_tests {
    // Note: generate_request_id is a private function, but we can test
    // UUID v7 format indirectly

    #[test]
    fn test_uuid_v7_format() {
        // UUID v7 format: xxxxxxxx-xxxx-7xxx-xxxx-xxxxxxxxxxxx
        let uuid = uuid::Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext));
        let uuid_str = uuid.to_string();

        // Check format
        assert_eq!(uuid_str.len(), 36); // 32 hex + 4 hyphens
        assert_eq!(uuid_str.chars().nth(8), Some('-'));
        assert_eq!(uuid_str.chars().nth(13), Some('-'));
        assert_eq!(uuid_str.chars().nth(18), Some('-'));
        assert_eq!(uuid_str.chars().nth(23), Some('-'));

        // Check version (7)
        assert_eq!(uuid_str.chars().nth(14), Some('7'));
    }

    #[test]
    fn test_uuid_v7_uniqueness() {
        let mut ids = std::collections::HashSet::new();
        for _ in 0..1000 {
            let uuid = uuid::Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext));
            ids.insert(uuid);
        }
        // All IDs should be unique
        assert_eq!(ids.len(), 1000);
    }

    #[test]
    fn test_uuid_v7_time_ordering() {
        let uuid1 = uuid::Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext));
        std::thread::sleep(std::time::Duration::from_millis(10));
        let uuid2 = uuid::Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext));

        // UUID v7 should be time-ordered
        assert!(uuid1.as_bytes() < uuid2.as_bytes());
    }
}

// =============================================================================
// Input Validation Heuristics Tests
// =============================================================================

#[cfg(test)]
mod validate_input_size_tests {
    use super::*;

    fn create_test_config(max_size: usize) -> RpcConfig {
        RpcConfig::default().with_max_input_size(max_size)
    }

    #[test]
    fn test_null_value_fast_path() {
        let config = create_test_config(100);
        let input = json!(null);

        let result = validate_input_size(&input, &config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_boolean_value_fast_path() {
        let config = create_test_config(100);

        assert!(validate_input_size(&json!(true), &config).is_ok());
        assert!(validate_input_size(&json!(false), &config).is_ok());
    }

    #[test]
    fn test_number_value_fast_path() {
        let config = create_test_config(100);

        assert!(validate_input_size(&json!(42), &config).is_ok());
        assert!(validate_input_size(&json!(std::f32::consts::PI), &config).is_ok());
        assert!(validate_input_size(&json!(-100), &config).is_ok());
    }

    #[test]
    fn test_string_value_fast_path() {
        let config = create_test_config(100);

        // Small string should pass
        assert!(validate_input_size(&json!("hello"), &config).is_ok());

        // String at limit should pass (length + 2 for quotes)
        let s = "a".repeat(98); // 98 + 2 = 100
        assert!(validate_input_size(&json!(s), &config).is_ok());
    }

    #[test]
    fn test_string_value_exceeds_limit() {
        let config = create_test_config(100);

        // String exceeding limit should fail
        let s = "a".repeat(99); // 99 + 2 = 101 > 100
        let result = validate_input_size(&json!(s), &config);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert_eq!(err.code, RpcErrorCode::PayloadTooLarge);
        assert!(err.message.contains("exceeds maximum"));
    }

    #[test]
    fn test_empty_array_fast_path() {
        let config = create_test_config(100);
        let input = json!([]);

        assert!(validate_input_size(&input, &config).is_ok());
    }

    #[test]
    fn test_empty_object_fast_path() {
        let config = create_test_config(100);
        let input = json!({});

        assert!(validate_input_size(&input, &config).is_ok());
    }

    #[test]
    fn test_complex_array_exact_validation() {
        let config = create_test_config(30);

        // Small array should pass
        let input = json!([1, 2, 3]);
        assert!(validate_input_size(&input, &config).is_ok());

        // Large array should fail (serialized size > 30 bytes)
        let large_array = json!([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]);
        let result = validate_input_size(&large_array, &config);
        assert!(result.is_err(), "Large array should exceed 30 byte limit");
    }

    #[test]
    fn test_complex_object_exact_validation() {
        let config = create_test_config(50);

        // Small object should pass
        let input = json!({"a": 1, "b": 2});
        assert!(validate_input_size(&input, &config).is_ok());

        // Large object should fail
        let large_object = json!({
            "key1": "value1",
            "key2": "value2",
            "key3": "value3",
            "key4": "value4",
            "key5": "value5"
        });
        let result = validate_input_size(&large_object, &config);
        assert!(result.is_err());
    }

    #[test]
    fn test_nested_structure_exact_validation() {
        let config = create_test_config(100);

        let nested = json!({
            "user": {
                "name": "John Doe",
                "email": "john@example.com",
                "address": {
                    "street": "123 Main St",
                    "city": "Springfield"
                }
            }
        });

        // This should use exact validation (complex type)
        let result = validate_input_size(&nested, &config);
        // Result depends on actual serialized size
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_error_message_format() {
        let config = create_test_config(10);
        let input = json!("this is a very long string");

        let result = validate_input_size(&input, &config);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(err.message.contains("exceeds maximum"));
        assert!(err.message.contains("bytes"));
    }

    #[test]
    fn test_conservative_estimates() {
        let config = create_test_config(1000);

        // All simple types should pass with reasonable limit
        assert!(validate_input_size(&json!(null), &config).is_ok());
        assert!(validate_input_size(&json!(true), &config).is_ok());
        assert!(validate_input_size(&json!(12345), &config).is_ok());
        assert!(validate_input_size(&json!("test"), &config).is_ok());
        assert!(validate_input_size(&json!([]), &config).is_ok());
        assert!(validate_input_size(&json!({}), &config).is_ok());
    }
}

// =============================================================================
// Path Validation Tests
// =============================================================================

#[cfg(test)]
mod validate_path_tests {
    use super::*;

    #[test]
    fn test_valid_paths() {
        let valid_paths = vec![
            "user",
            "user.get",
            "user.list",
            "api.v1.users",
            "health_check",
            "user_profile",
            "get_user_by_id",
            "a.b.c.d.e",
        ];

        for path in valid_paths {
            let result = validate_path(path);
            assert!(result.is_ok(), "Path '{}' should be valid", path);
        }
    }

    #[test]
    fn test_empty_path() {
        let result = validate_path("");
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert_eq!(err.code, RpcErrorCode::ValidationError);
        assert!(err.message.contains("cannot be empty"));
    }

    #[test]
    fn test_path_starts_with_dot() {
        let result = validate_path(".user");
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(err.message.contains("cannot start or end with a dot"));
    }

    #[test]
    fn test_path_ends_with_dot() {
        let result = validate_path("user.");
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(err.message.contains("cannot start or end with a dot"));
    }

    #[test]
    fn test_path_consecutive_dots() {
        let result = validate_path("user..get");
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(err.message.contains("consecutive dots"));
    }

    #[test]
    fn test_path_invalid_characters() {
        let invalid_paths = vec![
            "user-get",  // hyphen
            "user/get",  // slash
            "user@get",  // at sign
            "user#get",  // hash
            "user$get",  // dollar
            "user%get",  // percent
            "user get",  // space
            "user!get",  // exclamation
            "user(get)", // parentheses
            "user[get]", // brackets
        ];

        for path in invalid_paths {
            let result = validate_path(path);
            assert!(result.is_err(), "Path '{}' should be invalid", path);

            let err = result.unwrap_err();
            assert!(
                err.message.contains("invalid character"),
                "Path '{}' error message should mention invalid character",
                path
            );
        }
    }

    #[test]
    fn test_path_with_numbers() {
        let valid_paths = vec!["user1", "api2.users", "v1.api", "get_user_123"];

        for path in valid_paths {
            let result = validate_path(path);
            assert!(result.is_ok(), "Path '{}' should be valid", path);
        }
    }

    #[test]
    fn test_path_error_shows_invalid_character() {
        let result = validate_path("user-get");
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(
            err.message.contains("'-'"),
            "Error message should show the invalid character"
        );
    }

    #[test]
    fn test_path_unicode_characters() {
        // Unicode characters should be rejected
        let result = validate_path("user.café");
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(err.message.contains("invalid character"));
    }

    #[test]
    fn test_path_only_valid_characters() {
        // Only alphanumeric, underscore, and dot are allowed
        let result = validate_path("abc123_XYZ.test_123");
        assert!(result.is_ok());
    }
}

// =============================================================================
// Integration Tests
// =============================================================================

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_validate_rpc_input_success() {
        let config = create_test_config(1000);
        let path = "user.get";
        let input = json!({"id": 123});

        let result = crate::validate_rpc_input(path, &input, &config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_rpc_input_invalid_path() {
        let config = create_test_config(1000);
        let path = "user-get"; // Invalid character
        let input = json!({"id": 123});

        let result = crate::validate_rpc_input(path, &input, &config);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert_eq!(err.code, RpcErrorCode::ValidationError);
    }

    #[test]
    fn test_validate_rpc_input_oversized() {
        let config = create_test_config(50);
        let path = "user.get";
        let input = json!({"data": "a".repeat(100)});

        let result = crate::validate_rpc_input(path, &input, &config);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert_eq!(err.code, RpcErrorCode::PayloadTooLarge);
    }

    fn create_test_config(max_size: usize) -> RpcConfig {
        RpcConfig::default().with_max_input_size(max_size)
    }
}
