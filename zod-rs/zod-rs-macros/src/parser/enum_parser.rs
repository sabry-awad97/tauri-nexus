//! Enum parsing logic.
//!
//! This module handles parsing Rust enum definitions into IR.
//! It supports:
//! - Unit-only enums (for z.enum())
//! - Data enums with tuple and struct variants
//! - Various tagging strategies (external, internal, adjacent, untagged)
//! - Doc comment extraction
//! - Container and variant attributes

use darling::{FromDeriveInput, FromVariant};
use syn::{Data, DeriveInput, Fields, Variant};

use crate::ir::{
    EnumSchema, EnumTagging, FieldIR, FieldMetadata, GenericParam, SchemaIR, SchemaKind,
    SchemaMetadata, TypeIR, VariantIR, VariantKind,
};
use crate::parser::attributes::{ContainerAttrs, RenameRule, VariantAttrs};
use crate::parser::struct_parser::extract_doc_comments;
use crate::parser::type_parser::{ParseError, TypeParser};

#[cfg(feature = "serde-compat")]
use crate::parser::serde_compat::{SerdeContainerAttrs, SerdeVariantAttrs};

/// Error type for enum parsing failures.
#[derive(Debug, Clone, thiserror::Error)]
pub enum EnumParseError {
    #[error("Failed to parse container attributes: {0}")]
    ContainerAttrs(String),

    #[error("Failed to parse variant attributes: {0}")]
    VariantAttrs(String),

    #[error("Failed to parse field type: {0}")]
    FieldType(#[from] ParseError),

    #[error("Expected enum, found {0}")]
    NotAnEnum(String),

    #[error("Variant '{0}' has no identifier")]
    MissingVariantIdent(usize),

    #[error("Field in variant '{0}' has no identifier (index: {1})")]
    MissingFieldIdent(String, usize),
}

/// Parses Rust enum definitions into SchemaIR.
pub struct EnumParser;

impl EnumParser {
    /// Parse a DeriveInput into a SchemaIR for an enum.
    ///
    /// This is the main entry point for enum parsing.
    pub fn parse(input: &DeriveInput) -> Result<SchemaIR, EnumParseError> {
        // Parse container attributes using darling
        let container_attrs = ContainerAttrs::from_derive_input(input)
            .map_err(|e| EnumParseError::ContainerAttrs(e.to_string()))?;

        // Ensure we have an enum
        let data_enum = match &input.data {
            Data::Enum(e) => e,
            Data::Struct(_) => return Err(EnumParseError::NotAnEnum("struct".to_string())),
            Data::Union(_) => return Err(EnumParseError::NotAnEnum("union".to_string())),
        };

        // Parse serde attributes if feature is enabled
        #[cfg(feature = "serde-compat")]
        let serde_attrs = SerdeContainerAttrs::from_attrs(&input.attrs);

        // Determine tagging strategy
        #[cfg(feature = "serde-compat")]
        let tagging = Self::determine_tagging(&container_attrs, &serde_attrs);

        #[cfg(not(feature = "serde-compat"))]
        let tagging = Self::determine_tagging_no_serde(&container_attrs);

        // Parse all variants
        let variants = Self::parse_variants(
            &data_enum.variants,
            container_attrs.rename_all,
            #[cfg(feature = "serde-compat")]
            &serde_attrs,
        )?;

        // Create enum schema
        let enum_schema = EnumSchema::new(variants).with_tagging(tagging);

        // Extract doc comments from the enum
        let description = extract_doc_comments(&input.attrs);

        // Build metadata
        let mut metadata = SchemaMetadata::default();
        if let Some(desc) = description {
            metadata = metadata.description(desc);
        }
        if let Some(attr_desc) = &container_attrs.description {
            // Attribute description overrides doc comments
            metadata = SchemaMetadata::default().description(attr_desc.clone());
        }
        if container_attrs.deprecated {
            metadata = metadata.as_deprecated();
        }

        // Parse generic parameters
        let generics = Self::parse_generics(&input.generics);

        // Build the schema
        let schema_name = container_attrs.schema_name();
        let rust_name = container_attrs.ident.to_string();

        let schema = SchemaIR::new(&rust_name, SchemaKind::Enum(enum_schema))
            .with_name(schema_name)
            .with_generics(generics)
            .with_metadata(metadata)
            .with_export(container_attrs.should_export());

        Ok(schema)
    }

