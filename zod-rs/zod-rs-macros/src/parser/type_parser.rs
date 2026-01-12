//! Type parsing and resolution from Rust AST to IR.
//!
//! This module handles parsing Rust types (via syn) into the intermediate
//! representation (TypeIR). It supports:
//! - Primitive types (String, bool, integers, floats, char)
//! - Compound types (Option, Vec, HashMap, HashSet)
//! - Smart pointer unwrapping (Box, Arc, Rc, RefCell, etc.)
//! - Special types (Uuid, DateTime, Duration) - feature-gated
//! - Reference types (custom types)
//! - Tuples and fixed-size arrays

use syn::{GenericArgument, Path, PathArguments, Type, TypeArray, TypeTuple};

use crate::ir::{TypeIR, TypeKind};

/// Error type for type parsing failures.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ParseError {
    #[error("Unsupported type: {0}")]
    UnsupportedType(String),

    #[error("Empty path in type")]
    EmptyPath,

    #[error("Missing generic parameter for {0}")]
    MissingGeneric(String),

    #[error("Invalid array length")]
    InvalidArrayLength,
}

/// Parses Rust types into intermediate representation.
pub struct TypeParser;

impl TypeParser {
    /// Parse a syn::Type into TypeIR.
    pub fn parse(ty: &Type) -> Result<TypeIR, ParseError> {
        match ty {
            Type::Path(type_path) => {
                // Handle qualified paths (e.g., std::string::String)
                if let Some(qself) = &type_path.qself {
                    return Err(ParseError::UnsupportedType(format!(
                        "Qualified self types: <{:?}>",
                        qself.ty
                    )));
                }
                Self::parse_path(&type_path.path)
            }
            Type::Reference(type_ref) => {
                // Unwrap references and parse the inner type
                Self::parse(&type_ref.elem)
            }
            Type::Array(arr) => Self::parse_array(arr),
            Type::Tuple(tuple) => Self::parse_tuple(tuple),
            Type::Slice(slice) => {
                // Slices become arrays
                let inner = Self::parse(&slice.elem)?;
                Ok(TypeIR::new(TypeKind::Array(Box::new(inner))))
            }
            Type::Paren(paren) => {
                // Parenthesized types - just unwrap
                Self::parse(&paren.elem)
            }
            Type::Group(group) => {
                // Group types - just unwrap
                Self::parse(&group.elem)
            }
            Type::Never(_) => Ok(TypeIR::new(TypeKind::Never)),
            Type::Ptr(_) => Err(ParseError::UnsupportedType("Raw pointers".into())),
            Type::BareFn(_) => Err(ParseError::UnsupportedType("Function pointers".into())),
            Type::TraitObject(_) => Err(ParseError::UnsupportedType("Trait objects".into())),
            Type::ImplTrait(_) => Err(ParseError::UnsupportedType("impl Trait".into())),
            Type::Infer(_) => Err(ParseError::UnsupportedType("Inferred types (_)".into())),
            Type::Macro(_) => Err(ParseError::UnsupportedType("Macro types".into())),
            _ => Err(ParseError::UnsupportedType(format!("{:?}", ty))),
        }
    }

