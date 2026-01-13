//! Schema IR definitions.
//!
//! This module defines the root schema structures that represent
//! complete type definitions (structs, enums, type aliases).

use serde::{Deserialize, Serialize};

use super::metadata::SchemaMetadata;
use super::types::{GenericParam, TypeIR};
use super::validation::ValidationRule;

/// Root schema IR for a type definition.
///
/// This is the top-level structure that represents a complete Rust type
/// (struct, enum, or type alias) in the intermediate representation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SchemaIR {
    /// Type name in the generated schema (may be renamed)
    pub name: String,

    /// Original Rust type name
    pub rust_name: String,

    /// Schema kind (struct, enum, etc.)
    pub kind: SchemaKind,

    /// Generic parameters
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub generics: Vec<GenericParam>,

    /// Metadata (description, deprecated, etc.)
    #[serde(default)]
    pub metadata: SchemaMetadata,

    /// Whether to export this schema in the contract
    #[serde(default = "default_export")]
    pub export: bool,
}

fn default_export() -> bool {
    true
}

impl SchemaIR {
    /// Create a new SchemaIR with the given name and kind.
    pub fn new(name: impl Into<String>, kind: SchemaKind) -> Self {
        let name = name.into();
        Self {
            rust_name: name.clone(),
            name,
            kind,
            generics: Vec::new(),
            metadata: SchemaMetadata::default(),
            export: true,
        }
    }

    /// Set the schema name (for renaming).
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Add generic parameters.
    pub fn with_generics(mut self, generics: Vec<GenericParam>) -> Self {
        self.generics = generics;
        self
    }

    /// Set metadata.
    pub fn with_metadata(mut self, metadata: SchemaMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Set export flag.
    pub fn with_export(mut self, export: bool) -> Self {
        self.export = export;
        self
    }

    /// Check if this schema has generic parameters.
    #[allow(unused)]
    pub fn has_generics(&self) -> bool {
        !self.generics.is_empty()
    }
}

/// Kind of schema definition.
///
/// Represents the different kinds of type definitions that can be
/// expressed in the IR.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SchemaKind {
    /// Struct with named fields
    Struct(StructSchema),

    /// Tuple struct with unnamed fields
    TupleStruct(TupleStructSchema),

    /// Unit struct (no fields)
    UnitStruct,

    /// Enum with variants
    Enum(EnumSchema),

    /// Type alias
    Alias(TypeIR),
}

/// Struct schema definition.
///
/// Represents a Rust struct with named fields.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StructSchema {
    /// Fields in declaration order
    pub fields: Vec<FieldIR>,

    /// Strict mode - reject extra properties (Zod: .strict())
    #[serde(default)]
    pub strict: bool,

    /// Passthrough mode - allow extra properties (Zod: .passthrough())
    #[serde(default)]
    pub passthrough: bool,
}

impl StructSchema {
    /// Create a new struct schema with the given fields.
    pub fn new(fields: Vec<FieldIR>) -> Self {
        Self {
            fields,
            strict: false,
            passthrough: false,
        }
    }

    /// Set strict mode.
    pub fn with_strict(mut self, strict: bool) -> Self {
        self.strict = strict;
        self
    }

    /// Set passthrough mode.
    #[allow(unused)]
    pub fn with_passthrough(mut self, passthrough: bool) -> Self {
        self.passthrough = passthrough;
        self
    }
}

/// Tuple struct schema definition.
///
/// Represents a Rust tuple struct with unnamed fields.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TupleStructSchema {
    /// Field types in order
    pub fields: Vec<TypeIR>,
}

impl TupleStructSchema {
    /// Create a new tuple struct schema with the given field types.
    pub fn new(fields: Vec<TypeIR>) -> Self {
        Self { fields }
    }
}

/// Field intermediate representation.
///
/// Represents a single field in a struct.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldIR {
    /// Field name in Rust
    pub rust_name: String,

    /// Field name in schema (after rename transformations)
    pub schema_name: String,

    /// Field type
    pub ty: TypeIR,

    /// Whether field is optional (.optional() in Zod)
    #[serde(default)]
    pub optional: bool,

    /// Whether field is nullable (.nullable() in Zod)
    #[serde(default)]
    pub nullable: bool,

    /// Default value expression
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,

    /// Whether to flatten this field's properties into parent
    #[serde(default)]
    pub flatten: bool,

    /// Validation rules applied to this field
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub validation: Vec<ValidationRule>,

    /// Field metadata (description, deprecated, etc.)
    #[serde(default)]
    pub metadata: FieldMetadata,
}

impl FieldIR {
    /// Create a new field with the given name and type.
    pub fn new(name: impl Into<String>, ty: TypeIR) -> Self {
        let name = name.into();
        Self {
            rust_name: name.clone(),
            schema_name: name,
            ty,
            optional: false,
            nullable: false,
            default: None,
            flatten: false,
            validation: Vec::new(),
            metadata: FieldMetadata::default(),
        }
    }

