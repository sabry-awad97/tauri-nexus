//! Core traits for Zod schema generation.
//!
//! This module defines the [`ZodSchema`] trait, which is the primary interface
//! for generating TypeScript Zod schemas from Rust types.
//!
//! ## Overview
//!
//! The `ZodSchema` trait provides methods to:
//! - Generate Zod schema strings ([`ZodSchema::zod_schema`])
//! - Get TypeScript type names ([`ZodSchema::ts_type_name`])
//! - Get schema variable names ([`ZodSchema::schema_name`])
//! - Generate full TypeScript declarations ([`ZodSchema::ts_declaration`])
//! - Access schema metadata ([`ZodSchema::metadata`])
//!
//! ## Deriving vs Manual Implementation
//!
//! Most users will derive this trait using `#[derive(ZodSchema)]`:
//!
//! ```rust,ignore
//! use zod_rs::ZodSchema;
//!
//! #[derive(ZodSchema)]
//! struct User {
//!     name: String,
//!     age: u32,
//! }
//! ```
//!
//! However, you can also implement it manually for custom types:
//!
//! ```rust
//! use zod_rs::ZodSchema;
//!
//! struct MyCustomType;
//!
//! impl ZodSchema for MyCustomType {
//!     fn zod_schema() -> &'static str {
//!         "z.custom<MyCustomType>()"
//!     }
//!
//!     fn ts_type_name() -> &'static str {
//!         "MyCustomType"
//!     }
//!
//!     fn schema_name() -> &'static str {
//!         "MyCustomTypeSchema"
//!     }
//! }
//! ```
//!
//! ## Blanket Implementations
//!
//! This module provides blanket implementations for common Rust types:
//!
//! - **Primitives**: `String`, `bool`, `char`, integers (`i8`-`i128`, `u8`-`u128`), floats (`f32`, `f64`)
//! - **Collections**: `Option<T>`, `Vec<T>`, `HashMap<K, V>`, `HashSet<T>`, `BTreeMap<K, V>`, `BTreeSet<T>`
//! - **Feature-gated**: `Uuid` (uuid feature), `DateTime<Tz>` (chrono feature)

#[cfg(not(feature = "std"))]
use alloc::string::String;

use crate::types::SchemaMetadata;

/// Trait for types that can generate Zod schemas.
///
/// This trait is the core interface for generating TypeScript Zod schemas from Rust types.
/// It is typically derived using `#[derive(ZodSchema)]` from the `zod-rs-macros` crate,
/// but can also be implemented manually for custom types.
///
/// # Required Methods
///
/// - [`zod_schema`](ZodSchema::zod_schema) - Returns the Zod schema string
/// - [`ts_type_name`](ZodSchema::ts_type_name) - Returns the TypeScript type name
/// - [`schema_name`](ZodSchema::schema_name) - Returns the schema variable name
///
/// # Provided Methods
///
/// - [`ts_declaration`](ZodSchema::ts_declaration) - Returns the full TypeScript declaration
/// - [`metadata`](ZodSchema::metadata) - Returns schema metadata (description, deprecated, etc.)
///
/// # Example
///
/// ## Using the Derive Macro
///
/// ```rust,ignore
/// use zod_rs::ZodSchema;
///
/// #[derive(ZodSchema)]
/// struct User {
///     name: String,
///     age: u32,
/// }
///
/// // Get the Zod schema
/// assert_eq!(
///     User::zod_schema(),
///     "z.object({ name: z.string(), age: z.number().int().nonnegative() })"
/// );
///
/// // Get the TypeScript type name
/// assert_eq!(User::ts_type_name(), "User");
///
/// // Get the schema variable name
/// assert_eq!(User::schema_name(), "UserSchema");
/// ```
///
/// ## Manual Implementation
///
/// ```rust
/// use zod_rs::{ZodSchema, SchemaMetadata};
///
/// struct Point {
///     x: f64,
///     y: f64,
/// }
///
/// impl ZodSchema for Point {
///     fn zod_schema() -> &'static str {
///         "z.object({ x: z.number(), y: z.number() })"
///     }
///
///     fn ts_type_name() -> &'static str {
///         "Point"
///     }
///
///     fn schema_name() -> &'static str {
///         "PointSchema"
///     }
///
///     fn metadata() -> SchemaMetadata {
///         SchemaMetadata {
///             description: Some("A 2D point with x and y coordinates".to_string()),
///             deprecated: false,
///             deprecation_message: None,
///             examples: vec!["{ x: 0, y: 0 }".to_string()],
///             tags: vec!["geometry".to_string()],
///         }
///     }
/// }
/// ```
pub trait ZodSchema {
    /// Returns the Zod schema string for this type.
    ///
    /// This method returns a static string containing the Zod schema definition.
    /// The schema should be valid TypeScript code that can be used with the Zod library.
    ///
    /// # Example
    ///
    /// ```rust
    /// use zod_rs::ZodSchema;
    ///
    /// // Primitive types have built-in implementations
    /// assert_eq!(String::zod_schema(), "z.string()");
    /// assert_eq!(bool::zod_schema(), "z.boolean()");
    /// assert_eq!(i32::zod_schema(), "z.number().int()");
    /// ```
    fn zod_schema() -> &'static str;

