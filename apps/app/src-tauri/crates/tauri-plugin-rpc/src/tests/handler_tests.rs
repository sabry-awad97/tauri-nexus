//! Handler tests - Unit type serialization and deserialization
//!
//! Tests that handlers correctly handle unit type `()` inputs,
//! which is critical for procedures that take no input.

use proptest::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

// =============================================================================
// Unit Type Serialization Tests
// =============================================================================

/// Test that `null` deserializes to unit type `()`
#[test]
fn test_null_deserializes_to_unit() {
    let null_value: Value = json!(null);
    let result: Result<(), _> = serde_json::from_value(null_value);
    assert!(result.is_ok(), "null should deserialize to ()");
}

/// Test that empty object `{}` does NOT deserialize to unit type `()`
#[test]
fn test_empty_object_fails_to_deserialize_to_unit() {
    let empty_obj: Value = json!({});
    let result: Result<(), _> = serde_json::from_value(empty_obj);
    assert!(result.is_err(), "empty object should NOT deserialize to ()");

    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("invalid type"),
        "Error should mention invalid type: {}",
        err
    );
}

/// Test that unit type `()` serializes to `null`
#[test]
fn test_unit_serializes_to_null() {
    let unit: () = ();
    let result = serde_json::to_value(unit).unwrap();
    assert_eq!(result, json!(null), "() should serialize to null");
}

// =============================================================================
// Handler Input Deserialization Tests
// =============================================================================

/// Simulates how the handler deserializes input
fn deserialize_handler_input<T: for<'de> Deserialize<'de>>(input: Value) -> Result<T, String> {
    serde_json::from_value(input).map_err(|e| e.to_string())
}

#[test]
fn test_handler_accepts_null_for_unit_input() {
    let result = deserialize_handler_input::<()>(json!(null));
    assert!(result.is_ok(), "Handler should accept null for () input");
}

#[test]
fn test_handler_rejects_empty_object_for_unit_input() {
    let result = deserialize_handler_input::<()>(json!({}));
    assert!(result.is_err(), "Handler should reject {{}} for () input");
}

#[test]
fn test_handler_rejects_array_for_unit_input() {
    let result = deserialize_handler_input::<()>(json!([]));
    assert!(result.is_err(), "Handler should reject [] for () input");
}

#[test]
fn test_handler_rejects_string_for_unit_input() {
    let result = deserialize_handler_input::<()>(json!(""));
    assert!(result.is_err(), "Handler should reject string for () input");
}

#[test]
fn test_handler_rejects_number_for_unit_input() {
    let result = deserialize_handler_input::<()>(json!(0));
    assert!(result.is_err(), "Handler should reject number for () input");
}

// =============================================================================
// Struct Input Tests (for comparison)
// =============================================================================

#[derive(Debug, Deserialize, Serialize, PartialEq)]
struct TestInput {
    name: String,
}

#[test]
fn test_handler_accepts_object_for_struct_input() {
    let result = deserialize_handler_input::<TestInput>(json!({"name": "test"}));
    assert!(
        result.is_ok(),
        "Handler should accept object for struct input"
    );
    assert_eq!(result.unwrap().name, "test");
}

#[test]
fn test_handler_rejects_null_for_struct_input() {
    let result = deserialize_handler_input::<TestInput>(json!(null));
    assert!(
        result.is_err(),
        "Handler should reject null for struct input"
    );
}

// =============================================================================
// Optional Input Tests
// =============================================================================

#[derive(Debug, Deserialize, Serialize, PartialEq)]
struct OptionalInput {
    #[serde(default)]
    name: Option<String>,
}

#[test]
fn test_handler_accepts_empty_object_for_optional_fields() {
    let result = deserialize_handler_input::<OptionalInput>(json!({}));
    assert!(
        result.is_ok(),
        "Handler should accept {{}} for struct with optional fields"
    );
    assert_eq!(result.unwrap().name, None);
}

#[test]
fn test_handler_accepts_partial_object_for_optional_fields() {
    let result = deserialize_handler_input::<OptionalInput>(json!({"name": "test"}));
    assert!(result.is_ok());
    assert_eq!(result.unwrap().name, Some("test".to_string()));
}

// =============================================================================
// Property-Based Tests
// =============================================================================

