//! Struct parsing logic.
//!
//! This module handles parsing Rust struct definitions into IR.
//! It supports:
//! - Named structs with fields
//! - Tuple structs
//! - Unit structs
//! - Doc comment extraction
//! - Container and field attributes

use darling::FromDeriveInput;
use syn::{Attribute, Data, DeriveInput, Fields, Lit, Meta};

use crate::ir::{
    FieldIR, FieldMetadata, GenericParam, SchemaIR, SchemaKind, SchemaMetadata, StructSchema,
    TupleStructSchema, TypeIR,
};
use crate::parser::attributes::{ContainerAttrs, FieldAttrs, RenameRule};
use crate::parser::type_parser::{ParseError, TypeParser};

#[cfg(feature = "serde-compat")]
use crate::parser::serde_compat::{SerdeContainerAttrs, SerdeFieldAttrs};

/// Error type for struct parsing failures.
#[derive(Debug, Clone, thiserror::Error)]
pub enum StructParseError {
    #[error("Failed to parse container attributes: {0}")]
    ContainerAttrs(String),

    #[error("Failed to parse field attributes: {0}")]
    FieldAttrs(String),

    #[error("Failed to parse field type: {0}")]
    FieldType(#[from] ParseError),

    #[error("Expected struct, found {0}")]
    NotAStruct(String),

    #[error("Field '{0}' has no identifier")]
    MissingFieldIdent(usize),
}

/// Parses Rust struct definitions into SchemaIR.
pub struct StructParser;

impl StructParser {
    /// Parse a DeriveInput into a SchemaIR.
    ///
    /// This is the main entry point for struct parsing.
    pub fn parse(input: &DeriveInput) -> Result<SchemaIR, StructParseError> {
        // Parse container attributes using darling
        let container_attrs = ContainerAttrs::from_derive_input(input)
            .map_err(|e| StructParseError::ContainerAttrs(e.to_string()))?;

        // Parse serde container attributes if feature is enabled
        #[cfg(feature = "serde-compat")]
        let serde_container_attrs = SerdeContainerAttrs::from_attrs(&input.attrs);

        // Merge rename_all: zod takes precedence over serde
        #[cfg(feature = "serde-compat")]
        let effective_rename_all = container_attrs
            .rename_all
            .or(serde_container_attrs.rename_all);

        #[cfg(not(feature = "serde-compat"))]
        let effective_rename_all = container_attrs.rename_all;

        // Ensure we have a struct
        let data_struct = match &input.data {
            Data::Struct(s) => s,
            Data::Enum(_) => return Err(StructParseError::NotAStruct("enum".to_string())),
            Data::Union(_) => return Err(StructParseError::NotAStruct("union".to_string())),
        };

        // Parse based on struct kind
        let kind = match &data_struct.fields {
            Fields::Named(fields) => {
                let field_irs = Self::parse_named_fields(fields, effective_rename_all)?;
                SchemaKind::Struct(StructSchema::new(field_irs).with_strict(container_attrs.strict))
            }
            Fields::Unnamed(fields) => {
                let field_types = Self::parse_unnamed_fields(fields)?;
                SchemaKind::TupleStruct(TupleStructSchema::new(field_types))
            }
            Fields::Unit => SchemaKind::UnitStruct,
        };

        // Extract doc comments from the struct
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

        let schema = SchemaIR::new(&rust_name, kind)
            .with_name(schema_name)
            .with_generics(generics)
            .with_metadata(metadata)
            .with_export(container_attrs.should_export());

        Ok(schema)
    }

