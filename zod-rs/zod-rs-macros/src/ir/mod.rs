//! Intermediate Representation (IR) module.
//!
//! This module defines the schema-agnostic data structures that represent
//! parsed Rust types. The IR is consumed by code generators to produce
//! output in various formats (Zod, JSON Schema, etc.).

pub mod metadata;
pub mod schema;
pub mod types;
pub mod validation;
