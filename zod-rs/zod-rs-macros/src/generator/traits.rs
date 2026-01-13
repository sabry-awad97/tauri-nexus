//! Code generator trait definition.
//!
//! This module defines the `CodeGenerator` trait that all generators implement.
//! The trait provides a standard interface for generating schema code from IR,
//! enabling support for multiple output formats (Zod, JSON Schema, OpenAPI, etc.).

use std::collections::HashMap;

use crate::error::GeneratorError;
use crate::ir::SchemaIR;

/// Trait for schema code generators.
///
/// Implement this trait to add support for new schema formats.
/// Each generator transforms the intermediate representation (IR)
/// into target schema code.
///
/// # Example
///
/// ```rust,ignore
/// use zod_rs_macros::generator::{CodeGenerator, GeneratorConfig, GeneratedCode, GeneratorFeature};
/// use zod_rs_macros::ir::SchemaIR;
/// use zod_rs_macros::error::GeneratorError;
///
/// struct MyGenerator;
///
/// impl CodeGenerator for MyGenerator {
///     fn id(&self) -> &'static str { "my-generator" }
///     fn name(&self) -> &'static str { "My Schema Generator" }
///     fn file_extension(&self) -> &'static str { "ts" }
///
///     fn generate(&self, schema: &SchemaIR, config: &GeneratorConfig) -> Result<GeneratedCode, GeneratorError> {
///         // Generate schema code...
///         Ok(GeneratedCode::new("// generated code", &schema.name))
///     }
///
///     fn generate_preamble(&self, _schemas: &[&SchemaIR], _config: &GeneratorConfig) -> Result<String, GeneratorError> {
///         Ok("// preamble".to_string())
///     }
///
///     fn generate_postamble(&self, _schemas: &[&SchemaIR], _config: &GeneratorConfig) -> Result<String, GeneratorError> {
///         Ok("// postamble".to_string())
///     }
///
///     fn supports_feature(&self, feature: GeneratorFeature) -> bool {
///         false
///     }
/// }
/// ```
pub trait CodeGenerator: Send + Sync {
    /// Returns the unique identifier for this generator.
    ///
    /// This is used to select the generator and should be a short,
    /// lowercase string (e.g., "zod", "json-schema", "openapi").
    #[allow(unused)]
    fn id(&self) -> &'static str;

    /// Returns the human-readable name of this generator.
    ///
    /// This is used for display purposes (e.g., "Zod Schema Generator").
    #[allow(unused)]
    fn name(&self) -> &'static str;

    /// Returns the file extension for generated files.
    ///
    /// This is used when writing output files (e.g., "ts", "json", "yaml").
    #[allow(unused)]
    fn file_extension(&self) -> &'static str;

    /// Generate schema code from IR.
    ///
    /// This is the main generation method that transforms a single schema
    /// into the target format.
    ///
    /// # Arguments
    ///
    /// * `schema` - The schema IR to generate code for
    /// * `config` - Generator configuration options
    ///
    /// # Returns
    ///
    /// Returns `GeneratedCode` containing the generated code and metadata,
    /// or a `GeneratorError` if generation fails.
    fn generate(
        &self,
        schema: &SchemaIR,
        config: &GeneratorConfig,
    ) -> Result<GeneratedCode, GeneratorError>;

    /// Generate imports/preamble for the output file.
    ///
    /// This is called once at the beginning of file generation to produce
    /// any necessary imports, type definitions, or setup code.
    ///
    /// # Arguments
    ///
    /// * `schemas` - All schemas that will be generated in this file
    /// * `config` - Generator configuration options
    fn generate_preamble(
        &self,
        schemas: &[&SchemaIR],
        config: &GeneratorConfig,
    ) -> Result<String, GeneratorError>;

    /// Generate exports/postamble for the output file.
    ///
    /// This is called once at the end of file generation to produce
    /// any necessary exports, cleanup code, or summary.
    ///
    /// # Arguments
    ///
    /// * `schemas` - All schemas that were generated in this file
    /// * `config` - Generator configuration options
    fn generate_postamble(
        &self,
        schemas: &[&SchemaIR],
        config: &GeneratorConfig,
    ) -> Result<String, GeneratorError>;

