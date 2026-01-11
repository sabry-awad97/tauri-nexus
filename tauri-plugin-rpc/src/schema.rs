//! Schema export for RPC routers
//!
//! This module provides types and functions for exporting router schemas
//! as JSON or OpenAPI-compatible formats.
//!
//! # Example
//!
//! ```rust,ignore
//! use tauri_plugin_rpc::schema::{RouterSchema, SchemaExporter};
//!
//! let router = Router::new()
//!     .context(AppContext::default())
//!     .query("user.get", get_user)
//!     .mutation("user.create", create_user);
//!
//! // Export schema
//! let schema = router.export_schema();
//! let json = schema.to_json_pretty();
//! println!("{}", json);
//!
//! // Export as OpenAPI
//! let openapi = schema.to_openapi();
//! ```

use crate::middleware::ProcedureType;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// =============================================================================
// Procedure Meta (for builder pattern)
// =============================================================================

/// Metadata for a procedure, used with the `.meta()` builder method.
///
/// This provides an oRPC-style way to attach OpenAPI metadata directly
/// to procedure definitions.
///
/// # Example
///
/// ```rust,ignore
/// use tauri_plugin_rpc::prelude::*;
///
/// let router = Router::new()
///     .context(AppContext::new())
///     .procedure("users.get")
///         .meta(ProcedureMeta::new()
///             .description("Get a user by ID")
///             .tag("users")
///             .input(TypeSchema::object()
///                 .with_property("id", TypeSchema::integer())
///                 .with_required("id"))
///             .output(TypeSchema::object()
///                 .with_property("id", TypeSchema::integer())
///                 .with_property("name", TypeSchema::string())))
///         .input::<GetUserInput>()
///         .query(get_user);
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProcedureMeta {
    /// Human-readable description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Input type schema
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<TypeSchema>,
    /// Output type schema
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<TypeSchema>,
    /// Whether the procedure is deprecated
    #[serde(default)]
    pub deprecated: bool,
    /// Tags for categorization
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Additional metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
    /// Summary (short description for OpenAPI)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// Example input value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example_input: Option<serde_json::Value>,
    /// Example output value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example_output: Option<serde_json::Value>,
}

impl ProcedureMeta {
    /// Create a new empty procedure metadata.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the description.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the summary (short description).
    pub fn summary(mut self, summary: impl Into<String>) -> Self {
        self.summary = Some(summary.into());
        self
    }

    /// Set the input type schema.
    pub fn input(mut self, input: TypeSchema) -> Self {
        self.input = Some(input);
        self
    }

    /// Set the output type schema.
    pub fn output(mut self, output: TypeSchema) -> Self {
        self.output = Some(output);
        self
    }

    /// Mark as deprecated.
    pub fn deprecated(mut self) -> Self {
        self.deprecated = true;
        self
    }

    /// Add a tag.
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Add multiple tags.
    pub fn tags(mut self, tags: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.tags.extend(tags.into_iter().map(|t| t.into()));
        self
    }

    /// Set additional metadata.
    pub fn metadata(mut self, metadata: impl Serialize) -> Self {
        self.metadata = serde_json::to_value(metadata).ok();
        self
    }

    /// Set an example input value.
    pub fn example_input(mut self, example: impl Serialize) -> Self {
        self.example_input = serde_json::to_value(example).ok();
        self
    }

    /// Set an example output value.
    pub fn example_output(mut self, example: impl Serialize) -> Self {
        self.example_output = serde_json::to_value(example).ok();
        self
    }

    /// Convert to a ProcedureSchema with the given procedure type.
    pub fn to_schema(self, procedure_type: ProcedureType) -> ProcedureSchema {
        ProcedureSchema {
            procedure_type: procedure_type.into(),
            description: self.description,
            input: self.input,
            output: self.output,
            deprecated: self.deprecated,
            tags: self.tags,
            metadata: self.metadata,
        }
    }
}

// =============================================================================
// Schema Types
// =============================================================================

