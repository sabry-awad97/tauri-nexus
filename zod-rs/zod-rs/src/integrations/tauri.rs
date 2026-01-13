//! Tauri framework integration.
//!
//! This module provides conversion utilities between `zod-rs` schemas and
//! Tauri's `tauri-plugin-rpc` TypeSchema format.
//!
//! # Overview
//!
//! The Tauri RPC plugin uses a JSON Schema-like `TypeSchema` format for
//! documenting procedure inputs and outputs. This module provides:
//!
//! - [`TauriTypeSchema`] - A TypeSchema-compatible struct
//! - [`ToTauriSchema`] trait - Convert zod-rs types to Tauri TypeSchema
//! - Helper functions for building TypeSchema objects
//!
//! # Example
//!
//! ```rust,ignore
//! use zod_rs::ZodSchema;
//! use zod_rs::integrations::tauri::{TauriTypeSchema, ToTauriSchema};
//!
//! #[derive(ZodSchema)]
//! struct User {
//!     name: String,
//!     age: u32,
//! }
//!
//! // Convert to Tauri TypeSchema
//! let schema = User::to_tauri_schema();
//!
//! // Use with tauri-plugin-rpc
//! let procedure = ProcedureSchema::query()
//!     .with_input(schema)
//!     .with_output(TauriTypeSchema::object()
//!         .with_property("success", TauriTypeSchema::boolean()));
//! ```

use std::collections::HashMap;

#[cfg(feature = "serde-compat")]
use serde::{Deserialize, Serialize};

use crate::ZodSchema;

/// A TypeSchema compatible with Tauri's `tauri-plugin-rpc` schema format.
///
/// This struct mirrors the `TypeSchema` from `tauri-plugin-rpc` and can be
/// used directly with the RPC plugin's schema documentation features.
///
/// # Example
///
/// ```rust
/// use zod_rs::integrations::tauri::TauriTypeSchema;
///
/// let schema = TauriTypeSchema::object()
///     .with_property("id", TauriTypeSchema::integer())
///     .with_property("name", TauriTypeSchema::string().with_min_length(1))
///     .with_required("id")
///     .with_required("name");
/// ```
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde-compat", derive(Serialize, Deserialize))]
pub struct TauriTypeSchema {
    /// Type name (e.g., "string", "number", "object", "array")
    #[cfg_attr(feature = "serde-compat", serde(rename = "type"))]
    pub type_name: String,

    /// For object types, the properties
    #[cfg_attr(
        feature = "serde-compat",
        serde(skip_serializing_if = "Option::is_none")
    )]
    pub properties: Option<HashMap<String, TauriTypeSchema>>,

    /// Required properties for object types
    #[cfg_attr(
        feature = "serde-compat",
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
    pub required: Vec<String>,

    /// For array types, the item type
    #[cfg_attr(
        feature = "serde-compat",
        serde(skip_serializing_if = "Option::is_none")
    )]
    pub items: Option<Box<TauriTypeSchema>>,

    /// Description of the type
    #[cfg_attr(
        feature = "serde-compat",
        serde(skip_serializing_if = "Option::is_none")
    )]
    pub description: Option<String>,

    /// Example value
    #[cfg_attr(
        feature = "serde-compat",
        serde(skip_serializing_if = "Option::is_none")
    )]
    pub example: Option<serde_json::Value>,

    /// Enum values (for string enums)
    #[cfg_attr(
        feature = "serde-compat",
        serde(rename = "enum", skip_serializing_if = "Option::is_none")
    )]
    pub enum_values: Option<Vec<serde_json::Value>>,

    /// Format hint (e.g., "email", "uuid", "date-time")
    #[cfg_attr(
        feature = "serde-compat",
        serde(skip_serializing_if = "Option::is_none")
    )]
    pub format: Option<String>,

    /// Minimum value (for numbers)
    #[cfg_attr(
        feature = "serde-compat",
        serde(skip_serializing_if = "Option::is_none")
    )]
    pub minimum: Option<f64>,

    /// Maximum value (for numbers)
    #[cfg_attr(
        feature = "serde-compat",
        serde(skip_serializing_if = "Option::is_none")
    )]
    pub maximum: Option<f64>,

    /// Minimum length (for strings/arrays)
    #[cfg_attr(
        feature = "serde-compat",
        serde(rename = "minLength", skip_serializing_if = "Option::is_none")
    )]
    pub min_length: Option<usize>,

    /// Maximum length (for strings/arrays)
    #[cfg_attr(
        feature = "serde-compat",
        serde(rename = "maxLength", skip_serializing_if = "Option::is_none")
    )]
    pub max_length: Option<usize>,

    /// Pattern (for strings)
    #[cfg_attr(
        feature = "serde-compat",
        serde(skip_serializing_if = "Option::is_none")
    )]
    pub pattern: Option<String>,

    /// Whether the value can be null
    #[cfg_attr(
        feature = "serde-compat",
        serde(default, skip_serializing_if = "std::ops::Not::not")
    )]
    pub nullable: bool,
}