    /// Check if this generator supports a specific feature.
    ///
    /// This allows callers to check generator capabilities before
    /// attempting to use features that may not be supported.
    ///
    /// # Arguments
    ///
    /// * `feature` - The feature to check support for
    #[allow(unused)]
    fn supports_feature(&self, feature: GeneratorFeature) -> bool;
}

/// Generator configuration options.
///
/// Controls various aspects of code generation including output style,
/// formatting, and custom type mappings.
#[derive(Debug, Clone)]
pub struct GeneratorConfig {
    /// Output style (const export, function export, etc.)
    pub output_style: OutputStyle,

    /// Whether to generate type inference (e.g., `type X = z.infer<typeof XSchema>`)
    pub generate_types: bool,

    /// Whether to generate JSDoc/documentation comments
    pub generate_docs: bool,

    /// Indentation style
    pub indent: IndentStyle,

    /// Line ending style
    pub line_ending: LineEnding,

    /// Custom type mappings (Rust type name -> target schema)
    #[allow(unused)]
    pub type_overrides: HashMap<String, String>,
}

impl Default for GeneratorConfig {
    fn default() -> Self {
        Self {
            output_style: OutputStyle::default(),
            generate_types: true,
            generate_docs: true,
            indent: IndentStyle::default(),
            line_ending: LineEnding::default(),
            type_overrides: HashMap::new(),
        }
    }
}

impl GeneratorConfig {
    /// Create a new generator config with default settings.
    #[allow(unused)]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the output style.
    #[allow(unused)]
    pub fn with_output_style(mut self, style: OutputStyle) -> Self {
        self.output_style = style;
        self
    }

    /// Set whether to generate type inference.
    #[allow(unused)]
    pub fn with_generate_types(mut self, generate: bool) -> Self {
        self.generate_types = generate;
        self
    }

    /// Set whether to generate documentation comments.
    #[allow(unused)]
    pub fn with_generate_docs(mut self, generate: bool) -> Self {
        self.generate_docs = generate;
        self
    }

    /// Set the indentation style.
    #[allow(unused)]
    pub fn with_indent(mut self, indent: IndentStyle) -> Self {
        self.indent = indent;
        self
    }

    /// Set the line ending style.
    #[allow(unused)]
    pub fn with_line_ending(mut self, line_ending: LineEnding) -> Self {
        self.line_ending = line_ending;
        self
    }

    /// Add a custom type override.
    #[allow(unused)]
    pub fn with_type_override(
        mut self,
        rust_type: impl Into<String>,
        schema: impl Into<String>,
    ) -> Self {
        self.type_overrides.insert(rust_type.into(), schema.into());
        self
    }

    /// Get the indentation string based on current settings.
    #[allow(unused)]
    pub fn indent_str(&self) -> &str {
        self.indent.as_str()
    }

    /// Get the line ending string based on current settings.
    #[allow(unused)]
    pub fn line_ending_str(&self) -> &str {
        self.line_ending.as_str()
    }
}

/// Output style for generated code.
///
/// Determines how schemas are exported in the generated code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputStyle {
    /// Export as const: `export const UserSchema = z.object(...)`
    #[default]
    ConstExport,

    /// Export as function: `export function UserSchema() { return z.object(...) }`
    #[allow(unused)]
    FunctionExport,

    /// No export, just declaration: `const UserSchema = z.object(...)`
    #[allow(unused)]
    Declaration,

    /// Inline (no variable assignment): `z.object(...)`
    #[allow(unused)]
    Inline,
}

impl OutputStyle {
    /// Check if this style includes an export keyword.
    #[allow(unused)]
    pub fn is_exported(&self) -> bool {
        matches!(self, OutputStyle::ConstExport | OutputStyle::FunctionExport)
    }

    /// Check if this style uses a function wrapper.
    #[allow(unused)]
    pub fn is_function(&self) -> bool {
        matches!(self, OutputStyle::FunctionExport)
    }
}