/// Schema for a complete router
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterSchema {
    /// Schema version
    pub version: String,
    /// Router name/title
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Router description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// All procedures in the router
    pub procedures: HashMap<String, ProcedureSchema>,
    /// Metadata about the schema
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl RouterSchema {
    /// Create a new router schema
    pub fn new() -> Self {
        Self {
            version: "1.0.0".to_string(),
            name: None,
            description: None,
            procedures: HashMap::new(),
            metadata: None,
        }
    }

    /// Set the schema version
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }

    /// Set the router name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the router description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Add a procedure to the schema
    pub fn add_procedure(mut self, path: impl Into<String>, procedure: ProcedureSchema) -> Self {
        self.procedures.insert(path.into(), procedure);
        self
    }

    /// Set metadata
    pub fn with_metadata(mut self, metadata: impl Serialize) -> Self {
        self.metadata = serde_json::to_value(metadata).ok();
        self
    }

    /// Convert to JSON string
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    /// Convert to pretty-printed JSON string
    pub fn to_json_pretty(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    /// Convert to OpenAPI-compatible format
    pub fn to_openapi(&self) -> OpenApiSchema {
        OpenApiSchema::from_router_schema(self)
    }

    /// Get all query procedures
    pub fn queries(&self) -> impl Iterator<Item = (&String, &ProcedureSchema)> {
        self.procedures
            .iter()
            .filter(|(_, p)| p.procedure_type == ProcedureTypeSchema::Query)
    }

    /// Get all mutation procedures
    pub fn mutations(&self) -> impl Iterator<Item = (&String, &ProcedureSchema)> {
        self.procedures
            .iter()
            .filter(|(_, p)| p.procedure_type == ProcedureTypeSchema::Mutation)
    }

    /// Get all subscription procedures
    pub fn subscriptions(&self) -> impl Iterator<Item = (&String, &ProcedureSchema)> {
        self.procedures
            .iter()
            .filter(|(_, p)| p.procedure_type == ProcedureTypeSchema::Subscription)
    }
}

impl Default for RouterSchema {
    fn default() -> Self {
        Self::new()
    }
}

/// Schema for a single procedure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcedureSchema {
    /// Procedure type (query, mutation, subscription)
    pub procedure_type: ProcedureTypeSchema,
    /// Human-readable description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Input type schema
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<TypeSchema>,
    /// Output type schema
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<TypeSchema>,
    /// Whether the procedure is deprecated
    #[serde(default)]
    pub deprecated: bool,
    /// Tags for categorization
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Additional metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl ProcedureSchema {
    /// Create a new query procedure schema
    pub fn query() -> Self {
        Self {
            procedure_type: ProcedureTypeSchema::Query,
            description: None,
            input: None,
            output: None,
            deprecated: false,
            tags: Vec::new(),
            metadata: None,
        }
    }

    /// Create a new mutation procedure schema
    pub fn mutation() -> Self {
        Self {
            procedure_type: ProcedureTypeSchema::Mutation,
            description: None,
            input: None,
            output: None,
            deprecated: false,
            tags: Vec::new(),
            metadata: None,
        }
    }

    /// Create a new subscription procedure schema
    pub fn subscription() -> Self {
        Self {
            procedure_type: ProcedureTypeSchema::Subscription,
            description: None,
            input: None,
            output: None,
            deprecated: false,
            tags: Vec::new(),
            metadata: None,
        }
    }

    /// Create from a ProcedureType
    pub fn from_procedure_type(procedure_type: ProcedureType) -> Self {
        match procedure_type {
            ProcedureType::Query => Self::query(),
            ProcedureType::Mutation => Self::mutation(),
            ProcedureType::Subscription => Self::subscription(),
        }
    }

    /// Set the description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the input type schema
    pub fn with_input(mut self, input: TypeSchema) -> Self {
        self.input = Some(input);
        self
    }

    /// Set the output type schema
    pub fn with_output(mut self, output: TypeSchema) -> Self {
        self.output = Some(output);
        self
    }

    /// Mark as deprecated
    pub fn deprecated(mut self) -> Self {
        self.deprecated = true;
        self
    }

    /// Add a tag
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Add multiple tags
    pub fn with_tags(mut self, tags: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.tags.extend(tags.into_iter().map(|t| t.into()));
        self
    }

    /// Set metadata
    pub fn with_metadata(mut self, metadata: impl Serialize) -> Self {
        self.metadata = serde_json::to_value(metadata).ok();
        self
    }
}

/// Procedure type for schema
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProcedureTypeSchema {
    /// Read-only query
    Query,
    /// State-modifying mutation
    Mutation,
    /// Real-time subscription
    Subscription,
}

impl From<ProcedureType> for ProcedureTypeSchema {
    fn from(pt: ProcedureType) -> Self {
        match pt {
            ProcedureType::Query => Self::Query,
            ProcedureType::Mutation => Self::Mutation,
            ProcedureType::Subscription => Self::Subscription,
        }
    }
}

/// Schema for a type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeSchema {
    /// Type name (e.g., "string", "number", "object")
    #[serde(rename = "type")]
    pub type_name: String,
    /// For object types, the properties
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<HashMap<String, TypeSchema>>,
    /// Required properties for object types
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required: Vec<String>,
    /// For array types, the item type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Box<TypeSchema>>,
    /// Description of the type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Example value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example: Option<serde_json::Value>,
    /// Enum values (for string enums)
    #[serde(rename = "enum", skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<Vec<serde_json::Value>>,
    /// Format hint (e.g., "email", "uuid", "date-time")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    /// Minimum value (for numbers)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum: Option<f64>,
    /// Maximum value (for numbers)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum: Option<f64>,
    /// Minimum length (for strings/arrays)
    #[serde(rename = "minLength", skip_serializing_if = "Option::is_none")]
    pub min_length: Option<usize>,
    /// Maximum length (for strings/arrays)
    #[serde(rename = "maxLength", skip_serializing_if = "Option::is_none")]
    pub max_length: Option<usize>,
    /// Pattern (for strings)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    /// Whether the value can be null
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub nullable: bool,
}

