//! Rust to Zod type mappings.
//!
//! This module handles mapping Rust types to their Zod equivalents.
//! It provides the [`ZodTypeMapper`] struct which transforms [`TypeIR`]
//! into Zod schema strings.
//!
//! # Type Mappings
//!
//! | Rust Type | Zod Schema |
//! |-----------|------------|
//! | `String`, `&str` | `z.string()` |
//! | `bool` | `z.boolean()` |
//! | `char` | `z.string().length(1)` |
//! | `i8`-`i128`, `isize` | `z.number().int()` |
//! | `u8`-`u128`, `usize` | `z.number().int().nonnegative()` |
//! | `f32`, `f64` | `z.number()` |
//! | `Option<T>` | `T.optional()` |
//! | `Vec<T>` | `z.array(T)` |
//! | `HashMap<K, V>` | `z.record(K, V)` |
//! | `Uuid` | `z.string().uuid()` |
//! | `DateTime` | `z.string().datetime()` |
//! | Custom types | `{Name}Schema` |

use std::collections::HashMap;

use crate::ir::{TypeIR, TypeKind};

/// Maps Rust types to Zod schema strings.
///
/// The type mapper is responsible for converting the intermediate
/// representation of types into valid Zod schema code.
///
/// # Example
///
/// ```rust,ignore
/// use zod_rs_macros::generator::zod::ZodTypeMapper;
/// use zod_rs_macros::ir::{TypeIR, TypeKind};
///
/// let mapper = ZodTypeMapper::new();
/// let ty = TypeIR::new(TypeKind::String);
/// assert_eq!(mapper.map_type(&ty), "z.string()");
/// ```
#[derive(Debug, Clone)]
pub struct ZodTypeMapper {
    /// Custom type overrides (Rust type name -> Zod schema)
    type_overrides: HashMap<String, String>,
}

impl Default for ZodTypeMapper {
    fn default() -> Self {
        Self::new()
    }
}

impl ZodTypeMapper {
    /// Create a new ZodTypeMapper with default settings.
    pub fn new() -> Self {
        Self {
            type_overrides: HashMap::new(),
        }
    }

    /// Create a ZodTypeMapper with custom type overrides.
    pub fn with_overrides(overrides: HashMap<String, String>) -> Self {
        Self {
            type_overrides: overrides,
        }
    }

    /// Add a custom type override.
    pub fn add_override(&mut self, rust_type: impl Into<String>, zod_schema: impl Into<String>) {
        self.type_overrides
            .insert(rust_type.into(), zod_schema.into());
    }

    /// Map a TypeIR to its Zod schema string representation.
    ///
    /// This is the main entry point for type mapping. It handles
    /// all type kinds and applies nullable modifier if needed.
    pub fn map_type(&self, ty: &TypeIR) -> String {
        // Check for custom override first
        if let Some(original) = &ty.original_type {
            if let Some(override_schema) = self.type_overrides.get(original) {
                return self.apply_nullable(override_schema.clone(), ty.nullable);
            }
        }

        let schema = self.map_type_kind(&ty.kind);
        self.apply_nullable(schema, ty.nullable)
    }

