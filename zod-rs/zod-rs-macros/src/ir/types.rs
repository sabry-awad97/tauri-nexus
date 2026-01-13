//! Type IR definitions.
//!
//! This module defines the type representation structures that form the core
//! of the intermediate representation. These types are schema-agnostic and
//! can be consumed by any code generator.

use serde::{Deserialize, Serialize};

/// Type intermediate representation.
///
/// Represents a Rust type in a schema-agnostic way that can be
/// transformed into various schema formats (Zod, JSON Schema, etc.).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypeIR {
    /// The kind of type
    pub kind: TypeKind,

    /// Whether this type is nullable at this level
    pub nullable: bool,

    /// Original Rust type string (for debugging and error messages)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_type: Option<String>,
}

impl TypeIR {
    /// Create a new TypeIR with the given kind.
    pub fn new(kind: TypeKind) -> Self {
        Self {
            kind,
            nullable: false,
            original_type: None,
        }
    }

    /// Create a new TypeIR with nullable flag set.
    #[allow(unused)]
    pub fn nullable(kind: TypeKind) -> Self {
        Self {
            kind,
            nullable: true,
            original_type: None,
        }
    }

    /// Set the original type string for debugging.
    pub fn with_original_type(mut self, original: impl Into<String>) -> Self {
        self.original_type = Some(original.into());
        self
    }

    /// Mark this type as nullable.
    #[allow(unused)]
    #[allow(clippy::wrong_self_convention)]
    pub fn as_nullable(mut self) -> Self {
        self.nullable = true;
        self
    }
}

/// Type kind enumeration.
///
/// Represents all possible type variants that can be expressed in the IR.
/// This covers primitives, compound types, special types, and references.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum TypeKind {
    // ==========================================================================
    // Primitives
    // ==========================================================================
    /// String type (Rust: String, &str)
    String,

    /// Boolean type (Rust: bool)
    Boolean,

    /// Character type (Rust: char) - maps to single-char string
    Char,

    /// Integer type with signedness and bit width
    Integer {
        /// Whether the integer is signed
        signed: bool,
        /// Bit width (8, 16, 32, 64, 128, or None for isize/usize)
        #[serde(skip_serializing_if = "Option::is_none")]
        bits: Option<u8>,
    },

    /// Floating point type (Rust: f32, f64)
    Float,

    // ==========================================================================
    // Compound Types
    // ==========================================================================
    /// Array/Vec type
    Array(Box<TypeIR>),

    /// Tuple type with multiple elements
    Tuple(Vec<TypeIR>),

    /// Record/Map type with key and value types
    Record {
        key: Box<TypeIR>,
        value: Box<TypeIR>,
    },

    /// Set type (maps to array in most schemas)
    Set(Box<TypeIR>),

    /// Optional type (Rust: Option<T>)
    Optional(Box<TypeIR>),

    // ==========================================================================
    // Special Types
    // ==========================================================================
    /// UUID type (requires uuid feature)
    Uuid,

    /// DateTime type (requires chrono feature)
    DateTime,

    /// Duration type
    Duration,

    /// Decimal type for precise numeric values
    Decimal,

    /// Any type - accepts any value
    Any,

    /// Unknown type - type is not known at compile time
    Unknown,

    /// Never type - represents impossible values
    Never,

    /// Null type - represents null/None
    Null,

    /// Void type - represents no value
    Void,

    // ==========================================================================
    // Reference Types
    // ==========================================================================
    /// Reference to another schema by name
    Reference {
        /// The name of the referenced type
        name: String,
        /// Generic type parameters
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        generics: Vec<TypeIR>,
    },

    // ==========================================================================
    // Literal Types
    // ==========================================================================
    /// Literal string value
    LiteralString(String),

    /// Literal number value
    LiteralNumber(f64),

    /// Literal boolean value
    LiteralBoolean(bool),

    // ==========================================================================
    // Composite Types
    // ==========================================================================
    /// Union of multiple types (T | U | V)
    Union(Vec<TypeIR>),

    /// Intersection of multiple types (T & U & V)
    Intersection(Vec<TypeIR>),
}