impl TypeSchema {
    /// Create a string type schema
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

    /// Create a number type schema
    pub fn number() -> Self {
        Self {
            type_name: "number".to_string(),
            ..Self::string()
        }
    }

    /// Create an integer type schema
    pub fn integer() -> Self {
        Self {
            type_name: "integer".to_string(),
            ..Self::string()
        }
    }

    /// Create a boolean type schema
    pub fn boolean() -> Self {
        Self {
            type_name: "boolean".to_string(),
            ..Self::string()
        }
    }

    /// Create a null type schema
    pub fn null() -> Self {
        Self {
            type_name: "null".to_string(),
            ..Self::string()
        }
    }

    /// Create an object type schema
    pub fn object() -> Self {
        Self {
            type_name: "object".to_string(),
            properties: Some(HashMap::new()),
            ..Self::string()
        }
    }

    /// Create an array type schema
    pub fn array(items: TypeSchema) -> Self {
        Self {
            type_name: "array".to_string(),
            items: Some(Box::new(items)),
            ..Self::string()
        }
    }

    /// Create a custom type schema
    pub fn custom(type_name: impl Into<String>) -> Self {
        Self {
            type_name: type_name.into(),
            ..Self::string()
        }
    }

    /// Add a property to an object type
    pub fn with_property(mut self, name: impl Into<String>, schema: TypeSchema) -> Self {
        if self.properties.is_none() {
            self.properties = Some(HashMap::new());
        }
        if let Some(props) = &mut self.properties {
            props.insert(name.into(), schema);
        }
        self
    }

    /// Mark a property as required
    pub fn with_required(mut self, name: impl Into<String>) -> Self {
        self.required.push(name.into());
        self
    }

    /// Set the description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set an example value
    pub fn with_example(mut self, example: impl Serialize) -> Self {
        self.example = serde_json::to_value(example).ok();
        self
    }

    /// Set enum values
    pub fn with_enum(mut self, values: impl IntoIterator<Item = impl Serialize>) -> Self {
        self.enum_values = Some(
            values
                .into_iter()
                .filter_map(|v| serde_json::to_value(v).ok())
                .collect(),
        );
        self
    }

    /// Set the format
    pub fn with_format(mut self, format: impl Into<String>) -> Self {
        self.format = Some(format.into());
        self
    }

    /// Set minimum value
    pub fn with_minimum(mut self, min: f64) -> Self {
        self.minimum = Some(min);
        self
    }

    /// Set maximum value
    pub fn with_maximum(mut self, max: f64) -> Self {
        self.maximum = Some(max);
        self
    }

    /// Set minimum length
    pub fn with_min_length(mut self, min: usize) -> Self {
        self.min_length = Some(min);
        self
    }

    /// Set maximum length
    pub fn with_max_length(mut self, max: usize) -> Self {
        self.max_length = Some(max);
        self
    }

    /// Set pattern
    pub fn with_pattern(mut self, pattern: impl Into<String>) -> Self {
        self.pattern = Some(pattern.into());
        self
    }

    /// Mark as nullable
    pub fn nullable(mut self) -> Self {
        self.nullable = true;
        self
    }
}