/// Indentation style for generated code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IndentStyle {
    /// Use spaces for indentation (default: 2 spaces)
    #[default]
    Spaces2,

    /// Use 4 spaces for indentation
    #[allow(unused)]
    Spaces4,

    /// Use tabs for indentation
    #[allow(unused)]
    Tabs,
}

impl IndentStyle {
    /// Get the indentation string.
    pub fn as_str(&self) -> &str {
        match self {
            IndentStyle::Spaces2 => "  ",
            IndentStyle::Spaces4 => "    ",
            IndentStyle::Tabs => "\t",
        }
    }

    /// Create an indentation string for the given depth.
    #[allow(unused)]
    pub fn indent(&self, depth: usize) -> String {
        self.as_str().repeat(depth)
    }
}

/// Line ending style for generated code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LineEnding {
    /// Unix-style line endings (LF)
    #[default]
    Lf,

    /// Windows-style line endings (CRLF)
    #[allow(unused)]
    CrLf,
}

impl LineEnding {
    /// Get the line ending string.
    pub fn as_str(&self) -> &str {
        match self {
            LineEnding::Lf => "\n",
            LineEnding::CrLf => "\r\n",
        }
    }
}

/// Generated code output.
///
/// Contains the generated code along with metadata about the schema.
#[derive(Debug, Clone)]
pub struct GeneratedCode {
    /// The generated code string
    pub code: String,

    /// Type name for this schema (e.g., "User")
    #[allow(unused)]
    pub type_name: String,

    /// Schema variable name (e.g., "UserSchema")
    #[allow(unused)]
    pub schema_name: String,

    /// Dependencies on other schemas (by name)
    #[allow(unused)]
    pub dependencies: Vec<String>,
}

impl GeneratedCode {
    /// Create a new GeneratedCode instance.
    #[allow(unused)]
    pub fn new(code: impl Into<String>, type_name: impl Into<String>) -> Self {
        let type_name = type_name.into();
        let schema_name = format!("{}Schema", type_name);
        Self {
            code: code.into(),
            type_name,
            schema_name,
            dependencies: Vec::new(),
        }
    }

    /// Set the schema name.
    #[allow(unused)]
    pub fn with_schema_name(mut self, name: impl Into<String>) -> Self {
        self.schema_name = name.into();
        self
    }

    /// Add dependencies.
    #[allow(unused)]
    pub fn with_dependencies(mut self, deps: Vec<String>) -> Self {
        self.dependencies = deps;
        self
    }

    /// Add a single dependency.
    #[allow(unused)]
    pub fn add_dependency(mut self, dep: impl Into<String>) -> Self {
        self.dependencies.push(dep.into());
        self
    }

    /// Check if this schema has dependencies.
    #[allow(unused)]
    pub fn has_dependencies(&self) -> bool {
        !self.dependencies.is_empty()
    }
}

/// Generator features for capability checking.
///
/// Used to query whether a generator supports specific features
/// before attempting to use them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GeneratorFeature {
    /// Support for generic type parameters
    #[allow(unused)]
    Generics,

    /// Support for circular/recursive type references
    #[allow(unused)]
    CircularReferences,

    /// Support for discriminated unions (tagged enums)
    #[allow(unused)]
    DiscriminatedUnions,

    /// Support for refinement/custom validation
    #[allow(unused)]
    Refinements,

    /// Support for type transformations
    #[allow(unused)]
    Transforms,

    /// Support for type coercion
    #[allow(unused)]
    Coercion,

    /// Support for lazy evaluation (for recursive types)
    #[allow(unused)]
    Lazy,

    /// Support for strict mode (no extra properties)
    #[allow(unused)]
    StrictMode,

    /// Support for passthrough mode (allow extra properties)
    #[allow(unused)]
    PassthroughMode,

    /// Support for nullable types
    #[allow(unused)]
    Nullable,

    /// Support for optional types
    #[allow(unused)]
    Optional,

    /// Support for default values
    #[allow(unused)]
    DefaultValues,

    /// Support for description/documentation
    #[allow(unused)]
    Descriptions,

    /// Support for deprecation markers
    #[allow(unused)]
    Deprecation,
}

