//! Type definitions for schema metadata and configuration.

#[cfg(not(feature = "std"))]
use alloc::{string::String, vec::Vec};

/// Metadata associated with a schema.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde-compat", derive(serde::Serialize, serde::Deserialize))]
pub struct SchemaMetadata {
    /// Description of the type/field.
    pub description: Option<String>,

    /// Whether the type is deprecated.
    pub deprecated: bool,

    /// Deprecation message if deprecated.
    pub deprecation_message: Option<String>,

    /// Example values.
    pub examples: Vec<String>,

    /// Tags for categorization.
    pub tags: Vec<String>,
}

impl SchemaMetadata {
    /// Create new empty metadata.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Mark as deprecated.
    pub fn with_deprecated(mut self, deprecated: bool) -> Self {
        self.deprecated = deprecated;
        self
    }

    /// Set deprecation message.
    pub fn with_deprecation_message(mut self, message: impl Into<String>) -> Self {
        self.deprecation_message = Some(message.into());
        self.deprecated = true;
        self
    }

    /// Add an example.
    pub fn with_example(mut self, example: impl Into<String>) -> Self {
        self.examples.push(example.into());
        self
    }

    /// Add a tag.
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }
}

/// Represents a complete TypeScript schema with its metadata.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde-compat", derive(serde::Serialize, serde::Deserialize))]
pub struct TypeSchema {
    /// The schema name (e.g., "UserSchema").
    pub name: String,

    /// The TypeScript type name (e.g., "User").
    pub type_name: String,

    /// The Zod schema string.
    pub schema: String,

    /// Schema metadata.
    pub metadata: SchemaMetadata,

    /// Dependencies on other schemas.
    pub dependencies: Vec<String>,

    /// Whether to export this schema.
    pub export: bool,
}

impl TypeSchema {
    /// Create a new TypeSchema.
    pub fn new(
        name: impl Into<String>,
        type_name: impl Into<String>,
        schema: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            type_name: type_name.into(),
            schema: schema.into(),
            metadata: SchemaMetadata::default(),
            dependencies: Vec::new(),
            export: true,
        }
    }

    /// Set metadata.
    pub fn with_metadata(mut self, metadata: SchemaMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Add a dependency.
    pub fn with_dependency(mut self, dep: impl Into<String>) -> Self {
        self.dependencies.push(dep.into());
        self
    }

    /// Set export status.
    pub fn with_export(mut self, export: bool) -> Self {
        self.export = export;
        self
    }

    /// Generate the full TypeScript declaration.
    pub fn to_typescript(&self) -> String {
        let mut result = String::new();

        // Add JSDoc comment if there's a description
        if let Some(desc) = &self.metadata.description {
            result.push_str("/**\n");
            result.push_str(&format!(" * {}\n", desc));
            if self.metadata.deprecated {
                if let Some(msg) = &self.metadata.deprecation_message {
                    result.push_str(&format!(" * @deprecated {}\n", msg));
                } else {
                    result.push_str(" * @deprecated\n");
                }
            }
            result.push_str(" */\n");
        }

        // Export the schema
        if self.export {
            result.push_str(&format!("export const {} = {};\n", self.name, self.schema));
            result.push_str(&format!(
                "export type {} = z.infer<typeof {}>;\n",
                self.type_name, self.name
            ));
        } else {
            result.push_str(&format!("const {} = {};\n", self.name, self.schema));
            result.push_str(&format!(
                "type {} = z.infer<typeof {}>;\n",
                self.type_name, self.name
            ));
        }

        result
    }
}
