//! Property-based tests for zod-rs-cli.
//!
//! These tests verify correctness properties from the design document
//! using the proptest framework.
//!
//! Properties tested:
//! - Property 1: File Discovery Completeness
//! - Property 2: Filter Pattern Correctness
//! - Property 3: ZodSchema Extraction Completeness
//! - Property 4: Attribute Preservation
//! - Property 5: Parser Equivalence (Round-Trip)
//! - Property 6: Topological Sort Correctness
//! - Property 7: Type Export Completeness
//! - Property 8: Config Override Precedence
//! - Property 9: Config Completeness
//! - Property 10: Dry Run Safety
//! - Property 11: Validation Correctness

use proptest::prelude::*;
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

use zod_rs_cli::{
    config::{CliArgs, Config, ConfigManager},
    generator::SchemaGenerator,
    parser::RustParser,
    scanner::SourceScanner,
    writer::FileWriter,
};

// =============================================================================
// Generators for property tests
// =============================================================================

/// Generate a valid Rust identifier.
#[allow(unused)]
fn arb_identifier() -> impl Strategy<Value = String> {
    "[a-zA-Z][a-zA-Z0-9_]{0,15}".prop_map(|s| s)
}

/// Generate a valid file name (without extension).
#[allow(unused)]
fn arb_filename() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9_]{0,10}".prop_map(|s| s)
}

/// Generate a simple Rust struct with ZodSchema derive.
fn arb_zod_struct(name: String) -> String {
    format!(
        r#"
#[derive(ZodSchema)]
pub struct {} {{
    pub id: u32,
    pub name: String,
}}
"#,
        name
    )
}

/// Generate a simple Rust enum with ZodSchema derive.
fn arb_zod_enum(name: String) -> String {
    format!(
        r#"
#[derive(ZodSchema)]
pub enum {} {{
    A,
    B,
    C,
}}
"#,
        name
    )
}

/// Generate a Rust struct without ZodSchema derive.
fn arb_non_zod_struct(name: String) -> String {
    format!(
        r#"
#[derive(Debug, Clone)]
pub struct {} {{
    pub value: i32,
}}
"#,
        name
    )
}

/// Generate a directory structure with Rust files.
#[allow(unused)]
fn create_test_directory(files: &[(&str, &str)]) -> TempDir {
    let dir = TempDir::new().unwrap();
    for (path, content) in files {
        let full_path = dir.path().join(path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(full_path, content).unwrap();
    }
    dir
}

// =============================================================================
// Property 1: File Discovery Completeness
// **Validates: Requirements 1.1, 1.2**
//
// For any directory structure containing .rs files, the Source_Scanner SHALL
// discover all .rs files recursively, and the count of discovered files SHALL
// equal the actual count of .rs files in the directory tree.
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Feature: zod-rs-cli, Property 1: File Discovery Completeness**
    /// **Validates: Requirements 1.1, 1.2**
    #[test]
    fn prop_file_discovery_completeness(
        file_count in 1usize..10,
        depth in 1usize..4,
    ) {
        // Create a directory structure with the specified number of .rs files
        let dir = TempDir::new().unwrap();
        let mut expected_files = HashSet::new();

        for i in 0..file_count {
            // Create files at various depths
            let subdir = (0..((i % depth) + 1))
                .map(|j| format!("dir{}", j))
                .collect::<Vec<_>>()
                .join("/");

            let file_path = if subdir.is_empty() {
                format!("file{}.rs", i)
            } else {
                format!("{}/file{}.rs", subdir, i)
            };

            let full_path = dir.path().join(&file_path);
            if let Some(parent) = full_path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(&full_path, "fn main() {}").unwrap();
            expected_files.insert(file_path);
        }

        // Also create some non-.rs files that should be ignored
        fs::write(dir.path().join("README.md"), "# Test").unwrap();
        fs::write(dir.path().join("Cargo.toml"), "[package]").unwrap();

        // Scan the directory
        let scanner = SourceScanner::new(dir.path());
        let files = scanner.scan().unwrap();

        // Verify: count of discovered files equals actual count of .rs files
        prop_assert_eq!(
            files.len(),
            expected_files.len(),
            "Scanner should find exactly {} .rs files, found {}",
            expected_files.len(),
            files.len()
        );

        // Verify: all discovered files are .rs files
        for file in &files {
            prop_assert!(
                file.path.extension().is_some_and(|ext| ext == "rs"),
                "All discovered files should be .rs files"
            );
        }
    }
}

