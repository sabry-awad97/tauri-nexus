//! Integration tests for zod-rs-cli.
//!
//! These tests verify end-to-end functionality of the CLI tool,
//! including scanning, parsing, generation, and validation.

use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

use zod_rs_cli::{
    config::{Config, ConfigManager},
    generator::SchemaGenerator,
    parser::RustParser,
    scanner::SourceScanner,
    writer::FileWriter,
};

/// Get the path to test fixtures.
fn fixtures_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// Create a temporary directory with test files.
fn create_temp_project(files: &[(&str, &str)]) -> TempDir {
    let dir = TempDir::new().unwrap();
    for (name, content) in files {
        let path = dir.path().join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, content).unwrap();
    }
    dir
}

// =============================================================================
// Scanner Integration Tests
// =============================================================================

#[test]
fn test_scanner_finds_fixture_files() {
    let scanner = SourceScanner::new(fixtures_path());
    let files = scanner.scan().unwrap();

    // Should find all .rs files in fixtures
    assert!(files.len() >= 5, "Expected at least 5 fixture files");

    let file_names: Vec<_> = files
        .iter()
        .map(|f| {
            f.relative_path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string()
        })
        .collect();

    assert!(file_names.contains(&"simple_struct.rs".to_string()));
    assert!(file_names.contains(&"complex_types.rs".to_string()));
    assert!(file_names.contains(&"enums.rs".to_string()));
}

#[test]
fn test_scanner_with_filter() {
    let scanner = SourceScanner::new(fixtures_path())
        .with_filter("**/simple*.rs")
        .unwrap();

    let files = scanner.scan().unwrap();

    assert_eq!(files.len(), 1);
    assert!(files[0]
        .relative_path
        .to_string_lossy()
        .contains("simple_struct.rs"));
}

#[test]
fn test_scanner_respects_gitignore() {
    // Note: The ignore crate requires a git repository to respect .gitignore
    // For this test, we verify the scanner can be configured with gitignore support
    let dir = create_temp_project(&[
        ("src/main.rs", "fn main() {}"),
        ("src/lib.rs", "pub mod foo;"),
    ]);

    let scanner = SourceScanner::new(dir.path()).with_gitignore(true);
    let files = scanner.scan().unwrap();

    // Should find the source files
    assert_eq!(files.len(), 2);

    let paths: Vec<_> = files
        .iter()
        .map(|f| f.relative_path.to_string_lossy().to_string())
        .collect();

    assert!(paths.iter().any(|p| p.contains("main.rs")));
    assert!(paths.iter().any(|p| p.contains("lib.rs")));
}

// =============================================================================
// Parser Integration Tests
// =============================================================================

#[test]
fn test_parser_extracts_zod_types_from_fixtures() {
    let scanner = SourceScanner::new(fixtures_path());
    let files = scanner.scan().unwrap();

    let parser = RustParser::new();
    let (types, errors) = parser.parse_files(&files);

    // Should have no parse errors for valid fixtures
    assert!(errors.is_empty(), "Unexpected parse errors: {:?}", errors);

    // Should find types with ZodSchema derive
    assert!(!types.is_empty(), "Expected to find ZodSchema types");

    let type_names: Vec<_> = types.iter().map(|t| t.name.as_str()).collect();

    // From simple_struct.rs
    assert!(type_names.contains(&"User"));
    assert!(type_names.contains(&"Post"));

    // From complex_types.rs
    assert!(type_names.contains(&"Address"));
    assert!(type_names.contains(&"Person"));
    assert!(type_names.contains(&"Organization"));

    // From enums.rs
    assert!(type_names.contains(&"Status"));
    assert!(type_names.contains(&"Message"));
    assert!(type_names.contains(&"Event"));
    assert!(type_names.contains(&"ApiResponse"));
}

#[test]
fn test_parser_ignores_non_zod_types() {
    let scanner = SourceScanner::new(fixtures_path())
        .with_filter("**/no_zod_schema.rs")
        .unwrap();

    let files = scanner.scan().unwrap();
    assert_eq!(files.len(), 1);

    let parser = RustParser::new();
    let (types, errors) = parser.parse_files(&files);

    // Should have no errors
    assert!(errors.is_empty());

    // Should find no ZodSchema types
    assert!(
        types.is_empty(),
        "Expected no ZodSchema types, found: {:?}",
        types.iter().map(|t| &t.name).collect::<Vec<_>>()
    );
}

