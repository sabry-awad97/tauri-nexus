//! # zod-rs-macros
//!
//! Procedural macros for generating TypeScript Zod schemas from Rust types.
//!
//! This crate provides the `#[derive(ZodSchema)]` macro that generates
//! Zod schema code for Rust structs and enums.
//!
//! ## Overview
//!
//! The `ZodSchema` derive macro analyzes your Rust type definitions and generates
//! implementations of the `ZodSchema` trait, which provides methods to output
//! TypeScript Zod schema strings.
//!
//! ## Basic Usage
//!
//! ```rust,ignore
//! use zod_rs_macros::ZodSchema;
//!
//! #[derive(ZodSchema)]
//! struct User {
//!     name: String,
//!     age: u32,
//!     email: Option<String>,
//! }
//!
//! // Generated schema:
//! // z.object({
//! //   name: z.string(),
//! //   age: z.number().int().nonnegative(),
//! //   email: z.string().optional()
//! // })
//! ```
//!
//! ## Container Attributes
//!
//! Container attributes are applied to the struct or enum definition.
//!
//! ### `#[zod(rename = "NewName")]`
//!
//! Rename the type in the generated schema:
//!
//! ```rust,ignore
//! #[derive(ZodSchema)]
//! #[zod(rename = "UserDTO")]
//! struct User {
//!     name: String,
//! }
//! // Schema name: UserDTOSchema
//! // Type name: UserDTO
//! ```
//!
//! ### `#[zod(rename_all = "camelCase")]`
//!
//! Rename all fields using a naming convention:
//!
//! ```rust,ignore
//! #[derive(ZodSchema)]
//! #[zod(rename_all = "camelCase")]
//! struct User {
//!     first_name: String,  // becomes "firstName"
//!     last_name: String,   // becomes "lastName"
//! }
//! ```
//!
//! Supported conventions:
//! - `camelCase` - firstName
//! - `snake_case` - first_name
//! - `PascalCase` - FirstName
//! - `SCREAMING_SNAKE_CASE` - FIRST_NAME
//! - `kebab-case` - first-name
//!
//! ### `#[zod(tag = "type")]`
//!
//! Use internal tagging for enums (discriminated union):
//!
//! ```rust,ignore
//! #[derive(ZodSchema)]
//! #[zod(tag = "type")]
//! enum Message {
//!     Text { content: String },
//!     Image { url: String },
//! }
//! // Generates: z.discriminatedUnion("type", [...])
//! ```
//!
//! ### `#[zod(tag = "type", content = "data")]`
//!
//! Use adjacent tagging for enums:
//!
//! ```rust,ignore
//! #[derive(ZodSchema)]
//! #[zod(tag = "type", content = "data")]
//! enum Message {
//!     Text { content: String },
//!     Image { url: String },
//! }
//! // Each variant has { type: "...", data: { ... } }
//! ```
//!
//! ### `#[zod(description = "...")]`
//!
//! Add a description to the schema:
//!
//! ```rust,ignore
//! #[derive(ZodSchema)]
//! #[zod(description = "A user in the system")]
//! struct User {
//!     name: String,
//! }
//! ```
//!
//! ### `#[zod(deprecated)]`
//!
//! Mark the type as deprecated:
//!
//! ```rust,ignore
//! #[derive(ZodSchema)]
//! #[zod(deprecated)]
//! struct OldUser {
//!     name: String,
//! }
//! ```
//!
//! ### `#[zod(strict)]`
//!
//! Use strict mode (no extra properties allowed):
//!
//! ```rust,ignore
//! #[derive(ZodSchema)]
//! #[zod(strict)]
//! struct User {
//!     name: String,
//! }
//! // Generates: z.object({ ... }).strict()
//! ```
//!
//! ## Field Attributes
//!
//! Field attributes are applied to individual struct fields.
//!
//! ### `#[zod(rename = "newName")]`
//!
//! Rename a specific field:
//!
//! ```rust,ignore
//! #[derive(ZodSchema)]
//! struct User {
//!     #[zod(rename = "userName")]
//!     name: String,
//! }
//! ```
//!
//! ### `#[zod(skip)]`
//!
//! Skip a field in the schema:
//!
//! ```rust,ignore
//! #[derive(ZodSchema)]
//! struct User {
//!     name: String,
//!     #[zod(skip)]
//!     internal_id: u64,  // Not included in schema
//! }
//! ```
//!
//! ### `#[zod(optional)]`
//!
//! Mark a field as optional:
//!
//! ```rust,ignore
//! #[derive(ZodSchema)]
//! struct User {
//!     name: String,
//!     #[zod(optional)]
//!     nickname: String,  // Generates: z.string().optional()
//! }
//! ```
//!
//! ### `#[zod(nullable)]`
//!
//! Mark a field as nullable:
//!
//! ```rust,ignore
//! #[derive(ZodSchema)]
//! struct User {
//!     name: String,
//!     #[zod(nullable)]
//!     middle_name: String,  // Generates: z.string().nullable()
//! }
//! ```
//!
//! ### `#[zod(default = "value")]`
//!
//! Set a default value:
//!
//! ```rust,ignore
//! #[derive(ZodSchema)]
//! struct Config {
//!     #[zod(default = "\"default\"")]
//!     name: String,  // Generates: z.string().default("default")
//!     
//!     #[zod(default = "0")]
//!     count: u32,    // Generates: z.number().int().nonnegative().default(0)
//! }
//! ```
//!
//! ### `#[zod(flatten)]`
//!
//! Flatten nested object fields into the parent:
//!
//! ```rust,ignore
//! #[derive(ZodSchema)]
//! struct Address {
//!     street: String,
//!     city: String,
//! }
//!
//! #[derive(ZodSchema)]
//! struct User {
//!     name: String,
//!     #[zod(flatten)]
//!     address: Address,  // Fields merged into User
//! }
//! ```
//!
//! ### `#[zod(type = "...")]`
//!
//! Override with a custom Zod type:
//!
//! ```rust,ignore
//! #[derive(ZodSchema)]
//! struct User {
//!     #[zod(type = "z.string().brand<'UserId'>()")]
//!     id: String,
//! }
//! ```
//!
//! ## Validation Attributes
//!
//! ### String Validations
//!
//! ```rust,ignore
//! #[derive(ZodSchema)]
//! struct User {
//!     #[zod(min_length = 1, max_length = 100)]
//!     name: String,
//!     
//!     #[zod(email)]
//!     email: String,
//!     
//!     #[zod(url)]
//!     website: String,
//!     
//!     #[zod(uuid)]
//!     id: String,
//!     
//!     #[zod(regex = r"^\d{3}-\d{4}$")]
//!     phone: String,
//!     
//!     #[zod(starts_with = "https://")]
//!     secure_url: String,
//!     
//!     #[zod(ends_with = ".com")]
//!     domain: String,
//! }
//! ```
//!
//! Available string validations:
//! - `min_length = N` - Minimum length
//! - `max_length = N` - Maximum length
//! - `length = N` - Exact length
//! - `email` - Email format
//! - `url` - URL format
//! - `uuid` - UUID format
//! - `cuid` - CUID format
//! - `datetime` - ISO datetime format
//! - `ip` - IP address format
//! - `regex = "pattern"` - Regex pattern
//! - `starts_with = "prefix"` - String prefix
//! - `ends_with = "suffix"` - String suffix
//!
//! ### Number Validations
//!
//! ```rust,ignore
//! #[derive(ZodSchema)]
//! struct Product {
//!     #[zod(min = 0.0, max = 1000.0)]
//!     price: f64,
//!     
//!     #[zod(positive)]
//!     quantity: u32,
//!     
//!     #[zod(int)]
//!     count: f64,
//!     
//!     #[zod(finite)]
//!     value: f64,
//! }
//! ```
//!
//! Available number validations:
//! - `min = N` - Minimum value
//! - `max = N` - Maximum value
//! - `positive` - Must be > 0
//! - `negative` - Must be < 0
//! - `nonnegative` - Must be >= 0
//! - `nonpositive` - Must be <= 0
//! - `int` - Must be an integer
//! - `finite` - Must be finite
//!
//! ### Array Validations
//!
//! ```rust,ignore
//! #[derive(ZodSchema)]
//! struct Data {
//!     #[zod(nonempty)]
//!     items: Vec<String>,  // Generates: z.array(z.string()).nonempty()
//! }
//! ```
//!
//! ## Enum Support
//!
//! ### Unit Enums
//!
//! Unit enums generate `z.enum()`:
//!
//! ```rust,ignore
//! #[derive(ZodSchema)]
//! enum Status {
//!     Active,
//!     Inactive,
//!     Pending,
//! }
//! // Generates: z.enum(["Active", "Inactive", "Pending"])
//! ```
//!
//! ### Data Enums
//!
//! Enums with data generate discriminated unions:
//!
//! ```rust,ignore
//! #[derive(ZodSchema)]
//! #[zod(tag = "type")]
//! enum Shape {
//!     Circle { radius: f64 },
//!     Rectangle { width: f64, height: f64 },
//! }
//! // Generates: z.discriminatedUnion("type", [
//! //   z.object({ type: z.literal("Circle"), radius: z.number() }),
//! //   z.object({ type: z.literal("Rectangle"), width: z.number(), height: z.number() })
//! // ])
//! ```
//!
//! ## Serde Compatibility
//!
//! When the `serde-compat` feature is enabled (default), the macro respects
//! serde attributes:
//!
//! ```rust,ignore
//! use serde::{Serialize, Deserialize};
//!
//! #[derive(Serialize, Deserialize, ZodSchema)]
//! #[serde(rename_all = "camelCase")]
//! struct User {
//!     #[serde(rename = "userId")]
//!     id: u64,
//!     
//!     #[serde(skip)]
//!     internal: String,
//!     
//!     #[serde(default)]
//!     active: bool,
//! }
//! ```
//!
//! The `#[zod(...)]` attributes take precedence over `#[serde(...)]` when both
//! are present on the same item.
//!
//! ## Feature Flags
//!
//! | Feature | Description | Default |
//! |---------|-------------|---------|
//! | `serde-compat` | Respect serde attributes | ✅ |
//! | `chrono` | Support for `chrono::DateTime` | ❌ |
//! | `uuid` | Support for `uuid::Uuid` | ❌ |

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
/// This macro generates an implementation of the `ZodSchema` trait for your
/// struct or enum, providing methods to output TypeScript Zod schema strings.
///
/// # Basic Example
///
/// ```rust,ignore
/// use zod_rs_macros::ZodSchema;
///
/// #[derive(ZodSchema)]
/// struct User {
///     name: String,
///     age: u32,
/// }
///
/// // Use the generated methods:
/// let schema = User::zod_schema();
/// // => "z.object({ name: z.string(), age: z.number().int().nonnegative() })"
///
/// let declaration = User::ts_declaration();
/// // => "export const UserSchema = z.object({ ... });\nexport type User = z.infer<typeof UserSchema>;"
/// ```
///
/// # With Validation
///
/// ```rust,ignore
/// #[derive(ZodSchema)]
/// #[zod(rename_all = "camelCase")]
/// struct CreateUser {
///     #[zod(min_length = 1, max_length = 100)]
///     user_name: String,
///
///     #[zod(min = 0, max = 150)]
///     age: u32,
///
///     #[zod(email)]
///     email_address: String,
/// }
/// ```
///
/// # Enums
///
/// ```rust,ignore
/// // Unit enum
/// #[derive(ZodSchema)]
/// enum Status {
///     Active,
///     Inactive,
/// }
/// // => z.enum(["Active", "Inactive"])
///
/// // Tagged enum (discriminated union)
/// #[derive(ZodSchema)]
/// #[zod(tag = "type")]
/// enum Message {
///     Text { content: String },
///     Image { url: String },
/// }
/// // => z.discriminatedUnion("type", [...])
/// ```
///
/// # Supported Attributes
///
/// See the [module-level documentation](crate) for a complete list of
/// supported attributes.
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
