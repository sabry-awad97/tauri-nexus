//! Rust source parser for extracting types with `#[derive(ZodSchema)]`.
//!
//! This module parses Rust source files using `syn` and extracts
//! structs and enums that have the `ZodSchema` derive attribute.

use crate::error::{CliResult, ParseError};
use crate::scanner::SourceFile;
use std::path::{Path, PathBuf};
use syn::{Attribute, DeriveInput, Item};

/// A parsed type with its schema IR and source location.
#[derive(Debug, Clone)]
pub struct ParsedType {
    /// Name of the type.
    pub name: String,

    /// The parsed DeriveInput for further processing.
    pub derive_input: DeriveInput,

    /// Source location information.
    pub location: SourceLocation,
}

/// Source location for error reporting.
#[derive(Debug, Clone)]
pub struct SourceLocation {
    /// File path.
    pub file: PathBuf,

    /// Line number (1-indexed).
    pub line: usize,

    /// Column number (1-indexed).
    pub column: usize,
}

/// Parser for Rust source files.
#[derive(Debug, Default)]
pub struct RustParser {
    /// Whether to parse serde attributes.
    serde_compat: bool,
}

impl RustParser {
    /// Create a new parser with default settings.
    pub fn new() -> Self {
        Self { serde_compat: true }
    }

    /// Set whether to parse serde attributes.
    pub fn with_serde_compat(mut self, enabled: bool) -> Self {
        self.serde_compat = enabled;
        self
    }

    /// Parse a source file and extract types with `#[derive(ZodSchema)]`.
    pub fn parse_file(&self, source: &SourceFile) -> CliResult<Vec<ParsedType>> {
        self.parse_source(&source.content, &source.path)
    }

    /// Parse source code and extract types with `#[derive(ZodSchema)]`.
    pub fn parse_source(&self, content: &str, file_path: &Path) -> CliResult<Vec<ParsedType>> {
        // Parse the file with syn
        let syntax = syn::parse_file(content).map_err(|e| {
            // Note: span-locations feature not enabled, use line 1 as fallback
            ParseError::syntax(file_path.to_path_buf(), 1, 1, e.to_string())
        })?;

        let mut types = Vec::new();

        // Extract items with ZodSchema derive
        for item in syntax.items {
            if let Some(parsed) = self.extract_zod_type(&item, file_path)? {
                types.push(parsed);
            }
        }

        Ok(types)
    }

    /// Parse multiple source files, collecting errors.
    pub fn parse_files(&self, sources: &[SourceFile]) -> (Vec<ParsedType>, Vec<ParseError>) {
        let mut types = Vec::new();
        let mut errors = Vec::new();

        for source in sources {
            match self.parse_file(source) {
                Ok(parsed) => types.extend(parsed),
                Err(crate::error::CliError::Parse(e)) => errors.push(e),
                Err(_) => {} // Ignore other errors
            }
        }

        (types, errors)
    }

    /// Extract a type if it has `#[derive(ZodSchema)]`.
    fn extract_zod_type(&self, item: &Item, file_path: &Path) -> CliResult<Option<ParsedType>> {
        match item {
            Item::Struct(item_struct) => {
                if self.has_zod_derive(&item_struct.attrs) {
                    let derive_input = DeriveInput {
                        attrs: item_struct.attrs.clone(),
                        vis: item_struct.vis.clone(),
                        ident: item_struct.ident.clone(),
                        generics: item_struct.generics.clone(),
                        data: syn::Data::Struct(syn::DataStruct {
                            struct_token: item_struct.struct_token,
                            fields: item_struct.fields.clone(),
                            semi_token: item_struct.semi_token,
                        }),
                    };

                    let location = self.get_location(&item_struct.ident, file_path);

                    Ok(Some(ParsedType {
                        name: item_struct.ident.to_string(),
                        derive_input,
                        location,
                    }))
                } else {
                    Ok(None)
                }
            }
            Item::Enum(item_enum) => {
                if self.has_zod_derive(&item_enum.attrs) {
                    let derive_input = DeriveInput {
                        attrs: item_enum.attrs.clone(),
                        vis: item_enum.vis.clone(),
                        ident: item_enum.ident.clone(),
                        generics: item_enum.generics.clone(),
                        data: syn::Data::Enum(syn::DataEnum {
                            enum_token: item_enum.enum_token,
                            brace_token: item_enum.brace_token,
                            variants: item_enum.variants.clone(),
                        }),
                    };

                    let location = self.get_location(&item_enum.ident, file_path);

                    Ok(Some(ParsedType {
                        name: item_enum.ident.to_string(),
                        derive_input,
                        location,
                    }))
                } else {
                    Ok(None)
                }
            }
            _ => Ok(None),
        }
    }

