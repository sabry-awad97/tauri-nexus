//! Field parsing logic.
//!
//! This module handles parsing struct fields and their attributes.
//! It provides utilities for:
//! - Parsing field types using TypeParser
//! - Applying field attributes (rename, skip, optional, nullable)
//! - Applying validation attributes
//! - Extracting doc comments from fields

use darling::FromField;
use syn::Field;

use crate::ir::{FieldIR, FieldMetadata, TypeIR, ValidationRule};
use crate::parser::attributes::{FieldAttrs, RenameRule};
use crate::parser::struct_parser::extract_doc_comments;
use crate::parser::type_parser::{ParseError, TypeParser};

/// Error type for field parsing failures.
#[derive(Debug, Clone, thiserror::Error)]
pub enum FieldParseError {
    #[error("Failed to parse field attributes: {0}")]
    Attributes(String),

    #[error("Failed to parse field type: {0}")]
    Type(#[from] ParseError),

    #[error("Field has no identifier (index: {0})")]
    MissingIdent(usize),
}

/// Parses struct fields into FieldIR.
pub struct FieldParser;

impl FieldParser {
    /// Parse a single named field into FieldIR.
    pub fn parse_named(
        field: &Field,
        index: usize,
        rename_all: Option<RenameRule>,
    ) -> Result<Option<FieldIR>, FieldParseError> {
        let attrs = FieldAttrs::from_field(field)
            .map_err(|e| FieldParseError::Attributes(e.to_string()))?;

        if attrs.skip {
            return Ok(None);
        }

        let rust_name = field
            .ident
            .as_ref()
            .ok_or(FieldParseError::MissingIdent(index))?
            .to_string();

        let schema_name = attrs.schema_name(rename_all);
        let ty = Self::parse_field_type(field, &attrs)?;
        let description = extract_doc_comments(&field.attrs);
        let metadata = Self::build_metadata(&attrs, description);
        let validation = attrs.to_validation_rules();

        let mut field_ir = FieldIR::new(&rust_name, ty)
            .with_schema_name(schema_name)
            .with_optional(attrs.optional)
            .with_nullable(attrs.nullable)
            .with_flatten(attrs.flatten)
            .with_validation(validation)
            .with_metadata(metadata);

        if let Some(default) = &attrs.default {
            field_ir = field_ir.with_default(default.clone());
        }

        Ok(Some(field_ir))
    }

    /// Parse a tuple struct field into TypeIR.
    pub fn parse_unnamed(field: &Field) -> Result<TypeIR, FieldParseError> {
        TypeParser::parse(&field.ty).map_err(FieldParseError::from)
    }

    fn parse_field_type(field: &Field, attrs: &FieldAttrs) -> Result<TypeIR, FieldParseError> {
        if let Some(type_override) = &attrs.type_override {
            return Ok(TypeIR::new(crate::ir::TypeKind::Reference {
                name: type_override.clone(),
                generics: vec![],
            }));
        }
        TypeParser::parse(&field.ty).map_err(FieldParseError::from)
    }

    fn build_metadata(attrs: &FieldAttrs, doc_description: Option<String>) -> FieldMetadata {
        let mut metadata = FieldMetadata::default();
        if let Some(desc) = doc_description {
            metadata.description = Some(desc);
        }
        if let Some(attr_desc) = &attrs.description {
            metadata.description = Some(attr_desc.clone());
        }
        metadata
    }

    /// Extract validation rules from field attributes.
    pub fn extract_validation_rules(attrs: &FieldAttrs) -> Vec<ValidationRule> {
        attrs.to_validation_rules()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::TypeKind;

    fn parse_field(tokens: proc_macro2::TokenStream) -> Field {
        let item: syn::ItemStruct = syn::parse2(quote::quote! {
            struct Test { #tokens }
        })
        .unwrap();
        match item.fields {
            syn::Fields::Named(fields) => fields.named.into_iter().next().unwrap(),
            _ => panic!("Expected named fields"),
        }
    }

    #[test]
    fn test_parse_simple_field() {
        let field = parse_field(quote::quote! { name: String });
        let result = FieldParser::parse_named(&field, 0, None).unwrap();
        assert!(result.is_some());
        let field_ir = result.unwrap();
        assert_eq!(field_ir.rust_name, "name");
        assert_eq!(field_ir.schema_name, "name");
        assert_eq!(field_ir.ty.kind, TypeKind::String);
    }

    #[test]
    fn test_parse_field_with_rename() {
        let field = parse_field(quote::quote! {
            #[zod(rename = "userName")]
            name: String
        });
        let result = FieldParser::parse_named(&field, 0, None).unwrap().unwrap();
        assert_eq!(result.rust_name, "name");
        assert_eq!(result.schema_name, "userName");
    }

    #[test]
    fn test_parse_field_with_rename_all() {
        let field = parse_field(quote::quote! { user_name: String });
        let result = FieldParser::parse_named(&field, 0, Some(RenameRule::CamelCase))
            .unwrap()
            .unwrap();
        assert_eq!(result.rust_name, "user_name");
        assert_eq!(result.schema_name, "userName");
    }

    #[test]
    fn test_parse_field_with_skip() {
        let field = parse_field(quote::quote! {
            #[zod(skip)]
            internal: String
        });
        let result = FieldParser::parse_named(&field, 0, None).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_field_with_optional() {
        let field = parse_field(quote::quote! {
            #[zod(optional)]
            nickname: String
        });
        let result = FieldParser::parse_named(&field, 0, None).unwrap().unwrap();
        assert!(result.optional);
        assert!(!result.nullable);
    }

    #[test]
    fn test_parse_field_with_nullable() {
        let field = parse_field(quote::quote! {
            #[zod(nullable)]
            avatar: String
        });
        let result = FieldParser::parse_named(&field, 0, None).unwrap().unwrap();
        assert!(!result.optional);
        assert!(result.nullable);
    }

    #[test]
    fn test_parse_field_with_email_validation() {
        let field = parse_field(quote::quote! {
            #[zod(email)]
            email: String
        });
        let result = FieldParser::parse_named(&field, 0, None).unwrap().unwrap();
        assert!(result
            .validation
            .iter()
            .any(|v| matches!(v, ValidationRule::Email)));
    }

    #[test]
    fn test_parse_field_with_min_max_validation() {
        let field = parse_field(quote::quote! {
            #[zod(min = 0.0, max = 100.0)]
            score: f64
        });
        let result = FieldParser::parse_named(&field, 0, None).unwrap().unwrap();
        assert!(result
            .validation
            .iter()
            .any(|v| matches!(v, ValidationRule::Min(n) if *n == 0.0)));
        assert!(result
            .validation
            .iter()
            .any(|v| matches!(v, ValidationRule::Max(n) if *n == 100.0)));
    }

    #[test]
    fn test_parse_unnamed_field() {
        let item: syn::ItemStruct = syn::parse2(quote::quote! {
            struct Point(f64, f64);
        })
        .unwrap();
        if let syn::Fields::Unnamed(fields) = item.fields {
            let field = fields.unnamed.first().unwrap();
            let result = FieldParser::parse_unnamed(field).unwrap();
            assert_eq!(result.kind, TypeKind::Float);
        } else {
            panic!("Expected unnamed fields");
        }
    }
}
