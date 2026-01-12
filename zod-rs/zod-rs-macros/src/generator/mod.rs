//! Code generator module.
//!
//! This module defines the code generator trait and implementations
//! for various output formats.
//!
//! # Architecture
//!
//! The generator module follows a pluggable architecture:
//!
//! - [`CodeGenerator`] trait defines the interface all generators implement
//! - [`GeneratorConfig`] provides configuration options for code generation
//! - [`GeneratedCode`] represents the output of code generation
//! - [`GeneratorFeature`] allows checking generator capabilities
//!
//! # Generators
//!
//! Currently implemented generators:
//!
//! - `zod` - Generates TypeScript Zod schemas
//!
//! # Example
//!
//! ```rust,ignore
//! use zod_rs_macros::generator::{CodeGenerator, GeneratorConfig};
//! use zod_rs_macros::generator::zod::ZodEmitter;
//! use zod_rs_macros::ir::SchemaIR;
//!
//! let generator = ZodEmitter::new();
//! let config = GeneratorConfig::default();
//!
//! let schema = // ... create SchemaIR ...
//! let generated = generator.generate(&schema, &config)?;
//! println!("{}", generated.code);
//! ```

pub mod traits;
pub mod zod;

// Re-export main types for convenience
pub use traits::{
    CodeGenerator, GeneratedCode, GeneratorConfig, GeneratorFeature, IndentStyle, LineEnding,
    OutputStyle,
};