    /// Parse enum variants into VariantIR.
    fn parse_variants(
        variants: &syn::punctuated::Punctuated<Variant, syn::token::Comma>,
        rename_all: Option<RenameRule>,
        #[cfg(feature = "serde-compat")] _serde_container: &SerdeContainerAttrs,
    ) -> Result<Vec<VariantIR>, EnumParseError> {
        let mut variant_irs = Vec::with_capacity(variants.len());

        for (index, variant) in variants.iter().enumerate() {
            // Parse variant attributes using darling
            let variant_attrs = VariantAttrs::from_variant(variant)
                .map_err(|e| EnumParseError::VariantAttrs(e.to_string()))?;

            // Parse serde variant attributes if feature is enabled
            #[cfg(feature = "serde-compat")]
            let serde_variant_attrs = SerdeVariantAttrs::from_attrs(&variant.attrs);

            // Check if variant should be skipped
            #[cfg(feature = "serde-compat")]
            let should_skip = variant_attrs.skip || serde_variant_attrs.should_skip();

            #[cfg(not(feature = "serde-compat"))]
            let should_skip = variant_attrs.skip;

            if should_skip {
                continue;
            }

            // Get variant name
            let rust_name = variant.ident.to_string();

            // Get schema name (apply rename rules)
            // First check for explicit rename in zod attrs
            let schema_name = if let Some(ref rename) = variant_attrs.rename {
                rename.clone()
            } else {
                // Then check serde rename if feature enabled
                #[cfg(feature = "serde-compat")]
                let serde_rename = serde_variant_attrs.rename.clone();

                #[cfg(not(feature = "serde-compat"))]
                let serde_rename: Option<String> = None;

                if let Some(serde_name) = serde_rename {
                    serde_name
                } else {
                    // Apply rename_all rule
                    variant_attrs.schema_name(rename_all)
                }
            };

            // Parse variant kind based on fields
            let kind = Self::parse_variant_kind(&variant.fields, &rust_name)?;

            // Extract doc comments from variant
            let variant_description = extract_doc_comments(&variant.attrs);

            // Build variant metadata
            let mut variant_metadata = FieldMetadata::default();
            if let Some(desc) = variant_description {
                variant_metadata.description = Some(desc);
            }
            // Attribute description overrides doc comments
            if let Some(attr_desc) = &variant_attrs.description {
                variant_metadata.description = Some(attr_desc.clone());
            }
            if variant_attrs.deprecated {
                variant_metadata.deprecated = true;
            }

            // Build the variant IR
            let variant_ir = match kind {
                VariantKind::Unit => VariantIR::unit(&rust_name),
                VariantKind::Tuple(fields) => VariantIR::tuple(&rust_name, fields),
                VariantKind::Struct(fields) => VariantIR::struct_variant(&rust_name, fields),
            }
            .with_schema_name(schema_name)
            .with_metadata(variant_metadata);

            variant_irs.push(variant_ir);
        }

        Ok(variant_irs)
    }

    /// Parse the kind of a variant (unit, tuple, or struct).
    fn parse_variant_kind(
        fields: &Fields,
        variant_name: &str,
    ) -> Result<VariantKind, EnumParseError> {
        match fields {
            Fields::Unit => Ok(VariantKind::Unit),
            Fields::Unnamed(unnamed) => {
                let field_types: Result<Vec<TypeIR>, _> = unnamed
                    .unnamed
                    .iter()
                    .map(|f| TypeParser::parse(&f.ty))
                    .collect();
                Ok(VariantKind::Tuple(field_types?))
            }
            Fields::Named(named) => {
                let mut field_irs = Vec::with_capacity(named.named.len());
                for (index, field) in named.named.iter().enumerate() {
                    let rust_name = field
                        .ident
                        .as_ref()
                        .ok_or_else(|| {
                            EnumParseError::MissingFieldIdent(variant_name.to_string(), index)
                        })?
                        .to_string();

                    let ty = TypeParser::parse(&field.ty)?;

                    // Extract doc comments from field
                    let field_description = extract_doc_comments(&field.attrs);

                    let mut field_metadata = FieldMetadata::default();
                    if let Some(desc) = field_description {
                        field_metadata.description = Some(desc);
                    }

                    field_irs.push(FieldIR::new(&rust_name, ty).with_metadata(field_metadata));
                }
                Ok(VariantKind::Struct(field_irs))
            }
        }
    }

