//! Parser module for extracting type information from Rust AST.
//!
//! This module contains parsers for:
//! - Struct definitions
//! - Enum definitions
//! - Field types
//! - Attributes
//! - Type resolution

pub mod attributes;
pub mod enum_parser;
pub mod field_parser;
pub mod struct_parser;
pub mod type_parser;

#[cfg(feature = "serde-compat")]
pub mod serde_compat;

// Re-export attribute types for convenience
pub use attributes::{ContainerAttrs, FieldAttrs, RenameRule, VariantAttrs};

// Re-export type parser
pub use type_parser::{ParseError, TypeParser};

// Re-export struct parser
pub use struct_parser::{extract_doc_comments, StructParseError, StructParser};

// Re-export field parser
pub use field_parser::{FieldParseError, FieldParser};

// Re-export enum parser
pub use enum_parser::{EnumParseError, EnumParser};