    /// Parse a type path (e.g., String, Vec<T>, Option<T>).
    fn parse_path(path: &Path) -> Result<TypeIR, ParseError> {
        let segment = path.segments.last().ok_or(ParseError::EmptyPath)?;

        let ident = segment.ident.to_string();
        let generics = Self::parse_generics(&segment.arguments)?;

        let kind = match ident.as_str() {
            // =================================================================
            // Primitives
            // =================================================================
            "String" | "str" => TypeKind::String,
            "bool" => TypeKind::Boolean,
            "char" => TypeKind::Char,

            // Signed integers
            "i8" => TypeKind::Integer {
                signed: true,
                bits: Some(8),
            },
            "i16" => TypeKind::Integer {
                signed: true,
                bits: Some(16),
            },
            "i32" => TypeKind::Integer {
                signed: true,
                bits: Some(32),
            },
            "i64" => TypeKind::Integer {
                signed: true,
                bits: Some(64),
            },
            "i128" => TypeKind::Integer {
                signed: true,
                bits: Some(128),
            },
            "isize" => TypeKind::Integer {
                signed: true,
                bits: None,
            },

            // Unsigned integers
            "u8" => TypeKind::Integer {
                signed: false,
                bits: Some(8),
            },
            "u16" => TypeKind::Integer {
                signed: false,
                bits: Some(16),
            },
            "u32" => TypeKind::Integer {
                signed: false,
                bits: Some(32),
            },
            "u64" => TypeKind::Integer {
                signed: false,
                bits: Some(64),
            },
            "u128" => TypeKind::Integer {
                signed: false,
                bits: Some(128),
            },
            "usize" => TypeKind::Integer {
                signed: false,
                bits: None,
            },

            // Floats
            "f32" | "f64" => TypeKind::Float,

            // =================================================================
            // Compound Types
            // =================================================================
            "Option" => {
                let inner = generics
                    .into_iter()
                    .next()
                    .ok_or_else(|| ParseError::MissingGeneric("Option".into()))?;
                TypeKind::Optional(Box::new(inner))
            }

            "Vec" => {
                let inner = generics
                    .into_iter()
                    .next()
                    .ok_or_else(|| ParseError::MissingGeneric("Vec".into()))?;
                TypeKind::Array(Box::new(inner))
            }

            "HashMap" | "BTreeMap" => {
                let mut iter = generics.into_iter();
                let key = iter
                    .next()
                    .ok_or_else(|| ParseError::MissingGeneric("Map key".into()))?;
                let value = iter
                    .next()
                    .ok_or_else(|| ParseError::MissingGeneric("Map value".into()))?;
                TypeKind::Record {
                    key: Box::new(key),
                    value: Box::new(value),
                }
            }

            "HashSet" | "BTreeSet" => {
                let inner = generics
                    .into_iter()
                    .next()
                    .ok_or_else(|| ParseError::MissingGeneric("Set".into()))?;
                TypeKind::Set(Box::new(inner))
            }

            // =================================================================
            // Smart Pointers (unwrap to inner type)
            // =================================================================
            "Box" | "Arc" | "Rc" | "RefCell" | "Cell" | "Mutex" | "RwLock" => {
                let inner = generics
                    .into_iter()
                    .next()
                    .ok_or_else(|| ParseError::MissingGeneric(ident.clone()))?;
                // Return the inner type's kind directly
                return Ok(inner);
            }

            // =================================================================
            // Special Types (feature-gated at usage, not parsing)
            // =================================================================
            "Uuid" => TypeKind::Uuid,
            "DateTime" | "NaiveDateTime" | "NaiveDate" | "NaiveTime" => TypeKind::DateTime,
            "Duration" => TypeKind::Duration,
            "Decimal" => TypeKind::Decimal,

            // =================================================================
            // Unit type
            // =================================================================
            "()" => TypeKind::Void,

            // =================================================================
            // Reference to another schema (custom types)
            // =================================================================
            _ => TypeKind::Reference {
                name: ident.clone(),
                generics,
            },
        };

        Ok(TypeIR::new(kind).with_original_type(&ident))
    }

    /// Parse generic arguments from a path segment.
    fn parse_generics(args: &PathArguments) -> Result<Vec<TypeIR>, ParseError> {
        match args {
            PathArguments::None => Ok(vec![]),
            PathArguments::AngleBracketed(ab) => ab
                .args
                .iter()
                .filter_map(|arg| match arg {
                    GenericArgument::Type(ty) => Some(Self::parse(ty)),
                    GenericArgument::Lifetime(_) => None, // Ignore lifetimes
                    GenericArgument::Const(_) => None,    // Ignore const generics for now
                    _ => None,
                })
                .collect(),
            PathArguments::Parenthesized(_) => Err(ParseError::UnsupportedType("Fn types".into())),
        }
    }