    /// Parse named struct fields into FieldIR.
    fn parse_named_fields(
        fields: &syn::FieldsNamed,
        rename_all: Option<RenameRule>,
    ) -> Result<Vec<FieldIR>, StructParseError> {
        let mut field_irs = Vec::with_capacity(fields.named.len());

        for (index, field) in fields.named.iter().enumerate() {
            // Parse field attributes using darling
            let field_attrs = FieldAttrs::from_field(field)
                .map_err(|e| StructParseError::FieldAttrs(e.to_string()))?;

            // Parse serde field attributes if feature is enabled
            #[cfg(feature = "serde-compat")]
            let serde_field_attrs = SerdeFieldAttrs::from_attrs(&field.attrs);

            // Merge skip: zod takes precedence, but serde skip also applies
            #[cfg(feature = "serde-compat")]
            let should_skip = field_attrs.skip || serde_field_attrs.should_skip();

            #[cfg(not(feature = "serde-compat"))]
            let should_skip = field_attrs.skip;

            // Skip fields marked with #[zod(skip)] or #[serde(skip)]
            if should_skip {
                continue;
            }

            // Get field name
            let rust_name = field
                .ident
                .as_ref()
                .ok_or(StructParseError::MissingFieldIdent(index))?
                .to_string();

            // Get schema name (apply rename rules)
            // Priority: zod rename > serde rename > rename_all rule > original name
            #[cfg(feature = "serde-compat")]
            let schema_name = if field_attrs.rename.is_some() {
                field_attrs.schema_name(rename_all)
            } else if let Some(ref serde_rename) = serde_field_attrs.rename {
                serde_rename.clone()
            } else {
                field_attrs.schema_name(rename_all)
            };

            #[cfg(not(feature = "serde-compat"))]
            let schema_name = field_attrs.schema_name(rename_all);

            // Parse field type
            let ty = TypeParser::parse(&field.ty)?;

            // Extract doc comments from field
            let field_description = extract_doc_comments(&field.attrs);

            // Build field metadata
            let mut field_metadata = FieldMetadata::default();
            if let Some(desc) = field_description {
                field_metadata.description = Some(desc);
            }
            // Attribute description overrides doc comments
            if let Some(attr_desc) = &field_attrs.description {
                field_metadata.description = Some(attr_desc.clone());
            }

            // Get validation rules from attributes
            let validation = field_attrs.to_validation_rules();

            // Merge flatten: zod takes precedence, but serde flatten also applies
            #[cfg(feature = "serde-compat")]
            let is_flatten = field_attrs.flatten || serde_field_attrs.flatten;

            #[cfg(not(feature = "serde-compat"))]
            let is_flatten = field_attrs.flatten;

            // Merge optional: zod takes precedence, but serde default makes field optional
            #[cfg(feature = "serde-compat")]
            let is_optional = field_attrs.optional || serde_field_attrs.default;

            #[cfg(not(feature = "serde-compat"))]
            let is_optional = field_attrs.optional;

            // Build the field IR
            let field_ir = FieldIR::new(&rust_name, ty)
                .with_schema_name(schema_name)
                .with_optional(is_optional)
                .with_nullable(field_attrs.nullable)
                .with_flatten(is_flatten)
                .with_validation(validation)
                .with_metadata(field_metadata);

            // Add default if present
            let field_ir = if let Some(default) = &field_attrs.default {
                field_ir.with_default(default.clone())
            } else {
                field_ir
            };

            field_irs.push(field_ir);
        }

        Ok(field_irs)
    }

    /// Parse unnamed (tuple) struct fields into TypeIR.
    fn parse_unnamed_fields(fields: &syn::FieldsUnnamed) -> Result<Vec<TypeIR>, StructParseError> {
        let mut type_irs = Vec::with_capacity(fields.unnamed.len());

        for field in &fields.unnamed {
            let ty = TypeParser::parse(&field.ty)?;
            type_irs.push(ty);
        }

        Ok(type_irs)
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
}

/// Extract doc comments from attributes.
///
/// Doc comments in Rust are represented as `#[doc = "..."]` attributes.
/// This function extracts and concatenates them into a single description string.
pub fn extract_doc_comments(attrs: &[Attribute]) -> Option<String> {
    let doc_lines: Vec<String> = attrs
        .iter()
        .filter_map(|attr| {
            if !attr.path().is_ident("doc") {
                return None;
            }

            // Parse the doc attribute value
            if let Meta::NameValue(meta) = &attr.meta {
                if let syn::Expr::Lit(expr_lit) = &meta.value {
                    if let Lit::Str(lit_str) = &expr_lit.lit {
                        return Some(lit_str.value());
                    }
                }
            }
            None
        })
        .collect();

    if doc_lines.is_empty() {
        return None;
    }

    // Join doc lines, trimming leading whitespace from each line
    let description = doc_lines
        .iter()
        .map(|line| line.trim())
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string();

    if description.is_empty() {
        None
    } else {
        Some(description)
    }
}

use darling::FromField;

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_parse_simple_struct() {
        let input: DeriveInput = parse_quote! {
            struct User {
                id: i64,
                name: String,
            }
        };

        let schema = StructParser::parse(&input).unwrap();
        assert_eq!(schema.name, "User");
        assert_eq!(schema.rust_name, "User");

        if let SchemaKind::Struct(s) = &schema.kind {
            assert_eq!(s.fields.len(), 2);
            assert_eq!(s.fields[0].rust_name, "id");
            assert_eq!(s.fields[0].schema_name, "id");
            assert_eq!(s.fields[1].rust_name, "name");
            assert_eq!(s.fields[1].schema_name, "name");
        } else {
            panic!("Expected Struct kind");
        }
    }