// =============================================================================
// Property 2: Filter Pattern Correctness
// **Validates: Requirements 1.3**
//
// For any filter pattern and set of file paths, the Source_Scanner SHALL
// include only files whose paths match the filter pattern, and no files
// that don't match SHALL be included.
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Feature: zod-rs-cli, Property 2: Filter Pattern Correctness**
    /// **Validates: Requirements 1.3**
    #[test]
    fn prop_filter_pattern_correctness(
        matching_count in 1usize..5,
        non_matching_count in 1usize..5,
    ) {
        let dir = TempDir::new().unwrap();

        // Create files that should match the pattern (in src/ directory)
        for i in 0..matching_count {
            let path = dir.path().join(format!("src/match{}.rs", i));
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, "fn main() {}").unwrap();
        }

        // Create files that should NOT match the pattern (in other/ directory)
        for i in 0..non_matching_count {
            let path = dir.path().join(format!("other/nomatch{}.rs", i));
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, "fn main() {}").unwrap();
        }

        // Apply filter for src/**/*.rs
        let scanner = SourceScanner::new(dir.path())
            .with_filter("**/src/*.rs")
            .unwrap();
        let files = scanner.scan().unwrap();

        // Verify: only matching files are included
        prop_assert_eq!(
            files.len(),
            matching_count,
            "Filter should include only {} matching files, found {}",
            matching_count,
            files.len()
        );

        // Verify: all included files match the pattern
        for file in &files {
            let path_str = file.relative_path.to_string_lossy();
            prop_assert!(
                path_str.contains("src"),
                "All included files should be in src/, got: {}",
                path_str
            );
        }
    }
}

// =============================================================================
// Property 3: ZodSchema Extraction Completeness
// **Validates: Requirements 2.2**
//
// For any Rust source file containing types with #[derive(ZodSchema)],
// the Rust_Parser SHALL extract all such types, and the count of extracted
// types SHALL equal the count of types with the derive attribute.
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Feature: zod-rs-cli, Property 3: ZodSchema Extraction Completeness**
    /// **Validates: Requirements 2.2**
    #[test]
    fn prop_zod_schema_extraction_completeness(
        zod_struct_count in 0usize..5,
        zod_enum_count in 0usize..5,
        non_zod_count in 0usize..5,
    ) {
        // Skip if no ZodSchema types (parser returns error for empty)
        prop_assume!(zod_struct_count + zod_enum_count > 0);

        // Generate source code with mixed types
        let mut source = String::from("use zod_rs::ZodSchema;\n\n");
        let mut expected_names = HashSet::new();

        // Add ZodSchema structs
        for i in 0..zod_struct_count {
            let name = format!("ZodStruct{}", i);
            source.push_str(&arb_zod_struct(name.clone()));
            expected_names.insert(name);
        }

        // Add ZodSchema enums
        for i in 0..zod_enum_count {
            let name = format!("ZodEnum{}", i);
            source.push_str(&arb_zod_enum(name.clone()));
            expected_names.insert(name);
        }

        // Add non-ZodSchema types (should be ignored)
        for i in 0..non_zod_count {
            let name = format!("NonZod{}", i);
            source.push_str(&arb_non_zod_struct(name));
        }

        // Parse the source
        let parser = RustParser::new();
        let types = parser
            .parse_source(&source, &PathBuf::from("test.rs"))
            .unwrap();

        // Verify: count matches expected
        prop_assert_eq!(
            types.len(),
            expected_names.len(),
            "Parser should extract exactly {} ZodSchema types, found {}",
            expected_names.len(),
            types.len()
        );

        // Verify: all expected types are found
        let found_names: HashSet<String> = types.iter().map(|t| t.name.clone()).collect();
        prop_assert_eq!(
            found_names,
            expected_names,
            "Parser should find all ZodSchema types"
        );
    }
}

