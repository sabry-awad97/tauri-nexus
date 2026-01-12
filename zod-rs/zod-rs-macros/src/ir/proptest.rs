//! Property-based tests for IR types.
//!
//! This module contains property-based tests using proptest to verify
//! correctness properties of the IR types.

#[cfg(test)]
mod tests {
    use crate::ir::{
        EnumSchema, EnumTagging, FieldIR, FieldMetadata, GenericParam, SchemaIR, SchemaKind,
        SchemaMetadata, StructSchema, TupleStructSchema, TypeIR, TypeKind, ValidationRule,
        VariantIR, VariantKind,
    };
    use proptest::prelude::*;
    use proptest::{collection, option};

    // ==========================================================================
    // Arbitrary implementations for IR types
    // ==========================================================================

    /// Strategy for generating arbitrary TypeKind values.
    fn arb_type_kind() -> impl Strategy<Value = TypeKind> {
        prop_oneof![
            // Primitives (weighted higher for more realistic tests)
            3 => Just(TypeKind::String),
            3 => Just(TypeKind::Boolean),
            1 => Just(TypeKind::Char),
            3 => (any::<bool>(), prop_oneof![
                Just(Some(8u8)),
                Just(Some(16u8)),
                Just(Some(32u8)),
                Just(Some(64u8)),
                Just(None),
            ]).prop_map(|(signed, bits)| TypeKind::Integer { signed, bits }),
            2 => Just(TypeKind::Float),
            // Special types
            1 => Just(TypeKind::Uuid),
            1 => Just(TypeKind::DateTime),
            1 => Just(TypeKind::Duration),
            1 => Just(TypeKind::Decimal),
            1 => Just(TypeKind::Any),
            1 => Just(TypeKind::Unknown),
            1 => Just(TypeKind::Never),
            1 => Just(TypeKind::Null),
            1 => Just(TypeKind::Void),
            // Literals - use integers cast to f64 to avoid float precision issues in JSON
            1 => "[a-zA-Z_][a-zA-Z0-9_]{0,20}".prop_map(TypeKind::LiteralString),
            1 => (-1_000_000i64..1_000_000i64).prop_map(|n| TypeKind::LiteralNumber(n as f64)),
            1 => any::<bool>().prop_map(TypeKind::LiteralBoolean),
            // Reference (simple, non-recursive)
            2 => "[A-Z][a-zA-Z0-9]{0,20}".prop_map(|name| TypeKind::Reference {
                name,
                generics: vec![],
            }),
        ]
    }

    /// Strategy for generating arbitrary TypeIR values (non-recursive).
    fn arb_type_ir() -> impl Strategy<Value = TypeIR> {
        (
            arb_type_kind(),
            any::<bool>(),
            option::of("[a-zA-Z_][a-zA-Z0-9_]{0,30}"),
        )
            .prop_map(|(kind, nullable, original_type)| TypeIR {
                kind,
                nullable,
                original_type,
            })
    }

    /// Strategy for generating arbitrary ValidationRule values.
    fn arb_validation_rule() -> impl Strategy<Value = ValidationRule> {
        prop_oneof![
            // String validations
            (1usize..100).prop_map(ValidationRule::MinLength),
            (1usize..100).prop_map(ValidationRule::MaxLength),
            (1usize..100).prop_map(ValidationRule::Length),
            Just(ValidationRule::Email),
            Just(ValidationRule::Url),
            Just(ValidationRule::Uuid),
            Just(ValidationRule::Cuid),
            Just(ValidationRule::Datetime),
            Just(ValidationRule::Ip),
            "[a-zA-Z.*+?]{1,20}".prop_map(ValidationRule::Regex),
            "[a-zA-Z]{1,10}".prop_map(ValidationRule::StartsWith),
            "[a-zA-Z]{1,10}".prop_map(ValidationRule::EndsWith),
            // Number validations - use integers cast to f64 to avoid float precision issues in JSON
            (-1_000_000i64..1_000_000i64).prop_map(|n| ValidationRule::Min(n as f64)),
            (-1_000_000i64..1_000_000i64).prop_map(|n| ValidationRule::Max(n as f64)),
            Just(ValidationRule::Positive),
            Just(ValidationRule::Negative),
            Just(ValidationRule::NonNegative),
            Just(ValidationRule::NonPositive),
            Just(ValidationRule::Int),
            Just(ValidationRule::Finite),
            // Array validations
            (1usize..100).prop_map(ValidationRule::MinItems),
            (1usize..100).prop_map(ValidationRule::MaxItems),
            Just(ValidationRule::Nonempty),
            // Custom validations
            "[a-zA-Z_][a-zA-Z0-9_ =>]{0,30}".prop_map(ValidationRule::Custom),
        ]
    }