impl Default for TauriTypeSchema {
    fn default() -> Self {
        Self::string()
    }
}

impl TauriTypeSchema {
    // =========================================================================
    // Primitive Type Constructors
    // =========================================================================

    /// Create a string type schema.
    pub fn string() -> Self {
        Self {
            type_name: "string".to_string(),
            properties: None,
            required: Vec::new(),
            items: None,
            description: None,
            example: None,
            enum_values: None,
            format: None,
            minimum: None,
            maximum: None,
            min_length: None,
            max_length: None,
            pattern: None,
            nullable: false,
        }
    }

    /// Create a number type schema.
    pub fn number() -> Self {
        Self {
            type_name: "number".to_string(),
            ..Self::string()
        }
    }

    /// Create an integer type schema.
    pub fn integer() -> Self {
        Self {
            type_name: "integer".to_string(),
            ..Self::string()
        }
    }

    /// Create a boolean type schema.
    pub fn boolean() -> Self {
        Self {
            type_name: "boolean".to_string(),
            ..Self::string()
        }
    }

    /// Create a null type schema.
    pub fn null() -> Self {
        Self {
            type_name: "null".to_string(),
            ..Self::string()
        }
    }

    // =========================================================================
    // Compound Type Constructors
    // =========================================================================

    /// Create an object type schema.
    pub fn object() -> Self {
        Self {
            type_name: "object".to_string(),
            properties: Some(HashMap::new()),
            ..Self::string()
        }
    }

    /// Create an array type schema.
    pub fn array(items: TauriTypeSchema) -> Self {
        Self {
            type_name: "array".to_string(),
            items: Some(Box::new(items)),
            ..Self::string()
        }
    }

    /// Create a custom type schema.
    pub fn custom(type_name: impl Into<String>) -> Self {
        Self {
            type_name: type_name.into(),
            ..Self::string()
        }
    }

    // =========================================================================
    // Builder Methods
    // =========================================================================

    /// Add a property to an object type.
    pub fn with_property(mut self, name: impl Into<String>, schema: TauriTypeSchema) -> Self {
        if self.properties.is_none() {
            self.properties = Some(HashMap::new());
        }
        if let Some(props) = &mut self.properties {
            props.insert(name.into(), schema);
        }
        self
    }

    /// Mark a property as required.
    pub fn with_required(mut self, name: impl Into<String>) -> Self {
        self.required.push(name.into());
        self
    }

    /// Set the description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set an example value.
    pub fn with_example(mut self, example: impl serde::Serialize) -> Self {
        self.example = serde_json::to_value(example).ok();
        self
    }

    /// Set enum values.
    pub fn with_enum<I, V>(mut self, values: I) -> Self
    where
        I: IntoIterator<Item = V>,
        V: serde::Serialize,
    {
        self.enum_values = Some(
            values
                .into_iter()
                .filter_map(|v| serde_json::to_value(v).ok())
                .collect(),
        );
        self
    }

    /// Set the format.
    pub fn with_format(mut self, format: impl Into<String>) -> Self {
        self.format = Some(format.into());
        self
    }

    /// Set minimum value.
    pub fn with_minimum(mut self, min: f64) -> Self {
        self.minimum = Some(min);
        self
    }

    /// Set maximum value.
    pub fn with_maximum(mut self, max: f64) -> Self {
        self.maximum = Some(max);
        self
    }

    /// Set minimum length.
    pub fn with_min_length(mut self, min: usize) -> Self {
        self.min_length = Some(min);
        self
    }