    /// Map a TypeKind to its Zod schema string.
    fn map_type_kind(&self, kind: &TypeKind) -> String {
        match kind {
            // Primitives
            TypeKind::String => "z.string()".to_string(),
            TypeKind::Boolean => "z.boolean()".to_string(),
            TypeKind::Char => "z.string().length(1)".to_string(),
            TypeKind::Integer { signed, bits } => self.map_integer(*signed, *bits),
            TypeKind::Float => "z.number()".to_string(),

            // Compound types
            TypeKind::Array(inner) => self.map_array(inner),
            TypeKind::Tuple(elements) => self.map_tuple(elements),
            TypeKind::Record { key, value } => self.map_record(key, value),
            TypeKind::Set(inner) => self.map_set(inner),
            TypeKind::Optional(inner) => self.map_optional(inner),

            // Special types
            TypeKind::Uuid => "z.string().uuid()".to_string(),
            TypeKind::DateTime => "z.string().datetime()".to_string(),
            TypeKind::Duration => "z.number()".to_string(),
            TypeKind::Decimal => "z.number()".to_string(),
            TypeKind::Any => "z.any()".to_string(),
            TypeKind::Unknown => "z.unknown()".to_string(),
            TypeKind::Never => "z.never()".to_string(),
            TypeKind::Null => "z.null()".to_string(),
            TypeKind::Void => "z.void()".to_string(),

            // Reference types
            TypeKind::Reference { name, generics } => self.map_reference(name, generics),

            // Literal types
            TypeKind::LiteralString(s) => format!("z.literal(\"{}\")", escape_string(s)),
            TypeKind::LiteralNumber(n) => format!("z.literal({})", n),
            TypeKind::LiteralBoolean(b) => format!("z.literal({})", b),

            // Composite types
            TypeKind::Union(types) => self.map_union(types),
            TypeKind::Intersection(types) => self.map_intersection(types),
        }
    }

    /// Map an integer type to Zod schema.
    ///
    /// Signed integers map to `z.number().int()`.
    /// Unsigned integers map to `z.number().int().nonnegative()`.
    fn map_integer(&self, signed: bool, _bits: Option<u8>) -> String {
        if signed {
            "z.number().int()".to_string()
        } else {
            "z.number().int().nonnegative()".to_string()
        }
    }

    /// Map an array type to Zod schema.
    fn map_array(&self, inner: &TypeIR) -> String {
        let inner_schema = self.map_type(inner);
        format!("z.array({})", inner_schema)
    }

    /// Map a tuple type to Zod schema.
    fn map_tuple(&self, elements: &[TypeIR]) -> String {
        if elements.is_empty() {
            return "z.tuple([])".to_string();
        }

        let element_schemas: Vec<String> = elements.iter().map(|e| self.map_type(e)).collect();

        format!("z.tuple([{}])", element_schemas.join(", "))
    }

    /// Map a record/map type to Zod schema.
    fn map_record(&self, key: &TypeIR, value: &TypeIR) -> String {
        let key_schema = self.map_type(key);
        let value_schema = self.map_type(value);
        format!("z.record({}, {})", key_schema, value_schema)
    }

    /// Map a set type to Zod schema (represented as array).
    fn map_set(&self, inner: &TypeIR) -> String {
        let inner_schema = self.map_type(inner);
        format!("z.array({})", inner_schema)
    }

    /// Map an optional type to Zod schema.
    fn map_optional(&self, inner: &TypeIR) -> String {
        let inner_schema = self.map_type(inner);
        format!("{}.optional()", inner_schema)
    }

    /// Map a reference type to Zod schema.
    ///
    /// For types with generics, uses `z.lazy()` to handle potential
    /// circular references.
    fn map_reference(&self, name: &str, generics: &[TypeIR]) -> String {
        // Check for custom override
        if let Some(override_schema) = self.type_overrides.get(name) {
            return override_schema.clone();
        }

        let schema_name = format!("{}Schema", name);

        if generics.is_empty() {
            schema_name
        } else {
            // For generic types, use z.lazy() to handle circular references
            format!("z.lazy(() => {})", schema_name)
        }
    }

    /// Map a union type to Zod schema.
    fn map_union(&self, types: &[TypeIR]) -> String {
        if types.is_empty() {
            return "z.never()".to_string();
        }

        if types.len() == 1 {
            return self.map_type(&types[0]);
        }

        let type_schemas: Vec<String> = types.iter().map(|t| self.map_type(t)).collect();

        format!("z.union([{}])", type_schemas.join(", "))
    }

    /// Map an intersection type to Zod schema.
    fn map_intersection(&self, types: &[TypeIR]) -> String {
        if types.is_empty() {
            return "z.unknown()".to_string();
        }

        if types.len() == 1 {
            return self.map_type(&types[0]);
        }

        let type_schemas: Vec<String> = types.iter().map(|t| self.map_type(t)).collect();

        // Chain .and() calls for intersection
        let mut result = type_schemas[0].clone();
        for schema in &type_schemas[1..] {
            result = format!("{}.and({})", result, schema);
        }
        result
    }