    /// Set the schema name (for renaming).
    pub fn with_schema_name(mut self, name: impl Into<String>) -> Self {
        self.schema_name = name.into();
        self
    }

    /// Mark as optional.
    pub fn with_optional(mut self, optional: bool) -> Self {
        self.optional = optional;
        self
    }

    /// Mark as nullable.
    pub fn with_nullable(mut self, nullable: bool) -> Self {
        self.nullable = nullable;
        self
    }

    /// Set default value.
    pub fn with_default(mut self, default: impl Into<String>) -> Self {
        self.default = Some(default.into());
        self
    }

    /// Mark as flattened.
    pub fn with_flatten(mut self, flatten: bool) -> Self {
        self.flatten = flatten;
        self
    }

    /// Add validation rules.
    pub fn with_validation(mut self, validation: Vec<ValidationRule>) -> Self {
        self.validation = validation;
        self
    }

    /// Add a single validation rule.
    #[allow(unused)]
    pub fn add_validation(mut self, rule: ValidationRule) -> Self {
        self.validation.push(rule);
        self
    }

    /// Set field metadata.
    pub fn with_metadata(mut self, metadata: FieldMetadata) -> Self {
        self.metadata = metadata;
        self
    }
}

/// Field metadata.
///
/// Contains additional information about a field that doesn't affect
/// the schema structure but provides documentation and hints.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct FieldMetadata {
    /// Field description (from doc comments or attribute)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Whether the field is deprecated
    #[serde(default)]
    pub deprecated: bool,

    /// Example values for documentation
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub examples: Vec<String>,
}

impl FieldMetadata {
    /// Create metadata with a description.
    #[allow(unused)]
    pub fn with_description(description: impl Into<String>) -> Self {
        Self {
            description: Some(description.into()),
            deprecated: false,
            examples: Vec::new(),
        }
    }

    /// Mark as deprecated.
    #[allow(unused)]
    #[allow(clippy::wrong_self_convention)]
    pub fn as_deprecated(mut self) -> Self {
        self.deprecated = true;
        self
    }

    /// Add an example value.
    #[allow(unused)]
    pub fn with_example(mut self, example: impl Into<String>) -> Self {
        self.examples.push(example.into());
        self
    }
}

/// Enum schema definition.
///
/// Represents a Rust enum with its variants and tagging strategy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnumSchema {
    /// Enum variants
    pub variants: Vec<VariantIR>,

    /// Tagging strategy for serialization
    #[serde(default)]
    pub tagging: EnumTagging,

    /// Whether all variants are unit variants (for z.enum optimization)
    #[serde(default)]
    pub is_unit_only: bool,
}

impl EnumSchema {
    /// Create a new enum schema with the given variants.
    pub fn new(variants: Vec<VariantIR>) -> Self {
        let is_unit_only = variants.iter().all(|v| matches!(v.kind, VariantKind::Unit));
        Self {
            variants,
            tagging: EnumTagging::default(),
            is_unit_only,
        }
    }

    /// Set the tagging strategy.
    pub fn with_tagging(mut self, tagging: EnumTagging) -> Self {
        self.tagging = tagging;
        self
    }

    /// Recalculate is_unit_only based on current variants.
    #[allow(unused)]
    pub fn update_unit_only(&mut self) {
        self.is_unit_only = self
            .variants
            .iter()
            .all(|v| matches!(v.kind, VariantKind::Unit));
    }
}

/// Enum tagging strategy.
///
/// Determines how enum variants are represented in the serialized form.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "strategy")]
pub enum EnumTagging {
    /// External tagging: `{ "VariantName": { ...data } }`
    #[default]
    External,

    /// Internal tagging: `{ "type": "VariantName", ...data }`
    Internal {
        /// The tag field name
        tag: String,
    },

    /// Adjacent tagging: `{ "type": "VariantName", "content": { ...data } }`
    Adjacent {
        /// The tag field name
        tag: String,
        /// The content field name
        content: String,
    },

    /// Untagged: just the data, no discriminator
    Untagged,
}

impl EnumTagging {
    /// Create internal tagging with the given tag field name.
    #[allow(unused)]
    pub fn internal(tag: impl Into<String>) -> Self {
        EnumTagging::Internal { tag: tag.into() }
    }

    /// Create adjacent tagging with the given field names.
    #[allow(unused)]
    pub fn adjacent(tag: impl Into<String>, content: impl Into<String>) -> Self {
        EnumTagging::Adjacent {
            tag: tag.into(),
            content: content.into(),
        }
    }
}

/// Enum variant intermediate representation.
///
/// Represents a single variant in an enum.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VariantIR {
    /// Variant name in Rust
    pub rust_name: String,

    /// Variant name in schema (after rename)
    pub schema_name: String,

    /// Variant kind (unit, tuple, struct)
    pub kind: VariantKind,

    /// Variant metadata
    #[serde(default)]
    pub metadata: FieldMetadata,
}