    /// Set maximum length.
    pub fn with_max_length(mut self, max: usize) -> Self {
        self.max_length = Some(max);
        self
    }

    /// Set pattern.
    pub fn with_pattern(mut self, pattern: impl Into<String>) -> Self {
        self.pattern = Some(pattern.into());
        self
    }

    /// Mark as nullable.
    pub fn nullable(mut self) -> Self {
        self.nullable = true;
        self
    }

    // =========================================================================
    // Conversion Methods
    // =========================================================================

    /// Convert to JSON string.
    #[cfg(feature = "serde-compat")]
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    /// Convert to pretty-printed JSON string.
    #[cfg(feature = "serde-compat")]
    pub fn to_json_pretty(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }
}

/// Trait for converting zod-rs types to Tauri TypeSchema.
///
/// This trait is automatically implemented for all types that implement
/// [`ZodSchema`]. It provides a convenient way to generate Tauri-compatible
/// schema documentation.
///
/// # Example
///
/// ```rust,ignore
/// use zod_rs::ZodSchema;
/// use zod_rs::integrations::tauri::ToTauriSchema;
///
/// #[derive(ZodSchema)]
/// struct User {
///     name: String,
///     age: u32,
/// }
///
/// let schema = User::to_tauri_schema();
/// ```
pub trait ToTauriSchema {
    /// Convert this type to a Tauri TypeSchema.
    fn to_tauri_schema() -> TauriTypeSchema;
}

// Blanket implementation for all ZodSchema types
impl<T: ZodSchema> ToTauriSchema for T {
    fn to_tauri_schema() -> TauriTypeSchema {
        // Parse the Zod schema string to extract type information
        // This is a simplified implementation - a full implementation would
        // parse the Zod schema string more thoroughly
        let zod_schema = T::zod_schema();
        let description = T::metadata().description;

        let mut schema = parse_zod_to_tauri(zod_schema);

        if let Some(desc) = description {
            schema.description = Some(desc);
        }

        schema
    }
}

/// Parse a Zod schema string into a TauriTypeSchema.
///
/// This is a simplified parser that handles common Zod patterns.
/// For complex schemas, consider using the builder methods directly.
fn parse_zod_to_tauri(zod_schema: &str) -> TauriTypeSchema {
    let schema = zod_schema.trim();

    // Handle basic types
    if schema.starts_with("z.string()") {
        let mut ts = TauriTypeSchema::string();

        // Check for format validations
        if schema.contains(".email()") {
            ts.format = Some("email".to_string());
        } else if schema.contains(".uuid()") {
            ts.format = Some("uuid".to_string());
        } else if schema.contains(".url()") {
            ts.format = Some("uri".to_string());
        } else if schema.contains(".datetime()") {
            ts.format = Some("date-time".to_string());
        } else if schema.contains(".ip()") {
            ts.format = Some("ip".to_string());
        }

        // Check for length constraints
        if let Some(min) = extract_number(schema, ".min(") {
            ts.min_length = Some(min as usize);
        }
        if let Some(max) = extract_number(schema, ".max(") {
            ts.max_length = Some(max as usize);
        }
        if let Some(len) = extract_number(schema, ".length(") {
            ts.min_length = Some(len as usize);
            ts.max_length = Some(len as usize);
        }

        // Check for nullable/optional
        if schema.contains(".nullable()") {
            ts.nullable = true;
        }

        return ts;
    }

    if schema.starts_with("z.number()") {
        let mut ts = if schema.contains(".int()") {
            TauriTypeSchema::integer()
        } else {
            TauriTypeSchema::number()
        };

        // Check for constraints
        if let Some(min) = extract_number(schema, ".min(") {
            ts.minimum = Some(min);
        }
        if let Some(max) = extract_number(schema, ".max(") {
            ts.maximum = Some(max);
        }
        if schema.contains(".nonnegative()") {
            ts.minimum = Some(0.0);
        }
        if schema.contains(".positive()") {
            ts.minimum = Some(0.0); // Technically > 0, but JSON Schema uses >=
        }
        if schema.contains(".nullable()") {
            ts.nullable = true;
        }

        return ts;
    }

    if schema.starts_with("z.boolean()") {
        let mut ts = TauriTypeSchema::boolean();
        if schema.contains(".nullable()") {
            ts.nullable = true;
        }
        return ts;
    }

    if schema.starts_with("z.array(") {
        // Extract inner type (simplified)
        let mut ts = TauriTypeSchema::array(TauriTypeSchema::custom("unknown"));
        if schema.contains(".nullable()") {
            ts.nullable = true;
        }
        return ts;
    }

    if schema.starts_with("z.object(") {
        let mut ts = TauriTypeSchema::object();
        if schema.contains(".strict()") {
            // Strict mode - no additional properties
        }
        if schema.contains(".nullable()") {
            ts.nullable = true;
        }
        return ts;
    }

    if schema.starts_with("z.enum(") {
        // Extract enum values
        let mut ts = TauriTypeSchema::string();
        if let Some(start) = schema.find('[') {
            if let Some(end) = schema.find(']') {
                let values_str = &schema[start + 1..end];
                let values: Vec<serde_json::Value> = values_str
                    .split(',')
                    .map(|s| s.trim().trim_matches('"'))
                    .filter(|s| !s.is_empty())
                    .map(|s| serde_json::Value::String(s.to_string()))
                    .collect();
                if !values.is_empty() {
                    ts.enum_values = Some(values);
                }
            }
        }
        return ts;
    }

    // Default to unknown type
    TauriTypeSchema::custom("unknown")
}