// =============================================================================
// OpenAPI Schema
// =============================================================================

/// OpenAPI-compatible schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiSchema {
    /// OpenAPI version
    pub openapi: String,
    /// API info
    pub info: OpenApiInfo,
    /// API paths
    pub paths: HashMap<String, OpenApiPathItem>,
    /// Component schemas
    #[serde(skip_serializing_if = "Option::is_none")]
    pub components: Option<OpenApiComponents>,
}

impl OpenApiSchema {
    /// Create from a RouterSchema
    pub fn from_router_schema(schema: &RouterSchema) -> Self {
        let mut paths = HashMap::new();

        for (path, procedure) in &schema.procedures {
            let openapi_path = format!("/rpc/{}", path.replace('.', "/"));
            let method = match procedure.procedure_type {
                ProcedureTypeSchema::Query => "get",
                ProcedureTypeSchema::Mutation => "post",
                ProcedureTypeSchema::Subscription => "get", // WebSocket upgrade
            };

            let operation = OpenApiOperation {
                summary: procedure.description.clone(),
                description: procedure.description.clone(),
                tags: if procedure.tags.is_empty() {
                    None
                } else {
                    Some(procedure.tags.clone())
                },
                deprecated: if procedure.deprecated {
                    Some(true)
                } else {
                    None
                },
                request_body: procedure.input.as_ref().map(|input| OpenApiRequestBody {
                    required: true,
                    content: {
                        let mut content = HashMap::new();
                        content.insert(
                            "application/json".to_string(),
                            OpenApiMediaType {
                                schema: input.clone(),
                            },
                        );
                        content
                    },
                }),
                responses: {
                    let mut responses = HashMap::new();
                    responses.insert(
                        "200".to_string(),
                        OpenApiResponse {
                            description: "Successful response".to_string(),
                            content: procedure.output.as_ref().map(|output| {
                                let mut content = HashMap::new();
                                content.insert(
                                    "application/json".to_string(),
                                    OpenApiMediaType {
                                        schema: output.clone(),
                                    },
                                );
                                content
                            }),
                        },
                    );
                    responses
                },
            };

            let path_item = paths
                .entry(openapi_path)
                .or_insert_with(OpenApiPathItem::default);
            match method {
                "get" => path_item.get = Some(operation),
                "post" => path_item.post = Some(operation),
                _ => {}
            }
        }

        Self {
            openapi: "3.0.3".to_string(),
            info: OpenApiInfo {
                title: schema.name.clone().unwrap_or_else(|| "RPC API".to_string()),
                description: schema.description.clone(),
                version: schema.version.clone(),
            },
            paths,
            components: None,
        }
    }

    /// Convert to JSON string
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    /// Convert to pretty-printed JSON string
    pub fn to_json_pretty(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }
}

/// OpenAPI info object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiInfo {
    /// API title
    pub title: String,
    /// API description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// API version
    pub version: String,
}

/// OpenAPI path item
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OpenApiPathItem {
    /// GET operation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub get: Option<OpenApiOperation>,
    /// POST operation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post: Option<OpenApiOperation>,
    /// PUT operation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub put: Option<OpenApiOperation>,
    /// DELETE operation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delete: Option<OpenApiOperation>,
}

/// OpenAPI operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiOperation {
    /// Operation summary
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// Operation description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Operation tags
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    /// Whether deprecated
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deprecated: Option<bool>,
    /// Request body
    #[serde(rename = "requestBody", skip_serializing_if = "Option::is_none")]
    pub request_body: Option<OpenApiRequestBody>,
    /// Responses
    pub responses: HashMap<String, OpenApiResponse>,
}

/// OpenAPI request body
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiRequestBody {
    /// Whether required
    pub required: bool,
    /// Content by media type
    pub content: HashMap<String, OpenApiMediaType>,
}

/// OpenAPI media type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiMediaType {
    /// Schema
    pub schema: TypeSchema,
}

/// OpenAPI response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiResponse {
    /// Response description
    pub description: String,
    /// Content by media type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<HashMap<String, OpenApiMediaType>>,
}

/// OpenAPI components
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiComponents {
    /// Reusable schemas
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schemas: Option<HashMap<String, TypeSchema>>,
}

// =============================================================================
// Schema Builder
// =============================================================================

/// Builder for creating router schemas
#[derive(Debug, Clone, Default)]
pub struct SchemaBuilder {
    schema: RouterSchema,
}

