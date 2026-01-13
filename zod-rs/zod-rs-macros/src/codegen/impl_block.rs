//! Impl block generation for the ZodSchema trait.
//!
//! This module generates the `impl ZodSchema for Type` blocks that are
//! emitted by the derive macro.

use proc_macro2::TokenStream;
use quote::quote;

use crate::generator::traits::GeneratorConfig;
use crate::generator::zod::emitter::ZodEmitter;
use crate::ir::SchemaIR;

/// Generates the impl block for ZodSchema trait.
///
/// This struct takes a SchemaIR and generates the Rust code that implements
/// the ZodSchema trait for the type.
pub struct ImplBlockGenerator {
    /// The Zod emitter for generating schema strings.
    emitter: ZodEmitter,
    /// Configuration for code generation.
    #[allow(unused)]
    config: GeneratorConfig,
}

impl Default for ImplBlockGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl ImplBlockGenerator {
    /// Create a new ImplBlockGenerator with default settings.
    pub fn new() -> Self {
        Self {
            emitter: ZodEmitter::new(),
            config: GeneratorConfig::default(),
        }
    }

    /// Create a new ImplBlockGenerator with custom config.
    #[allow(unused)]
    pub fn with_config(config: GeneratorConfig) -> Self {
        Self {
            emitter: ZodEmitter::new(),
            config,
        }
    }

    /// Generate the complete impl block for a schema.
    ///
    /// This generates an `impl ZodSchema for TypeName` block with all
    /// required trait methods.
    pub fn generate(&self, schema: &SchemaIR) -> TokenStream {
        let name = syn::Ident::new(&schema.rust_name, proc_macro2::Span::call_site());
        let zod_schema = self.emitter.generate_schema_string(schema);
        let ts_type = &schema.name;
        let schema_name = format!("{}Schema", schema.name);

        // Generate metadata if present
        let metadata_impl = self.generate_metadata(schema);

        // Generate generics handling
        let (impl_generics, ty_generics, where_clause) = self.generate_generics(schema);

        quote! {
            impl #impl_generics ::zod_rs::ZodSchema for #name #ty_generics #where_clause {
                fn zod_schema() -> &'static str {
                    #zod_schema
                }

                fn ts_type_name() -> &'static str {
                    #ts_type
                }