    /// Returns the TypeScript type name for this type.
    ///
    /// This is the name used in TypeScript type declarations, typically matching
    /// the Rust type name or a renamed version specified via attributes.
    ///
    /// # Example
    ///
    /// ```rust
    /// use zod_rs::ZodSchema;
    ///
    /// assert_eq!(String::ts_type_name(), "string");
    /// assert_eq!(bool::ts_type_name(), "boolean");
    /// ```
    fn ts_type_name() -> &'static str;

    /// Returns the schema variable name (typically `{TypeName}Schema`).
    ///
    /// This is the name of the exported const that holds the Zod schema.
    ///
    /// # Example
    ///
    /// ```rust
    /// use zod_rs::ZodSchema;
    ///
    /// assert_eq!(String::schema_name(), "StringSchema");
    /// ```
    fn schema_name() -> &'static str;

    /// Returns the full TypeScript declaration including schema and type inference.
    ///
    /// This method generates a complete TypeScript declaration that includes:
    /// - The schema export: `export const {Name}Schema = {schema};`
    /// - The type inference: `export type {Name} = z.infer<typeof {Name}Schema>;`
    ///
    /// # Example
    ///
    /// ```rust
    /// use zod_rs::ZodSchema;
    ///
    /// let decl = String::ts_declaration();
    /// assert!(decl.contains("export const StringSchema = z.string();"));
    /// assert!(decl.contains("export type string = z.infer<typeof StringSchema>;"));
    /// ```
    fn ts_declaration() -> String {
        format!(
            "export const {} = {};\nexport type {} = z.infer<typeof {}>;",
            Self::schema_name(),
            Self::zod_schema(),
            Self::ts_type_name(),
            Self::schema_name()
        )
    }

    /// Returns metadata about this schema (description, deprecated, etc.).
    ///
    /// Override this method to provide additional metadata for your schema,
    /// such as descriptions, deprecation notices, examples, and tags.
    ///
    /// # Example
    ///
    /// ```rust
    /// use zod_rs::{ZodSchema, SchemaMetadata};
    ///
    /// struct MyType;
    ///
    /// impl ZodSchema for MyType {
    ///     fn zod_schema() -> &'static str { "z.object({})" }
    ///     fn ts_type_name() -> &'static str { "MyType" }
    ///     fn schema_name() -> &'static str { "MyTypeSchema" }
    ///
    ///     fn metadata() -> SchemaMetadata {
    ///         SchemaMetadata {
    ///             description: Some("My custom type".to_string()),
    ///             deprecated: true,
    ///             deprecation_message: Some("Use NewType instead".to_string()),
    ///             ..Default::default()
    ///         }
    ///     }
    /// }
    /// ```
    fn metadata() -> SchemaMetadata {
        SchemaMetadata::default()
    }
}

// =============================================================================
// Blanket implementations for primitive types
// =============================================================================

impl ZodSchema for String {
    fn zod_schema() -> &'static str {
        "z.string()"
    }

    fn ts_type_name() -> &'static str {
        "string"
    }

    fn schema_name() -> &'static str {
        "StringSchema"
    }
}

impl ZodSchema for bool {
    fn zod_schema() -> &'static str {
        "z.boolean()"
    }

    fn ts_type_name() -> &'static str {
        "boolean"
    }

    fn schema_name() -> &'static str {
        "BooleanSchema"
    }
}