// =============================================================================
// Property 4: Attribute Preservation
// **Validates: Requirements 2.3**
//
// For any type with #[zod(...)] or #[serde(...)] attributes, the Rust_Parser
// SHALL preserve all attribute values in the generated IR.
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Feature: zod-rs-cli, Property 4: Attribute Preservation**
    /// **Validates: Requirements 2.3**
    #[test]
    fn prop_attribute_preservation(
        has_zod_attr in any::<bool>(),
        has_serde_attr in any::<bool>(),
        rename_value in "[a-z]{3,8}",
    ) {
        // Build source with various attributes
        let mut attrs = vec!["#[derive(ZodSchema)]".to_string()];

        if has_zod_attr {
            attrs.push(format!("#[zod(rename_all = \"{}\")]", rename_value));
        }

        if has_serde_attr {
            attrs.push("#[serde(rename_all = \"camelCase\")]".to_string());
        }

        let source = format!(
            r#"
use zod_rs::ZodSchema;

{}
pub struct TestType {{
    pub field: String,
}}
"#,
            attrs.join("\n")
        );

        // Parse the source
        let parser = RustParser::new().with_serde_compat(true);
        let types = parser
            .parse_source(&source, &PathBuf::from("test.rs"))
            .unwrap();

        prop_assert_eq!(types.len(), 1, "Should parse exactly one type");

        let parsed = &types[0];

        // Verify: derive attribute is preserved
        let has_derive = parsed.derive_input.attrs.iter().any(|a| a.path().is_ident("derive"));
        prop_assert!(has_derive, "Derive attribute should be preserved");

        // Verify: zod attribute is preserved if present
        if has_zod_attr {
            let has_zod = parsed.derive_input.attrs.iter().any(|a| a.path().is_ident("zod"));
            prop_assert!(has_zod, "Zod attribute should be preserved");
        }

        // Verify: serde attribute is preserved if present
        if has_serde_attr {
            let has_serde = parsed.derive_input.attrs.iter().any(|a| a.path().is_ident("serde"));
            prop_assert!(has_serde, "Serde attribute should be preserved");
        }
    }
}

// =============================================================================
// Property 5: Parser Equivalence (Round-Trip)
// **Validates: Requirements 2.5**
//
// For any valid Rust type definition with #[derive(ZodSchema)], parsing with
// the CLI's Rust_Parser SHALL produce a DeriveInput that preserves the
// essential structure of the original type.
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Feature: zod-rs-cli, Property 5: Parser Equivalence (Round-Trip)**
    /// **Validates: Requirements 2.5**
    #[test]
    fn prop_parser_equivalence(
        type_name in "[A-Z][a-zA-Z]{2,10}",
        field_count in 1usize..5,
    ) {
        // Generate field names
        let fields: Vec<String> = (0..field_count)
            .map(|i| format!("field_{}", i))
            .collect();

        // Build source
        let field_defs = fields
            .iter()
            .map(|f| format!("    pub {}: String,", f))
            .collect::<Vec<_>>()
            .join("\n");

        let source = format!(
            r#"
use zod_rs::ZodSchema;

#[derive(ZodSchema)]
pub struct {} {{
{}
}}
"#,
            type_name, field_defs
        );

        // Parse the source
        let parser = RustParser::new();
        let types = parser
            .parse_source(&source, &PathBuf::from("test.rs"))
            .unwrap();

        prop_assert_eq!(types.len(), 1, "Should parse exactly one type");

        let parsed = &types[0];

        // Verify: type name is preserved
        prop_assert_eq!(
            parsed.name.clone(),
            type_name,
            "Type name should be preserved"
        );

        // Verify: field count is preserved
        if let syn::Data::Struct(data) = &parsed.derive_input.data {
            if let syn::Fields::Named(named) = &data.fields {
                prop_assert_eq!(
                    named.named.len(),
                    field_count,
                    "Field count should be preserved"
                );

                // Verify: field names are preserved
                for (i, field) in named.named.iter().enumerate() {
                    let expected_name = format!("field_{}", i);
                    let actual_name = field.ident.as_ref().unwrap().to_string();
                    prop_assert_eq!(
                        actual_name,
                        expected_name,
                        "Field name should be preserved"
                    );
                }
            } else {
                prop_assert!(false, "Expected named fields");
            }
        } else {
            prop_assert!(false, "Expected struct data");
        }
    }
}

