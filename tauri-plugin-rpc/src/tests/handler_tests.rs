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
    /// Property: Any non-null JSON value should fail to deserialize as unit type
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