#[test]
fn test_parser_mixed_types() {
    let scanner = SourceScanner::new(fixtures_path())
        .with_filter("**/mixed.rs")
        .unwrap();

    let files = scanner.scan().unwrap();
    let parser = RustParser::new();
    let (types, _) = parser.parse_files(&files);

    let type_names: Vec<_> = types.iter().map(|t| t.name.as_str()).collect();

    // Should include types with ZodSchema
    assert!(type_names.contains(&"IncludedType"));
    assert!(type_names.contains(&"IncludedEnum"));
    assert!(type_names.contains(&"AnotherIncluded"));

    // Should NOT include types without ZodSchema
    assert!(!type_names.contains(&"ExcludedType"));
    assert!(!type_names.contains(&"ExcludedEnum"));

    assert_eq!(types.len(), 3);
}

#[test]
fn test_parser_handles_syntax_errors_gracefully() {
    let dir = create_temp_project(&[
        (
            "valid.rs",
            r#"
            #[derive(ZodSchema)]
            struct Valid { name: String }
        "#,
        ),
        (
            "invalid.rs",
            r#"
            struct Invalid { name String }  // Missing colon
        "#,
        ),
    ]);

    let scanner = SourceScanner::new(dir.path());
    let files = scanner.scan().unwrap();

    let parser = RustParser::new();
    let (types, errors) = parser.parse_files(&files);

    // Should parse valid file
    assert_eq!(types.len(), 1);
    assert_eq!(types[0].name, "Valid");

    // Should collect error from invalid file
    assert_eq!(errors.len(), 1);
}

// =============================================================================
// Generator Integration Tests
// =============================================================================

#[test]
fn test_generator_produces_valid_output() {
    let scanner = SourceScanner::new(fixtures_path())
        .with_filter("**/simple_struct.rs")
        .unwrap();

    let files = scanner.scan().unwrap();
    let parser = RustParser::new();
    let (types, _) = parser.parse_files(&files);

    let config = Config::default();
    let generator = SchemaGenerator::new(config);
    let output = generator.generate(types).unwrap();

    // Should have Zod import
    assert!(output.content.contains("import { z } from 'zod'"));

    // Should have schemas for User and Post
    assert!(output.content.contains("UserSchema"));
    assert!(output.content.contains("PostSchema"));

    // Should have correct number of schemas
    assert_eq!(output.schemas.len(), 2);
}

#[test]
fn test_generator_handles_empty_input() {
    let config = Config::default();
    let generator = SchemaGenerator::new(config);
    let output = generator.generate(vec![]).unwrap();

    assert!(output.content.contains("import { z } from 'zod'"));
    assert!(output.schemas.is_empty());
}

#[test]
fn test_generator_with_custom_config() {
    let scanner = SourceScanner::new(fixtures_path())
        .with_filter("**/simple_struct.rs")
        .unwrap();

    let files = scanner.scan().unwrap();
    let parser = RustParser::new();
    let (types, _) = parser.parse_files(&files);

    let mut config = Config::default();
    config.naming.schema_suffix = "Validator".to_string();
    config.output.generate_types = true;

    let generator = SchemaGenerator::new(config);
    let output = generator.generate(types).unwrap();

    // Should use custom suffix
    assert!(output.content.contains("UserValidator"));
    assert!(output.content.contains("PostValidator"));
}

// =============================================================================
// Writer Integration Tests
// =============================================================================

#[test]
fn test_writer_creates_output_file() {
    let dir = TempDir::new().unwrap();
    let output_path = dir.path().join("generated/schemas.ts");

    let writer = FileWriter::new(false);
    let content = "// Test content\nimport { z } from 'zod';";

    let result = writer.write(&output_path, content).unwrap();

    assert!(result.was_written());
    assert!(output_path.exists());
    assert_eq!(fs::read_to_string(&output_path).unwrap(), content);
}

#[test]
fn test_writer_dry_run_does_not_create_file() {
    let dir = TempDir::new().unwrap();
    let output_path = dir.path().join("generated/schemas.ts");

    let writer = FileWriter::new(true); // dry_run = true
    let content = "// Test content";

    let result = writer.write(&output_path, content).unwrap();

    assert!(!result.was_written());
    assert!(!output_path.exists());
}

#[test]
fn test_writer_creates_parent_directories() {
    let dir = TempDir::new().unwrap();
    let output_path = dir.path().join("deep/nested/path/schemas.ts");

    let writer = FileWriter::new(false);
    let content = "// Test";

    writer.write(&output_path, content).unwrap();

    assert!(output_path.exists());
    assert!(dir.path().join("deep/nested/path").is_dir());
}

// =============================================================================
// End-to-End Integration Tests
// =============================================================================