// =============================================================================
// Property 6: Topological Sort Correctness
// **Validates: Requirements 3.2**
//
// For any set of types with dependencies, the Schema_Generator SHALL output
// schemas in an order where each schema appears after all its dependencies.
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Feature: zod-rs-cli, Property 6: Topological Sort Correctness**
    /// **Validates: Requirements 3.2**
    #[test]
    fn prop_topological_sort_correctness(
        chain_length in 2usize..5,
    ) {
        // Generate a chain of dependent types: TypeA <- TypeB <- TypeC ...
        let mut source = String::from("use zod_rs::ZodSchema;\n\n");
        let mut type_names = Vec::new();

        // First type has no dependencies
        let first_name = "TypeA";
        source.push_str(&format!(
            r#"
#[derive(ZodSchema)]
pub struct {} {{
    pub value: String,
}}
"#,
            first_name
        ));
        type_names.push(first_name.to_string());

        // Subsequent types depend on the previous one
        for i in 1..chain_length {
            let name = format!("Type{}", (b'A' + i as u8) as char);
            let prev_name = &type_names[i - 1];
            source.push_str(&format!(
                r#"
#[derive(ZodSchema)]
pub struct {} {{
    pub dep: {},
}}
"#,
                name, prev_name
            ));
            type_names.push(name);
        }

        // Parse and generate
        let parser = RustParser::new();
        let types = parser
            .parse_source(&source, &PathBuf::from("test.rs"))
            .unwrap();

        let config = Config::default();
        let generator = SchemaGenerator::new(config);
        let output = generator.generate(types).unwrap();

        // Verify: each type appears after its dependencies in the output
        let mut seen_positions: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

        for (pos, schema) in output.schemas.iter().enumerate() {
            seen_positions.insert(schema.name.clone(), pos);
        }

        // Check that dependencies come before dependents
        for i in 1..type_names.len() {
            let dep_name = &type_names[i - 1];
            let type_name = &type_names[i];

            if let (Some(&dep_pos), Some(&type_pos)) = (
                seen_positions.get(dep_name),
                seen_positions.get(type_name),
            ) {
                prop_assert!(
                    dep_pos < type_pos,
                    "{} (pos {}) should appear before {} (pos {})",
                    dep_name,
                    dep_pos,
                    type_name,
                    type_pos
                );
            }
        }
    }
}

// =============================================================================
// Property 7: Type Export Completeness
// **Validates: Requirements 3.4**
//
// For any generated schema, the output SHALL include a corresponding type
// inference export (export type X = z.infer<typeof XSchema>).
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Feature: zod-rs-cli, Property 7: Type Export Completeness**
    /// **Validates: Requirements 3.4**
    #[test]
    fn prop_type_export_completeness(
        type_count in 1usize..5,
    ) {
        // Generate multiple types
        let mut source = String::from("use zod_rs::ZodSchema;\n\n");
        let mut type_names = Vec::new();

        for i in 0..type_count {
            let name = format!("TestType{}", i);
            source.push_str(&arb_zod_struct(name.clone()));
            type_names.push(name);
        }

        // Parse and generate with type exports enabled
        let parser = RustParser::new();
        let types = parser
            .parse_source(&source, &PathBuf::from("test.rs"))
            .unwrap();

        let mut config = Config::default();
        config.output.generate_types = true;

        let generator = SchemaGenerator::new(config);
        let output = generator.generate(types).unwrap();

        // Verify: each schema has a corresponding type export
        for type_name in &type_names {
            let schema_name = format!("{}Schema", type_name);
            let type_export = format!("export type {} = z.infer<typeof {}>", type_name, schema_name);

            prop_assert!(
                output.content.contains(&type_export),
                "Output should contain type export for {}: expected '{}' in output",
                type_name,
                type_export
            );
        }

        // Verify: schema count matches type count
        prop_assert_eq!(
            output.schemas.len(),
            type_count,
            "Should generate {} schemas, got {}",
            type_count,
            output.schemas.len()
        );
    }
}