impl TypeKind {
    /// Create a signed integer type with the given bit width.
    #[allow(unused)]
    pub fn signed_int(bits: u8) -> Self {
        TypeKind::Integer {
            signed: true,
            bits: Some(bits),
        }
    }

    /// Create an unsigned integer type with the given bit width.
    #[allow(unused)]
    pub fn unsigned_int(bits: u8) -> Self {
        TypeKind::Integer {
            signed: false,
            bits: Some(bits),
        }
    }

    /// Create a reference to another type by name.
    #[allow(unused)]
    pub fn reference(name: impl Into<String>) -> Self {
        TypeKind::Reference {
            name: name.into(),
            generics: Vec::new(),
        }
    }

    /// Create a reference with generic parameters.
    #[allow(unused)]
    pub fn reference_with_generics(name: impl Into<String>, generics: Vec<TypeIR>) -> Self {
        TypeKind::Reference {
            name: name.into(),
            generics,
        }
    }

    /// Check if this is a primitive type.
    #[allow(unused)]
    pub fn is_primitive(&self) -> bool {
        matches!(
            self,
            TypeKind::String
                | TypeKind::Boolean
                | TypeKind::Char
                | TypeKind::Integer { .. }
                | TypeKind::Float
        )
    }

    /// Check if this is a compound type.
    #[allow(unused)]
    pub fn is_compound(&self) -> bool {
        matches!(
            self,
            TypeKind::Array(_)
                | TypeKind::Tuple(_)
                | TypeKind::Record { .. }
                | TypeKind::Set(_)
                | TypeKind::Optional(_)
        )
    }

    /// Check if this is a reference type.
    #[allow(unused)]
    pub fn is_reference(&self) -> bool {
        matches!(self, TypeKind::Reference { .. })
    }
}

/// Generic parameter definition.
///
/// Represents a generic type parameter with optional bounds and default.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GenericParam {
    /// Parameter name (e.g., "T", "K", "V")
    pub name: String,

    /// Trait bounds on this parameter
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bounds: Vec<String>,

    /// Default type if not specified
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<Box<TypeIR>>,
}

impl GenericParam {
    /// Create a new generic parameter with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            bounds: Vec::new(),
            default: None,
        }
    }

    /// Add a trait bound to this parameter.
    pub fn with_bound(mut self, bound: impl Into<String>) -> Self {
        self.bounds.push(bound.into());
        self
    }

    /// Set the default type for this parameter.
    #[allow(unused)]
    pub fn with_default(mut self, default: TypeIR) -> Self {
        self.default = Some(Box::new(default));
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_ir_creation() {
        let ty = TypeIR::new(TypeKind::String);
        assert_eq!(ty.kind, TypeKind::String);
        assert!(!ty.nullable);
        assert!(ty.original_type.is_none());
    }

    #[test]
    fn test_type_ir_nullable() {
        let ty = TypeIR::nullable(TypeKind::String);
        assert!(ty.nullable);
    }

    #[test]
    fn test_type_ir_with_original() {
        let ty = TypeIR::new(TypeKind::String).with_original_type("String");
        assert_eq!(ty.original_type, Some("String".to_string()));
    }

    #[test]
    fn test_type_kind_primitives() {
        assert!(TypeKind::String.is_primitive());
        assert!(TypeKind::Boolean.is_primitive());
        assert!(TypeKind::Char.is_primitive());
        assert!(TypeKind::signed_int(32).is_primitive());
        assert!(TypeKind::Float.is_primitive());
    }

    #[test]
    fn test_type_kind_compound() {
        let inner = TypeIR::new(TypeKind::String);
        assert!(TypeKind::Array(Box::new(inner.clone())).is_compound());
        assert!(TypeKind::Optional(Box::new(inner)).is_compound());
    }

    #[test]
    fn test_type_kind_reference() {
        assert!(TypeKind::reference("User").is_reference());
        assert!(!TypeKind::String.is_reference());
    }

    #[test]
    fn test_generic_param() {
        let param = GenericParam::new("T")
            .with_bound("ZodSchema")
            .with_default(TypeIR::new(TypeKind::String));

        assert_eq!(param.name, "T");
        assert_eq!(param.bounds, vec!["ZodSchema"]);
        assert!(param.default.is_some());
    }
}