/// Extract a number from a method call like `.min(5)`.
fn extract_number(schema: &str, method: &str) -> Option<f64> {
    if let Some(start) = schema.find(method) {
        let rest = &schema[start + method.len()..];
        if let Some(end) = rest.find(')') {
            let num_str = &rest[..end];
            return num_str.parse().ok();
        }
    }
    None
}

// =============================================================================
// Schema Collection Utilities
// =============================================================================

/// A collection of Tauri TypeSchemas for use with tauri-plugin-rpc.
///
/// This struct provides utilities for collecting and managing multiple
/// schemas, useful for documenting RPC procedures.
///
/// # Example
///
/// ```rust,ignore
/// use zod_rs::ZodSchema;
/// use zod_rs::integrations::tauri::{TauriSchemaCollection, ToTauriSchema};
///
/// #[derive(ZodSchema)]
/// struct User { name: String }
///
/// #[derive(ZodSchema)]
/// struct Post { title: String }
///
/// let mut collection = TauriSchemaCollection::new();
/// collection.register::<User>("User");
/// collection.register::<Post>("Post");
///
/// // Get all schemas as a map
/// let schemas = collection.schemas();
/// ```
#[derive(Debug, Clone, Default)]
pub struct TauriSchemaCollection {
    schemas: HashMap<String, TauriTypeSchema>,
}

impl TauriSchemaCollection {
    /// Create a new empty schema collection.
    pub fn new() -> Self {
        Self {
            schemas: HashMap::new(),
        }
    }

    /// Register a type's schema with a given name.
    pub fn register<T: ToTauriSchema>(&mut self, name: impl Into<String>) {
        self.schemas.insert(name.into(), T::to_tauri_schema());
    }

    /// Register a schema directly with a given name.
    pub fn register_schema(&mut self, name: impl Into<String>, schema: TauriTypeSchema) {
        self.schemas.insert(name.into(), schema);
    }

    /// Get a schema by name.
    pub fn get(&self, name: &str) -> Option<&TauriTypeSchema> {
        self.schemas.get(name)
    }

    /// Get all schemas as a reference to the internal map.
    pub fn schemas(&self) -> &HashMap<String, TauriTypeSchema> {
        &self.schemas
    }

    /// Get all schemas, consuming the collection.
    pub fn into_schemas(self) -> HashMap<String, TauriTypeSchema> {
        self.schemas
    }

    /// Get the number of registered schemas.
    pub fn len(&self) -> usize {
        self.schemas.len()
    }

    /// Check if the collection is empty.
    pub fn is_empty(&self) -> bool {
        self.schemas.is_empty()
    }

    /// Convert all schemas to JSON.
    #[cfg(feature = "serde-compat")]
    pub fn to_json(&self) -> String {
        serde_json::to_string(&self.schemas).unwrap_or_default()
    }

