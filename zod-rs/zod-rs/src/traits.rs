//! Core traits for Zod schema generation.

#[cfg(not(feature = "std"))]
use alloc::string::String;

use crate::types::SchemaMetadata;

/// Trait for types that can generate Zod schemas.
///
/// This trait is typically derived using `#[derive(ZodSchema)]` from the
/// `zod-rs-macros` crate, but can also be implemented manually for custom types.
///
/// # Example
///
/// ```rust,ignore
/// use zod_rs::ZodSchema;
///
/// struct MyType {
///     value: i32,
/// }
///
/// impl ZodSchema for MyType {
///     fn zod_schema() -> &'static str {
///         "z.object({ value: z.number().int() })"
///     }
///
///     fn ts_type_name() -> &'static str {
///         "MyType"
///     }
///
///     fn schema_name() -> &'static str {
///         "MyTypeSchema"
///     }
/// }
/// ```
pub trait ZodSchema {
    /// Returns the Zod schema string for this type.
    fn zod_schema() -> &'static str;

    /// Returns the TypeScript type name for this type.
    fn ts_type_name() -> &'static str;

    /// Returns the schema variable name (typically `{TypeName}Schema`).
    fn schema_name() -> &'static str;

    /// Returns the full TypeScript declaration including schema and type inference.
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
    fn metadata() -> SchemaMetadata {
        SchemaMetadata::default()
    }
}

// Blanket implementations for primitive types

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

// Compound type implementations

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

// Feature-gated implementations

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