    /// Check if attributes contain `#[derive(ZodSchema)]`.
    fn has_zod_derive(&self, attrs: &[Attribute]) -> bool {
        for attr in attrs {
            if attr.path().is_ident("derive") {
                if let Ok(nested) = attr.parse_args_with(
                    syn::punctuated::Punctuated::<syn::Path, syn::Token![,]>::parse_terminated,
                ) {
                    for path in nested {
                        if path.is_ident("ZodSchema") {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    /// Get source location from an identifier.
    fn get_location(&self, _ident: &syn::Ident, file_path: &Path) -> SourceLocation {
        // Note: span-locations feature not enabled in proc-macro2, use defaults
        SourceLocation {
            file: file_path.to_path_buf(),
            line: 1,
            column: 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_code(code: &str) -> Vec<ParsedType> {
        let parser = RustParser::new();
        parser
            .parse_source(code, &PathBuf::from("test.rs"))
            .unwrap()
    }

    #[test]
    fn test_parse_struct_with_zod_derive() {
        let code = r#"
            use zod_rs::ZodSchema;

            #[derive(ZodSchema)]
            struct User {
                name: String,
                age: u32,
            }
        "#;

        let types = parse_code(code);
        assert_eq!(types.len(), 1);
        assert_eq!(types[0].name, "User");
    }

    #[test]
    fn test_parse_enum_with_zod_derive() {
        let code = r#"
            use zod_rs::ZodSchema;

            #[derive(ZodSchema)]
            enum Status {
                Active,
                Inactive,
            }
        "#;

        let types = parse_code(code);
        assert_eq!(types.len(), 1);
        assert_eq!(types[0].name, "Status");
    }

    #[test]
    fn test_parse_multiple_types() {
        let code = r#"
            use zod_rs::ZodSchema;

            #[derive(ZodSchema)]
            struct User {
                name: String,
            }

            #[derive(ZodSchema)]
            struct Post {
                title: String,
            }

            #[derive(ZodSchema)]
            enum Status {
                Active,
            }
        "#;

        let types = parse_code(code);
        assert_eq!(types.len(), 3);
    }

    #[test]
    fn test_skip_types_without_zod_derive() {
        let code = r#"
            use serde::Serialize;

            #[derive(Serialize)]
            struct NotZod {
                name: String,
            }

            #[derive(ZodSchema)]
            struct WithZod {
                name: String,
            }
        "#;

        let types = parse_code(code);
        assert_eq!(types.len(), 1);
        assert_eq!(types[0].name, "WithZod");
    }

    #[test]
    fn test_parse_with_multiple_derives() {
        let code = r#"
            use serde::{Serialize, Deserialize};
            use zod_rs::ZodSchema;

            #[derive(Debug, Clone, Serialize, Deserialize, ZodSchema)]
            struct User {
                name: String,
            }
        "#;

        let types = parse_code(code);
        assert_eq!(types.len(), 1);
        assert_eq!(types[0].name, "User");
    }

    #[test]
    fn test_parse_with_zod_attributes() {
        let code = r#"
            use zod_rs::ZodSchema;

            #[derive(ZodSchema)]
            #[zod(rename_all = "camelCase")]
            struct User {
                #[zod(rename = "userName")]
                user_name: String,
            }
        "#;

        let types = parse_code(code);
        assert_eq!(types.len(), 1);

        // Verify attributes are preserved
        let attrs = &types[0].derive_input.attrs;
        assert!(attrs.iter().any(|a| a.path().is_ident("zod")));
    }

    #[test]
    fn test_parse_syntax_error() {
        let code = r#"
            struct Invalid {
                name String  // Missing colon
            }
        "#;

        let parser = RustParser::new();
        let result = parser.parse_source(code, &PathBuf::from("test.rs"));

        assert!(result.is_err());
    }

    #[test]
    fn test_source_location() {
        let code = r#"use zod_rs::ZodSchema;

#[derive(ZodSchema)]
struct User {
    name: String,
}"#;

        let types = parse_code(code);
        assert_eq!(types.len(), 1);
        // Note: span-locations not enabled, defaults to line 1
        assert_eq!(types[0].location.line, 1);
    }

    #[test]
    fn test_parse_files_collects_errors() {
        let parser = RustParser::new();

        let valid = SourceFile {
            path: PathBuf::from("valid.rs"),
            relative_path: PathBuf::from("valid.rs"),
            content: r#"
                #[derive(ZodSchema)]
                struct Valid { name: String }
            "#
            .to_string(),
        };

        let invalid = SourceFile {
            path: PathBuf::from("invalid.rs"),
            relative_path: PathBuf::from("invalid.rs"),
            content: "struct Invalid { name String }".to_string(),
        };

        let (types, errors) = parser.parse_files(&[valid, invalid]);

        assert_eq!(types.len(), 1);
        assert_eq!(errors.len(), 1);
    }
}
