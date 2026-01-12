//! # zod-rs-macros
//!
//! Procedural macros for generating TypeScript Zod schemas from Rust types.
//!
//! This crate provides the `#[derive(ZodSchema)]` macro that generates
//! Zod schema code for Rust structs and enums.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use zod_rs_macros::ZodSchema;
//!
//! #[derive(ZodSchema)]
//! #[zod(rename_all = "camelCase")]
//! struct User {
//!     #[zod(min_length = 1, max_length = 100)]
//!     name: String,
//!     
//!     #[zod(min = 0, max = 150)]
//!     age: u32,
//!     
//!     #[zod(email)]
//!     email: Option<String>,
//! }
//! ```
//!
//! ## Attributes
//!
//! ### Container Attributes (on struct/enum)
//!
//! - `#[zod(rename = "NewName")]` - Rename the type in generated schema
//! - `#[zod(rename_all = "camelCase")]` - Rename all fields (camelCase, snake_case, PascalCase, etc.)
//! - `#[zod(tag = "type")]` - Use internal tagging for enums
//! - `#[zod(tag = "type", content = "data")]` - Use adjacent tagging for enums
//! - `#[zod(description = "...")]` - Add description to schema
//! - `#[zod(deprecated)]` - Mark as deprecated
//! - `#[zod(strict)]` - Use strict mode (no extra properties)
//!
//! ### Field Attributes
//!
//! - `#[zod(rename = "newName")]` - Rename this field
//! - `#[zod(skip)]` - Skip this field
//! - `#[zod(optional)]` - Mark as optional
//! - `#[zod(nullable)]` - Mark as nullable
//! - `#[zod(default = "value")]` - Set default value
//! - `#[zod(flatten)]` - Flatten nested object
//!
//! ### Validation Attributes
//!
//! - `#[zod(min = N)]` - Minimum value for numbers
//! - `#[zod(max = N)]` - Maximum value for numbers
//! - `#[zod(min_length = N)]` - Minimum length for strings
//! - `#[zod(max_length = N)]` - Maximum length for strings
//! - `#[zod(email)]` - Email format validation
//! - `#[zod(url)]` - URL format validation
//! - `#[zod(uuid)]` - UUID format validation
//! - `#[zod(regex = "pattern")]` - Regex pattern validation

use proc_macro::TokenStream;

mod codegen;
mod error;
mod generator;
mod ir;
mod parser;

/// Derive macro for generating Zod schemas from Rust types.
///
/// # Example
///
/// ```rust,ignore
/// use zod_rs_macros::ZodSchema;
///
/// #[derive(ZodSchema)]
/// struct User {
///     name: String,
///     age: u32,
/// }
/// ```
#[proc_macro_derive(ZodSchema, attributes(zod, serde))]
pub fn derive_zod_schema(input: TokenStream) -> TokenStream {
    // Placeholder implementation - will be completed in later tasks
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    let name = &input.ident;
    let schema_name = format!("{}Schema", name);

    let expanded = quote::quote! {
        impl zod_rs::ZodSchema for #name {
            fn zod_schema() -> &'static str {
                "z.object({})"
            }

            fn ts_type_name() -> &'static str {
                stringify!(#name)
            }

            fn schema_name() -> &'static str {
                #schema_name
            }
        }
    };

    TokenStream::from(expanded)
}
