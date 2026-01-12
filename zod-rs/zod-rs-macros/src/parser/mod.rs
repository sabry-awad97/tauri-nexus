//! Parser module for extracting type information from Rust AST.
//!
//! This module contains parsers for:
//! - Struct definitions
//! - Enum definitions
//! - Field types
//! - Attributes

pub mod attributes;
pub mod enum_parser;
pub mod field_parser;
pub mod struct_parser;
pub mod type_parser;

#[cfg(feature = "serde-compat")]
pub mod serde_compat;
