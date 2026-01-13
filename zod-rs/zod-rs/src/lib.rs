//! # zod-rs
//!
//! A Rust crate for generating TypeScript [Zod](https://zod.dev/) schemas from Rust types.
//!
//! This crate provides the runtime traits and types needed for Zod schema generation.
//! Use the `#[derive(ZodSchema)]` procedural macro to automatically generate Zod schemas
//! for your Rust structs and enums.
//!
//! ## Overview
//!
//! `zod-rs` bridges the gap between Rust and TypeScript by generating type-safe Zod schemas
//! from your Rust type definitions. This ensures your API contracts stay in sync between
//! your Rust backend and TypeScript frontend.
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use zod_rs::ZodSchema;
//!
//! #[derive(ZodSchema)]
//! struct User {
//!     name: String,
//!     age: u32,
//!     email: Option<String>,
//! }
//!
//! // Get the Zod schema string
//! let schema = User::zod_schema();
//! // => "z.object({ name: z.string(), age: z.number().int().nonnegative(), email: z.string().optional() })"
//!
//! // Get the full TypeScript declaration
//! let declaration = User::ts_declaration();
//! // => "export const UserSchema = z.object({ ... });\nexport type User = z.infer<typeof UserSchema>;"
//! ```
//!
//! ## Features
//!
//! | Feature | Description | Default |
//! |---------|-------------|---------|
//! | `std` | Standard library support | ✅ |
//! | `serde-compat` | Respect serde attributes | ✅ |
//! | `chrono` | Support for `chrono::DateTime` types | ❌ |
//! | `uuid` | Support for `uuid::Uuid` type | ❌ |
//! | `tauri` | Tauri framework integration | ❌ |
//!
//! ## Container Attributes
//!
//! These attributes can be applied to structs and enums:
//!
//! | Attribute | Description |
//! |-----------|-------------|
//! | `#[zod(rename = "Name")]` | Rename the type in generated schema |
//! | `#[zod(rename_all = "camelCase")]` | Rename all fields using a naming convention |
//! | `#[zod(tag = "type")]` | Use internal tagging for enums |
//! | `#[zod(tag = "t", content = "c")]` | Use adjacent tagging for enums |
//! | `#[zod(description = "...")]` | Add description to schema |
//! | `#[zod(deprecated)]` | Mark as deprecated |
//! | `#[zod(strict)]` | Use strict mode (no extra properties) |
//!
//! ### Rename Conventions
//!
//! The `rename_all` attribute supports these conventions:
//! - `camelCase` - firstName
//! - `snake_case` - first_name
//! - `PascalCase` - FirstName
//! - `SCREAMING_SNAKE_CASE` - FIRST_NAME
//! - `kebab-case` - first-name
//!
//! ## Field Attributes
//!
//! These attributes can be applied to struct fields:
//!
//! | Attribute | Description |
//! |-----------|-------------|
//! | `#[zod(rename = "name")]` | Rename this field |
//! | `#[zod(skip)]` | Skip this field in schema |
//! | `#[zod(optional)]` | Mark as optional (`.optional()`) |
//! | `#[zod(nullable)]` | Mark as nullable (`.nullable()`) |
//! | `#[zod(default = "value")]` | Set default value |
//! | `#[zod(flatten)]` | Flatten nested object fields |
//! | `#[zod(type = "z.custom()")]` | Override with custom Zod type |
//!
//! ## Validation Attributes
//!
//! ### String Validations
//!
//! | Attribute | Zod Output |
//! |-----------|------------|
//! | `#[zod(min_length = N)]` | `.min(N)` |
//! | `#[zod(max_length = N)]` | `.max(N)` |
//! | `#[zod(length = N)]` | `.length(N)` |
//! | `#[zod(email)]` | `.email()` |
//! | `#[zod(url)]` | `.url()` |
//! | `#[zod(uuid)]` | `.uuid()` |
//! | `#[zod(cuid)]` | `.cuid()` |
//! | `#[zod(datetime)]` | `.datetime()` |
//! | `#[zod(ip)]` | `.ip()` |
//! | `#[zod(regex = "pattern")]` | `.regex(/pattern/)` |
//! | `#[zod(starts_with = "prefix")]` | `.startsWith("prefix")` |
//! | `#[zod(ends_with = "suffix")]` | `.endsWith("suffix")` |
//!
//! ### Number Validations
//!
//! | Attribute | Zod Output |
//! |-----------|------------|
//! | `#[zod(min = N)]` | `.min(N)` |
//! | `#[zod(max = N)]` | `.max(N)` |
//! | `#[zod(positive)]` | `.positive()` |
//! | `#[zod(negative)]` | `.negative()` |
//! | `#[zod(nonnegative)]` | `.nonnegative()` |
//! | `#[zod(nonpositive)]` | `.nonpositive()` |
//! | `#[zod(int)]` | `.int()` |
//! | `#[zod(finite)]` | `.finite()` |
//!
//! ### Array Validations
//!
//! | Attribute | Zod Output |
//! |-----------|------------|
//! | `#[zod(nonempty)]` | `.nonempty()` |
//!
//! ## Type Mappings
//!
//! | Rust Type | Zod Schema |
//! |-----------|------------|
//! | `String`, `&str` | `z.string()` |
//! | `bool` | `z.boolean()` |
//! | `i8`-`i128`, `isize` | `z.number().int()` |
//! | `u8`-`u128`, `usize` | `z.number().int().nonnegative()` |
//! | `f32`, `f64` | `z.number()` |
//! | `char` | `z.string().length(1)` |
//! | `Option<T>` | `T.optional()` |
//! | `Vec<T>` | `z.array(T)` |
//! | `HashMap<K, V>` | `z.record(K, V)` |
//! | `HashSet<T>` | `z.set(T)` |
//! | `Box<T>`, `Arc<T>`, `Rc<T>` | `T` (unwrapped) |
//! | `Uuid` (with feature) | `z.string().uuid()` |
//! | `DateTime` (with feature) | `z.string().datetime()` |
//!
//! ## Examples
//!
//! ### Basic Struct
//!
//! ```rust,ignore
//! use zod_rs::ZodSchema;
//!
//! #[derive(ZodSchema)]
//! struct User {
//!     name: String,
//!     age: u32,
//! }
//! // Generates: z.object({ name: z.string(), age: z.number().int().nonnegative() })
//! ```
//!
//! ### With Validation
//!
//! ```rust,ignore
//! use zod_rs::ZodSchema;
//!
//! #[derive(ZodSchema)]
//! #[zod(rename_all = "camelCase")]
//! struct CreateUser {
//!     #[zod(min_length = 1, max_length = 100)]
//!     user_name: String,
//!
//!     #[zod(min = 0, max = 150)]
//!     age: u32,
//!
//!     #[zod(email)]
//!     email_address: String,
//! }
//! // Generates: z.object({
//! //   userName: z.string().min(1).max(100),
//! //   age: z.number().int().nonnegative().min(0).max(150),
//! //   emailAddress: z.string().email()
//! // })
//! ```
//!
//! ### Unit Enum
//!
//! ```rust,ignore
//! use zod_rs::ZodSchema;
//!
//! #[derive(ZodSchema)]
//! enum Status {
//!     Active,
//!     Inactive,
//!     Pending,
//! }
//! // Generates: z.enum(["Active", "Inactive", "Pending"])
//! ```
//!
//! ### Tagged Enum (Discriminated Union)
//!
//! ```rust,ignore
//! use zod_rs::ZodSchema;
//!
//! #[derive(ZodSchema)]
//! #[zod(tag = "type")]
//! enum Message {
//!     Text { content: String },
//!     Image { url: String },
//! }
//! // Generates: z.discriminatedUnion("type", [
//! //   z.object({ type: z.literal("Text"), content: z.string() }),
//! //   z.object({ type: z.literal("Image"), url: z.string() })
//! // ])
//! ```
//!
//! ### Serde Compatibility
//!
//! When the `serde-compat` feature is enabled (default), `zod-rs` respects serde attributes:
//!
//! ```rust,ignore
//! use zod_rs::ZodSchema;
//! use serde::{Serialize, Deserialize};
//!
//! #[derive(Serialize, Deserialize, ZodSchema)]
//! #[serde(rename_all = "camelCase")]
//! struct User {
//!     #[serde(rename = "id")]
//!     user_id: u64,
//!
//!     #[serde(skip)]
//!     internal_field: String,
//! }
//! // The serde attributes are respected in schema generation
//! ```
//!
//! ## Manual Implementation
//!
//! You can also implement the `ZodSchema` trait manually:
//!
//! ```rust
//! use zod_rs::ZodSchema;
//!
//! struct CustomType {
//!     value: i32,
//! }
//!
//! impl ZodSchema for CustomType {
//!     fn zod_schema() -> &'static str {
//!         "z.object({ value: z.number().int() })"
//!     }
//!
//!     fn ts_type_name() -> &'static str {
//!         "CustomType"
//!     }
//!
//!     fn schema_name() -> &'static str {
//!         "CustomTypeSchema"
//!     }
//! }
//! ```
//!
//! ## Schema Registry
//!
//! Use the [`SchemaRegistry`] to collect and export multiple schemas:
//!
//! ```rust,ignore
//! use zod_rs::{SchemaRegistry, ZodSchema};
//!
//! let mut registry = SchemaRegistry::new();
//! registry.register::<User>();
//! registry.register::<Post>();
//!
//! // Get all schemas sorted by dependencies
//! let schemas = registry.get_sorted_schemas();
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

#[cfg(feature = "tauri")]
pub mod integrations;

// Re-export main trait
pub use registry::SchemaRegistry;
pub use traits::ZodSchema;
pub use types::{SchemaMetadata, TypeSchema};

// Re-export derive macro when available
#[cfg(feature = "derive")]
pub use zod_rs_macros::ZodSchema;