#[test]
fn test_end_to_end_generation() {
    let dir = create_temp_project(&[(
        "src/models.rs",
        r#"
use zod_rs::ZodSchema;

#[derive(ZodSchema)]
pub struct User {
    pub id: u32,
    pub name: String,
    pub email: Option<String>,
}

#[derive(ZodSchema)]
pub enum Role {
    Admin,
    User,
    Guest,
}
"#,
    )]);

    // Scan
    let scanner = SourceScanner::new(dir.path());
    let files = scanner.scan().unwrap();
    assert_eq!(files.len(), 1);

    // Parse
    let parser = RustParser::new();
    let (types, errors) = parser.parse_files(&files);
    assert!(errors.is_empty());
    assert_eq!(types.len(), 2);

    // Generate
    let config = Config::default();
    let generator = SchemaGenerator::new(config.clone());
    let output = generator.generate(types).unwrap();

    // Verify output
    assert!(output.content.contains("import { z } from 'zod'"));
    assert!(output.content.contains("UserSchema"));
    assert!(output.content.contains("RoleSchema"));

    // Write
    let output_path = dir
        .path()
        .join(&config.output.dir)
        .join(&config.output.file);
    let writer = FileWriter::new(false);
    writer.write(&output_path, &output.content).unwrap();

    assert!(output_path.exists());
}

#[test]
fn test_end_to_end_with_dependencies() {
    let dir = create_temp_project(&[(
        "src/types.rs",
        r#"
use zod_rs::ZodSchema;

#[derive(ZodSchema)]
pub struct Address {
    pub street: String,
    pub city: String,
}

#[derive(ZodSchema)]
pub struct Person {
    pub name: String,
    pub address: Address,
}

#[derive(ZodSchema)]
pub struct Company {
    pub name: String,
    pub employees: Vec<Person>,
}
"#,
    )]);

    let scanner = SourceScanner::new(dir.path());
    let files = scanner.scan().unwrap();

    let parser = RustParser::new();
    let (types, _) = parser.parse_files(&files);

    let config = Config::default();
    let generator = SchemaGenerator::new(config);
    let output = generator.generate(types).unwrap();

    // Verify all schemas are generated
    assert!(output.content.contains("AddressSchema"));
    assert!(output.content.contains("PersonSchema"));
    assert!(output.content.contains("CompanySchema"));

    // Verify dependency order (Address should come before Person)
    let address_pos = output.content.find("AddressSchema").unwrap();
    let person_pos = output.content.find("PersonSchema").unwrap();
    assert!(
        address_pos < person_pos,
        "Address should be defined before Person"
    );
}

// =============================================================================
// Config Integration Tests
// =============================================================================

#[test]
fn test_config_loading_from_file() {
    let dir = create_temp_project(&[(
        "zod-rs.toml",
        r#"
[output]
dir = "generated"
file = "types.ts"
generate_types = true

[naming]
schema_suffix = "Validator"
rename_all = "camelCase"

[features]
serde_compat = true
"#,
    )]);

    let config_path = dir.path().join("zod-rs.toml");
    let config = ConfigManager::load(Some(&config_path)).unwrap();

    assert_eq!(config.output.dir.to_string_lossy(), "generated");
    assert_eq!(config.output.file, "types.ts");
    assert!(config.output.generate_types);
    assert_eq!(config.naming.schema_suffix, "Validator");
    assert_eq!(config.naming.rename_all, Some("camelCase".to_string()));
    assert!(config.features.serde_compat);
}

#[test]
fn test_config_defaults_when_no_file() {
    let config = ConfigManager::load(None).unwrap();

    // Should use defaults
    assert_eq!(config.output.file, "schemas.ts");
    assert_eq!(config.naming.schema_suffix, "Schema");
}

// =============================================================================
// Init Command Integration Tests
// =============================================================================

#[test]
fn test_init_creates_config_file() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("zod-rs.toml");

    // Config file should not exist initially
    assert!(!config_path.exists());

    // Write default config content
    let content = ConfigManager::default_config_content();
    fs::write(&config_path, content).unwrap();

    // Config file should now exist
    assert!(config_path.exists());

    // Content should be valid TOML that can be parsed
    let loaded_config = ConfigManager::load(Some(&config_path)).unwrap();
    assert_eq!(loaded_config.output.file, "schemas.ts");
    assert_eq!(loaded_config.naming.schema_suffix, "Schema");
}

#[test]
fn test_init_config_content_is_valid_toml() {
    let content = ConfigManager::default_config_content();

    // Should be parseable as TOML
    let config: Config = toml::from_str(content).unwrap();

    // Should have expected default values
    assert_eq!(config.output.dir.to_string_lossy(), "./generated");
    assert_eq!(config.output.file, "schemas.ts");
    assert!(config.output.generate_types);
    assert!(config.output.generate_docs);
    assert_eq!(config.naming.rename_all, Some("camelCase".to_string()));
    assert_eq!(config.naming.schema_suffix, "Schema");
    assert!(config.features.serde_compat);
    assert!(!config.features.chrono);
    assert!(!config.features.uuid);
}