    /// Convert all schemas to pretty-printed JSON.
    #[cfg(feature = "serde-compat")]
    pub fn to_json_pretty(&self) -> String {
        serde_json::to_string_pretty(&self.schemas).unwrap_or_default()
    }
}

// =============================================================================
// Runtime Schema Generation Helpers
// =============================================================================

/// Generate a Tauri TypeSchema from a ZodSchema type at runtime.
///
/// This is a convenience function that wraps `ToTauriSchema::to_tauri_schema()`.
///
/// # Example
///
/// ```rust,ignore
/// use zod_rs::ZodSchema;
/// use zod_rs::integrations::tauri::schema_for;
///
/// #[derive(ZodSchema)]
/// struct User { name: String }
///
/// let schema = schema_for::<User>();
/// ```
pub fn schema_for<T: ToTauriSchema>() -> TauriTypeSchema {
    T::to_tauri_schema()
}

/// Create a procedure input schema from multiple types.
///
/// This is useful for RPC procedures that accept multiple parameters.
///
/// # Example
///
/// ```rust,ignore
/// use zod_rs::integrations::tauri::{procedure_input, TauriTypeSchema};
///
/// let input = procedure_input(&[
///     ("userId", TauriTypeSchema::integer()),
///     ("name", TauriTypeSchema::string()),
/// ]);
/// ```
pub fn procedure_input<S: Into<String> + Clone>(
    params: &[(S, TauriTypeSchema)],
) -> TauriTypeSchema {
    let mut schema = TauriTypeSchema::object();
    for (name, param_schema) in params {
        let name_str: String = name.clone().into();
        schema = schema.with_property(name_str.clone(), param_schema.clone());
        schema = schema.with_required(name_str);
    }
    schema
}

/// Create a procedure output schema for a success response.
///
/// # Example
///
/// ```rust,ignore
/// use zod_rs::integrations::tauri::{success_response, TauriTypeSchema};
///
/// let output = success_response(TauriTypeSchema::object()
///     .with_property("id", TauriTypeSchema::integer()));
/// ```
pub fn success_response(data: TauriTypeSchema) -> TauriTypeSchema {
    TauriTypeSchema::object()
        .with_property("success", TauriTypeSchema::boolean().with_example(true))
        .with_property("data", data)
        .with_required("success")
        .with_required("data")
}

