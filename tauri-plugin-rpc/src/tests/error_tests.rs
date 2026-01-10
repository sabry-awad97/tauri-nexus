//! Property-based tests for error handling
//!
//! These tests validate the correctness properties of error serialization
//! and error code handling.

use crate::{RpcError, RpcErrorCode};
use proptest::prelude::*;

/// Strategy to generate arbitrary RpcErrorCode values
fn arb_error_code() -> impl Strategy<Value = RpcErrorCode> {
    prop_oneof![
        Just(RpcErrorCode::BadRequest),
        Just(RpcErrorCode::Unauthorized),
        Just(RpcErrorCode::Forbidden),
        Just(RpcErrorCode::NotFound),
        Just(RpcErrorCode::ValidationError),
        Just(RpcErrorCode::Conflict),
        Just(RpcErrorCode::PayloadTooLarge),
        Just(RpcErrorCode::InternalError),
        Just(RpcErrorCode::NotImplemented),
        Just(RpcErrorCode::ServiceUnavailable),
        Just(RpcErrorCode::ProcedureNotFound),
        Just(RpcErrorCode::SubscriptionError),
        Just(RpcErrorCode::MiddlewareError),
        Just(RpcErrorCode::SerializationError),
    ]
}

/// Strategy to generate arbitrary RpcError values
fn arb_rpc_error() -> impl Strategy<Value = RpcError> {
    (
        arb_error_code(),
        ".*",                                  // arbitrary message
        proptest::option::of(any::<String>()), // optional cause
    )
        .prop_map(|(code, message, cause)| {
            let mut error = RpcError::new(code, message);
            if let Some(c) = cause {
                error = error.with_cause(c);
            }
            error
        })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Property 3: Error Serialization Completeness**
    /// *For any* RpcError returned by a handler, the serialized JSON output SHALL
    /// contain at minimum the `code` and `message` fields, and the error SHALL be
    /// deserializable back to an equivalent RpcError.
    /// **Validates: Requirements 3.1, 9.2**
    /// **Feature: tauri-rpc-plugin-optimization, Property 3: Error Serialization Completeness**
    #[test]
    fn prop_error_serialization_roundtrip(error in arb_rpc_error()) {
        // Serialize to JSON
        let json = serde_json::to_value(&error).expect("Failed to serialize error");

        // Verify required fields are present
        prop_assert!(json.get("code").is_some(), "Serialized error must have 'code' field");
        prop_assert!(json.get("message").is_some(), "Serialized error must have 'message' field");

        // Deserialize back
        let restored: RpcError = serde_json::from_value(json.clone())
            .expect("Failed to deserialize error");

        // Verify code and message are preserved
        prop_assert_eq!(
            error.code, restored.code,
            "Error code must be preserved after round-trip"
        );
        prop_assert_eq!(
            error.message, restored.message,
            "Error message must be preserved after round-trip"
        );
        prop_assert_eq!(
            error.cause, restored.cause,
            "Error cause must be preserved after round-trip"
        );
    }

    /// Test that error codes serialize to SCREAMING_SNAKE_CASE strings
    /// **Feature: tauri-rpc-plugin-optimization, Property 3: Error Serialization Completeness**
    #[test]
    fn prop_error_code_serializes_to_screaming_snake_case(code in arb_error_code()) {
        let error = RpcError::new(code, "test message");
        let json = serde_json::to_value(&error).expect("Failed to serialize error");

        let code_str = json.get("code")
            .and_then(|v| v.as_str())
            .expect("Code should be a string");

        // Verify it's SCREAMING_SNAKE_CASE (all uppercase with underscores)
        prop_assert!(
            code_str.chars().all(|c| c.is_uppercase() || c == '_'),
            "Error code '{}' should be SCREAMING_SNAKE_CASE", code_str
        );

        // Verify it matches the as_str() representation
        prop_assert_eq!(
            code_str, code.as_str(),
            "Serialized code should match as_str()"
        );
    }

    /// Test that all error codes can be deserialized from their string representation
    /// **Feature: tauri-rpc-plugin-optimization, Property 3: Error Serialization Completeness**
    #[test]
    fn prop_error_code_deserializes_from_string(code in arb_error_code()) {
        let code_str = code.as_str();
        let json_str = format!("\"{}\"", code_str);

        let restored: RpcErrorCode = serde_json::from_str(&json_str)
            .expect("Failed to deserialize error code");

        prop_assert_eq!(code, restored, "Error code should round-trip through string");
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_error_with_details_serializes() {
        let error = RpcError::validation("Invalid input")
            .with_details(serde_json::json!({"field": "email", "reason": "invalid format"}));

        let json = serde_json::to_value(&error).unwrap();

        assert_eq!(json["code"], "VALIDATION_ERROR");
        assert_eq!(json["message"], "Invalid input");
        assert!(json["details"].is_object());
        assert_eq!(json["details"]["field"], "email");
    }

    #[test]
    fn test_error_sanitize_removes_internal_details() {
        let error = RpcError::internal("Database connection failed")
            .with_details(serde_json::json!({"connection_string": "secret"}))
            .with_cause("Connection timeout after 30s");

        let sanitized = error.sanitize();

        assert_eq!(sanitized.code, RpcErrorCode::InternalError);
        assert_eq!(sanitized.message, "An internal error occurred");
        assert!(sanitized.details.is_none());
        assert!(sanitized.cause.is_none());
    }

    #[test]
    fn test_error_display_format() {
        let error = RpcError::not_found("User not found");
        let display = format!("{}", error);
        assert_eq!(display, "[NOT_FOUND] User not found");
    }

    #[test]
    fn test_error_code_classification() {
        assert!(RpcErrorCode::BadRequest.is_client_error());
        assert!(RpcErrorCode::NotFound.is_client_error());
        assert!(!RpcErrorCode::BadRequest.is_server_error());

        assert!(RpcErrorCode::InternalError.is_server_error());
        assert!(RpcErrorCode::ServiceUnavailable.is_server_error());
        assert!(!RpcErrorCode::InternalError.is_client_error());
    }
}