                fn schema_name() -> &'static str {
                    #schema_name
                }

                #metadata_impl
            }
        }
    }

    /// Generate the metadata() method implementation.
    fn generate_metadata(&self, schema: &SchemaIR) -> TokenStream {
        let description = schema
            .metadata
            .description
            .as_ref()
            .map(|d| quote! { .with_description(#d) })
            .unwrap_or_default();

        let deprecated = if schema.metadata.deprecated {
            if let Some(msg) = &schema.metadata.deprecation_message {
                quote! { .with_deprecation_message(#msg) }
            } else {
                quote! { .with_deprecated(true) }
            }
        } else {
            quote! {}
        };

        let examples: Vec<_> = schema
            .metadata
            .examples
            .iter()
            .map(|e| quote! { .with_example(#e) })
            .collect();

        let tags: Vec<_> = schema
            .metadata
            .tags
            .iter()
            .map(|t| quote! { .with_tag(#t) })
            .collect();

        // Only generate custom metadata if there's something to add
        if description.is_empty() && deprecated.is_empty() && examples.is_empty() && tags.is_empty()
        {
            quote! {}
        } else {
            quote! {
                fn metadata() -> ::zod_rs::SchemaMetadata {
                    ::zod_rs::SchemaMetadata::new()
                        #description
                        #deprecated
                        #(#examples)*
                        #(#tags)*
                }
            }
        }
    }

    /// Generate generics handling for the impl block.
    fn generate_generics(&self, schema: &SchemaIR) -> (TokenStream, TokenStream, TokenStream) {
        if schema.generics.is_empty() {
            return (quote! {}, quote! {}, quote! {});
        }

        let generic_params: Vec<_> = schema
            .generics
            .iter()
            .map(|g| {
                let name = syn::Ident::new(&g.name, proc_macro2::Span::call_site());
                if g.bounds.is_empty() {
                    quote! { #name: ::zod_rs::ZodSchema }
                } else {
                    let bounds: Vec<_> = g
                        .bounds
                        .iter()
                        .map(|b| {
                            let bound: syn::Path = syn::parse_str(b)
                                .unwrap_or_else(|_| syn::parse_str("::zod_rs::ZodSchema").unwrap());
                            quote! { #bound }
                        })
                        .collect();
                    quote! { #name: #(#bounds)+* + ::zod_rs::ZodSchema }
                }
            })
            .collect();

        let generic_names: Vec<_> = schema
            .generics
            .iter()
            .map(|g| {
                let name = syn::Ident::new(&g.name, proc_macro2::Span::call_site());
                quote! { #name }
            })
            .collect();

        let impl_generics = quote! { <#(#generic_params),*> };
        let ty_generics = quote! { <#(#generic_names),*> };
        let where_clause = quote! {};

        (impl_generics, ty_generics, where_clause)
    }

    /// Generate a schema string for a given SchemaIR.
    ///
    /// This is a convenience method that delegates to the emitter.
    #[allow(unused)]
    pub fn generate_schema_string(&self, schema: &SchemaIR) -> String {
        self.emitter.generate_schema_string(schema)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{
        EnumSchema, FieldIR, GenericParam, SchemaKind, SchemaMetadata, StructSchema,
        TupleStructSchema, TypeIR, TypeKind, VariantIR,
    };

    /// Helper to create a simple struct SchemaIR for testing.
    fn create_simple_struct_schema(name: &str) -> SchemaIR {
        SchemaIR::new(
            name,
            SchemaKind::Struct(StructSchema::new(vec![
                FieldIR::new(
                    "id",
                    TypeIR::new(TypeKind::Integer {
                        signed: false,
                        bits: Some(32),
                    }),
                ),
                FieldIR::new("name", TypeIR::new(TypeKind::String)),
            ])),
        )
    }

    /// Helper to create a struct with metadata.
    fn create_struct_with_metadata(name: &str, description: &str) -> SchemaIR {
        SchemaIR::new(
            name,
            SchemaKind::Struct(StructSchema::new(vec![FieldIR::new(
                "value",
                TypeIR::new(TypeKind::String),
            )])),
        )
        .with_metadata(SchemaMetadata::with_description(description))
    }

    /// Helper to create a struct with deprecated metadata.
    fn create_deprecated_struct(name: &str, message: Option<&str>) -> SchemaIR {
        let metadata = if let Some(msg) = message {
            SchemaMetadata::default().deprecated_with_message(msg)
        } else {
            SchemaMetadata::default().as_deprecated()
        };
        SchemaIR::new(
            name,
            SchemaKind::Struct(StructSchema::new(vec![FieldIR::new(
                "value",
                TypeIR::new(TypeKind::String),
            )])),
        )
        .with_metadata(metadata)
    }

    /// Helper to create a struct with generics.
    fn create_generic_struct(name: &str, generic_names: &[&str]) -> SchemaIR {
        let generics: Vec<GenericParam> = generic_names
            .iter()
            .map(|n| GenericParam::new(*n))
            .collect();
        SchemaIR::new(
            name,
            SchemaKind::Struct(StructSchema::new(vec![FieldIR::new(
                "value",
                TypeIR::new(TypeKind::String),
            )])),
        )
        .with_generics(generics)
    }

    #[test]
    fn test_impl_block_generator_new() {
        let generator = ImplBlockGenerator::new();
        // Just verify it creates without panic
        let _ = generator;
    }

    #[test]
    fn test_impl_block_generator_default() {
        let generator = ImplBlockGenerator::default();
        let _ = generator;
    }

    #[test]
    fn test_impl_block_generator_with_config() {
        let config = GeneratorConfig::default();
        let generator = ImplBlockGenerator::with_config(config);
        let _ = generator;
    }

    #[test]
    fn test_generate_simple_struct() {
        let generator = ImplBlockGenerator::new();
        let schema = create_simple_struct_schema("User");
        let tokens = generator.generate(&schema);
        let code = tokens.to_string();

        // Verify impl block structure
        assert!(code.contains("impl"));
        assert!(code.contains("ZodSchema"));
        assert!(code.contains("User"));
        assert!(code.contains("zod_schema"));
        assert!(code.contains("ts_type_name"));
        assert!(code.contains("schema_name"));
    }

    #[test]
    fn test_generate_zod_schema_method() {
        let generator = ImplBlockGenerator::new();
        let schema = create_simple_struct_schema("User");
        let tokens = generator.generate(&schema);
        let code = tokens.to_string();

        // Should have zod_schema method returning static str
        assert!(code.contains("fn zod_schema"));
        assert!(code.contains("& 'static str"));
    }

    #[test]
    fn test_generate_ts_type_name_method() {
        let generator = ImplBlockGenerator::new();
        let schema = create_simple_struct_schema("User");
        let tokens = generator.generate(&schema);
        let code = tokens.to_string();

        // Should have ts_type_name method
        assert!(code.contains("fn ts_type_name"));
        assert!(code.contains("\"User\""));
    }

    #[test]
    fn test_generate_schema_name_method() {
        let generator = ImplBlockGenerator::new();
        let schema = create_simple_struct_schema("User");
        let tokens = generator.generate(&schema);
        let code = tokens.to_string();

        // Should have schema_name method returning "UserSchema"
        assert!(code.contains("fn schema_name"));
        assert!(code.contains("\"UserSchema\""));
    }

    #[test]
    fn test_generate_with_description_metadata() {
        let generator = ImplBlockGenerator::new();
        let schema = create_struct_with_metadata("User", "A user in the system");
        let tokens = generator.generate(&schema);
        let code = tokens.to_string();

        // Should have metadata method with description
        assert!(code.contains("fn metadata"));
        assert!(code.contains("SchemaMetadata"));
        assert!(code.contains("with_description"));
        assert!(code.contains("A user in the system"));
    }

    #[test]
    fn test_generate_with_deprecated_metadata() {
        let generator = ImplBlockGenerator::new();
        let schema = create_deprecated_struct("OldUser", None);
        let tokens = generator.generate(&schema);
        let code = tokens.to_string();

        // Should have metadata method with deprecated
        assert!(code.contains("fn metadata"));
        assert!(code.contains("with_deprecated"));
    }

    #[test]
    fn test_generate_with_deprecation_message() {
        let generator = ImplBlockGenerator::new();
        let schema = create_deprecated_struct("OldUser", Some("Use NewUser instead"));
        let tokens = generator.generate(&schema);
        let code = tokens.to_string();

        // Should have metadata method with deprecation message
        assert!(code.contains("fn metadata"));
        assert!(code.contains("with_deprecation_message"));
        assert!(code.contains("Use NewUser instead"));
    }

    #[test]
    fn test_generate_with_examples() {
        let generator = ImplBlockGenerator::new();
        let schema = SchemaIR::new(
            "User",
            SchemaKind::Struct(StructSchema::new(vec![FieldIR::new(
                "name",
                TypeIR::new(TypeKind::String),
            )])),
        )
        .with_metadata(
            SchemaMetadata::default()
                .with_example("{ \"name\": \"John\" }")
                .with_example("{ \"name\": \"Jane\" }"),
        );
        let tokens = generator.generate(&schema);
        let code = tokens.to_string();

        // Should have metadata method with examples
        assert!(code.contains("fn metadata"));
        assert!(code.contains("with_example"));
    }

    #[test]
    fn test_generate_with_tags() {
        let generator = ImplBlockGenerator::new();
        let schema = SchemaIR::new(
            "User",
            SchemaKind::Struct(StructSchema::new(vec![FieldIR::new(
                "name",
                TypeIR::new(TypeKind::String),
            )])),
        )
        .with_metadata(SchemaMetadata::default().with_tag("auth").with_tag("user"));
        let tokens = generator.generate(&schema);
        let code = tokens.to_string();

        // Should have metadata method with tags
        assert!(code.contains("fn metadata"));
        assert!(code.contains("with_tag"));
    }

    #[test]
    fn test_generate_no_metadata_when_empty() {
        let generator = ImplBlockGenerator::new();
        let schema = create_simple_struct_schema("User");
        let tokens = generator.generate(&schema);
        let code = tokens.to_string();

        // Should NOT have metadata method when no metadata is set
        // (the default metadata is empty)
        assert!(!code.contains("fn metadata"));
    }

    #[test]
    fn test_generate_with_single_generic() {
        let generator = ImplBlockGenerator::new();
        let schema = create_generic_struct("Container", &["T"]);
        let tokens = generator.generate(&schema);
        let code = tokens.to_string();

        // Should have generic impl block
        assert!(code.contains("impl <"));
        assert!(code.contains("T :"));
        assert!(code.contains("ZodSchema"));
        assert!(code.contains("Container <"));
    }

    #[test]
    fn test_generate_with_multiple_generics() {
        let generator = ImplBlockGenerator::new();
        let schema = create_generic_struct("Pair", &["K", "V"]);
        let tokens = generator.generate(&schema);
        let code = tokens.to_string();

        // Should have multiple generic parameters
        assert!(code.contains("K :"));
        assert!(code.contains("V :"));
        assert!(code.contains("Pair <"));
    }

    #[test]
    fn test_generate_with_bounded_generic() {
        let generator = ImplBlockGenerator::new();
        let mut schema = SchemaIR::new(
            "Container",
            SchemaKind::Struct(StructSchema::new(vec![FieldIR::new(
                "value",
                TypeIR::new(TypeKind::String),
            )])),
        );
        schema.generics = vec![GenericParam::new("T").with_bound("Clone")];
        let tokens = generator.generate(&schema);
        let code = tokens.to_string();

        // Should have bounded generic
        assert!(code.contains("Clone"));
        assert!(code.contains("ZodSchema"));
    }

    #[test]
    fn test_generate_schema_string() {
        let generator = ImplBlockGenerator::new();
        let schema = create_simple_struct_schema("User");
        let schema_string = generator.generate_schema_string(&schema);

        // Should generate valid Zod schema string
        assert!(schema_string.contains("z.object"));
        assert!(schema_string.contains("id"));
        assert!(schema_string.contains("name"));
    }

    #[test]
    fn test_generate_unit_struct() {
        let generator = ImplBlockGenerator::new();
        let schema = SchemaIR::new("Empty", SchemaKind::UnitStruct);
        let tokens = generator.generate(&schema);
        let code = tokens.to_string();

        // Should generate impl for unit struct
        assert!(code.contains("impl"));
        assert!(code.contains("Empty"));
        assert!(code.contains("ZodSchema"));
    }

    #[test]
    fn test_generate_tuple_struct() {
        let generator = ImplBlockGenerator::new();
        let schema = SchemaIR::new(
            "Point",
            SchemaKind::TupleStruct(TupleStructSchema::new(vec![
                TypeIR::new(TypeKind::Float),
                TypeIR::new(TypeKind::Float),
            ])),
        );
        let tokens = generator.generate(&schema);
        let code = tokens.to_string();

        // Should generate impl for tuple struct
        assert!(code.contains("impl"));
        assert!(code.contains("Point"));
        assert!(code.contains("ZodSchema"));
    }

    #[test]
    fn test_generate_enum() {
        let generator = ImplBlockGenerator::new();
        let schema = SchemaIR::new(
            "Status",
            SchemaKind::Enum(EnumSchema::new(vec![
                VariantIR::unit("Active"),
                VariantIR::unit("Inactive"),
            ])),
        );
        let tokens = generator.generate(&schema);
        let code = tokens.to_string();

        // Should generate impl for enum
        assert!(code.contains("impl"));
        assert!(code.contains("Status"));
        assert!(code.contains("ZodSchema"));
    }

    #[test]
    fn test_generate_preserves_rust_name() {
        let generator = ImplBlockGenerator::new();
        let mut schema = create_simple_struct_schema("user_data");
        schema.rust_name = "UserData".to_string();
        let tokens = generator.generate(&schema);
        let code = tokens.to_string();

        // Should use rust_name for the impl target
        assert!(code.contains("UserData"));
    }

    #[test]
    fn test_generate_combined_metadata() {
        let generator = ImplBlockGenerator::new();
        let schema = SchemaIR::new(
            "User",
            SchemaKind::Struct(StructSchema::new(vec![FieldIR::new(
                "name",
                TypeIR::new(TypeKind::String),
            )])),
        )
        .with_metadata(
            SchemaMetadata::with_description("A user")
                .as_deprecated()
                .with_example("example1")
                .with_tag("auth"),
        );
        let tokens = generator.generate(&schema);
        let code = tokens.to_string();

        // Should have all metadata combined
        assert!(code.contains("fn metadata"));
        assert!(code.contains("with_description"));
        assert!(code.contains("with_deprecated"));
        assert!(code.contains("with_example"));
        assert!(code.contains("with_tag"));
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use crate::ir::{FieldIR, SchemaKind, SchemaMetadata, StructSchema, TypeIR, TypeKind};
    use proptest::prelude::*;

    /// Generate arbitrary valid Rust identifiers for type names.
    fn arb_type_name() -> impl Strategy<Value = String> {
        "[A-Z][a-z]{2,8}".prop_map(|s| s)
    }

    /// Generate arbitrary valid Rust identifiers for field names.
    fn arb_field_name() -> impl Strategy<Value = String> {
        "[a-z][a-z_]{2,8}".prop_map(|s| s)
    }

    /// Generate arbitrary doc comment text.
    fn arb_doc_comment() -> impl Strategy<Value = String> {
        "[A-Za-z ]{5,50}".prop_map(|s| s.trim().to_string())
    }

    /// Generate a struct with a nested type reference.
    fn arb_struct_with_reference() -> impl Strategy<Value = (SchemaIR, String)> {
        (arb_type_name(), arb_type_name(), arb_field_name()).prop_map(
            |(struct_name, ref_type_name, field_name)| {
                let schema = SchemaIR::new(
                    struct_name.clone(),
                    SchemaKind::Struct(StructSchema::new(vec![FieldIR::new(
                        field_name,
                        TypeIR::new(TypeKind::Reference {
                            name: ref_type_name.clone(),
                            generics: vec![],
                        }),
                    )])),
                );
                (schema, ref_type_name)
            },
        )
    }

    /// Generate a struct with a description.
    fn arb_struct_with_description() -> impl Strategy<Value = (SchemaIR, String)> {
        (arb_type_name(), arb_doc_comment()).prop_map(|(name, description)| {
            let schema = SchemaIR::new(
                name,
                SchemaKind::Struct(StructSchema::new(vec![FieldIR::new(
                    "value",
                    TypeIR::new(TypeKind::String),
                )])),
            )
            .with_metadata(SchemaMetadata::with_description(description.clone()));
            (schema, description)
        })
    }

    /// Generate a struct with a field that has a description.
    fn arb_struct_with_field_description() -> impl Strategy<Value = (SchemaIR, String)> {
        (arb_type_name(), arb_field_name(), arb_doc_comment()).prop_map(
            |(name, field_name, description)| {
                let mut field = FieldIR::new(field_name, TypeIR::new(TypeKind::String));
                field.metadata.description = Some(description.clone());
                let schema =
                    SchemaIR::new(name, SchemaKind::Struct(StructSchema::new(vec![field])));
                (schema, description)
            },
        )
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// **Property 15: Nested Type Reference**
        ///
        /// *For any* struct containing a field of another ZodSchema type, the generated
        /// schema SHALL reference the nested type's schema by name.
        ///
        /// This test verifies that when a struct has a field with a Reference type,
        /// the generated Zod schema string contains a reference to that type's schema.
        ///
        /// **Feature: zod-schema-macro, Property 15: Nested Type Reference**
        #[test]
        fn property_15_nested_type_reference((schema, ref_type_name) in arb_struct_with_reference()) {
            let generator = ImplBlockGenerator::new();
            let schema_string = generator.generate_schema_string(&schema);

            // The generated schema should reference the nested type's schema
            // Reference types are mapped to {TypeName}Schema in Zod
            let expected_ref = format!("{}Schema", ref_type_name);
            prop_assert!(
                schema_string.contains(&expected_ref),
                "Generated schema '{}' should contain reference to '{}' for type '{}'",
                schema_string,
                expected_ref,
                ref_type_name
            );
        }

        /// **Property 16: Doc Comment Extraction**
        ///
        /// *For any* type with Rust doc comments (description in metadata), the generated
        /// schema SHALL include a `.describe()` call with the comment text.
        ///
        /// This test verifies that when a struct has a description in its metadata,
        /// the generated impl block includes a metadata() method with the description.
        ///
        /// **Feature: zod-schema-macro, Property 16: Doc Comment Extraction**
        #[test]
        fn property_16_doc_comment_extraction_type((schema, description) in arb_struct_with_description()) {
            let generator = ImplBlockGenerator::new();
            let tokens = generator.generate(&schema);
            let code = tokens.to_string();

            // The generated impl should have a metadata() method with the description
            prop_assert!(
                code.contains("fn metadata"),
                "Generated code should contain metadata() method when description is present"
            );
            prop_assert!(
                code.contains("with_description"),
                "Generated code should contain with_description call"
            );
            // The description text should be in the generated code
            prop_assert!(
                code.contains(&description),
                "Generated code '{}' should contain the description text '{}'",
                code,
                description
            );
        }

        /// **Property 16 (Field): Doc Comment Extraction for Fields**
        ///
        /// *For any* field with Rust doc comments (description in field metadata),
        /// the generated Zod schema SHALL include a `.describe()` call with the comment text.
        ///
        /// This test verifies that when a field has a description, the generated
        /// Zod schema string includes a .describe() call.
        ///
        /// **Feature: zod-schema-macro, Property 16: Doc Comment Extraction**
        #[test]
        fn property_16_doc_comment_extraction_field((schema, description) in arb_struct_with_field_description()) {
            let generator = ImplBlockGenerator::new();
            let schema_string = generator.generate_schema_string(&schema);

            // The generated schema should have a .describe() call with the description
            prop_assert!(
                schema_string.contains(".describe("),
                "Generated schema '{}' should contain .describe() call when field has description",
                schema_string
            );
            prop_assert!(
                schema_string.contains(&description),
                "Generated schema '{}' should contain the description text '{}'",
                schema_string,
                description
            );
        }
    }
}