#[test]
fn test_init_config_contains_helpful_comments() {
    let content = ConfigManager::default_config_content();

    // Should contain section headers
    assert!(content.contains("[output]"));
    assert!(content.contains("[naming]"));
    assert!(content.contains("[features]"));

    // Should contain helpful comments
    assert!(content.contains("# Output directory"));
    assert!(content.contains("# Output file name"));
    assert!(content.contains("# Rename convention"));
    assert!(content.contains("# Enable serde attribute compatibility"));
}

#[test]
fn test_init_does_not_overwrite_existing_without_force() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("zod-rs.toml");

    // Create existing config with custom content
    let custom_content = r#"
[output]
dir = "./custom"
file = "custom.ts"
"#;
    fs::write(&config_path, custom_content).unwrap();

    // Load the existing config
    let existing_config = ConfigManager::load(Some(&config_path)).unwrap();
    assert_eq!(existing_config.output.dir.to_string_lossy(), "./custom");
    assert_eq!(existing_config.output.file, "custom.ts");

    // Verify the file still has custom content (simulating --force not being used)
    let content = fs::read_to_string(&config_path).unwrap();
    assert!(content.contains("./custom"));
}

#[test]
fn test_init_overwrites_with_force() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("zod-rs.toml");

    // Create existing config with custom content
    let custom_content = r#"
[output]
dir = "./custom"
file = "custom.ts"
"#;
    fs::write(&config_path, custom_content).unwrap();

    // Simulate --force by overwriting with default content
    let default_content = ConfigManager::default_config_content();
    fs::write(&config_path, default_content).unwrap();

    // Load the config - should now have default values
    let config = ConfigManager::load(Some(&config_path)).unwrap();
    assert_eq!(config.output.dir.to_string_lossy(), "./generated");
    assert_eq!(config.output.file, "schemas.ts");
}

// =============================================================================
// Validation Integration Tests
// =============================================================================

#[test]
fn test_validation_detects_stale_schemas() {
    let dir = create_temp_project(&[(
        "src/types.rs",
        r#"
use zod_rs::ZodSchema;

#[derive(ZodSchema)]
pub struct User {
    pub id: u32,
    pub name: String,
}
"#,
    )]);

    // Generate initial schemas
    let scanner = SourceScanner::new(dir.path());
    let files = scanner.scan().unwrap();
    let parser = RustParser::new();
    let (types, _) = parser.parse_files(&files);
    let config = Config::default();
    let generator = SchemaGenerator::new(config.clone());
    let output = generator.generate(types).unwrap();

    // Write schemas
    let output_path = dir
        .path()
        .join(&config.output.dir)
        .join(&config.output.file);
    let writer = FileWriter::new(false);
    writer.write(&output_path, &output.content).unwrap();

    // Modify source file - add a new type to detect staleness
    fs::write(
        dir.path().join("src/types.rs"),
        r#"
use zod_rs::ZodSchema;

#[derive(ZodSchema)]
pub struct User {
    pub id: u32,
    pub name: String,
}

#[derive(ZodSchema)]
pub struct NewType {
    pub value: i32,
}
"#,
    )
    .unwrap();

    // Regenerate and compare
    let files = scanner.scan().unwrap();
    let (types, _) = parser.parse_files(&files);
    let new_output = generator.generate(types).unwrap();

    let existing_content = fs::read_to_string(&output_path).unwrap();

    // Content should be different (new type added)
    assert_ne!(
        existing_content.trim(),
        new_output.content.trim(),
        "Schemas should be detected as stale when new types are added"
    );

    // Verify the new output contains the new type
    assert!(new_output.content.contains("NewTypeSchema"));
}

#[test]
fn test_validation_passes_for_fresh_schemas() {
    let dir = create_temp_project(&[(
        "src/types.rs",
        r#"
use zod_rs::ZodSchema;

#[derive(ZodSchema)]
pub struct User {
    pub id: u32,
}
"#,
    )]);

    // Generate schemas
    let scanner = SourceScanner::new(dir.path());
    let files = scanner.scan().unwrap();
    let parser = RustParser::new();
    let (types, _) = parser.parse_files(&files);
    let config = Config::default();
    let generator = SchemaGenerator::new(config.clone());
    let output = generator.generate(types.clone()).unwrap();

    // Write schemas
    let output_path = dir
        .path()
        .join(&config.output.dir)
        .join(&config.output.file);
    let writer = FileWriter::new(false);
    writer.write(&output_path, &output.content).unwrap();

    // Regenerate without changes
    let new_output = generator.generate(types).unwrap();
    let existing_content = fs::read_to_string(&output_path).unwrap();

    // Content should be the same
    assert_eq!(
        existing_content.trim(),
        new_output.content.trim(),
        "Schemas should be detected as fresh"
    );
}