    #[test]
    fn test_parse_struct_with_rename() {
        let input: DeriveInput = parse_quote! {
            #[zod(rename = "UserDTO")]
            struct User {
                id: i64,
            }
        };

        let schema = StructParser::parse(&input).unwrap();
        assert_eq!(schema.name, "UserDTO");
        assert_eq!(schema.rust_name, "User");
    }

    #[test]
    fn test_parse_struct_with_rename_all() {
        let input: DeriveInput = parse_quote! {
            #[zod(rename_all = "camelCase")]
            struct User {
                user_name: String,
                email_address: String,
            }
        };

        let schema = StructParser::parse(&input).unwrap();

        if let SchemaKind::Struct(s) = &schema.kind {
            assert_eq!(s.fields[0].rust_name, "user_name");
            assert_eq!(s.fields[0].schema_name, "userName");
            assert_eq!(s.fields[1].rust_name, "email_address");
            assert_eq!(s.fields[1].schema_name, "emailAddress");
        } else {
            panic!("Expected Struct kind");
        }
    }

    #[test]
    fn test_parse_struct_with_field_rename() {
        let input: DeriveInput = parse_quote! {
            struct User {
                #[zod(rename = "userId")]
                id: i64,
            }
        };

        let schema = StructParser::parse(&input).unwrap();

        if let SchemaKind::Struct(s) = &schema.kind {
            assert_eq!(s.fields[0].rust_name, "id");
            assert_eq!(s.fields[0].schema_name, "userId");
        } else {
            panic!("Expected Struct kind");
        }
    }

    #[test]
    fn test_parse_struct_with_skip() {
        let input: DeriveInput = parse_quote! {
            struct User {
                id: i64,
                #[zod(skip)]
                internal: String,
                name: String,
            }
        };

        let schema = StructParser::parse(&input).unwrap();

        if let SchemaKind::Struct(s) = &schema.kind {
            assert_eq!(s.fields.len(), 2);
            assert_eq!(s.fields[0].rust_name, "id");
            assert_eq!(s.fields[1].rust_name, "name");
        } else {
            panic!("Expected Struct kind");
        }
    }

    #[test]
    fn test_parse_struct_with_optional_nullable() {
        let input: DeriveInput = parse_quote! {
            struct User {
                #[zod(optional)]
                nickname: String,
                #[zod(nullable)]
                avatar: String,
            }
        };

        let schema = StructParser::parse(&input).unwrap();

        if let SchemaKind::Struct(s) = &schema.kind {
            assert!(s.fields[0].optional);
            assert!(!s.fields[0].nullable);
            assert!(!s.fields[1].optional);
            assert!(s.fields[1].nullable);
        } else {
            panic!("Expected Struct kind");
        }
    }

    #[test]
    fn test_parse_struct_with_validation() {
        let input: DeriveInput = parse_quote! {
            struct User {
                #[zod(email)]
                email: String,
                #[zod(min = 0.0, max = 150.0)]
                age: i32,
            }
        };

        let schema = StructParser::parse(&input).unwrap();

        if let SchemaKind::Struct(s) = &schema.kind {
            assert!(!s.fields[0].validation.is_empty());
            assert!(!s.fields[1].validation.is_empty());
        } else {
            panic!("Expected Struct kind");
        }
    }

    #[test]
    fn test_parse_struct_with_strict() {
        let input: DeriveInput = parse_quote! {
            #[zod(strict)]
            struct User {
                id: i64,
            }
        };

        let schema = StructParser::parse(&input).unwrap();

        if let SchemaKind::Struct(s) = &schema.kind {
            assert!(s.strict);
        } else {
            panic!("Expected Struct kind");
        }
    }

    #[test]
    fn test_parse_tuple_struct() {
        let input: DeriveInput = parse_quote! {
            struct Point(f64, f64);
        };

        let schema = StructParser::parse(&input).unwrap();
        assert_eq!(schema.name, "Point");

        if let SchemaKind::TupleStruct(ts) = &schema.kind {
            assert_eq!(ts.fields.len(), 2);
        } else {
            panic!("Expected TupleStruct kind");
        }
    }