/// Create a procedure output schema for an error response.
///
/// # Example
///
/// ```rust,ignore
/// use zod_rs::integrations::tauri::error_response;
///
/// let output = error_response();
/// ```
pub fn error_response() -> TauriTypeSchema {
    TauriTypeSchema::object()
        .with_property("success", TauriTypeSchema::boolean().with_example(false))
        .with_property(
            "error",
            TauriTypeSchema::object()
                .with_property("code", TauriTypeSchema::string())
                .with_property("message", TauriTypeSchema::string())
                .with_required("code")
                .with_required("message"),
        )
        .with_required("success")
        .with_required("error")
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_schema() {
        let schema = TauriTypeSchema::string();
        assert_eq!(schema.type_name, "string");
        assert!(!schema.nullable);
    }

    #[test]
    fn test_number_schema() {
        let schema = TauriTypeSchema::number();
        assert_eq!(schema.type_name, "number");
    }

    #[test]
    fn test_integer_schema() {
        let schema = TauriTypeSchema::integer();
        assert_eq!(schema.type_name, "integer");
    }

    #[test]
    fn test_boolean_schema() {
        let schema = TauriTypeSchema::boolean();
        assert_eq!(schema.type_name, "boolean");
    }

    #[test]
    fn test_object_schema() {
        let schema = TauriTypeSchema::object()
            .with_property("name", TauriTypeSchema::string())
            .with_property("age", TauriTypeSchema::integer())
            .with_required("name");

        assert_eq!(schema.type_name, "object");
        assert!(schema.properties.is_some());
        let props = schema.properties.unwrap();
        assert!(props.contains_key("name"));
        assert!(props.contains_key("age"));
        assert!(schema.required.contains(&"name".to_string()));
    }

    #[test]
    fn test_array_schema() {
        let schema = TauriTypeSchema::array(TauriTypeSchema::string());
        assert_eq!(schema.type_name, "array");
        assert!(schema.items.is_some());
    }

    #[test]
    fn test_string_with_constraints() {
        let schema = TauriTypeSchema::string()
            .with_min_length(1)
            .with_max_length(100)
            .with_format("email");

        assert_eq!(schema.min_length, Some(1));
        assert_eq!(schema.max_length, Some(100));
        assert_eq!(schema.format, Some("email".to_string()));
    }

    #[test]
    fn test_number_with_constraints() {
        let schema = TauriTypeSchema::number()
            .with_minimum(0.0)
            .with_maximum(100.0);

        assert_eq!(schema.minimum, Some(0.0));
        assert_eq!(schema.maximum, Some(100.0));
    }

    #[test]
    fn test_nullable() {
        let schema = TauriTypeSchema::string().nullable();
        assert!(schema.nullable);
    }

    #[test]
    fn test_parse_zod_string() {
        let schema = parse_zod_to_tauri("z.string()");
        assert_eq!(schema.type_name, "string");
    }

    #[test]
    fn test_parse_zod_string_email() {
        let schema = parse_zod_to_tauri("z.string().email()");
        assert_eq!(schema.type_name, "string");
        assert_eq!(schema.format, Some("email".to_string()));
    }

    #[test]
    fn test_parse_zod_number_int() {
        let schema = parse_zod_to_tauri("z.number().int()");
        assert_eq!(schema.type_name, "integer");
    }

    #[test]
    fn test_parse_zod_number_nonnegative() {
        let schema = parse_zod_to_tauri("z.number().int().nonnegative()");
        assert_eq!(schema.type_name, "integer");
        assert_eq!(schema.minimum, Some(0.0));
    }

    #[test]
    fn test_parse_zod_boolean() {
        let schema = parse_zod_to_tauri("z.boolean()");
        assert_eq!(schema.type_name, "boolean");
    }

    #[test]
    fn test_parse_zod_enum() {
        let schema = parse_zod_to_tauri(r#"z.enum(["Active", "Inactive"])"#);
        assert_eq!(schema.type_name, "string");
        assert!(schema.enum_values.is_some());
        let values = schema.enum_values.unwrap();
        assert_eq!(values.len(), 2);
    }

    #[cfg(feature = "serde-compat")]
    #[test]
    fn test_to_json() {
        let schema = TauriTypeSchema::string().with_description("A name");
        let json = schema.to_json();
        assert!(json.contains("\"type\":\"string\""));
        assert!(json.contains("\"description\":\"A name\""));
    }

    #[test]
    fn test_schema_collection() {
        let mut collection = TauriSchemaCollection::new();
        collection.register_schema(
            "User",
            TauriTypeSchema::object().with_property("name", TauriTypeSchema::string()),
        );
        collection.register_schema(
            "Post",
            TauriTypeSchema::object().with_property("title", TauriTypeSchema::string()),
        );

        assert_eq!(collection.len(), 2);
        assert!(!collection.is_empty());
        assert!(collection.get("User").is_some());
        assert!(collection.get("Post").is_some());
        assert!(collection.get("Unknown").is_none());
    }

    #[test]
    fn test_procedure_input() {
        let input = procedure_input(&[
            ("userId", TauriTypeSchema::integer()),
            ("name", TauriTypeSchema::string()),
        ]);

        assert_eq!(input.type_name, "object");
        assert!(input.properties.is_some());
        let props = input.properties.unwrap();
        assert!(props.contains_key("userId"));
        assert!(props.contains_key("name"));
        assert!(input.required.contains(&"userId".to_string()));
        assert!(input.required.contains(&"name".to_string()));
    }

    #[test]
    fn test_success_response() {
        let response = success_response(TauriTypeSchema::string());

        assert_eq!(response.type_name, "object");
        assert!(response.properties.is_some());
        let props = response.properties.unwrap();
        assert!(props.contains_key("success"));
        assert!(props.contains_key("data"));
        assert!(response.required.contains(&"success".to_string()));
        assert!(response.required.contains(&"data".to_string()));
    }

    #[test]
    fn test_error_response() {
        let response = error_response();

        assert_eq!(response.type_name, "object");
        assert!(response.properties.is_some());
        let props = response.properties.unwrap();
        assert!(props.contains_key("success"));
        assert!(props.contains_key("error"));

        let error_schema = props.get("error").unwrap();
        assert_eq!(error_schema.type_name, "object");
    }
}