    /// Determine the tagging strategy from container attributes (with serde-compat).
    #[cfg(feature = "serde-compat")]
    fn determine_tagging(
        zod_attrs: &ContainerAttrs,
        serde_attrs: &SerdeContainerAttrs,
    ) -> EnumTagging {
        // Zod attributes take precedence over serde
        let tag = zod_attrs.tag.clone().or_else(|| serde_attrs.tag.clone());
        let content = zod_attrs
            .content
            .clone()
            .or_else(|| serde_attrs.content.clone());

        // Check for untagged (serde only)
        if serde_attrs.untagged && zod_attrs.tag.is_none() {
            return EnumTagging::Untagged;
        }

        match (tag, content) {
            (Some(tag), Some(content)) => EnumTagging::Adjacent { tag, content },
            (Some(tag), None) => EnumTagging::Internal { tag },
            (None, _) => EnumTagging::External,
        }
    }

    /// Determine the tagging strategy from container attributes (without serde-compat).
    #[cfg(not(feature = "serde-compat"))]
    fn determine_tagging_no_serde(zod_attrs: &ContainerAttrs) -> EnumTagging {
        match (&zod_attrs.tag, &zod_attrs.content) {
            (Some(tag), Some(content)) => EnumTagging::Adjacent {
                tag: tag.clone(),
                content: content.clone(),
            },
            (Some(tag), None) => EnumTagging::Internal { tag: tag.clone() },
            (None, _) => EnumTagging::External,
        }
    }

