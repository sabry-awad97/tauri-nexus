//! Intermediate Representation (IR) module.
//!
//! This module defines the schema-agnostic data structures that represent
//! parsed Rust types. The IR is consumed by code generators to produce
//! output in various formats (Zod, JSON Schema, etc.).
//!
//! # Architecture
//!
//! The IR is designed to be:
//! - **Schema-agnostic**: Can be transformed into any schema format
//! - **Serializable**: Can be serialized to JSON for debugging and tooling
//! - **Complete**: Contains all information needed for code generation
//! - **Deterministic**: Same input always produces the same IR
//!
//! # Main Types
//!
//! - [`SchemaIR`]: Root type representing a complete type definition
//! - [`TypeIR`]: Represents a type reference with nullability
//! - [`TypeKind`]: Enumeration of all possible type kinds
//! - [`FieldIR`]: Represents a struct field with validation
//! - [`ValidationRule`]: Validation constraints on fields
//!
//! # Example
//!
//! ```rust,ignore
//! use zod_rs_macros::ir::*;
//!
//! // Create a simple struct schema
//! let schema = SchemaIR::new(
//!     "User",
//!     SchemaKind::Struct(StructSchema::new(vec![
//!         FieldIR::new("id", TypeIR::new(TypeKind::signed_int(64))),
//!         FieldIR::new("email", TypeIR::new(TypeKind::String))
//!             .add_validation(ValidationRule::Email),
//!     ])),
//! );
//! ```

pub mod metadata;
pub mod schema;
pub mod types;
pub mod validation;

#[cfg(test)]
mod proptest;

// Re-export main types for convenience
pub use metadata::SchemaMetadata;
pub use schema::{
    EnumSchema, EnumTagging, FieldIR, FieldMetadata, SchemaIR, SchemaKind, StructSchema,
    TupleStructSchema, VariantIR, VariantKind,
};
pub use types::{GenericParam, TypeIR, TypeKind};
pub use validation::ValidationRule;