impl ZodSchema for char {
    fn zod_schema() -> &'static str {
        "z.string().length(1)"
    }

    fn ts_type_name() -> &'static str {
        "string"
    }

    fn schema_name() -> &'static str {
        "CharSchema"
    }
}

// =============================================================================
// Signed integer implementations
// =============================================================================

macro_rules! impl_zod_schema_for_int {
    ($($ty:ty => $name:literal),* $(,)?) => {
        $(
            impl ZodSchema for $ty {
                fn zod_schema() -> &'static str {
                    "z.number().int()"
                }

                fn ts_type_name() -> &'static str {
                    "number"
                }

                fn schema_name() -> &'static str {
                    concat!($name, "Schema")
                }
            }
        )*
    };
}

impl_zod_schema_for_int!(
    i8 => "I8",
    i16 => "I16",
    i32 => "I32",
    i64 => "I64",
    i128 => "I128",
    isize => "Isize",
);

// =============================================================================
// Unsigned integer implementations
// =============================================================================

macro_rules! impl_zod_schema_for_uint {
    ($($ty:ty => $name:literal),* $(,)?) => {
        $(
            impl ZodSchema for $ty {
                fn zod_schema() -> &'static str {
                    "z.number().int().nonnegative()"
                }

                fn ts_type_name() -> &'static str {
                    "number"
                }

                fn schema_name() -> &'static str {
                    concat!($name, "Schema")
                }
            }
        )*
    };
}

impl_zod_schema_for_uint!(
    u8 => "U8",
    u16 => "U16",
    u32 => "U32",
    u64 => "U64",
    u128 => "U128",
    usize => "Usize",
);

// =============================================================================
// Float implementations
// =============================================================================

macro_rules! impl_zod_schema_for_float {
    ($($ty:ty => $name:literal),* $(,)?) => {
        $(
            impl ZodSchema for $ty {
                fn zod_schema() -> &'static str {
                    "z.number()"
                }

                fn ts_type_name() -> &'static str {
                    "number"
                }

                fn schema_name() -> &'static str {
                    concat!($name, "Schema")
                }
            }
        )*
    };
}

impl_zod_schema_for_float!(
    f32 => "F32",
    f64 => "F64",
);

// =============================================================================
// Compound type implementations
// =============================================================================

impl<T: ZodSchema> ZodSchema for Option<T> {
    fn zod_schema() -> &'static str {
        // Note: This returns a static str, so we can't dynamically compose
        // The actual implementation will be in the derive macro
        "z.unknown().optional()"
    }

    fn ts_type_name() -> &'static str {
        "unknown"
    }

    fn schema_name() -> &'static str {
        "OptionSchema"
    }
}

impl<T: ZodSchema> ZodSchema for Vec<T> {
    fn zod_schema() -> &'static str {
        "z.array(z.unknown())"
    }

    fn ts_type_name() -> &'static str {
        "unknown[]"
    }

    fn schema_name() -> &'static str {
        "VecSchema"
    }
}

#[cfg(feature = "std")]
impl<K: ZodSchema, V: ZodSchema> ZodSchema for std::collections::HashMap<K, V> {
    fn zod_schema() -> &'static str {
        "z.record(z.unknown(), z.unknown())"
    }

    fn ts_type_name() -> &'static str {
        "Record<unknown, unknown>"
    }

    fn schema_name() -> &'static str {
        "HashMapSchema"
    }
}

#[cfg(feature = "std")]
impl<T: ZodSchema> ZodSchema for std::collections::HashSet<T> {
    fn zod_schema() -> &'static str {
        "z.set(z.unknown())"
    }

    fn ts_type_name() -> &'static str {
        "Set<unknown>"
    }

    fn schema_name() -> &'static str {
        "HashSetSchema"
    }
}

impl<K: ZodSchema + Ord, V: ZodSchema> ZodSchema for std::collections::BTreeMap<K, V> {
    fn zod_schema() -> &'static str {
        "z.record(z.unknown(), z.unknown())"
    }

    fn ts_type_name() -> &'static str {
        "Record<unknown, unknown>"
    }

    fn schema_name() -> &'static str {
        "BTreeMapSchema"
    }
}

