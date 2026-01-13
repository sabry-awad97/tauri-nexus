//! Zod schema generator.
//!
//! This module implements the Zod schema code generator that transforms
//! the intermediate representation (IR) into TypeScript Zod schema code.
//!
//! # Components
//!
//! - [`ZodEmitter`] - The main code generator implementing [`CodeGenerator`]
//! - [`ZodTypeMapper`] - Maps Rust types to Zod schema strings
//!
//! # Example
//!
//! ```rust,ignore
//! use zod_rs_macros::generator::zod::ZodEmitter;
//! use zod_rs_macros::generator::{CodeGenerator, GeneratorConfig};
//!
//! let emitter = ZodEmitter::new();
//! let config = GeneratorConfig::default();
//! let schema = // ... create SchemaIR ...
//! let generated = emitter.generate(&schema, &config)?;
//! println!("{}", generated.code);
//! ```

pub mod emitter;
pub mod formatter;
pub mod type_mapper;

// Re-export main types for convenience
#[allow(unused)]
pub use emitter::ZodEmitter;

#[allow(unused)]
pub use type_mapper::ZodTypeMapper;