    #[test]
    fn test_parse_unit_struct() {
        let input: DeriveInput = parse_quote! {
            struct Empty;
        };

        let schema = StructParser::parse(&input).unwrap();
        assert_eq!(schema.name, "Empty");
        assert!(matches!(schema.kind, SchemaKind::UnitStruct));
    }

    #[test]
    fn test_parse_struct_with_doc_comments() {
        let input: DeriveInput = parse_quote! {
            /// A user in the system.
            /// This is a multi-line doc comment.
            struct User {
                /// The user's unique identifier.
                id: i64,
            }
        };

        let schema = StructParser::parse(&input).unwrap();
        assert!(schema.metadata.description.is_some());
        let desc = schema.metadata.description.as_ref().unwrap();
        assert!(desc.contains("A user in the system"));

        if let SchemaKind::Struct(s) = &schema.kind {
            assert!(s.fields[0].metadata.description.is_some());
            let field_desc = s.fields[0].metadata.description.as_ref().unwrap();
            assert!(field_desc.contains("unique identifier"));
        } else {
            panic!("Expected Struct kind");
        }
    }

    #[test]
    fn test_parse_struct_with_generics() {
        let input: DeriveInput = parse_quote! {
            struct Response<T> {
                data: T,
                status: i32,
            }
        };

        let schema = StructParser::parse(&input).unwrap();
        assert_eq!(schema.generics.len(), 1);
        assert_eq!(schema.generics[0].name, "T");
    }

    #[test]
    fn test_parse_struct_with_generic_bounds() {
        let input: DeriveInput = parse_quote! {
            struct Container<T: Clone + Send> {
                value: T,
            }
        };

        let schema = StructParser::parse(&input).unwrap();
        assert_eq!(schema.generics.len(), 1);
        assert_eq!(schema.generics[0].name, "T");
        assert!(!schema.generics[0].bounds.is_empty());
    }

    #[test]
    fn test_parse_struct_preserves_field_order() {
        let input: DeriveInput = parse_quote! {
            struct User {
                z_field: String,
                a_field: String,
                m_field: String,
            }
        };

        let schema = StructParser::parse(&input).unwrap();

        if let SchemaKind::Struct(s) = &schema.kind {
            assert_eq!(s.fields[0].rust_name, "z_field");
            assert_eq!(s.fields[1].rust_name, "a_field");
            assert_eq!(s.fields[2].rust_name, "m_field");
        } else {
            panic!("Expected Struct kind");
        }
    }

    #[test]
    fn test_parse_struct_with_flatten() {
        let input: DeriveInput = parse_quote! {
            struct User {
                id: i64,
                #[zod(flatten)]
                metadata: Metadata,
            }
        };

        let schema = StructParser::parse(&input).unwrap();

        if let SchemaKind::Struct(s) = &schema.kind {
            assert!(!s.fields[0].flatten);
            assert!(s.fields[1].flatten);
        } else {
            panic!("Expected Struct kind");
        }
    }

    #[test]
    fn test_parse_struct_with_default() {
        let input: DeriveInput = parse_quote! {
            struct Config {
                #[zod(default = "\"default_value\"")]
                name: String,
            }
        };

        let schema = StructParser::parse(&input).unwrap();

        if let SchemaKind::Struct(s) = &schema.kind {
            assert_eq!(s.fields[0].default, Some("\"default_value\"".to_string()));
        } else {
            panic!("Expected Struct kind");
        }
    }

    #[test]
    fn test_parse_struct_with_deprecated() {
        let input: DeriveInput = parse_quote! {
            #[zod(deprecated)]
            struct OldUser {
                id: i64,
            }
        };

        let schema = StructParser::parse(&input).unwrap();
        assert!(schema.metadata.deprecated);
    }

    #[test]
    fn test_parse_struct_with_export_false() {
        let input: DeriveInput = parse_quote! {
            #[zod(export = false)]
            struct Internal {
                id: i64,
            }
        };

        let schema = StructParser::parse(&input).unwrap();
        assert!(!schema.export);
    }