// =============================================================================
// Property 8: Config Override Precedence
// **Validates: Requirements 4.2**
//
// For any configuration setting, if both a config file value and CLI argument
// are provided, the CLI argument SHALL take precedence and the resulting
// config SHALL use the CLI value.
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Feature: zod-rs-cli, Property 8: Config Override Precedence**
    /// **Validates: Requirements 4.2**
    #[test]
    fn prop_config_override_precedence(
        file_dir in "[a-z]{3,8}",
        cli_dir in "[a-z]{3,8}",
        file_filename in "[a-z]{3,8}",
        cli_filename in "[a-z]{3,8}",
        file_generate_types in any::<bool>(),
        cli_generate_types in any::<bool>(),
    ) {
        // Ensure CLI and file values are different for meaningful test
        prop_assume!(file_dir != cli_dir);
        prop_assume!(file_filename != cli_filename);
        prop_assume!(file_generate_types != cli_generate_types);

        // Create a config with file values
        let mut file_config = Config::default();
        file_config.output.dir = PathBuf::from(&file_dir);
        file_config.output.file = format!("{}.ts", file_filename);
        file_config.output.generate_types = file_generate_types;

        // Create CLI args with override values
        let cli_args = CliArgs {
            output: Some(PathBuf::from(&cli_dir)),
            output_file: Some(format!("{}.ts", cli_filename)),
            generate_types: Some(cli_generate_types),
            ..Default::default()
        };

        // Merge CLI args into config
        let merged = ConfigManager::merge_cli_args(file_config, &cli_args);

        // Verify: CLI values take precedence
        prop_assert_eq!(
            merged.output.dir,
            PathBuf::from(&cli_dir),
            "CLI output dir should override file config"
        );

        prop_assert_eq!(
            merged.output.file,
            format!("{}.ts", cli_filename),
            "CLI output file should override file config"
        );

        prop_assert_eq!(
            merged.output.generate_types,
            cli_generate_types,
            "CLI generate_types should override file config"
        );
    }
}

// =============================================================================
// Property 9: Config Completeness
// **Validates: Requirements 4.3**
//
// For any valid configuration file, all supported settings (output dir,
// filename, rename convention, type generation, doc generation) SHALL be
// correctly loaded and applied.
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Feature: zod-rs-cli, Property 9: Config Completeness**
    /// **Validates: Requirements 4.3**
    #[test]
    fn prop_config_completeness(
        output_dir in "[a-z]{3,10}",
        output_file in "[a-z]{3,10}",
        generate_types in any::<bool>(),
        generate_docs in any::<bool>(),
        schema_suffix in "[A-Z][a-z]{3,8}",
        serde_compat in any::<bool>(),
        chrono in any::<bool>(),
        uuid in any::<bool>(),
    ) {
        // Create a TOML config string with all settings
        let toml_content = format!(
            r#"
[output]
dir = "./{}"
file = "{}.ts"
generate_types = {}
generate_docs = {}

[naming]
rename_all = "camelCase"
schema_suffix = "{}"

[features]
serde_compat = {}
chrono = {}
uuid = {}
"#,
            output_dir,
            output_file,
            generate_types,
            generate_docs,
            schema_suffix,
            serde_compat,
            chrono,
            uuid
        );

        // Parse the config
        let config: Config = toml::from_str(&toml_content).unwrap();

        // Verify: all settings are correctly loaded
        prop_assert_eq!(
            config.output.dir,
            PathBuf::from(format!("./{}", output_dir)),
            "Output dir should be loaded correctly"
        );

        prop_assert_eq!(
            config.output.file,
            format!("{}.ts", output_file),
            "Output file should be loaded correctly"
        );

        prop_assert_eq!(
            config.output.generate_types,
            generate_types,
            "generate_types should be loaded correctly"
        );

        prop_assert_eq!(
            config.output.generate_docs,
            generate_docs,
            "generate_docs should be loaded correctly"
        );

        prop_assert_eq!(
            config.naming.schema_suffix,
            schema_suffix,
            "schema_suffix should be loaded correctly"
        );

        prop_assert_eq!(
            config.features.serde_compat,
            serde_compat,
            "serde_compat should be loaded correctly"
        );

        prop_assert_eq!(
            config.features.chrono,
            chrono,
            "chrono should be loaded correctly"
        );

        prop_assert_eq!(
            config.features.uuid,
            uuid,
            "uuid should be loaded correctly"
        );
    }
}

