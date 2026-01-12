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
use syn::{Data, DeriveInput};

mod codegen;
mod error;
mod generator;
mod ir;
mod parser;

use codegen::impl_block::ImplBlockGenerator;
use parser::enum_parser::EnumParser;
use parser::struct_parser::StructParser;
use parser::type_parser::ParseError as TypeParseError;

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
    let input = syn::parse_macro_input!(input as DeriveInput);

    match derive_zod_schema_impl(&input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.into_compile_error().into(),
    }
}

/// Internal implementation of the derive macro.
///
/// This function routes to the appropriate parser based on the input type
/// (struct or enum) and generates the impl block.
fn derive_zod_schema_impl(input: &DeriveInput) -> Result<proc_macro2::TokenStream, syn::Error> {
    // Route to the appropriate parser based on the data type
    let schema_ir = match &input.data {
        Data::Struct(_) => StructParser::parse(input).map_err(|e| convert_parse_error(e, input))?,
        Data::Enum(_) => {
            EnumParser::parse(input).map_err(|e| convert_enum_parse_error(e, input))?
        }
        Data::Union(_) => {
            return Err(syn::Error::new_spanned(
                &input.ident,
                "ZodSchema cannot be derived for unions",
            ));
        }
    };

    // Generate the impl block
    let generator = ImplBlockGenerator::new();
    let impl_block = generator.generate(&schema_ir);

    Ok(impl_block)
}

/// Convert a StructParseError to a syn::Error with proper span information.
fn convert_parse_error(
    error: parser::struct_parser::StructParseError,
    input: &DeriveInput,
) -> syn::Error {
    use parser::struct_parser::StructParseError;

    match error {
        StructParseError::ContainerAttrs(msg) => syn::Error::new_spanned(
            &input.ident,
            format!("Invalid container attributes: {}", msg),
        ),
        StructParseError::FieldAttrs(msg) => {
            syn::Error::new_spanned(&input.ident, format!("Invalid field attributes: {}", msg))
        }
        StructParseError::FieldType(parse_error) => convert_type_parse_error(parse_error, input),
        StructParseError::NotAStruct(kind) => {
            syn::Error::new_spanned(&input.ident, format!("Expected struct, found {}", kind))
        }
        StructParseError::MissingFieldIdent(index) => syn::Error::new_spanned(
            &input.ident,
            format!("Field at index {} has no identifier", index),
        ),
    }
}

/// Convert an EnumParseError to a syn::Error with proper span information.
fn convert_enum_parse_error(
    error: parser::enum_parser::EnumParseError,
    input: &DeriveInput,
) -> syn::Error {
    use parser::enum_parser::EnumParseError;

    match error {
        EnumParseError::ContainerAttrs(msg) => syn::Error::new_spanned(
            &input.ident,
            format!("Invalid container attributes: {}", msg),
        ),
        EnumParseError::VariantAttrs(msg) => {
            syn::Error::new_spanned(&input.ident, format!("Invalid variant attributes: {}", msg))
        }
        EnumParseError::FieldType(parse_error) => convert_type_parse_error(parse_error, input),
        EnumParseError::NotAnEnum(kind) => {
            syn::Error::new_spanned(&input.ident, format!("Expected enum, found {}", kind))
        }
        EnumParseError::MissingVariantIdent(index) => syn::Error::new_spanned(
            &input.ident,
            format!("Variant at index {} has no identifier", index),
        ),
        EnumParseError::MissingFieldIdent(variant, index) => syn::Error::new_spanned(
            &input.ident,
            format!(
                "Field at index {} in variant '{}' has no identifier",
                index, variant
            ),
        ),
    }
}

/// Convert a TypeParseError to a syn::Error with proper span information.
fn convert_type_parse_error(error: TypeParseError, input: &DeriveInput) -> syn::Error {
    let message = match &error {
        TypeParseError::UnsupportedType(ty) => format!("Unsupported type: {}", ty),
        TypeParseError::EmptyPath => "Empty path in type".to_string(),
        TypeParseError::MissingGeneric(ty) => format!("Missing generic parameter for {}", ty),
        TypeParseError::InvalidArrayLength => "Invalid array length".to_string(),
    };
    syn::Error::new_spanned(&input.ident, message)
}