proptest! {
    /// **Property 14: Unit Type Deserialization Rejection**
    /// *For any* non-null JSON value (string, number, boolean, array, object),
    /// attempting to deserialize it as unit type `()` SHALL fail with a deserialization error.
    /// **Feature: tauri-rpc-plugin-optimization, Property 14: Unit Type Deserialization Rejection**
    #[test]
    fn prop_non_null_values_fail_for_unit_type(
        s in ".*",
        n in any::<i64>(),
        b in any::<bool>(),
    ) {
        // String values should fail
        let string_result = deserialize_handler_input::<()>(json!(s));
        prop_assert!(string_result.is_err(), "String '{}' should not deserialize to ()", s);

        // Number values should fail
        let number_result = deserialize_handler_input::<()>(json!(n));
        prop_assert!(number_result.is_err(), "Number {} should not deserialize to ()", n);

        // Boolean values should fail
        let bool_result = deserialize_handler_input::<()>(json!(b));
        prop_assert!(bool_result.is_err(), "Boolean {} should not deserialize to ()", b);
    }

    /// Property: Round-trip serialization of unit type always produces null
    #[test]
    fn prop_unit_roundtrip_is_null(_dummy in 0..1i32) {
        let unit: () = ();
        let serialized = serde_json::to_value(unit).unwrap();
        prop_assert_eq!(serialized.clone(), json!(null));

        let deserialized: () = serde_json::from_value(serialized).unwrap();
        prop_assert_eq!(deserialized, ());
    }
}

// =============================================================================
// Integration-style Tests
// =============================================================================

/// Simulates the full handler call flow
async fn simulate_handler_call<Input, Output>(
    input_json: Value,
    handler: impl Fn(Input) -> Output,
) -> Result<Value, String>
where
    Input: for<'de> Deserialize<'de>,
    Output: Serialize,
{
    let input: Input = serde_json::from_value(input_json).map_err(|e| e.to_string())?;
    let output = handler(input);
    serde_json::to_value(output).map_err(|e| e.to_string())
}

#[tokio::test]
async fn test_void_handler_with_null_input() {
    let result = simulate_handler_call(json!(null), |_: ()| "success").await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), json!("success"));
}

#[tokio::test]
async fn test_void_handler_with_empty_object_fails() {
    let result = simulate_handler_call(json!({}), |_: ()| "success").await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("invalid type"));
}

#[tokio::test]
async fn test_struct_handler_with_valid_input() {
    let result = simulate_handler_call(json!({"name": "Alice"}), |input: TestInput| {
        format!("Hello, {}", input.name)
    })
    .await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), json!("Hello, Alice"));
}

#[tokio::test]
async fn test_struct_handler_with_null_fails() {
    let result = simulate_handler_call(json!(null), |input: TestInput| {
        format!("Hello, {}", input.name)
    })
    .await;
    assert!(result.is_err());
}

// =============================================================================
// Property 2: Input Validation Execution
// =============================================================================

use crate::validation::{Validate, ValidationResult, ValidationRules};
use crate::{Context, EmptyContext, Router, RpcErrorCode, RpcResult};

/// Test input that implements Validate
#[derive(Debug, Clone, Deserialize, Serialize)]
struct ValidatedInput {
    name: String,
    email: String,
    age: i64,
}

impl Validate for ValidatedInput {
    fn validate(&self) -> ValidationResult {
        ValidationRules::new()
            .required("name", &self.name)
            .min_length("name", &self.name, 2)
            .email("email", &self.email)
            .range("age", self.age, 0, 150)
            .build()
    }
}

/// Handler that uses validated input
async fn validated_handler(
    _ctx: Context<EmptyContext>,
    input: ValidatedInput,
) -> RpcResult<String> {
    Ok(format!("Hello, {}!", input.name))
}