    /// Parse a fixed-size array type [T; N].
    fn parse_array(arr: &TypeArray) -> Result<TypeIR, ParseError> {
        let inner = Self::parse(&arr.elem)?;

        // Try to extract the array length
        // For now, we treat fixed-size arrays as tuples if small, or arrays otherwise
        if let syn::Expr::Lit(lit) = &arr.len {
            if let syn::Lit::Int(int_lit) = &lit.lit {
                if let Ok(len) = int_lit.base10_parse::<usize>() {
                    // Small arrays (up to 12 elements) become tuples
                    if len <= 12 {
                        let elements = vec![inner; len];
                        return Ok(TypeIR::new(TypeKind::Tuple(elements)));
                    }
                }
            }
        }

        // Larger or dynamic arrays become Array type
        Ok(TypeIR::new(TypeKind::Array(Box::new(inner))))
    }

    /// Parse a tuple type (T1, T2, ...).
    fn parse_tuple(tuple: &TypeTuple) -> Result<TypeIR, ParseError> {
        if tuple.elems.is_empty() {
            // Empty tuple is unit/void
            return Ok(TypeIR::new(TypeKind::Void));
        }

        let elements: Result<Vec<TypeIR>, ParseError> =
            tuple.elems.iter().map(Self::parse).collect();

        Ok(TypeIR::new(TypeKind::Tuple(elements?)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    /// Helper to parse a type string into TypeIR
    fn parse_type(ty: Type) -> Result<TypeIR, ParseError> {
        TypeParser::parse(&ty)
    }

    // =========================================================================
    // Primitive Type Tests
    // =========================================================================

    #[test]
    fn test_parse_string() {
        let ty: Type = parse_quote!(String);
        let ir = parse_type(ty).unwrap();
        assert_eq!(ir.kind, TypeKind::String);
    }

    #[test]
    fn test_parse_str() {
        let ty: Type = parse_quote!(str);
        let ir = parse_type(ty).unwrap();
        assert_eq!(ir.kind, TypeKind::String);
    }

    #[test]
    fn test_parse_bool() {
        let ty: Type = parse_quote!(bool);
        let ir = parse_type(ty).unwrap();
        assert_eq!(ir.kind, TypeKind::Boolean);
    }

    #[test]
    fn test_parse_char() {
        let ty: Type = parse_quote!(char);
        let ir = parse_type(ty).unwrap();
        assert_eq!(ir.kind, TypeKind::Char);
    }

    #[test]
    fn test_parse_signed_integers() {
        let cases = [
            (parse_quote!(i8), 8),
            (parse_quote!(i16), 16),
            (parse_quote!(i32), 32),
            (parse_quote!(i64), 64),
            (parse_quote!(i128), 128),
        ];

        for (ty, expected_bits) in cases {
            let ir = parse_type(ty).unwrap();
            assert!(matches!(
                ir.kind,
                TypeKind::Integer { signed: true, bits: Some(b) } if b == expected_bits
            ));
        }
    }

    #[test]
    fn test_parse_isize() {
        let ty: Type = parse_quote!(isize);
        let ir = parse_type(ty).unwrap();
        assert!(matches!(
            ir.kind,
            TypeKind::Integer {
                signed: true,
                bits: None
            }
        ));
    }

    #[test]
    fn test_parse_unsigned_integers() {
        let cases = [
            (parse_quote!(u8), 8),
            (parse_quote!(u16), 16),
            (parse_quote!(u32), 32),
            (parse_quote!(u64), 64),
            (parse_quote!(u128), 128),
        ];

        for (ty, expected_bits) in cases {
            let ir = parse_type(ty).unwrap();
            assert!(matches!(
                ir.kind,
                TypeKind::Integer { signed: false, bits: Some(b) } if b == expected_bits
            ));
        }
    }

    #[test]
    fn test_parse_usize() {
        let ty: Type = parse_quote!(usize);
        let ir = parse_type(ty).unwrap();
        assert!(matches!(
            ir.kind,
            TypeKind::Integer {
                signed: false,
                bits: None
            }
        ));
    }

    #[test]
    fn test_parse_floats() {
        for ty in [parse_quote!(f32), parse_quote!(f64)] {
            let ir = parse_type(ty).unwrap();
            assert_eq!(ir.kind, TypeKind::Float);
        }
    }

    // =========================================================================
    // Compound Type Tests
    // =========================================================================

    #[test]
    fn test_parse_option() {
        let ty: Type = parse_quote!(Option<String>);
        let ir = parse_type(ty).unwrap();

        if let TypeKind::Optional(inner) = ir.kind {
            assert_eq!(inner.kind, TypeKind::String);
        } else {
            panic!("Expected Optional, got {:?}", ir.kind);
        }
    }

    #[test]
    fn test_parse_vec() {
        let ty: Type = parse_quote!(Vec<i32>);
        let ir = parse_type(ty).unwrap();

        if let TypeKind::Array(inner) = ir.kind {
            assert!(matches!(
                inner.kind,
                TypeKind::Integer {
                    signed: true,
                    bits: Some(32)
                }
            ));
        } else {
            panic!("Expected Array, got {:?}", ir.kind);
        }
    }

    #[test]
    fn test_parse_hashmap() {
        let ty: Type = parse_quote!(HashMap<String, i32>);
        let ir = parse_type(ty).unwrap();

        if let TypeKind::Record { key, value } = ir.kind {
            assert_eq!(key.kind, TypeKind::String);
            assert!(matches!(
                value.kind,
                TypeKind::Integer {
                    signed: true,
                    bits: Some(32)
                }
            ));
        } else {
            panic!("Expected Record, got {:?}", ir.kind);
        }
    }

    #[test]
    fn test_parse_btreemap() {
        let ty: Type = parse_quote!(BTreeMap<String, bool>);
        let ir = parse_type(ty).unwrap();

        if let TypeKind::Record { key, value } = ir.kind {
            assert_eq!(key.kind, TypeKind::String);
            assert_eq!(value.kind, TypeKind::Boolean);
        } else {
            panic!("Expected Record, got {:?}", ir.kind);
        }
    }

    #[test]
    fn test_parse_hashset() {
        let ty: Type = parse_quote!(HashSet<String>);
        let ir = parse_type(ty).unwrap();

        if let TypeKind::Set(inner) = ir.kind {
            assert_eq!(inner.kind, TypeKind::String);
        } else {
            panic!("Expected Set, got {:?}", ir.kind);
        }
    }

    #[test]
    fn test_parse_btreeset() {
        let ty: Type = parse_quote!(BTreeSet<i64>);
        let ir = parse_type(ty).unwrap();

        if let TypeKind::Set(inner) = ir.kind {
            assert!(matches!(
                inner.kind,
                TypeKind::Integer {
                    signed: true,
                    bits: Some(64)
                }
            ));
        } else {
            panic!("Expected Set, got {:?}", ir.kind);
        }
    }

    // =========================================================================
    // Smart Pointer Tests
    // =========================================================================

    #[test]
    fn test_parse_box() {
        let ty: Type = parse_quote!(Box<String>);
        let ir = parse_type(ty).unwrap();
        assert_eq!(ir.kind, TypeKind::String);
    }

    #[test]
    fn test_parse_arc() {
        let ty: Type = parse_quote!(Arc<i32>);
        let ir = parse_type(ty).unwrap();
        assert!(matches!(
            ir.kind,
            TypeKind::Integer {
                signed: true,
                bits: Some(32)
            }
        ));
    }

    #[test]
    fn test_parse_rc() {
        let ty: Type = parse_quote!(Rc<bool>);
        let ir = parse_type(ty).unwrap();
        assert_eq!(ir.kind, TypeKind::Boolean);
    }

    #[test]
    fn test_parse_refcell() {
        let ty: Type = parse_quote!(RefCell<String>);
        let ir = parse_type(ty).unwrap();
        assert_eq!(ir.kind, TypeKind::String);
    }

    #[test]
    fn test_parse_mutex() {
        let ty: Type = parse_quote!(Mutex<Vec<i32>>);
        let ir = parse_type(ty).unwrap();

        if let TypeKind::Array(inner) = ir.kind {
            assert!(matches!(
                inner.kind,
                TypeKind::Integer {
                    signed: true,
                    bits: Some(32)
                }
            ));
        } else {
            panic!("Expected Array, got {:?}", ir.kind);
        }
    }

    // =========================================================================
    // Special Type Tests
    // =========================================================================

    #[test]
    fn test_parse_uuid() {
        let ty: Type = parse_quote!(Uuid);
        let ir = parse_type(ty).unwrap();
        assert_eq!(ir.kind, TypeKind::Uuid);
    }

    #[test]
    fn test_parse_datetime() {
        let ty: Type = parse_quote!(DateTime);
        let ir = parse_type(ty).unwrap();
        assert_eq!(ir.kind, TypeKind::DateTime);
    }

    #[test]
    fn test_parse_duration() {
        let ty: Type = parse_quote!(Duration);
        let ir = parse_type(ty).unwrap();
        assert_eq!(ir.kind, TypeKind::Duration);
    }

    // =========================================================================
    // Reference Type Tests
    // =========================================================================

    #[test]
    fn test_parse_custom_type() {
        let ty: Type = parse_quote!(User);
        let ir = parse_type(ty).unwrap();

        if let TypeKind::Reference { name, generics } = ir.kind {
            assert_eq!(name, "User");
            assert!(generics.is_empty());
        } else {
            panic!("Expected Reference, got {:?}", ir.kind);
        }
    }

    #[test]
    fn test_parse_generic_custom_type() {
        let ty: Type = parse_quote!(Response<User>);
        let ir = parse_type(ty).unwrap();

        if let TypeKind::Reference { name, generics } = ir.kind {
            assert_eq!(name, "Response");
            assert_eq!(generics.len(), 1);
            if let TypeKind::Reference {
                name: inner_name, ..
            } = &generics[0].kind
            {
                assert_eq!(inner_name, "User");
            } else {
                panic!("Expected inner Reference");
            }
        } else {
            panic!("Expected Reference, got {:?}", ir.kind);
        }
    }

    // =========================================================================
    // Tuple and Array Tests
    // =========================================================================

    #[test]
    fn test_parse_tuple() {
        let ty: Type = parse_quote!((String, i32, bool));
        let ir = parse_type(ty).unwrap();

        if let TypeKind::Tuple(elements) = ir.kind {
            assert_eq!(elements.len(), 3);
            assert_eq!(elements[0].kind, TypeKind::String);
            assert!(matches!(
                elements[1].kind,
                TypeKind::Integer {
                    signed: true,
                    bits: Some(32)
                }
            ));
            assert_eq!(elements[2].kind, TypeKind::Boolean);
        } else {
            panic!("Expected Tuple, got {:?}", ir.kind);
        }
    }

    #[test]
    fn test_parse_unit_tuple() {
        let ty: Type = parse_quote!(());
        let ir = parse_type(ty).unwrap();
        assert_eq!(ir.kind, TypeKind::Void);
    }

    #[test]
    fn test_parse_small_fixed_array() {
        let ty: Type = parse_quote!([i32; 3]);
        let ir = parse_type(ty).unwrap();

        if let TypeKind::Tuple(elements) = ir.kind {
            assert_eq!(elements.len(), 3);
            for elem in elements {
                assert!(matches!(
                    elem.kind,
                    TypeKind::Integer {
                        signed: true,
                        bits: Some(32)
                    }
                ));
            }
        } else {
            panic!("Expected Tuple for small array, got {:?}", ir.kind);
        }
    }

    // =========================================================================
    // Reference Unwrapping Tests
    // =========================================================================

    #[test]
    fn test_parse_reference() {
        let ty: Type = parse_quote!(&str);
        let ir = parse_type(ty).unwrap();
        assert_eq!(ir.kind, TypeKind::String);
    }

    #[test]
    fn test_parse_mutable_reference() {
        let ty: Type = parse_quote!(&mut String);
        let ir = parse_type(ty).unwrap();
        assert_eq!(ir.kind, TypeKind::String);
    }

    // =========================================================================
    // Nested Type Tests
    // =========================================================================

    #[test]
    fn test_parse_nested_option_vec() {
        let ty: Type = parse_quote!(Option<Vec<String>>);
        let ir = parse_type(ty).unwrap();

        if let TypeKind::Optional(inner) = ir.kind {
            if let TypeKind::Array(inner2) = inner.kind {
                assert_eq!(inner2.kind, TypeKind::String);
            } else {
                panic!("Expected Array inside Optional");
            }
        } else {
            panic!("Expected Optional, got {:?}", ir.kind);
        }
    }

    #[test]
    fn test_parse_vec_option() {
        let ty: Type = parse_quote!(Vec<Option<i32>>);
        let ir = parse_type(ty).unwrap();

        if let TypeKind::Array(inner) = ir.kind {
            if let TypeKind::Optional(inner2) = inner.kind {
                assert!(matches!(
                    inner2.kind,
                    TypeKind::Integer {
                        signed: true,
                        bits: Some(32)
                    }
                ));
            } else {
                panic!("Expected Optional inside Array");
            }
        } else {
            panic!("Expected Array, got {:?}", ir.kind);
        }
    }

    #[test]
    fn test_parse_hashmap_with_vec_value() {
        let ty: Type = parse_quote!(HashMap<String, Vec<User>>);
        let ir = parse_type(ty).unwrap();

        if let TypeKind::Record { key, value } = ir.kind {
            assert_eq!(key.kind, TypeKind::String);
            if let TypeKind::Array(inner) = value.kind {
                if let TypeKind::Reference { name, .. } = inner.kind {
                    assert_eq!(name, "User");
                } else {
                    panic!("Expected Reference inside Array");
                }
            } else {
                panic!("Expected Array as value");
            }
        } else {
            panic!("Expected Record, got {:?}", ir.kind);
        }
    }

    // =========================================================================
    // Error Cases
    // =========================================================================

    #[test]
    fn test_parse_option_missing_generic() {
        // This would be a compile error in real Rust, but we test our error handling
        let ty: Type = parse_quote!(Option);
        let result = parse_type(ty);
        assert!(matches!(result, Err(ParseError::MissingGeneric(_))));
    }

    #[test]
    fn test_parse_vec_missing_generic() {
        let ty: Type = parse_quote!(Vec);
        let result = parse_type(ty);
        assert!(matches!(result, Err(ParseError::MissingGeneric(_))));
    }

    #[test]
    fn test_parse_hashmap_missing_value() {
        // HashMap with only one generic - would be compile error in Rust
        let ty: Type = parse_quote!(HashMap<String>);
        let result = parse_type(ty);
        assert!(matches!(result, Err(ParseError::MissingGeneric(_))));
    }
}

#[cfg(test)]
mod proptest_tests {
    use super::*;
    use proptest::prelude::*;

    /// Strategy for generating arbitrary TypeKind values (primitives only).
    fn arb_primitive_type_kind() -> impl Strategy<Value = TypeKind> {
        prop_oneof![
            Just(TypeKind::String),
            Just(TypeKind::Boolean),
            Just(TypeKind::Char),
            Just(TypeKind::Float),
            // Signed integers
            Just(TypeKind::Integer {
                signed: true,
                bits: Some(8)
            }),
            Just(TypeKind::Integer {
                signed: true,
                bits: Some(16)
            }),
            Just(TypeKind::Integer {
                signed: true,
                bits: Some(32)
            }),
            Just(TypeKind::Integer {
                signed: true,
                bits: Some(64)
            }),
            Just(TypeKind::Integer {
                signed: true,
                bits: None
            }),
            // Unsigned integers
            Just(TypeKind::Integer {
                signed: false,
                bits: Some(8)
            }),
            Just(TypeKind::Integer {
                signed: false,
                bits: Some(16)
            }),
            Just(TypeKind::Integer {
                signed: false,
                bits: Some(32)
            }),
            Just(TypeKind::Integer {
                signed: false,
                bits: Some(64)
            }),
            Just(TypeKind::Integer {
                signed: false,
                bits: None
            }),
        ]
    }

    /// Strategy for generating type names that map to primitives.
    fn arb_primitive_type_name() -> impl Strategy<Value = &'static str> {
        prop_oneof![
            Just("String"),
            Just("str"),
            Just("bool"),
            Just("char"),
            Just("i8"),
            Just("i16"),
            Just("i32"),
            Just("i64"),
            Just("i128"),
            Just("isize"),
            Just("u8"),
            Just("u16"),
            Just("u32"),
            Just("u64"),
            Just("u128"),
            Just("usize"),
            Just("f32"),
            Just("f64"),
        ]
    }

    /// Strategy for generating special type names.
    fn arb_special_type_name() -> impl Strategy<Value = &'static str> {
        prop_oneof![
            Just("Uuid"),
            Just("DateTime"),
            Just("NaiveDateTime"),
            Just("Duration"),
            Just("Decimal"),
        ]
    }

    proptest! {
        /// **Property 4: Type Mapping Consistency**
        ///
        /// *For any* supported Rust primitive type, the TypeParser SHALL produce
        /// a valid TypeIR with the correct TypeKind.
        ///
        /// This property verifies that:
        /// 1. Parsing a primitive type name produces a valid TypeIR
        /// 2. The TypeKind is a primitive type
        /// 3. Parsing is deterministic (same input -> same output)
        ///
        /// **Validates: Requirements 6.1-6.6**
        #[test]
        fn prop_primitive_type_mapping_is_consistent(
            type_name in arb_primitive_type_name()
        ) {
            // Parse the type name as a syn::Type
            let ty: syn::Type = syn::parse_str(type_name)
                .expect("Should parse valid type name");

            // Parse with TypeParser
            let result = TypeParser::parse(&ty);

            // Should succeed
            prop_assert!(result.is_ok(), "Parsing {} should succeed", type_name);

            let ir = result.unwrap();

            // Should be a primitive type
            prop_assert!(
                ir.kind.is_primitive(),
                "Type {} should map to primitive, got {:?}",
                type_name,
                ir.kind
            );

            // Parsing again should produce the same result (determinism)
            let result2 = TypeParser::parse(&ty).unwrap();
            prop_assert_eq!(
                ir.kind, result2.kind,
                "Parsing should be deterministic for {}",
                type_name
            );
        }

        /// Property: Special types are mapped correctly.
        ///
        /// *For any* special type (Uuid, DateTime, etc.), the TypeParser SHALL
        /// produce the corresponding TypeKind.
        #[test]
        fn prop_special_type_mapping_is_consistent(
            type_name in arb_special_type_name()
        ) {
            let ty: syn::Type = syn::parse_str(type_name)
                .expect("Should parse valid type name");

            let result = TypeParser::parse(&ty);
            prop_assert!(result.is_ok(), "Parsing {} should succeed", type_name);

            let ir = result.unwrap();

            // Verify correct mapping
            match type_name {
                "Uuid" => prop_assert_eq!(ir.kind, TypeKind::Uuid),
                "DateTime" | "NaiveDateTime" => prop_assert_eq!(ir.kind, TypeKind::DateTime),
                "Duration" => prop_assert_eq!(ir.kind, TypeKind::Duration),
                "Decimal" => prop_assert_eq!(ir.kind, TypeKind::Decimal),
                _ => unreachable!(),
            };
        }

        /// Property: Compound type wrapping is consistent.
        ///
        /// *For any* compound type (Option<T>, Vec<T>), the TypeParser SHALL
        /// correctly wrap the inner type.
        #[test]
        fn prop_compound_type_wrapping_is_consistent(
            inner_type in arb_primitive_type_name()
        ) {
            // Test Option<T>
            let option_str = format!("Option<{}>", inner_type);
            let option_ty: syn::Type = syn::parse_str(&option_str)
                .expect("Should parse Option type");

            let option_result = TypeParser::parse(&option_ty);
            prop_assert!(option_result.is_ok(), "Parsing {} should succeed", option_str);

            let option_ir = option_result.unwrap();
            prop_assert!(
                matches!(option_ir.kind, TypeKind::Optional(_)),
                "Option<{}> should map to Optional, got {:?}",
                inner_type,
                option_ir.kind
            );

            // Test Vec<T>
            let vec_str = format!("Vec<{}>", inner_type);
            let vec_ty: syn::Type = syn::parse_str(&vec_str)
                .expect("Should parse Vec type");

            let vec_result = TypeParser::parse(&vec_ty);
            prop_assert!(vec_result.is_ok(), "Parsing {} should succeed", vec_str);

            let vec_ir = vec_result.unwrap();
            prop_assert!(
                matches!(vec_ir.kind, TypeKind::Array(_)),
                "Vec<{}> should map to Array, got {:?}",
                inner_type,
                vec_ir.kind
            );
        }

        /// Property: Smart pointer unwrapping is consistent.
        ///
        /// *For any* smart pointer type (Box<T>, Arc<T>, etc.), the TypeParser
        /// SHALL unwrap to the inner type.
        #[test]
        fn prop_smart_pointer_unwrapping_is_consistent(
            inner_type in arb_primitive_type_name()
        ) {
            let smart_pointers = ["Box", "Arc", "Rc", "RefCell", "Cell", "Mutex", "RwLock"];

            for wrapper in smart_pointers {
                let wrapped_str = format!("{}<{}>", wrapper, inner_type);
                let wrapped_ty: syn::Type = syn::parse_str(&wrapped_str)
                    .expect("Should parse wrapped type");

                let wrapped_result = TypeParser::parse(&wrapped_ty);
                prop_assert!(
                    wrapped_result.is_ok(),
                    "Parsing {} should succeed",
                    wrapped_str
                );

                // Parse the inner type directly
                let inner_ty: syn::Type = syn::parse_str(inner_type)
                    .expect("Should parse inner type");
                let inner_result = TypeParser::parse(&inner_ty).unwrap();

                // The wrapped type should unwrap to the same kind as the inner type
                let wrapped_ir = wrapped_result.unwrap();
                prop_assert_eq!(
                    wrapped_ir.kind, inner_result.kind,
                    "{}<{}> should unwrap to same kind as {}",
                    wrapper, inner_type, inner_type
                );
            }
        }

        /// Property: Reference types preserve the type name.
        ///
        /// *For any* custom type name, the TypeParser SHALL create a Reference
        /// with that name.
        #[test]
        fn prop_reference_type_preserves_name(
            name in "[A-Z][a-zA-Z0-9]{0,20}"
        ) {
            // Skip names that match known types
            let known_types = [
                "String", "Option", "Vec", "HashMap", "HashSet", "Box", "Arc",
                "Rc", "RefCell", "Cell", "Mutex", "RwLock", "Uuid", "DateTime",
                "Duration", "Decimal", "BTreeMap", "BTreeSet", "NaiveDateTime",
                "NaiveDate", "NaiveTime",
            ];

            if known_types.contains(&name.as_str()) {
                return Ok(());
            }

            let ty: syn::Type = syn::parse_str(&name)
                .expect("Should parse type name");

            let result = TypeParser::parse(&ty);
            prop_assert!(result.is_ok(), "Parsing {} should succeed", name);

            let ir = result.unwrap();
            if let TypeKind::Reference { name: ref_name, generics } = ir.kind {
                prop_assert_eq!(ref_name, name, "Reference name should match");
                prop_assert!(generics.is_empty(), "Should have no generics");
            } else {
                prop_assert!(false, "Expected Reference, got {:?}", ir.kind);
            }
        }
    }
}