    /// Strategy for generating arbitrary FieldMetadata values.
    fn arb_field_metadata() -> impl Strategy<Value = FieldMetadata> {
        (
            option::of("[a-zA-Z ]{0,50}"),
            any::<bool>(),
            collection::vec("[a-zA-Z0-9]{1,20}", 0..3),
        )
            .prop_map(|(description, deprecated, examples)| FieldMetadata {
                description,
                deprecated,
                examples,
            })
    }

    /// Strategy for generating arbitrary SchemaMetadata values.
    fn arb_schema_metadata() -> impl Strategy<Value = SchemaMetadata> {
        (
            option::of("[a-zA-Z ]{0,100}"),
            any::<bool>(),
            option::of("[a-zA-Z ]{0,50}"),
            collection::vec("[a-zA-Z0-9]{1,20}", 0..3),
            collection::vec("[a-zA-Z]{1,10}", 0..3),
            option::of("[0-9]+\\.[0-9]+\\.[0-9]+"),
            option::of("https://[a-z]+\\.com/[a-z]+"),
        )
            .prop_map(
                |(
                    description,
                    deprecated,
                    deprecation_message,
                    examples,
                    tags,
                    since,
                    docs_url,
                )| {
                    SchemaMetadata {
                        description,
                        deprecated,
                        deprecation_message,
                        examples,
                        tags,
                        since,
                        docs_url,
                    }
                },
            )
    }

    /// Strategy for generating arbitrary FieldIR values.
    fn arb_field_ir() -> impl Strategy<Value = FieldIR> {
        (
            "[a-z_][a-z0-9_]{0,20}",
            "[a-z][a-zA-Z0-9]{0,20}",
            arb_type_ir(),
            any::<bool>(),
            any::<bool>(),
            option::of("[a-zA-Z0-9\"{}:,]{0,30}"),
            any::<bool>(),
            collection::vec(arb_validation_rule(), 0..3),
            arb_field_metadata(),
        )
            .prop_map(
                |(
                    rust_name,
                    schema_name,
                    ty,
                    optional,
                    nullable,
                    default,
                    flatten,
                    validation,
                    metadata,
                )| {
                    FieldIR {
                        rust_name,
                        schema_name,
                        ty,
                        optional,
                        nullable,
                        default,
                        flatten,
                        validation,
                        metadata,
                    }
                },
            )
    }

    /// Strategy for generating arbitrary GenericParam values.
    fn arb_generic_param() -> impl Strategy<Value = GenericParam> {
        (
            "[A-Z][a-zA-Z0-9]{0,5}",
            collection::vec("[A-Z][a-zA-Z]+", 0..2),
        )
            .prop_map(|(name, bounds)| GenericParam {
                name,
                bounds,
                default: None, // Keep simple to avoid deep recursion
            })
    }

    /// Strategy for generating arbitrary EnumTagging values.
    fn arb_enum_tagging() -> impl Strategy<Value = EnumTagging> {
        prop_oneof![
            Just(EnumTagging::External),
            "[a-z]{1,10}".prop_map(|tag| EnumTagging::Internal { tag }),
            ("[a-z]{1,10}", "[a-z]{1,10}")
                .prop_map(|(tag, content)| EnumTagging::Adjacent { tag, content }),
            Just(EnumTagging::Untagged),
        ]
    }

    /// Strategy for generating arbitrary VariantKind values.
    fn arb_variant_kind() -> impl Strategy<Value = VariantKind> {
        prop_oneof![
            3 => Just(VariantKind::Unit),
            1 => collection::vec(arb_type_ir(), 1..3).prop_map(VariantKind::Tuple),
            1 => collection::vec(arb_field_ir(), 1..3).prop_map(VariantKind::Struct),
        ]
    }

    /// Strategy for generating arbitrary VariantIR values.
    fn arb_variant_ir() -> impl Strategy<Value = VariantIR> {
        (
            "[A-Z][a-zA-Z0-9]{0,15}",
            "[A-Z][a-zA-Z0-9]{0,15}",
            arb_variant_kind(),
            arb_field_metadata(),
        )
            .prop_map(|(rust_name, schema_name, kind, metadata)| VariantIR {
                rust_name,
                schema_name,
                kind,
                metadata,
            })
    }

    /// Strategy for generating arbitrary StructSchema values.
    fn arb_struct_schema() -> impl Strategy<Value = StructSchema> {
        (
            collection::vec(arb_field_ir(), 0..5),
            any::<bool>(),
            any::<bool>(),
        )
            .prop_map(|(fields, strict, passthrough)| StructSchema {
                fields,
                strict,
                passthrough,
            })
    }

    /// Strategy for generating arbitrary TupleStructSchema values.
    fn arb_tuple_struct_schema() -> impl Strategy<Value = TupleStructSchema> {
        collection::vec(arb_type_ir(), 1..5).prop_map(|fields| TupleStructSchema { fields })
    }