    /// Apply nullable modifier if needed.
    fn apply_nullable(&self, schema: String, nullable: bool) -> String {
        if nullable {
            format!("{}.nullable()", schema)
        } else {
            schema
        }
    }

    /// Map a type with a custom override.
    ///
    /// This is used when `#[zod(type = "...")]` is specified.
    pub fn map_with_override(&self, ty: &TypeIR, override_schema: &str) -> String {
        self.apply_nullable(override_schema.to_string(), ty.nullable)
    }
}

/// Escape a string for use in JavaScript/TypeScript.
fn escape_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mapper() -> ZodTypeMapper {
        ZodTypeMapper::new()
    }

    // =========================================================================
    // Primitive Type Tests (Task 9.1)
    // =========================================================================

    #[test]
    fn test_map_string() {
        let ty = TypeIR::new(TypeKind::String);
        assert_eq!(mapper().map_type(&ty), "z.string()");
    }

    #[test]
    fn test_map_boolean() {
        let ty = TypeIR::new(TypeKind::Boolean);
        assert_eq!(mapper().map_type(&ty), "z.boolean()");
    }

    #[test]
    fn test_map_char() {
        let ty = TypeIR::new(TypeKind::Char);
        assert_eq!(mapper().map_type(&ty), "z.string().length(1)");
    }

    #[test]
    fn test_map_signed_integers() {
        // i8
        let ty = TypeIR::new(TypeKind::Integer {
            signed: true,
            bits: Some(8),
        });
        assert_eq!(mapper().map_type(&ty), "z.number().int()");

        // i16
        let ty = TypeIR::new(TypeKind::Integer {
            signed: true,
            bits: Some(16),
        });
        assert_eq!(mapper().map_type(&ty), "z.number().int()");

        // i32
        let ty = TypeIR::new(TypeKind::Integer {
            signed: true,
            bits: Some(32),
        });
        assert_eq!(mapper().map_type(&ty), "z.number().int()");

        // i64
        let ty = TypeIR::new(TypeKind::Integer {
            signed: true,
            bits: Some(64),
        });
        assert_eq!(mapper().map_type(&ty), "z.number().int()");

        // i128
        let ty = TypeIR::new(TypeKind::Integer {
            signed: true,
            bits: Some(128),
        });
        assert_eq!(mapper().map_type(&ty), "z.number().int()");

        // isize (no bits)
        let ty = TypeIR::new(TypeKind::Integer {
            signed: true,
            bits: None,
        });
        assert_eq!(mapper().map_type(&ty), "z.number().int()");
    }

    #[test]
    fn test_map_unsigned_integers() {
        // u8
        let ty = TypeIR::new(TypeKind::Integer {
            signed: false,
            bits: Some(8),
        });
        assert_eq!(mapper().map_type(&ty), "z.number().int().nonnegative()");

        // u16
        let ty = TypeIR::new(TypeKind::Integer {
            signed: false,
            bits: Some(16),
        });
        assert_eq!(mapper().map_type(&ty), "z.number().int().nonnegative()");

        // u32
        let ty = TypeIR::new(TypeKind::Integer {
            signed: false,
            bits: Some(32),
        });
        assert_eq!(mapper().map_type(&ty), "z.number().int().nonnegative()");

        // u64
        let ty = TypeIR::new(TypeKind::Integer {
            signed: false,
            bits: Some(64),
        });
        assert_eq!(mapper().map_type(&ty), "z.number().int().nonnegative()");

        // u128
        let ty = TypeIR::new(TypeKind::Integer {
            signed: false,
            bits: Some(128),
        });
        assert_eq!(mapper().map_type(&ty), "z.number().int().nonnegative()");

        // usize (no bits)
        let ty = TypeIR::new(TypeKind::Integer {
            signed: false,
            bits: None,
        });
        assert_eq!(mapper().map_type(&ty), "z.number().int().nonnegative()");
    }

    #[test]
    fn test_map_float() {
        let ty = TypeIR::new(TypeKind::Float);
        assert_eq!(mapper().map_type(&ty), "z.number()");
    }

    // =========================================================================
    // Compound Type Tests (Task 9.2)
    // =========================================================================

    #[test]
    fn test_map_array() {
        let inner = TypeIR::new(TypeKind::String);
        let ty = TypeIR::new(TypeKind::Array(Box::new(inner)));
        assert_eq!(mapper().map_type(&ty), "z.array(z.string())");
    }

    #[test]
    fn test_map_nested_array() {
        let inner = TypeIR::new(TypeKind::String);
        let middle = TypeIR::new(TypeKind::Array(Box::new(inner)));
        let ty = TypeIR::new(TypeKind::Array(Box::new(middle)));
        assert_eq!(mapper().map_type(&ty), "z.array(z.array(z.string()))");
    }

    #[test]
    fn test_map_tuple() {
        let elements = vec![
            TypeIR::new(TypeKind::String),
            TypeIR::new(TypeKind::Integer {
                signed: true,
                bits: Some(32),
            }),
            TypeIR::new(TypeKind::Boolean),
        ];
        let ty = TypeIR::new(TypeKind::Tuple(elements));
        assert_eq!(
            mapper().map_type(&ty),
            "z.tuple([z.string(), z.number().int(), z.boolean()])"
        );
    }

    #[test]
    fn test_map_empty_tuple() {
        let ty = TypeIR::new(TypeKind::Tuple(vec![]));
        assert_eq!(mapper().map_type(&ty), "z.tuple([])");
    }

    #[test]
    fn test_map_record() {
        let key = TypeIR::new(TypeKind::String);
        let value = TypeIR::new(TypeKind::Integer {
            signed: true,
            bits: Some(32),
        });
        let ty = TypeIR::new(TypeKind::Record {
            key: Box::new(key),
            value: Box::new(value),
        });
        assert_eq!(
            mapper().map_type(&ty),
            "z.record(z.string(), z.number().int())"
        );
    }

    #[test]
    fn test_map_optional() {
        let inner = TypeIR::new(TypeKind::String);
        let ty = TypeIR::new(TypeKind::Optional(Box::new(inner)));
        assert_eq!(mapper().map_type(&ty), "z.string().optional()");
    }

    #[test]
    fn test_map_set() {
        let inner = TypeIR::new(TypeKind::String);
        let ty = TypeIR::new(TypeKind::Set(Box::new(inner)));
        assert_eq!(mapper().map_type(&ty), "z.array(z.string())");
    }

    // =========================================================================
    // Special Type Tests (Task 9.3)
    // =========================================================================

    #[test]
    fn test_map_uuid() {
        let ty = TypeIR::new(TypeKind::Uuid);
        assert_eq!(mapper().map_type(&ty), "z.string().uuid()");
    }

    #[test]
    fn test_map_datetime() {
        let ty = TypeIR::new(TypeKind::DateTime);
        assert_eq!(mapper().map_type(&ty), "z.string().datetime()");
    }

    #[test]
    fn test_map_duration() {
        let ty = TypeIR::new(TypeKind::Duration);
        assert_eq!(mapper().map_type(&ty), "z.number()");
    }

    #[test]
    fn test_map_decimal() {
        let ty = TypeIR::new(TypeKind::Decimal);
        assert_eq!(mapper().map_type(&ty), "z.number()");
    }

    #[test]
    fn test_map_any() {
        let ty = TypeIR::new(TypeKind::Any);
        assert_eq!(mapper().map_type(&ty), "z.any()");
    }

    #[test]
    fn test_map_unknown() {
        let ty = TypeIR::new(TypeKind::Unknown);
        assert_eq!(mapper().map_type(&ty), "z.unknown()");
    }

    #[test]
    fn test_map_never() {
        let ty = TypeIR::new(TypeKind::Never);
        assert_eq!(mapper().map_type(&ty), "z.never()");
    }

    #[test]
    fn test_map_null() {
        let ty = TypeIR::new(TypeKind::Null);
        assert_eq!(mapper().map_type(&ty), "z.null()");
    }

    #[test]
    fn test_map_void() {
        let ty = TypeIR::new(TypeKind::Void);
        assert_eq!(mapper().map_type(&ty), "z.void()");
    }

    // =========================================================================
    // Reference Type Tests (Task 9.4)
    // =========================================================================

    #[test]
    fn test_map_reference_simple() {
        let ty = TypeIR::new(TypeKind::Reference {
            name: "User".to_string(),
            generics: vec![],
        });
        assert_eq!(mapper().map_type(&ty), "UserSchema");
    }

    #[test]
    fn test_map_reference_with_generics() {
        let generic = TypeIR::new(TypeKind::String);
        let ty = TypeIR::new(TypeKind::Reference {
            name: "Container".to_string(),
            generics: vec![generic],
        });
        assert_eq!(mapper().map_type(&ty), "z.lazy(() => ContainerSchema)");
    }

    // =========================================================================
    // Literal Type Tests
    // =========================================================================

    #[test]
    fn test_map_literal_string() {
        let ty = TypeIR::new(TypeKind::LiteralString("hello".to_string()));
        assert_eq!(mapper().map_type(&ty), "z.literal(\"hello\")");
    }

    #[test]
    fn test_map_literal_string_with_escapes() {
        let ty = TypeIR::new(TypeKind::LiteralString("hello\nworld".to_string()));
        assert_eq!(mapper().map_type(&ty), "z.literal(\"hello\\nworld\")");
    }

    #[test]
    fn test_map_literal_number() {
        let ty = TypeIR::new(TypeKind::LiteralNumber(42.0));
        assert_eq!(mapper().map_type(&ty), "z.literal(42)");
    }

    #[test]
    fn test_map_literal_boolean() {
        let ty = TypeIR::new(TypeKind::LiteralBoolean(true));
        assert_eq!(mapper().map_type(&ty), "z.literal(true)");

        let ty = TypeIR::new(TypeKind::LiteralBoolean(false));
        assert_eq!(mapper().map_type(&ty), "z.literal(false)");
    }

    // =========================================================================
    // Composite Type Tests
    // =========================================================================

    #[test]
    fn test_map_union() {
        let types = vec![
            TypeIR::new(TypeKind::String),
            TypeIR::new(TypeKind::Integer {
                signed: true,
                bits: Some(32),
            }),
        ];
        let ty = TypeIR::new(TypeKind::Union(types));
        assert_eq!(
            mapper().map_type(&ty),
            "z.union([z.string(), z.number().int()])"
        );
    }

    #[test]
    fn test_map_union_single() {
        let types = vec![TypeIR::new(TypeKind::String)];
        let ty = TypeIR::new(TypeKind::Union(types));
        assert_eq!(mapper().map_type(&ty), "z.string()");
    }

    #[test]
    fn test_map_union_empty() {
        let ty = TypeIR::new(TypeKind::Union(vec![]));
        assert_eq!(mapper().map_type(&ty), "z.never()");
    }

    #[test]
    fn test_map_intersection() {
        let types = vec![
            TypeIR::new(TypeKind::Reference {
                name: "A".to_string(),
                generics: vec![],
            }),
            TypeIR::new(TypeKind::Reference {
                name: "B".to_string(),
                generics: vec![],
            }),
        ];
        let ty = TypeIR::new(TypeKind::Intersection(types));
        assert_eq!(mapper().map_type(&ty), "ASchema.and(BSchema)");
    }

    #[test]
    fn test_map_intersection_single() {
        let types = vec![TypeIR::new(TypeKind::String)];
        let ty = TypeIR::new(TypeKind::Intersection(types));
        assert_eq!(mapper().map_type(&ty), "z.string()");
    }

    #[test]
    fn test_map_intersection_empty() {
        let ty = TypeIR::new(TypeKind::Intersection(vec![]));
        assert_eq!(mapper().map_type(&ty), "z.unknown()");
    }

    // =========================================================================
    // Nullable Tests
    // =========================================================================

    #[test]
    fn test_map_nullable_string() {
        let ty = TypeIR::nullable(TypeKind::String);
        assert_eq!(mapper().map_type(&ty), "z.string().nullable()");
    }

    #[test]
    fn test_map_nullable_array() {
        let inner = TypeIR::new(TypeKind::String);
        let ty = TypeIR::nullable(TypeKind::Array(Box::new(inner)));
        assert_eq!(mapper().map_type(&ty), "z.array(z.string()).nullable()");
    }

    // =========================================================================
    // Custom Override Tests (Task 9.5)
    // =========================================================================

    #[test]
    fn test_custom_override() {
        let mut m = mapper();
        m.add_override("MyCustomType", "z.custom<MyCustomType>()");

        let ty = TypeIR::new(TypeKind::Reference {
            name: "MyCustomType".to_string(),
            generics: vec![],
        });
        assert_eq!(m.map_type(&ty), "z.custom<MyCustomType>()");
    }

    #[test]
    fn test_custom_override_with_original_type() {
        let mut m = mapper();
        m.add_override("MyType", "z.custom()");

        let ty = TypeIR::new(TypeKind::String).with_original_type("MyType");
        assert_eq!(m.map_type(&ty), "z.custom()");
    }

    #[test]
    fn test_map_with_override() {
        let ty = TypeIR::new(TypeKind::String);
        assert_eq!(mapper().map_with_override(&ty, "z.custom()"), "z.custom()");
    }

    #[test]
    fn test_map_with_override_nullable() {
        let ty = TypeIR::nullable(TypeKind::String);
        assert_eq!(
            mapper().map_with_override(&ty, "z.custom()"),
            "z.custom().nullable()"
        );
    }

    // =========================================================================
    // Complex Nested Type Tests
    // =========================================================================

    #[test]
    fn test_complex_nested_type() {
        // Vec<Option<HashMap<String, User>>>
        let user_ref = TypeIR::new(TypeKind::Reference {
            name: "User".to_string(),
            generics: vec![],
        });
        let record = TypeIR::new(TypeKind::Record {
            key: Box::new(TypeIR::new(TypeKind::String)),
            value: Box::new(user_ref),
        });
        let optional = TypeIR::new(TypeKind::Optional(Box::new(record)));
        let array = TypeIR::new(TypeKind::Array(Box::new(optional)));

        assert_eq!(
            mapper().map_type(&array),
            "z.array(z.record(z.string(), UserSchema).optional())"
        );
    }
}