    #[test]
    fn test_extract_doc_comments() {
        let attrs: Vec<Attribute> = vec![
            parse_quote!(#[doc = " First line"]),
            parse_quote!(#[doc = " Second line"]),
        ];

        let description = extract_doc_comments(&attrs);
        assert!(description.is_some());
        let desc = description.unwrap();
        assert!(desc.contains("First line"));
        assert!(desc.contains("Second line"));
    }

    #[test]
    fn test_extract_doc_comments_empty() {
        let attrs: Vec<Attribute> = vec![];
        let description = extract_doc_comments(&attrs);
        assert!(description.is_none());
    }

    #[test]
    fn test_parse_not_a_struct() {
        let input: DeriveInput = parse_quote! {
            enum Status {
                Active,
                Inactive,
            }
        };

        let result = StructParser::parse(&input);
        assert!(matches!(result, Err(StructParseError::NotAStruct(_))));
    }
}

#[cfg(test)]
mod proptest_tests {
    use super::*;
    use proptest::prelude::*;

    /// Strategy for generating valid Rust identifier names.
    fn arb_identifier() -> impl Strategy<Value = String> {
        // Use a predefined list of safe identifiers for speed
        prop::sample::select(vec![
            "alpha",
            "beta",
            "gamma",
            "delta",
            "epsilon",
            "zeta",
            "eta",
            "theta",
            "field_a",
            "field_b",
            "field_c",
            "field_d",
            "field_e",
            "field_f",
            "name",
            "value",
            "data",
            "info",
            "item",
            "count",
            "index",
            "key",
            "user_id",
            "created_at",
            "updated_at",
            "is_active",
            "has_value",
        ])
        .prop_map(|s| s.to_string())
    }

    /// Strategy for generating a list of unique field names.
    fn arb_field_names(min: usize, max: usize) -> impl Strategy<Value = Vec<String>> {
        proptest::collection::hash_set(arb_identifier(), min..=max)
            .prop_map(|set| set.into_iter().collect())
    }

    // Configure proptest to run fewer cases for faster tests
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(32))]

        /// **Property 3: Field Order Preservation**
        ///
        /// *For any* struct with named fields, the Parser_Module SHALL preserve
        /// field ordering from the original Rust definition.
        ///
        /// **Validates: Requirements 2.2**
        #[test]
        fn prop_field_order_preservation(field_names in arb_field_names(2, 6)) {
            // Build a struct with the generated field names
            let fields_tokens: Vec<proc_macro2::TokenStream> = field_names
                .iter()
                .map(|name| {
                    let ident = syn::Ident::new(name, proc_macro2::Span::call_site());
                    quote::quote! { #ident: String }
                })
                .collect();

            let input: DeriveInput = syn::parse2(quote::quote! {
                struct TestStruct {
                    #(#fields_tokens),*
                }
            }).expect("Should parse generated struct");

            let schema = StructParser::parse(&input)
                .expect("Should parse struct successfully");

            if let SchemaKind::Struct(s) = &schema.kind {
                // Verify field count matches
                prop_assert_eq!(s.fields.len(), field_names.len());

                // Verify field order is preserved
                for (field, expected_name) in s.fields.iter().zip(field_names.iter()) {
                    prop_assert_eq!(&field.rust_name, expected_name);
                }
            } else {
                prop_assert!(false, "Expected Struct kind");
            }
        }

        /// **Property 1: Parser Determinism**
        ///
        /// *For any* valid struct definition, parsing it multiple times SHALL
        /// produce the same IR output.
        ///
        /// **Validates: Requirements 2.8**
        #[test]
        fn prop_parser_determinism(field_names in arb_field_names(1, 5)) {
            // Build a struct with the generated field names
            let fields_tokens: Vec<proc_macro2::TokenStream> = field_names
                .iter()
                .map(|name| {
                    let ident = syn::Ident::new(name, proc_macro2::Span::call_site());
                    quote::quote! { #ident: String }
                })
                .collect();

            let input: DeriveInput = syn::parse2(quote::quote! {
                struct TestStruct {
                    #(#fields_tokens),*
                }
            }).expect("Should parse generated struct");

            // Parse the same input multiple times
            let result1 = StructParser::parse(&input).expect("First parse should succeed");
            let result2 = StructParser::parse(&input).expect("Second parse should succeed");

            // Results should be equal
            prop_assert_eq!(&result1.name, &result2.name);

            if let (SchemaKind::Struct(s1), SchemaKind::Struct(s2)) = (&result1.kind, &result2.kind) {
                prop_assert_eq!(s1.fields.len(), s2.fields.len());
                for i in 0..s1.fields.len() {
                    prop_assert_eq!(&s1.fields[i].rust_name, &s2.fields[i].rust_name);
                    prop_assert_eq!(&s1.fields[i].schema_name, &s2.fields[i].schema_name);
                }
            } else {
                prop_assert!(false, "Expected Struct kind");
            }
        }
    }
}