/// Handler without validation
async fn unvalidated_handler(
    _ctx: Context<EmptyContext>,
    input: ValidatedInput,
) -> RpcResult<String> {
    Ok(format!("Hello, {}!", input.name))
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Property 2: Input Validation Execution**
    /// *For any* handler with a validatable input type and any invalid input,
    /// the RPC_Plugin SHALL execute validation before the handler and return
    /// a VALIDATION_ERROR with field-level details.
    /// **Feature: tauri-rpc-framework, Property 2: Input Validation Execution**
    /// **Validates: Requirements 6.2, 6.3, 6.5**

    /// Valid inputs should pass validation and execute handler
    #[test]
    fn prop_valid_input_passes_validation(
        name in "[a-zA-Z]{2,20}",
        domain in "[a-z]{3,10}",
        tld in "[a-z]{2,4}",
        age in 0i64..150
    ) {
        let email = format!("{}@{}.{}", name.to_lowercase(), domain, tld);
        let input = ValidatedInput {
            name: name.clone(),
            email,
            age,
        };

        // Validate directly
        let result = input.validate();
        prop_assert!(result.is_valid(), "Valid input should pass validation");
    }

    /// Invalid name (empty) should fail validation
    #[test]
    fn prop_empty_name_fails_validation(
        domain in "[a-z]{3,10}",
        tld in "[a-z]{2,4}",
        age in 0i64..150
    ) {
        let email = format!("test@{}.{}", domain, tld);
        let input = ValidatedInput {
            name: "".to_string(),
            email,
            age,
        };

        let result = input.validate();
        prop_assert!(!result.is_valid(), "Empty name should fail validation");
        prop_assert!(
            result.errors.iter().any(|e| e.field == "name"),
            "Should have error for name field"
        );
    }

    /// Invalid email should fail validation
    #[test]
    fn prop_invalid_email_fails_validation(
        name in "[a-zA-Z]{2,20}",
        invalid_email in "[a-z]{5,15}",  // No @ symbol
        age in 0i64..150
    ) {
        let input = ValidatedInput {
            name,
            email: invalid_email,
            age,
        };

        let result = input.validate();
        prop_assert!(!result.is_valid(), "Invalid email should fail validation");
        prop_assert!(
            result.errors.iter().any(|e| e.field == "email"),
            "Should have error for email field"
        );
    }

    /// Age out of range should fail validation
    #[test]
    fn prop_age_out_of_range_fails_validation(
        name in "[a-zA-Z]{2,20}",
        domain in "[a-z]{3,10}",
        tld in "[a-z]{2,4}",
        age in 151i64..500
    ) {
        let email = format!("{}@{}.{}", name.to_lowercase(), domain, tld);
        let input = ValidatedInput {
            name,
            email,
            age,
        };

        let result = input.validate();
        prop_assert!(!result.is_valid(), "Age out of range should fail validation");
        prop_assert!(
            result.errors.iter().any(|e| e.field == "age"),
            "Should have error for age field"
        );
    }

    /// Multiple invalid fields should aggregate all errors
    #[test]
    fn prop_multiple_errors_aggregated(
        invalid_email in "[a-z]{5,15}",  // No @ symbol
        age in 151i64..500
    ) {
        let input = ValidatedInput {
            name: "".to_string(),  // Invalid: empty
            email: invalid_email,  // Invalid: no @
            age,                   // Invalid: out of range
        };

        let result = input.validate();
        prop_assert!(!result.is_valid(), "Multiple invalid fields should fail");
        prop_assert!(
            result.errors.len() >= 2,
            "Should have at least 2 errors, got {}",
            result.errors.len()
        );
    }
}

#[cfg(test)]
mod validation_execution_unit_tests {
    use super::*;

    #[tokio::test]
    async fn test_router_with_validated_handler_valid_input() {
        let router = Router::new()
            .context(EmptyContext)
            .procedure("test")
            .input_validated::<ValidatedInput>()
            .query(validated_handler)
            .compile();

        let input = json!({
            "name": "John",
            "email": "john@example.com",
            "age": 30
        });

        let result = router.call("test", input).await;
        assert!(result.is_ok(), "Valid input should succeed: {:?}", result);
        assert_eq!(result.unwrap(), json!("Hello, John!"));
    }

    #[tokio::test]
    async fn test_router_with_validated_handler_invalid_input() {
        let router = Router::new()
            .context(EmptyContext)
            .procedure("test")
            .input_validated::<ValidatedInput>()
            .query(validated_handler)
            .compile();

        let input = json!({
            "name": "",
            "email": "invalid",
            "age": 200
        });

        let result = router.call("test", input).await;
        assert!(result.is_err(), "Invalid input should fail");

        let err = result.unwrap_err();
        assert_eq!(err.code, RpcErrorCode::ValidationError);
        assert!(err.details.is_some(), "Should have error details");

        let details = err.details.unwrap();
        let errors = details.get("errors").expect("Should have errors array");
        assert!(errors.is_array(), "Errors should be an array");
        assert!(
            !errors.as_array().unwrap().is_empty(),
            "Should have at least one error"
        );
    }

    #[tokio::test]
    async fn test_router_without_validation_accepts_invalid_input() {
        let router = Router::new()
            .context(EmptyContext)
            .query("test", unvalidated_handler) // Not using query_validated
            .compile();

        let input = json!({
            "name": "",
            "email": "invalid",
            "age": 200
        });

        // Without validation, the handler should still execute
        let result = router.call("test", input).await;
        assert!(result.is_ok(), "Without validation, handler should execute");
    }

    #[tokio::test]
    async fn test_validation_error_contains_field_details() {
        let router = Router::new()
            .context(EmptyContext)
            .procedure("create")
            .input_validated::<ValidatedInput>()
            .mutation(validated_handler)
            .compile();

        let input = json!({
            "name": "a",  // Too short (min 2)
            "email": "test@example.com",
            "age": 30
        });

        let result = router.call("create", input).await;
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert_eq!(err.code, RpcErrorCode::ValidationError);

        let details = err.details.unwrap();
        let errors = details.get("errors").unwrap().as_array().unwrap();

        // Should have error for name field
        let name_error = errors.iter().find(|e| e.get("field").unwrap() == "name");
        assert!(name_error.is_some(), "Should have error for name field");
        assert_eq!(name_error.unwrap().get("code").unwrap(), "min_length");
    }
}