// =============================================================================
// Property-Based Tests
// =============================================================================

#[cfg(test)]
mod proptest_tests {
    use super::*;
    use proptest::prelude::*;

    /// Strategy for generating arbitrary primitive TypeKind values.
    fn arb_primitive_type_kind() -> impl Strategy<Value = TypeKind> {
        prop_oneof![
            Just(TypeKind::String),
            Just(TypeKind::Boolean),
            Just(TypeKind::Char),
            (
                any::<bool>(),
                prop_oneof![
                    Just(Some(8u8)),
                    Just(Some(16u8)),
                    Just(Some(32u8)),
                    Just(Some(64u8)),
                    Just(None),
                ]
            )
                .prop_map(|(signed, bits)| TypeKind::Integer { signed, bits }),
            Just(TypeKind::Float),
            Just(TypeKind::Uuid),
            Just(TypeKind::DateTime),
            Just(TypeKind::Duration),
        ]
    }

    /// Strategy for generating arbitrary primitive TypeIR values.
    fn arb_primitive_type_ir() -> impl Strategy<Value = TypeIR> {
        (arb_primitive_type_kind(), any::<bool>()).prop_map(|(kind, nullable)| TypeIR {
            kind,
            nullable,
            original_type: None,
        })
    }

    /// Strategy for generating arbitrary reference TypeIR values.
    fn arb_reference_type_ir() -> impl Strategy<Value = TypeIR> {
        "[A-Z][a-zA-Z0-9]{0,15}".prop_map(|name| TypeIR {
            kind: TypeKind::Reference {
                name,
                generics: vec![],
            },
            nullable: false,
            original_type: None,
        })
    }

