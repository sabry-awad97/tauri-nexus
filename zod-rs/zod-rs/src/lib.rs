//! # zod-rs
//!
//! A Rust crate for generating TypeScript Zod schemas from Rust types.
//!
//! This crate provides the runtime traits and types needed for Zod schema generation.
//! Use the `zod-rs-macros` crate for the `#[derive(ZodSchema)]` procedural macro.
//!
//! ## Features
//!
//! - `std` - Standard library support (enabled by default)
//! - `serde-compat` - Respect serde attributes (enabled by default)
//! - `chrono` - Support for chrono DateTime types
//! - `uuid` - Support for uuid::Uuid type
//! - `tauri` - Tauri framework integration
//!
//! ## Example
//!
//! ```rust,ignore
//! use zod_rs::ZodSchema;
//!
//! #[derive(ZodSchema)]
//! struct User {
//!     name: String,
//!     age: u32,
//! }
//!
//! // Get the Zod schema string
//! let schema = User::zod_schema();
//! ```

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::{string::String, vec::Vec};

pub mod export;
pub mod registry;
pub mod traits;
pub mod types;

// Re-export main trait
pub use registry::SchemaRegistry;
pub use traits::ZodSchema;
pub use types::{SchemaMetadata, TypeSchema};

// Re-export derive macro when available
#[cfg(feature = "zod-rs-macros")]
pub use zod_rs_macros::ZodSchema;