impl<T: ZodSchema + Ord> ZodSchema for std::collections::BTreeSet<T> {
    fn zod_schema() -> &'static str {
        "z.set(z.unknown())"
    }

    fn ts_type_name() -> &'static str {
        "Set<unknown>"
    }

    fn schema_name() -> &'static str {
        "BTreeSetSchema"
    }
}

// =============================================================================
// Feature-gated implementations
// =============================================================================

#[cfg(feature = "uuid")]
impl ZodSchema for uuid::Uuid {
    fn zod_schema() -> &'static str {
        "z.string().uuid()"
    }

    fn ts_type_name() -> &'static str {
        "string"
    }

    fn schema_name() -> &'static str {
        "UuidSchema"
    }
}

#[cfg(feature = "chrono")]
impl<Tz: chrono::TimeZone> ZodSchema for chrono::DateTime<Tz> {
    fn zod_schema() -> &'static str {
        "z.string().datetime()"
    }

    fn ts_type_name() -> &'static str {
        "string"
    }

    fn schema_name() -> &'static str {
        "DateTimeSchema"
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_zod_schema() {
        assert_eq!(String::zod_schema(), "z.string()");
        assert_eq!(String::ts_type_name(), "string");
        assert_eq!(String::schema_name(), "StringSchema");
    }

    #[test]
    fn test_bool_zod_schema() {
        assert_eq!(bool::zod_schema(), "z.boolean()");
        assert_eq!(bool::ts_type_name(), "boolean");
        assert_eq!(bool::schema_name(), "BooleanSchema");
    }

    #[test]
    fn test_i32_zod_schema() {
        assert_eq!(i32::zod_schema(), "z.number().int()");
        assert_eq!(i32::ts_type_name(), "number");
        assert_eq!(i32::schema_name(), "I32Schema");
    }

    #[test]
    fn test_u32_zod_schema() {
        assert_eq!(u32::zod_schema(), "z.number().int().nonnegative()");
        assert_eq!(u32::ts_type_name(), "number");
        assert_eq!(u32::schema_name(), "U32Schema");
    }

    #[test]
    fn test_f64_zod_schema() {
        assert_eq!(f64::zod_schema(), "z.number()");
        assert_eq!(f64::ts_type_name(), "number");
        assert_eq!(f64::schema_name(), "F64Schema");
    }

    #[test]
    fn test_char_zod_schema() {
        assert_eq!(char::zod_schema(), "z.string().length(1)");
        assert_eq!(char::ts_type_name(), "string");
        assert_eq!(char::schema_name(), "CharSchema");
    }

    #[test]
    fn test_option_zod_schema() {
        assert_eq!(Option::<String>::zod_schema(), "z.unknown().optional()");
        assert_eq!(Option::<String>::ts_type_name(), "unknown");
        assert_eq!(Option::<String>::schema_name(), "OptionSchema");
    }

    #[test]
    fn test_vec_zod_schema() {
        assert_eq!(Vec::<String>::zod_schema(), "z.array(z.unknown())");
        assert_eq!(Vec::<String>::ts_type_name(), "unknown[]");
        assert_eq!(Vec::<String>::schema_name(), "VecSchema");
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_hashmap_zod_schema() {
        use std::collections::HashMap;
        assert_eq!(
            HashMap::<String, i32>::zod_schema(),
            "z.record(z.unknown(), z.unknown())"
        );
        assert_eq!(
            HashMap::<String, i32>::ts_type_name(),
            "Record<unknown, unknown>"
        );
        assert_eq!(HashMap::<String, i32>::schema_name(), "HashMapSchema");
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_hashset_zod_schema() {
        use std::collections::HashSet;
        assert_eq!(HashSet::<String>::zod_schema(), "z.set(z.unknown())");
        assert_eq!(HashSet::<String>::ts_type_name(), "Set<unknown>");
        assert_eq!(HashSet::<String>::schema_name(), "HashSetSchema");
    }

    #[test]
    fn test_ts_declaration() {
        let decl = String::ts_declaration();
        assert!(decl.contains("export const StringSchema = z.string();"));
        assert!(decl.contains("export type string = z.infer<typeof StringSchema>;"));
    }

    #[test]
    fn test_metadata_default() {
        let meta = String::metadata();
        assert!(meta.description.is_none());
        assert!(!meta.deprecated);
    }
}