impl GeneratorFeature {
    /// Get a human-readable name for this feature.
    #[allow(unused)]
    pub fn name(&self) -> &'static str {
        match self {
            GeneratorFeature::Generics => "Generics",
            GeneratorFeature::CircularReferences => "Circular References",
            GeneratorFeature::DiscriminatedUnions => "Discriminated Unions",
            GeneratorFeature::Refinements => "Refinements",
            GeneratorFeature::Transforms => "Transforms",
            GeneratorFeature::Coercion => "Coercion",
            GeneratorFeature::Lazy => "Lazy Evaluation",
            GeneratorFeature::StrictMode => "Strict Mode",
            GeneratorFeature::PassthroughMode => "Passthrough Mode",
            GeneratorFeature::Nullable => "Nullable Types",
            GeneratorFeature::Optional => "Optional Types",
            GeneratorFeature::DefaultValues => "Default Values",
            GeneratorFeature::Descriptions => "Descriptions",
            GeneratorFeature::Deprecation => "Deprecation",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generator_config_default() {
        let config = GeneratorConfig::default();
        assert!(matches!(config.output_style, OutputStyle::ConstExport));
        assert!(config.generate_types);
        assert!(config.generate_docs);
        assert!(matches!(config.indent, IndentStyle::Spaces2));
        assert!(matches!(config.line_ending, LineEnding::Lf));
        assert!(config.type_overrides.is_empty());
    }

    #[test]
    fn test_generator_config_builder() {
        let config = GeneratorConfig::new()
            .with_output_style(OutputStyle::FunctionExport)
            .with_generate_types(false)
            .with_indent(IndentStyle::Spaces4)
            .with_type_override("MyType", "z.custom()");

        assert!(matches!(config.output_style, OutputStyle::FunctionExport));
        assert!(!config.generate_types);
        assert!(matches!(config.indent, IndentStyle::Spaces4));
        assert_eq!(
            config.type_overrides.get("MyType"),
            Some(&"z.custom()".to_string())
        );
    }

    #[test]
    fn test_output_style() {
        assert!(OutputStyle::ConstExport.is_exported());
        assert!(OutputStyle::FunctionExport.is_exported());
        assert!(!OutputStyle::Declaration.is_exported());
        assert!(!OutputStyle::Inline.is_exported());

        assert!(!OutputStyle::ConstExport.is_function());
        assert!(OutputStyle::FunctionExport.is_function());
    }

    #[test]
    fn test_indent_style() {
        assert_eq!(IndentStyle::Spaces2.as_str(), "  ");
        assert_eq!(IndentStyle::Spaces4.as_str(), "    ");
        assert_eq!(IndentStyle::Tabs.as_str(), "\t");

        assert_eq!(IndentStyle::Spaces2.indent(2), "    ");
        assert_eq!(IndentStyle::Spaces4.indent(2), "        ");
    }

    #[test]
    fn test_line_ending() {
        assert_eq!(LineEnding::Lf.as_str(), "\n");
        assert_eq!(LineEnding::CrLf.as_str(), "\r\n");
    }

    #[test]
    fn test_generated_code() {
        let code = GeneratedCode::new("z.object({})", "User");
        assert_eq!(code.type_name, "User");
        assert_eq!(code.schema_name, "UserSchema");
        assert!(!code.has_dependencies());

        let code_with_deps = code.add_dependency("Address");
        assert!(code_with_deps.has_dependencies());
        assert_eq!(code_with_deps.dependencies, vec!["Address"]);
    }

    #[test]
    fn test_generator_feature_name() {
        assert_eq!(GeneratorFeature::Generics.name(), "Generics");
        assert_eq!(
            GeneratorFeature::DiscriminatedUnions.name(),
            "Discriminated Unions"
        );
        assert_eq!(
            GeneratorFeature::CircularReferences.name(),
            "Circular References"
        );
    }
}