impl VariantIR {
    /// Create a new unit variant.
    pub fn unit(name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            rust_name: name.clone(),
            schema_name: name,
            kind: VariantKind::Unit,
            metadata: FieldMetadata::default(),
        }
    }

    /// Create a new tuple variant.
    pub fn tuple(name: impl Into<String>, fields: Vec<TypeIR>) -> Self {
        let name = name.into();
        Self {
            rust_name: name.clone(),
            schema_name: name,
            kind: VariantKind::Tuple(fields),
            metadata: FieldMetadata::default(),
        }
    }

    /// Create a new struct variant.
    pub fn struct_variant(name: impl Into<String>, fields: Vec<FieldIR>) -> Self {
        let name = name.into();
        Self {
            rust_name: name.clone(),
            schema_name: name,
            kind: VariantKind::Struct(fields),
            metadata: FieldMetadata::default(),
        }
    }

    /// Set the schema name (for renaming).
    pub fn with_schema_name(mut self, name: impl Into<String>) -> Self {
        self.schema_name = name.into();
        self
    }

    /// Set variant metadata.
    pub fn with_metadata(mut self, metadata: FieldMetadata) -> Self {
        self.metadata = metadata;
        self
    }
}

/// Variant kind.
///
/// Represents the different kinds of enum variants.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "fields")]
pub enum VariantKind {
    /// Unit variant: `Variant`
    Unit,

    /// Tuple variant: `Variant(T1, T2)`
    Tuple(Vec<TypeIR>),

    /// Struct variant: `Variant { field: T }`
    Struct(Vec<FieldIR>),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::types::TypeKind;

    #[test]
    fn test_schema_ir_creation() {
        let schema = SchemaIR::new("User", SchemaKind::Struct(StructSchema::new(vec![])));
        assert_eq!(schema.name, "User");
        assert_eq!(schema.rust_name, "User");
        assert!(schema.export);
    }

    #[test]
    fn test_schema_ir_with_rename() {
        let schema = SchemaIR::new("User", SchemaKind::Struct(StructSchema::new(vec![])))
            .with_name("UserDTO");
        assert_eq!(schema.name, "UserDTO");
        assert_eq!(schema.rust_name, "User");
    }

    #[test]
    fn test_struct_schema() {
        let field = FieldIR::new("id", TypeIR::new(TypeKind::signed_int(64)));
        let schema = StructSchema::new(vec![field]).with_strict(true);
        assert!(schema.strict);
        assert!(!schema.passthrough);
        assert_eq!(schema.fields.len(), 1);
    }

    #[test]
    fn test_field_ir() {
        let field = FieldIR::new("email", TypeIR::new(TypeKind::String))
            .with_schema_name("emailAddress")
            .with_optional(true)
            .with_default("\"\"");

        assert_eq!(field.rust_name, "email");
        assert_eq!(field.schema_name, "emailAddress");
        assert!(field.optional);
        assert_eq!(field.default, Some("\"\"".to_string()));
    }

    #[test]
    fn test_enum_schema_unit_only() {
        let variants = vec![VariantIR::unit("Active"), VariantIR::unit("Inactive")];
        let schema = EnumSchema::new(variants);
        assert!(schema.is_unit_only);
    }

    #[test]
    fn test_enum_schema_with_data() {
        let variants = vec![
            VariantIR::unit("None"),
            VariantIR::tuple("Some", vec![TypeIR::new(TypeKind::String)]),
        ];
        let schema = EnumSchema::new(variants);
        assert!(!schema.is_unit_only);
    }

    #[test]
    fn test_enum_tagging() {
        assert!(matches!(EnumTagging::default(), EnumTagging::External));

        let internal = EnumTagging::internal("type");
        assert!(matches!(internal, EnumTagging::Internal { tag } if tag == "type"));

        let adjacent = EnumTagging::adjacent("t", "c");
        assert!(
            matches!(adjacent, EnumTagging::Adjacent { tag, content } if tag == "t" && content == "c")
        );
    }

    #[test]
    fn test_variant_ir() {
        let unit = VariantIR::unit("None");
        assert!(matches!(unit.kind, VariantKind::Unit));

        let tuple = VariantIR::tuple(
            "Point",
            vec![TypeIR::new(TypeKind::Float), TypeIR::new(TypeKind::Float)],
        );
        assert!(matches!(tuple.kind, VariantKind::Tuple(ref fields) if fields.len() == 2));

        let struct_var = VariantIR::struct_variant(
            "User",
            vec![FieldIR::new("name", TypeIR::new(TypeKind::String))],
        );
        assert!(matches!(struct_var.kind, VariantKind::Struct(ref fields) if fields.len() == 1));
    }
}