    /// Parse generic parameters from syn::Generics.
    fn parse_generics(generics: &syn::Generics) -> Vec<GenericParam> {
        generics
            .type_params()
            .map(|param| {
                let name = param.ident.to_string();
                let bounds: Vec<String> = param
                    .bounds
                    .iter()
                    .map(|b| quote::quote!(#b).to_string())
                    .collect();

                let mut generic_param = GenericParam::new(name);
                for bound in bounds {
                    generic_param = generic_param.with_bound(bound);
                }
                generic_param
            })
            .collect()
    }

    /// Check if an enum is unit-only (all variants are unit variants).
    pub fn is_unit_only(input: &DeriveInput) -> bool {
        if let Data::Enum(data_enum) = &input.data {
            data_enum
                .variants
                .iter()
                .all(|v| matches!(v.fields, Fields::Unit))
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_parse_unit_enum() {
        let input: DeriveInput = parse_quote! {
            enum Status {
                Active,
                Inactive,
                Pending,
            }
        };

        let schema = EnumParser::parse(&input).unwrap();
        assert_eq!(schema.name, "Status");
        assert_eq!(schema.rust_name, "Status");

        if let SchemaKind::Enum(e) = &schema.kind {
            assert!(e.is_unit_only);
            assert_eq!(e.variants.len(), 3);
            assert_eq!(e.variants[0].rust_name, "Active");
            assert_eq!(e.variants[1].rust_name, "Inactive");
            assert_eq!(e.variants[2].rust_name, "Pending");

            // All should be unit variants
            for variant in &e.variants {
                assert!(matches!(variant.kind, VariantKind::Unit));
            }
        } else {
            panic!("Expected Enum kind");
        }
    }

    #[test]
    fn test_parse_unit_enum_with_rename() {
        let input: DeriveInput = parse_quote! {
            #[zod(rename = "UserStatus")]
            enum Status {
                Active,
                Inactive,
            }
        };

        let schema = EnumParser::parse(&input).unwrap();
        assert_eq!(schema.name, "UserStatus");
        assert_eq!(schema.rust_name, "Status");
    }

    #[test]
    fn test_parse_unit_enum_with_rename_all() {
        let input: DeriveInput = parse_quote! {
            #[zod(rename_all = "camelCase")]
            enum Status {
                ActiveUser,
                InactiveUser,
            }
        };

        let schema = EnumParser::parse(&input).unwrap();

        if let SchemaKind::Enum(e) = &schema.kind {
            assert_eq!(e.variants[0].rust_name, "ActiveUser");
            assert_eq!(e.variants[0].schema_name, "activeUser");
            assert_eq!(e.variants[1].rust_name, "InactiveUser");
            assert_eq!(e.variants[1].schema_name, "inactiveUser");
        } else {
            panic!("Expected Enum kind");
        }
    }

    #[test]
    fn test_parse_unit_enum_with_variant_rename() {
        let input: DeriveInput = parse_quote! {
            enum Status {
                #[zod(rename = "ACTIVE")]
                Active,
                Inactive,
            }
        };

        let schema = EnumParser::parse(&input).unwrap();

        if let SchemaKind::Enum(e) = &schema.kind {
            assert_eq!(e.variants[0].rust_name, "Active");
            assert_eq!(e.variants[0].schema_name, "ACTIVE");
            assert_eq!(e.variants[1].rust_name, "Inactive");
            assert_eq!(e.variants[1].schema_name, "Inactive");
        } else {
            panic!("Expected Enum kind");
        }
    }

    #[test]
    fn test_parse_unit_enum_with_skip() {
        let input: DeriveInput = parse_quote! {
            enum Status {
                Active,
                #[zod(skip)]
                Internal,
                Inactive,
            }
        };

        let schema = EnumParser::parse(&input).unwrap();

        if let SchemaKind::Enum(e) = &schema.kind {
            assert_eq!(e.variants.len(), 2);
            assert_eq!(e.variants[0].rust_name, "Active");
            assert_eq!(e.variants[1].rust_name, "Inactive");
        } else {
            panic!("Expected Enum kind");
        }
    }

    #[test]
    fn test_parse_tuple_variant_enum() {
        let input: DeriveInput = parse_quote! {
            enum Message {
                Text(String),
                Number(i32),
                Pair(String, i32),
            }
        };

        let schema = EnumParser::parse(&input).unwrap();

        if let SchemaKind::Enum(e) = &schema.kind {
            assert!(!e.is_unit_only);
            assert_eq!(e.variants.len(), 3);

            // Check Text variant
            if let VariantKind::Tuple(fields) = &e.variants[0].kind {
                assert_eq!(fields.len(), 1);
            } else {
                panic!("Expected Tuple variant for Text");
            }

            // Check Number variant
            if let VariantKind::Tuple(fields) = &e.variants[1].kind {
                assert_eq!(fields.len(), 1);
            } else {
                panic!("Expected Tuple variant for Number");
            }

            // Check Pair variant
            if let VariantKind::Tuple(fields) = &e.variants[2].kind {
                assert_eq!(fields.len(), 2);
            } else {
                panic!("Expected Tuple variant for Pair");
            }
        } else {
            panic!("Expected Enum kind");
        }
    }

    #[test]
    fn test_parse_struct_variant_enum() {
        let input: DeriveInput = parse_quote! {
            enum Event {
                Click { x: i32, y: i32 },
                KeyPress { key: String },
            }
        };

        let schema = EnumParser::parse(&input).unwrap();

        if let SchemaKind::Enum(e) = &schema.kind {
            assert!(!e.is_unit_only);
            assert_eq!(e.variants.len(), 2);

            // Check Click variant
            if let VariantKind::Struct(fields) = &e.variants[0].kind {
                assert_eq!(fields.len(), 2);
                assert_eq!(fields[0].rust_name, "x");
                assert_eq!(fields[1].rust_name, "y");
            } else {
                panic!("Expected Struct variant for Click");
            }

            // Check KeyPress variant
            if let VariantKind::Struct(fields) = &e.variants[1].kind {
                assert_eq!(fields.len(), 1);
                assert_eq!(fields[0].rust_name, "key");
            } else {
                panic!("Expected Struct variant for KeyPress");
            }
        } else {
            panic!("Expected Enum kind");
        }
    }

    #[test]
    fn test_parse_mixed_variant_enum() {
        let input: DeriveInput = parse_quote! {
            enum Value {
                Null,
                Bool(bool),
                Object { name: String, value: i32 },
            }
        };

        let schema = EnumParser::parse(&input).unwrap();

        if let SchemaKind::Enum(e) = &schema.kind {
            assert!(!e.is_unit_only);
            assert_eq!(e.variants.len(), 3);

            assert!(matches!(e.variants[0].kind, VariantKind::Unit));
            assert!(matches!(e.variants[1].kind, VariantKind::Tuple(_)));
            assert!(matches!(e.variants[2].kind, VariantKind::Struct(_)));
        } else {
            panic!("Expected Enum kind");
        }
    }

    #[test]
    fn test_parse_enum_with_internal_tag() {
        let input: DeriveInput = parse_quote! {
            #[zod(tag = "type")]
            enum Message {
                Text { content: String },
                Image { url: String },
            }
        };

        let schema = EnumParser::parse(&input).unwrap();

        if let SchemaKind::Enum(e) = &schema.kind {
            assert!(matches!(
                e.tagging,
                EnumTagging::Internal { ref tag } if tag == "type"
            ));
        } else {
            panic!("Expected Enum kind");
        }
    }

    #[test]
    fn test_parse_enum_with_adjacent_tag() {
        let input: DeriveInput = parse_quote! {
            #[zod(tag = "t", content = "c")]
            enum Message {
                Text(String),
                Number(i32),
            }
        };

        let schema = EnumParser::parse(&input).unwrap();

        if let SchemaKind::Enum(e) = &schema.kind {
            assert!(matches!(
                e.tagging,
                EnumTagging::Adjacent { ref tag, ref content } if tag == "t" && content == "c"
            ));
        } else {
            panic!("Expected Enum kind");
        }
    }

    #[test]
    fn test_parse_enum_external_tag_default() {
        let input: DeriveInput = parse_quote! {
            enum Message {
                Text(String),
                Number(i32),
            }
        };

        let schema = EnumParser::parse(&input).unwrap();

        if let SchemaKind::Enum(e) = &schema.kind {
            assert!(matches!(e.tagging, EnumTagging::External));
        } else {
            panic!("Expected Enum kind");
        }
    }

    #[test]
    fn test_parse_enum_with_doc_comments() {
        let input: DeriveInput = parse_quote! {
            /// A status enum.
            enum Status {
                /// The active state.
                Active,
                /// The inactive state.
                Inactive,
            }
        };

        let schema = EnumParser::parse(&input).unwrap();
        assert!(schema.metadata.description.is_some());
        let desc = schema.metadata.description.as_ref().unwrap();
        assert!(desc.contains("status enum"));

        if let SchemaKind::Enum(e) = &schema.kind {
            assert!(e.variants[0].metadata.description.is_some());
            let variant_desc = e.variants[0].metadata.description.as_ref().unwrap();
            assert!(variant_desc.contains("active state"));
        } else {
            panic!("Expected Enum kind");
        }
    }

    #[test]
    fn test_parse_enum_with_generics() {
        let input: DeriveInput = parse_quote! {
            enum Result<T, E> {
                Ok(T),
                Err(E),
            }
        };

        let schema = EnumParser::parse(&input).unwrap();
        assert_eq!(schema.generics.len(), 2);
        assert_eq!(schema.generics[0].name, "T");
        assert_eq!(schema.generics[1].name, "E");
    }

    #[test]
    fn test_parse_enum_with_deprecated() {
        let input: DeriveInput = parse_quote! {
            #[zod(deprecated)]
            enum OldStatus {
                Active,
                Inactive,
            }
        };

        let schema = EnumParser::parse(&input).unwrap();
        assert!(schema.metadata.deprecated);
    }

    #[test]
    fn test_parse_enum_with_export_false() {
        let input: DeriveInput = parse_quote! {
            #[zod(export = false)]
            enum Internal {
                A,
                B,
            }
        };

        let schema = EnumParser::parse(&input).unwrap();
        assert!(!schema.export);
    }

    #[test]
    fn test_parse_not_an_enum() {
        let input: DeriveInput = parse_quote! {
            struct User {
                id: i64,
            }
        };

        let result = EnumParser::parse(&input);
        assert!(matches!(result, Err(EnumParseError::NotAnEnum(_))));
    }

    #[test]
    fn test_is_unit_only_true() {
        let input: DeriveInput = parse_quote! {
            enum Status {
                Active,
                Inactive,
            }
        };

        assert!(EnumParser::is_unit_only(&input));
    }

    #[test]
    fn test_is_unit_only_false() {
        let input: DeriveInput = parse_quote! {
            enum Message {
                Text(String),
                Empty,
            }
        };

        assert!(!EnumParser::is_unit_only(&input));
    }

    #[test]
    fn test_is_unit_only_not_enum() {
        let input: DeriveInput = parse_quote! {
            struct User {
                id: i64,
            }
        };

        assert!(!EnumParser::is_unit_only(&input));
    }
}

#[cfg(test)]
mod proptest_tests {
    use super::*;
    use crate::ir::VariantKind;
    use proptest::prelude::*;

    /// Strategy for generating valid Rust identifier names for enum variants.
    fn arb_variant_name() -> impl Strategy<Value = String> {
        // Use a predefined list of safe PascalCase identifiers for speed
        prop::sample::select(vec![
            "Active",
            "Inactive",
            "Pending",
            "Completed",
            "Failed",
            "Running",
            "Stopped",
            "Paused",
            "Ready",
            "Waiting",
            "Success",
            "Error",
            "Warning",
            "Info",
            "Debug",
            "Critical",
            "Normal",
            "High",
            "Low",
            "Medium",
            "Alpha",
            "Beta",
            "Gamma",
            "Delta",
            "Epsilon",
        ])
        .prop_map(|s| s.to_string())
    }

    /// Strategy for generating a list of unique variant names.
    fn arb_variant_names(min: usize, max: usize) -> impl Strategy<Value = Vec<String>> {
        proptest::collection::hash_set(arb_variant_name(), min..=max)
            .prop_map(|set| set.into_iter().collect())
    }

    // Configure proptest to run fewer cases for faster tests
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(32))]

        /// **Property 9: Unit Enum Generation**
        ///
        /// *For any* enum with only unit variants, the generated schema SHALL be
        /// a valid `z.enum([...])` containing all variant names.
        ///
        /// This property verifies that:
        /// 1. Parsing a unit-only enum produces an EnumSchema with is_unit_only = true
        /// 2. All variant names are preserved in the schema
        /// 3. All variants are of VariantKind::Unit
        ///
        /// **Validates: Requirements 7.1**
        #[test]
        fn prop_unit_enum_generation(variant_names in arb_variant_names(2, 8)) {
            // Build a unit enum with the generated variant names
            let variants_tokens: Vec<proc_macro2::TokenStream> = variant_names
                .iter()
                .map(|name| {
                    let ident = syn::Ident::new(name, proc_macro2::Span::call_site());
                    quote::quote! { #ident }
                })
                .collect();

            let input: DeriveInput = syn::parse2(quote::quote! {
                enum TestEnum {
                    #(#variants_tokens),*
                }
            }).expect("Should parse generated enum");

            let schema = EnumParser::parse(&input)
                .expect("Should parse enum successfully");

            // Verify it's an enum schema
            if let SchemaKind::Enum(e) = &schema.kind {
                // Property: is_unit_only should be true for unit-only enums
                prop_assert!(
                    e.is_unit_only,
                    "Unit-only enum should have is_unit_only = true"
                );

                // Property: variant count should match
                prop_assert_eq!(
                    e.variants.len(),
                    variant_names.len(),
                    "Variant count should match"
                );

                // Property: all variants should be Unit kind
                for variant in &e.variants {
                    prop_assert!(
                        matches!(variant.kind, VariantKind::Unit),
                        "All variants should be Unit kind, got {:?}",
                        variant.kind
                    );
                }

                // Property: all variant names should be present
                let schema_names: std::collections::HashSet<_> = e.variants
                    .iter()
                    .map(|v| v.rust_name.clone())
                    .collect();
                let input_names: std::collections::HashSet<_> = variant_names
                    .iter()
                    .cloned()
                    .collect();
                prop_assert_eq!(
                    schema_names,
                    input_names,
                    "All variant names should be preserved"
                );
            } else {
                prop_assert!(false, "Expected Enum kind");
            }
        }

        /// **Property 10: Data Enum Discriminated Union**
        ///
        /// *For any* enum with data variants and internal tagging, the generated
        /// schema SHALL be a valid `z.discriminatedUnion()` with the correct tag field.
        ///
        /// This property verifies that:
        /// 1. Parsing an enum with data variants produces is_unit_only = false
        /// 2. Internal tagging is correctly detected from #[zod(tag = "...")]
        /// 3. All variants are preserved with their correct kinds
        ///
        /// **Validates: Requirements 7.2, 7.3**
        #[test]
        fn prop_data_enum_discriminated_union(
            variant_names in arb_variant_names(2, 6),
            tag_name in "[a-z]{1,10}"
        ) {
            // Build a data enum with struct variants and internal tagging
            let variants_tokens: Vec<proc_macro2::TokenStream> = variant_names
                .iter()
                .map(|name| {
                    let ident = syn::Ident::new(name, proc_macro2::Span::call_site());
                    quote::quote! { #ident { value: String } }
                })
                .collect();

            let tag_ident = syn::LitStr::new(&tag_name, proc_macro2::Span::call_site());

            let input: DeriveInput = syn::parse2(quote::quote! {
                #[zod(tag = #tag_ident)]
                enum TestEnum {
                    #(#variants_tokens),*
                }
            }).expect("Should parse generated enum");

            let schema = EnumParser::parse(&input)
                .expect("Should parse enum successfully");

            // Verify it's an enum schema
            if let SchemaKind::Enum(e) = &schema.kind {
                // Property: is_unit_only should be false for data enums
                prop_assert!(
                    !e.is_unit_only,
                    "Data enum should have is_unit_only = false"
                );

                // Property: tagging should be Internal with correct tag
                match &e.tagging {
                    EnumTagging::Internal { tag } => {
                        prop_assert_eq!(
                            tag,
                            &tag_name,
                            "Tag name should match"
                        );
                    }
                    other => {
                        prop_assert!(
                            false,
                            "Expected Internal tagging, got {:?}",
                            other
                        );
                    }
                }

                // Property: variant count should match
                prop_assert_eq!(
                    e.variants.len(),
                    variant_names.len(),
                    "Variant count should match"
                );

                // Property: all variants should be Struct kind (since we created struct variants)
                for variant in &e.variants {
                    prop_assert!(
                        matches!(variant.kind, VariantKind::Struct(_)),
                        "All variants should be Struct kind, got {:?}",
                        variant.kind
                    );
                }

                // Property: all variant names should be present
                let schema_names: std::collections::HashSet<_> = e.variants
                    .iter()
                    .map(|v| v.rust_name.clone())
                    .collect();
                let input_names: std::collections::HashSet<_> = variant_names
                    .iter()
                    .cloned()
                    .collect();
                prop_assert_eq!(
                    schema_names,
                    input_names,
                    "All variant names should be preserved"
                );
            } else {
                prop_assert!(false, "Expected Enum kind");
            }
        }

        /// Property: Parser determinism for enums.
        ///
        /// *For any* valid enum definition, parsing it multiple times SHALL
        /// produce the same IR output.
        #[test]
        fn prop_enum_parser_determinism(variant_names in arb_variant_names(1, 5)) {
            // Build a unit enum with the generated variant names
            let variants_tokens: Vec<proc_macro2::TokenStream> = variant_names
                .iter()
                .map(|name| {
                    let ident = syn::Ident::new(name, proc_macro2::Span::call_site());
                    quote::quote! { #ident }
                })
                .collect();

            let input: DeriveInput = syn::parse2(quote::quote! {
                enum TestEnum {
                    #(#variants_tokens),*
                }
            }).expect("Should parse generated enum");

            // Parse the same input multiple times
            let result1 = EnumParser::parse(&input).expect("First parse should succeed");
            let result2 = EnumParser::parse(&input).expect("Second parse should succeed");

            // Results should be equal
            prop_assert_eq!(&result1.name, &result2.name);

            if let (SchemaKind::Enum(e1), SchemaKind::Enum(e2)) = (&result1.kind, &result2.kind) {
                prop_assert_eq!(e1.is_unit_only, e2.is_unit_only);
                prop_assert_eq!(e1.variants.len(), e2.variants.len());
                for i in 0..e1.variants.len() {
                    prop_assert_eq!(&e1.variants[i].rust_name, &e2.variants[i].rust_name);
                    prop_assert_eq!(&e1.variants[i].schema_name, &e2.variants[i].schema_name);
                }
            } else {
                prop_assert!(false, "Expected Enum kind");
            }
        }
    }
}