impl SchemaBuilder {
    /// Create a new schema builder
    pub fn new() -> Self {
        Self {
            schema: RouterSchema::new(),
        }
    }

    /// Set the version
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.schema.version = version.into();
        self
    }

    /// Set the name
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.schema.name = Some(name.into());
        self
    }

    /// Set the description
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.schema.description = Some(description.into());
        self
    }

    /// Add a query procedure
    pub fn query(mut self, path: impl Into<String>, procedure: ProcedureSchema) -> Self {
        let mut proc = procedure;
        proc.procedure_type = ProcedureTypeSchema::Query;
        self.schema.procedures.insert(path.into(), proc);
        self
    }

    /// Add a mutation procedure
    pub fn mutation(mut self, path: impl Into<String>, procedure: ProcedureSchema) -> Self {
        let mut proc = procedure;
        proc.procedure_type = ProcedureTypeSchema::Mutation;
        self.schema.procedures.insert(path.into(), proc);
        self
    }

    /// Add a subscription procedure
    pub fn subscription(mut self, path: impl Into<String>, procedure: ProcedureSchema) -> Self {
        let mut proc = procedure;
        proc.procedure_type = ProcedureTypeSchema::Subscription;
        self.schema.procedures.insert(path.into(), proc);
        self
    }

    /// Add a procedure with explicit type
    pub fn procedure(mut self, path: impl Into<String>, procedure: ProcedureSchema) -> Self {
        self.schema.procedures.insert(path.into(), procedure);
        self
    }

    /// Build the schema
    pub fn build(self) -> RouterSchema {
        self.schema
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_router_schema_new() {
        let schema = RouterSchema::new();
        assert_eq!(schema.version, "1.0.0");
        assert!(schema.name.is_none());
        assert!(schema.description.is_none());
        assert!(schema.procedures.is_empty());
    }

    #[test]
    fn test_router_schema_builder() {
        let schema = RouterSchema::new()
            .with_version("2.0.0")
            .with_name("My API")
            .with_description("A test API");

        assert_eq!(schema.version, "2.0.0");
        assert_eq!(schema.name, Some("My API".to_string()));
        assert_eq!(schema.description, Some("A test API".to_string()));
    }

    #[test]
    fn test_router_schema_add_procedure() {
        let schema = RouterSchema::new()
            .add_procedure("user.get", ProcedureSchema::query())
            .add_procedure("user.create", ProcedureSchema::mutation());

        assert_eq!(schema.procedures.len(), 2);
        assert!(schema.procedures.contains_key("user.get"));
        assert!(schema.procedures.contains_key("user.create"));
    }

    #[test]
    fn test_router_schema_queries() {
        let schema = RouterSchema::new()
            .add_procedure("user.get", ProcedureSchema::query())
            .add_procedure("user.create", ProcedureSchema::mutation())
            .add_procedure("user.list", ProcedureSchema::query());

        let queries: Vec<_> = schema.queries().collect();
        assert_eq!(queries.len(), 2);
    }

    #[test]
    fn test_router_schema_mutations() {
        let schema = RouterSchema::new()
            .add_procedure("user.get", ProcedureSchema::query())
            .add_procedure("user.create", ProcedureSchema::mutation())
            .add_procedure("user.update", ProcedureSchema::mutation());

        let mutations: Vec<_> = schema.mutations().collect();
        assert_eq!(mutations.len(), 2);
    }

    #[test]
    fn test_router_schema_subscriptions() {
        let schema = RouterSchema::new()
            .add_procedure("user.get", ProcedureSchema::query())
            .add_procedure("events", ProcedureSchema::subscription());

        let subscriptions: Vec<_> = schema.subscriptions().collect();
        assert_eq!(subscriptions.len(), 1);
    }

    #[test]
    fn test_router_schema_to_json() {
        let schema = RouterSchema::new()
            .with_name("Test API")
            .add_procedure("health", ProcedureSchema::query());

        let json = schema.to_json();
        assert!(json.contains("Test API"));
        assert!(json.contains("health"));
    }

    #[test]
    fn test_procedure_schema_query() {
        let proc = ProcedureSchema::query();
        assert_eq!(proc.procedure_type, ProcedureTypeSchema::Query);
        assert!(!proc.deprecated);
    }

    #[test]
    fn test_procedure_schema_mutation() {
        let proc = ProcedureSchema::mutation();
        assert_eq!(proc.procedure_type, ProcedureTypeSchema::Mutation);
    }

    #[test]
    fn test_procedure_schema_subscription() {
        let proc = ProcedureSchema::subscription();
        assert_eq!(proc.procedure_type, ProcedureTypeSchema::Subscription);
    }

    #[test]
    fn test_procedure_schema_builder() {
        let proc = ProcedureSchema::query()
            .with_description("Get a user by ID")
            .with_tag("users")
            .with_tag("read")
            .deprecated();

        assert_eq!(proc.description, Some("Get a user by ID".to_string()));
        assert_eq!(proc.tags, vec!["users", "read"]);
        assert!(proc.deprecated);
    }

    #[test]
    fn test_procedure_schema_with_types() {
        let input = TypeSchema::object()
            .with_property("id", TypeSchema::integer())
            .with_required("id");

        let output = TypeSchema::object()
            .with_property("name", TypeSchema::string())
            .with_property("email", TypeSchema::string().with_format("email"));

        let proc = ProcedureSchema::query()
            .with_input(input)
            .with_output(output);

        assert!(proc.input.is_some());
        assert!(proc.output.is_some());
    }

    #[test]
    fn test_type_schema_string() {
        let schema = TypeSchema::string();
        assert_eq!(schema.type_name, "string");
    }

    #[test]
    fn test_type_schema_number() {
        let schema = TypeSchema::number();
        assert_eq!(schema.type_name, "number");
    }

    #[test]
    fn test_type_schema_integer() {
        let schema = TypeSchema::integer();
        assert_eq!(schema.type_name, "integer");
    }

    #[test]
    fn test_type_schema_boolean() {
        let schema = TypeSchema::boolean();
        assert_eq!(schema.type_name, "boolean");
    }

    #[test]
    fn test_type_schema_object() {
        let schema = TypeSchema::object()
            .with_property("name", TypeSchema::string())
            .with_property("age", TypeSchema::integer())
            .with_required("name");

        assert_eq!(schema.type_name, "object");
        assert!(schema.properties.is_some());
        assert_eq!(schema.properties.as_ref().unwrap().len(), 2);
        assert_eq!(schema.required, vec!["name"]);
    }

    #[test]
    fn test_type_schema_array() {
        let schema = TypeSchema::array(TypeSchema::string());
        assert_eq!(schema.type_name, "array");
        assert!(schema.items.is_some());
    }

    #[test]
    fn test_type_schema_with_constraints() {
        let schema = TypeSchema::string()
            .with_min_length(1)
            .with_max_length(100)
            .with_pattern("^[a-z]+$");

        assert_eq!(schema.min_length, Some(1));
        assert_eq!(schema.max_length, Some(100));
        assert_eq!(schema.pattern, Some("^[a-z]+$".to_string()));
    }

    #[test]
    fn test_type_schema_with_enum() {
        let schema = TypeSchema::string().with_enum(vec!["active", "inactive", "pending"]);

        assert!(schema.enum_values.is_some());
        assert_eq!(schema.enum_values.as_ref().unwrap().len(), 3);
    }

    #[test]
    fn test_type_schema_nullable() {
        let schema = TypeSchema::string().nullable();
        assert!(schema.nullable);
    }

    #[test]
    fn test_type_schema_with_example() {
        let schema = TypeSchema::string().with_example("hello@example.com");
        assert!(schema.example.is_some());
        assert_eq!(schema.example.unwrap(), json!("hello@example.com"));
    }

    #[test]
    fn test_openapi_from_router_schema() {
        let schema = RouterSchema::new()
            .with_name("Test API")
            .with_version("1.0.0")
            .add_procedure(
                "user.get",
                ProcedureSchema::query()
                    .with_description("Get a user")
                    .with_input(TypeSchema::object().with_property("id", TypeSchema::integer()))
                    .with_output(TypeSchema::object().with_property("name", TypeSchema::string())),
            )
            .add_procedure(
                "user.create",
                ProcedureSchema::mutation()
                    .with_description("Create a user")
                    .with_input(TypeSchema::object().with_property("name", TypeSchema::string())),
            );

        let openapi = schema.to_openapi();

        assert_eq!(openapi.openapi, "3.0.3");
        assert_eq!(openapi.info.title, "Test API");
        assert_eq!(openapi.info.version, "1.0.0");
        assert_eq!(openapi.paths.len(), 2);
        assert!(openapi.paths.contains_key("/rpc/user/get"));
        assert!(openapi.paths.contains_key("/rpc/user/create"));
    }

    #[test]
    fn test_openapi_query_uses_get() {
        let schema = RouterSchema::new().add_procedure("health", ProcedureSchema::query());

        let openapi = schema.to_openapi();
        let path = openapi.paths.get("/rpc/health").unwrap();

        assert!(path.get.is_some());
        assert!(path.post.is_none());
    }

    #[test]
    fn test_openapi_mutation_uses_post() {
        let schema = RouterSchema::new().add_procedure("user.create", ProcedureSchema::mutation());

        let openapi = schema.to_openapi();
        let path = openapi.paths.get("/rpc/user/create").unwrap();

        assert!(path.post.is_some());
        assert!(path.get.is_none());
    }

    #[test]
    fn test_openapi_to_json() {
        let schema = RouterSchema::new()
            .with_name("Test API")
            .add_procedure("health", ProcedureSchema::query());

        let openapi = schema.to_openapi();
        let json = openapi.to_json_pretty();

        assert!(json.contains("openapi"));
        assert!(json.contains("3.0.3"));
        assert!(json.contains("Test API"));
    }

    #[test]
    fn test_schema_builder() {
        let schema = SchemaBuilder::new()
            .version("2.0.0")
            .name("My API")
            .description("A test API")
            .query("user.get", ProcedureSchema::query())
            .mutation("user.create", ProcedureSchema::mutation())
            .subscription("events", ProcedureSchema::subscription())
            .build();

        assert_eq!(schema.version, "2.0.0");
        assert_eq!(schema.name, Some("My API".to_string()));
        assert_eq!(schema.procedures.len(), 3);
    }

    #[test]
    fn test_procedure_type_schema_from() {
        assert_eq!(
            ProcedureTypeSchema::from(ProcedureType::Query),
            ProcedureTypeSchema::Query
        );
        assert_eq!(
            ProcedureTypeSchema::from(ProcedureType::Mutation),
            ProcedureTypeSchema::Mutation
        );
        assert_eq!(
            ProcedureTypeSchema::from(ProcedureType::Subscription),
            ProcedureTypeSchema::Subscription
        );
    }

    #[test]
    fn test_procedure_schema_from_procedure_type() {
        let query = ProcedureSchema::from_procedure_type(ProcedureType::Query);
        assert_eq!(query.procedure_type, ProcedureTypeSchema::Query);

        let mutation = ProcedureSchema::from_procedure_type(ProcedureType::Mutation);
        assert_eq!(mutation.procedure_type, ProcedureTypeSchema::Mutation);

        let subscription = ProcedureSchema::from_procedure_type(ProcedureType::Subscription);
        assert_eq!(
            subscription.procedure_type,
            ProcedureTypeSchema::Subscription
        );
    }

    #[test]
    fn test_router_schema_with_metadata() {
        let schema = RouterSchema::new().with_metadata(json!({
            "author": "Test",
            "license": "MIT"
        }));

        assert!(schema.metadata.is_some());
        let meta = schema.metadata.unwrap();
        assert_eq!(meta["author"], "Test");
    }

    #[test]
    fn test_procedure_schema_with_metadata() {
        let proc = ProcedureSchema::query().with_metadata(json!({
            "rate_limit": 100,
            "cache_ttl": 60
        }));

        assert!(proc.metadata.is_some());
        let meta = proc.metadata.unwrap();
        assert_eq!(meta["rate_limit"], 100);
    }

    #[test]
    fn test_type_schema_number_constraints() {
        let schema = TypeSchema::number().with_minimum(0.0).with_maximum(100.0);

        assert_eq!(schema.minimum, Some(0.0));
        assert_eq!(schema.maximum, Some(100.0));
    }

    #[test]
    fn test_schema_serialization_roundtrip() {
        let schema = RouterSchema::new()
            .with_name("Test API")
            .with_version("1.0.0")
            .add_procedure(
                "user.get",
                ProcedureSchema::query()
                    .with_description("Get user")
                    .with_input(TypeSchema::object().with_property("id", TypeSchema::integer())),
            );

        let json = schema.to_json();
        let parsed: RouterSchema = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.name, schema.name);
        assert_eq!(parsed.version, schema.version);
        assert_eq!(parsed.procedures.len(), schema.procedures.len());
    }
}