    /// Strategy for generating arbitrary EnumSchema values.
    fn arb_enum_schema() -> impl Strategy<Value = EnumSchema> {
        (collection::vec(arb_variant_ir(), 1..5), arb_enum_tagging()).prop_map(
            |(variants, tagging)| {
                let is_unit_only = variants.iter().all(|v| matches!(v.kind, VariantKind::Unit));
                EnumSchema {
                    variants,
                    tagging,
                    is_unit_only,
                }
            },
        )
    }

    /// Strategy for generating arbitrary SchemaKind values.
    fn arb_schema_kind() -> impl Strategy<Value = SchemaKind> {
        prop_oneof![
            3 => arb_struct_schema().prop_map(SchemaKind::Struct),
            1 => arb_tuple_struct_schema().prop_map(SchemaKind::TupleStruct),
            1 => Just(SchemaKind::UnitStruct),
            2 => arb_enum_schema().prop_map(SchemaKind::Enum),
            1 => arb_type_ir().prop_map(SchemaKind::Alias),
        ]
    }

    /// Strategy for generating arbitrary SchemaIR values.
    fn arb_schema_ir() -> impl Strategy<Value = SchemaIR> {
        (
            "[A-Z][a-zA-Z0-9]{0,20}",
            "[A-Z][a-zA-Z0-9]{0,20}",
            arb_schema_kind(),
            collection::vec(arb_generic_param(), 0..2),
            arb_schema_metadata(),
            any::<bool>(),
        )
            .prop_map(
                |(name, rust_name, kind, generics, metadata, export)| SchemaIR {
                    name,
                    rust_name,
                    kind,
                    generics,
                    metadata,
                    export,
                },
            )
    }

    // ==========================================================================
    // Property Tests
    // ==========================================================================

    proptest! {
        /// **Property 2: IR Serialization Round-Trip**
        ///
        /// *For any* SchemaIR instance, serializing to JSON and deserializing back
        /// SHALL produce an equivalent SchemaIR.
        ///
        /// **Validates: Requirements 3.5**
        #[test]
        fn prop_schema_ir_serialization_roundtrip(schema in arb_schema_ir()) {
            // Serialize to JSON
            let json = serde_json::to_string(&schema)
                .expect("SchemaIR should serialize to JSON");

            // Deserialize back
            let deserialized: SchemaIR = serde_json::from_str(&json)
                .expect("JSON should deserialize back to SchemaIR");

            // Verify equality
            prop_assert_eq!(schema, deserialized,
                "Round-trip serialization should preserve SchemaIR");
        }

        /// Property: TypeIR serialization round-trip
        #[test]
        fn prop_type_ir_serialization_roundtrip(ty in arb_type_ir()) {
            let json = serde_json::to_string(&ty)
                .expect("TypeIR should serialize to JSON");

            let deserialized: TypeIR = serde_json::from_str(&json)
                .expect("JSON should deserialize back to TypeIR");

            prop_assert_eq!(ty, deserialized,
                "Round-trip serialization should preserve TypeIR");
        }

        /// Property: ValidationRule serialization round-trip
        #[test]
        fn prop_validation_rule_serialization_roundtrip(rule in arb_validation_rule()) {
            let json = serde_json::to_string(&rule)
                .expect("ValidationRule should serialize to JSON");

            let deserialized: ValidationRule = serde_json::from_str(&json)
                .expect("JSON should deserialize back to ValidationRule");

            prop_assert_eq!(rule, deserialized,
                "Round-trip serialization should preserve ValidationRule");
        }

        /// Property: FieldIR serialization round-trip
        #[test]
        fn prop_field_ir_serialization_roundtrip(field in arb_field_ir()) {
            let json = serde_json::to_string(&field)
                .expect("FieldIR should serialize to JSON");

            let deserialized: FieldIR = serde_json::from_str(&json)
                .expect("JSON should deserialize back to FieldIR");

            prop_assert_eq!(field, deserialized,
                "Round-trip serialization should preserve FieldIR");
        }

        /// Property: EnumSchema serialization round-trip
        #[test]
        fn prop_enum_schema_serialization_roundtrip(schema in arb_enum_schema()) {
            let json = serde_json::to_string(&schema)
                .expect("EnumSchema should serialize to JSON");

            let deserialized: EnumSchema = serde_json::from_str(&json)
                .expect("JSON should deserialize back to EnumSchema");

            prop_assert_eq!(schema, deserialized,
                "Round-trip serialization should preserve EnumSchema");
        }

        /// Property: SchemaMetadata serialization round-trip
        #[test]
        fn prop_schema_metadata_serialization_roundtrip(metadata in arb_schema_metadata()) {
            let json = serde_json::to_string(&metadata)
                .expect("SchemaMetadata should serialize to JSON");

            let deserialized: SchemaMetadata = serde_json::from_str(&json)
                .expect("JSON should deserialize back to SchemaMetadata");

            prop_assert_eq!(metadata, deserialized,
                "Round-trip serialization should preserve SchemaMetadata");
        }
    }
}