    /// Strategy for generating inner types (primitives or references).
    fn arb_inner_type_ir() -> impl Strategy<Value = TypeIR> {
        prop_oneof![
            3 => arb_primitive_type_ir(),
            1 => arb_reference_type_ir(),
        ]
    }

    proptest! {
        /// **Property 5: Compound Type Mapping**
        ///
        /// *For any* compound type (Option<T>, Vec<T>, HashMap<K,V>) where inner types
        /// implement ZodSchema, the generated schema SHALL correctly wrap the inner
        /// type's schema.
        ///
        /// **Validates: Requirements 1.4, 1.5, 1.6**
        ///
        /// **Feature: zod-schema-macro, Property 5: Compound Type Mapping**
        #[test]
        fn prop_optional_wraps_inner_type(inner in arb_inner_type_ir()) {
            let mapper = ZodTypeMapper::new();
            let inner_schema = mapper.map_type(&inner);

            let optional_type = TypeIR::new(TypeKind::Optional(Box::new(inner)));
            let optional_schema = mapper.map_type(&optional_type);

            // The optional schema should be the inner schema with .optional() appended
            prop_assert_eq!(
                optional_schema,
                format!("{}.optional()", inner_schema),
                "Option<T> should generate T.optional()"
            );
        }

        /// **Property 5: Compound Type Mapping - Vec<T>**
        ///
        /// *For any* Vec<T> where T implements ZodSchema, the generated schema
        /// SHALL be z.array(T_schema).
        ///
        /// **Validates: Requirements 1.5**
        ///
        /// **Feature: zod-schema-macro, Property 5: Compound Type Mapping**
        #[test]
        fn prop_array_wraps_inner_type(inner in arb_inner_type_ir()) {
            let mapper = ZodTypeMapper::new();
            let inner_schema = mapper.map_type(&inner);

            let array_type = TypeIR::new(TypeKind::Array(Box::new(inner)));
            let array_schema = mapper.map_type(&array_type);

            // The array schema should be z.array(inner_schema)
            prop_assert_eq!(
                array_schema,
                format!("z.array({})", inner_schema),
                "Vec<T> should generate z.array(T_schema)"
            );
        }

        /// **Property 5: Compound Type Mapping - HashMap<K, V>**
        ///
        /// *For any* HashMap<K, V> where K and V implement ZodSchema, the generated
        /// schema SHALL be z.record(K_schema, V_schema).
        ///
        /// **Validates: Requirements 1.6**
        ///
        /// **Feature: zod-schema-macro, Property 5: Compound Type Mapping**
        #[test]
        fn prop_record_wraps_key_value_types(
            key in arb_inner_type_ir(),
            value in arb_inner_type_ir()
        ) {
            let mapper = ZodTypeMapper::new();
            let key_schema = mapper.map_type(&key);
            let value_schema = mapper.map_type(&value);

            let record_type = TypeIR::new(TypeKind::Record {
                key: Box::new(key),
                value: Box::new(value),
            });
            let record_schema = mapper.map_type(&record_type);

            // The record schema should be z.record(key_schema, value_schema)
            prop_assert_eq!(
                record_schema,
                format!("z.record({}, {})", key_schema, value_schema),
                "HashMap<K, V> should generate z.record(K_schema, V_schema)"
            );
        }

        /// **Property 5: Compound Type Mapping - Set<T>**
        ///
        /// *For any* HashSet<T> where T implements ZodSchema, the generated schema
        /// SHALL be z.array(T_schema) (sets are represented as arrays in JSON).
        ///
        /// **Validates: Requirements 1.5 (similar to Vec)**
        ///
        /// **Feature: zod-schema-macro, Property 5: Compound Type Mapping**
        #[test]
        fn prop_set_wraps_inner_type(inner in arb_inner_type_ir()) {
            let mapper = ZodTypeMapper::new();
            let inner_schema = mapper.map_type(&inner);

            let set_type = TypeIR::new(TypeKind::Set(Box::new(inner)));
            let set_schema = mapper.map_type(&set_type);

            // Sets are represented as arrays in JSON
            prop_assert_eq!(
                set_schema,
                format!("z.array({})", inner_schema),
                "HashSet<T> should generate z.array(T_schema)"
            );
        }

        /// **Property 5: Compound Type Mapping - Tuple**
        ///
        /// *For any* tuple (T1, T2, ...) where all Ti implement ZodSchema, the generated
        /// schema SHALL be z.tuple([T1_schema, T2_schema, ...]).
        ///
        /// **Validates: Requirements 1.8**
        ///
        /// **Feature: zod-schema-macro, Property 5: Compound Type Mapping**
        #[test]
        fn prop_tuple_wraps_element_types(
            elements in proptest::collection::vec(arb_inner_type_ir(), 1..5)
        ) {
            let mapper = ZodTypeMapper::new();
            let element_schemas: Vec<String> = elements.iter()
                .map(|e| mapper.map_type(e))
                .collect();

            let tuple_type = TypeIR::new(TypeKind::Tuple(elements));
            let tuple_schema = mapper.map_type(&tuple_type);

            // The tuple schema should be z.tuple([element_schemas...])
            prop_assert_eq!(
                tuple_schema,
                format!("z.tuple([{}])", element_schemas.join(", ")),
                "Tuple should generate z.tuple([element_schemas...])"
            );
        }

        /// **Property 5: Compound Type Mapping - Nested Compounds**
        ///
        /// *For any* nested compound type (e.g., Vec<Option<T>>), the generated schema
        /// SHALL correctly nest the inner schemas.
        ///
        /// **Validates: Requirements 1.4, 1.5**
        ///
        /// **Feature: zod-schema-macro, Property 5: Compound Type Mapping**
        #[test]
        fn prop_nested_compound_types(inner in arb_inner_type_ir()) {
            let mapper = ZodTypeMapper::new();
            let inner_schema = mapper.map_type(&inner);

            // Create Vec<Option<T>>
            let optional_type = TypeIR::new(TypeKind::Optional(Box::new(inner)));
            let array_of_optional = TypeIR::new(TypeKind::Array(Box::new(optional_type)));
            let nested_schema = mapper.map_type(&array_of_optional);

            // Should be z.array(inner_schema.optional())
            prop_assert_eq!(
                nested_schema,
                format!("z.array({}.optional())", inner_schema),
                "Vec<Option<T>> should generate z.array(T_schema.optional())"
            );
        }

        /// **Property 5: Compound Type Mapping - Nullable Compound**
        ///
        /// *For any* nullable compound type, the .nullable() modifier SHALL be
        /// applied to the outermost schema.
        ///
        /// **Validates: Requirements 1.4**
        ///
        /// **Feature: zod-schema-macro, Property 5: Compound Type Mapping**
        #[test]
        fn prop_nullable_compound_type(inner in arb_inner_type_ir()) {
            let mapper = ZodTypeMapper::new();
            let inner_schema = mapper.map_type(&inner);

            // Create nullable Vec<T>
            let array_type = TypeIR {
                kind: TypeKind::Array(Box::new(inner)),
                nullable: true,
                original_type: None,
            };
            let nullable_array_schema = mapper.map_type(&array_type);

            // Should be z.array(inner_schema).nullable()
            prop_assert_eq!(
                nullable_array_schema,
                format!("z.array({}).nullable()", inner_schema),
                "Nullable Vec<T> should generate z.array(T_schema).nullable()"
            );
        }
    }
}