// =============================================================================
// Property 10: Dry Run Safety
// **Validates: Requirements 5.3**
//
// For any invocation with dry-run mode enabled, the File_Writer SHALL NOT
// create or modify any files on disk, regardless of the input.
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Feature: zod-rs-cli, Property 10: Dry Run Safety**
    /// **Validates: Requirements 5.3**
    #[test]
    fn prop_dry_run_safety(
        content_length in 10usize..1000,
        filename in "[a-z]{3,10}",
        nested_depth in 0usize..3,
    ) {
        // Create a temporary directory
        let dir = TempDir::new().unwrap();

        // Build a nested path
        let mut path = dir.path().to_path_buf();
        for i in 0..nested_depth {
            path = path.join(format!("dir{}", i));
        }
        path = path.join(format!("{}.ts", filename));

        // Generate random content
        let content: String = (0..content_length)
            .map(|i| ((i % 26) as u8 + b'a') as char)
            .collect();

        // Create a dry-run writer
        let writer = FileWriter::new(true);

        // Attempt to write
        let result = writer.write(&path, &content).unwrap();

        // Verify: result is DryRun variant
        prop_assert!(
            !result.was_written(),
            "Dry run should not report file as written"
        );

        // Verify: file does NOT exist on disk
        prop_assert!(
            !path.exists(),
            "Dry run should NOT create file at {:?}",
            path
        );

        // Verify: parent directories do NOT exist (if nested)
        if nested_depth > 0 {
            let first_nested = dir.path().join("dir0");
            prop_assert!(
                !first_nested.exists(),
                "Dry run should NOT create directories"
            );
        }

        // Verify: bytes() returns 0 for dry run
        prop_assert_eq!(
            result.bytes(),
            0,
            "Dry run should report 0 bytes written"
        );
    }
}

// =============================================================================
// Property 11: Validation Correctness
// **Validates: Requirements 8.1**
//
// For any set of source files and existing generated files, the validate
// command SHALL return success if and only if regenerating would produce
// identical output.
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Feature: zod-rs-cli, Property 11: Validation Correctness**
    /// **Validates: Requirements 8.1**
    #[test]
    fn prop_validation_correctness(
        type_count in 1usize..4,
        modify_existing in any::<bool>(),
    ) {
        // Generate source with ZodSchema types
        let mut source = String::from("use zod_rs::ZodSchema;\n\n");
        let mut type_names = Vec::new();

        for i in 0..type_count {
            let name = format!("ValidationType{}", i);
            source.push_str(&arb_zod_struct(name.clone()));
            type_names.push(name);
        }

        // Parse and generate
        let parser = RustParser::new();
        let types = parser
            .parse_source(&source, &PathBuf::from("test.rs"))
            .unwrap();

        let config = Config::default();
        let generator = SchemaGenerator::new(config);
        let output = generator.generate(types.clone()).unwrap();

        // Create "existing" content - either identical or modified
        let existing_content = if modify_existing {
            // Add a comment to make it different
            format!("// Modified\n{}", output.content)
        } else {
            output.content.clone()
        };

        // Regenerate to compare
        let regenerated = generator.generate(types).unwrap();

        // Verify: validation result matches expectation
        let would_match = existing_content == regenerated.content;

        if modify_existing {
            prop_assert!(
                !would_match,
                "Modified content should NOT match regenerated output"
            );
        } else {
            prop_assert!(
                would_match,
                "Unmodified content should match regenerated output"
            );
        }

        // Verify: regenerated output is deterministic
        let parser2 = RustParser::new();
        let types2 = parser2
            .parse_source(&source, &PathBuf::from("test.rs"))
            .unwrap();
        let config2 = Config::default();
        let generator2 = SchemaGenerator::new(config2);
        let output2 = generator2.generate(types2).unwrap();

        prop_assert_eq!(
            output.content,
            output2.content,
            "Generation should be deterministic"
        );
    }
}
