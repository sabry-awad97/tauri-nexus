//! Contract Generator for generating complete TypeScript contract files.
//!
//! The ContractGenerator combines schemas from a registry with a code generator
//! to produce complete, valid TypeScript files with proper imports and exports.

use std::collections::HashMap;

use crate::error::GeneratorError;
use crate::generator::registry::{CycleError, SchemaRegistry};
use crate::generator::traits::{CodeGenerator, GeneratorConfig};
use crate::ir::SchemaIR;

/// Error type for contract generation.
#[derive(Debug, Clone)]
pub enum ContractError {
    /// Circular dependency detected in schemas.
    CyclicDependency(CycleError),
    /// Error during code generation.
    GeneratorError(String),
}

impl std::fmt::Display for ContractError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContractError::CyclicDependency(e) => write!(f, "Cyclic dependency: {}", e),
            ContractError::GeneratorError(msg) => write!(f, "Generator error: {}", msg),
        }
    }
}

impl std::error::Error for ContractError {}

impl From<CycleError> for ContractError {
    fn from(e: CycleError) -> Self {
        ContractError::CyclicDependency(e)
    }
}

impl From<GeneratorError> for ContractError {
    fn from(e: GeneratorError) -> Self {
        ContractError::GeneratorError(e.to_string())
    }
}

/// Generates complete TypeScript contract files.
///
/// The ContractGenerator takes a code generator (like ZodEmitter) and configuration,
/// then produces complete contract files from a schema registry.
pub struct ContractGenerator<G: CodeGenerator> {
    /// The code generator to use for schema generation.
    generator: G,
    /// Configuration for code generation.
    config: GeneratorConfig,
}

impl<G: CodeGenerator> ContractGenerator<G> {
    /// Create a new ContractGenerator with the given generator and config.
    pub fn new(generator: G, config: GeneratorConfig) -> Self {
        Self { generator, config }
    }

    /// Get a reference to the underlying generator.
    pub fn generator(&self) -> &G {
        &self.generator
    }

    /// Get a reference to the configuration.
    pub fn config(&self) -> &GeneratorConfig {
        &self.config
    }

    /// Generate a complete contract file from a registry.
    ///
    /// This generates:
    /// 1. Preamble (imports)
    /// 2. All schemas in dependency order
    /// 3. Postamble (exports)
    pub fn generate(&self, registry: &SchemaRegistry) -> Result<String, ContractError> {
        let schemas = registry.sorted_schemas()?;
        let schema_refs: Vec<&SchemaIR> = schemas.to_vec();

        let mut output = String::new();

        // Generate preamble (imports)
        output.push_str(
            &self
                .generator
                .generate_preamble(&schema_refs, &self.config)?,
        );

        // Generate each schema
        for schema in &schemas {
            let generated = self.generator.generate(schema, &self.config)?;
            output.push_str(&generated.code);
            output.push('\n');
        }

        // Generate postamble (exports)
        output.push_str(
            &self
                .generator
                .generate_postamble(&schema_refs, &self.config)?,
        );

        Ok(output)
    }

    /// Generate a contract file with schemas grouped by namespace.
    ///
    /// Schemas are grouped based on their module path or a custom namespace attribute.
    pub fn generate_with_namespaces(
        &self,
        registry: &SchemaRegistry,
        namespace_map: &HashMap<String, String>,
    ) -> Result<String, ContractError> {
        let schemas = registry.sorted_schemas()?;
        let schema_refs: Vec<&SchemaIR> = schemas.to_vec();

        let mut output = String::new();

        // Generate preamble
        output.push_str(
            &self
                .generator
                .generate_preamble(&schema_refs, &self.config)?,
        );

        // Group schemas by namespace
        let mut namespaces: HashMap<Option<&str>, Vec<&SchemaIR>> = HashMap::new();
        for schema in &schemas {
            let ns = namespace_map.get(&schema.name).map(|s| s.as_str());
            namespaces.entry(ns).or_default().push(schema);
        }

        // Generate schemas without namespace first
        if let Some(root_schemas) = namespaces.get(&None) {
            for schema in root_schemas {
                let generated = self.generator.generate(schema, &self.config)?;
                output.push_str(&generated.code);
                output.push('\n');
            }
        }

        // Generate namespaced schemas
        let mut ns_keys: Vec<_> = namespaces.keys().filter_map(|k| *k).collect();
        ns_keys.sort();

        for ns in ns_keys {
            if let Some(ns_schemas) = namespaces.get(&Some(ns)) {
                output.push_str(&format!("\n// Namespace: {}\n", ns));
                output.push_str(&format!("export namespace {} {{\n", ns));

                for schema in ns_schemas {
                    let generated = self.generator.generate(schema, &self.config)?;
                    // Indent the generated code
                    for line in generated.code.lines() {
                        if !line.is_empty() {
                            output.push_str("  ");
                        }
                        output.push_str(line);
                        output.push('\n');
                    }
                }

                output.push_str("}\n");
            }
        }

        // Generate postamble
        output.push_str(
            &self
                .generator
                .generate_postamble(&schema_refs, &self.config)?,
        );

        Ok(output)
    }

    /// Generate only the schemas that are marked for export.
    pub fn generate_exports_only(
        &self,
        registry: &SchemaRegistry,
    ) -> Result<String, ContractError> {
        let schemas = registry.sorted_schemas()?;
        let exported: Vec<&SchemaIR> = schemas
            .iter()
            .filter(|s| registry.is_exported(&s.name))
            .copied()
            .collect();

        let mut output = String::new();

        // Generate preamble
        output.push_str(&self.generator.generate_preamble(&exported, &self.config)?);

        // Generate each exported schema
        for schema in &exported {
            let generated = self.generator.generate(schema, &self.config)?;
            output.push_str(&generated.code);
            output.push('\n');
        }

        // Generate postamble
        output.push_str(&self.generator.generate_postamble(&exported, &self.config)?);

        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator::traits::{GeneratedCode, GeneratorFeature, OutputStyle};
    use crate::ir::{FieldIR, SchemaIR, SchemaKind, StructSchema, TypeIR, TypeKind};

    /// A simple mock generator for testing.
    struct MockGenerator;

    impl CodeGenerator for MockGenerator {
        fn id(&self) -> &'static str {
            "mock"
        }

        fn name(&self) -> &'static str {
            "Mock Generator"
        }

        fn file_extension(&self) -> &'static str {
            "ts"
        }

        fn generate(
            &self,
            schema: &SchemaIR,
            _config: &GeneratorConfig,
        ) -> Result<GeneratedCode, GeneratorError> {
            Ok(GeneratedCode::new(
                format!("export const {}Schema = mock();", schema.name),
                schema.name.clone(),
            ))
        }

        fn generate_preamble(
            &self,
            _schemas: &[&SchemaIR],
            _config: &GeneratorConfig,
        ) -> Result<String, GeneratorError> {
            Ok("// Mock preamble\n".to_string())
        }

        fn generate_postamble(
            &self,
            schemas: &[&SchemaIR],
            _config: &GeneratorConfig,
        ) -> Result<String, GeneratorError> {
            let names: Vec<_> = schemas
                .iter()
                .map(|s| format!("{}Schema", s.name))
                .collect();
            Ok(format!("\n// Exports: {}\n", names.join(", ")))
        }

        fn supports_feature(&self, _feature: GeneratorFeature) -> bool {
            true
        }
    }

    fn make_simple_struct(name: &str) -> SchemaIR {
        SchemaIR::new(
            name,
            SchemaKind::Struct(StructSchema::new(vec![FieldIR::new(
                "id",
                TypeIR::new(TypeKind::String),
            )])),
        )
    }

    fn make_struct_with_ref(name: &str, ref_name: &str) -> SchemaIR {
        SchemaIR::new(
            name,
            SchemaKind::Struct(StructSchema::new(vec![FieldIR::new(
                "ref_field",
                TypeIR::new(TypeKind::Reference {
                    name: ref_name.to_string(),
                    generics: vec![],
                }),
            )])),
        )
    }

    #[test]
    fn test_contract_generator_new() {
        let generator = MockGenerator;
        let config = GeneratorConfig::default();
        let contract = ContractGenerator::new(generator, config);

        assert_eq!(contract.generator().id(), "mock");
    }

    #[test]
    fn test_generate_empty_registry() {
        let generator = MockGenerator;
        let config = GeneratorConfig::default();
        let contract = ContractGenerator::new(generator, config);
        let registry = SchemaRegistry::new();

        let result = contract.generate(&registry);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(output.contains("// Mock preamble"));
        assert!(output.contains("// Exports:"));
    }

    #[test]
    fn test_generate_single_schema() {
        let generator = MockGenerator;
        let config = GeneratorConfig::default();
        let contract = ContractGenerator::new(generator, config);

        let mut registry = SchemaRegistry::new();
        registry.register(make_simple_struct("User"));

        let result = contract.generate(&registry);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(output.contains("// Mock preamble"));
        assert!(output.contains("export const UserSchema = mock();"));
        assert!(output.contains("UserSchema"));
    }

    #[test]
    fn test_generate_multiple_schemas_in_order() {
        let generator = MockGenerator;
        let config = GeneratorConfig::default();
        let contract = ContractGenerator::new(generator, config);

        let mut registry = SchemaRegistry::new();
        // Register in reverse dependency order
        registry.register(make_struct_with_ref("Post", "User"));
        registry.register(make_simple_struct("User"));

        let result = contract.generate(&registry);
        assert!(result.is_ok());

        let output = result.unwrap();
        // User should come before Post in the output
        let user_pos = output.find("UserSchema = mock()").unwrap();
        let post_pos = output.find("PostSchema = mock()").unwrap();
        assert!(user_pos < post_pos, "User should be generated before Post");
    }

    #[test]
    fn test_generate_with_cycle_returns_error() {
        let generator = MockGenerator;
        let config = GeneratorConfig::default();
        let contract = ContractGenerator::new(generator, config);

        let mut registry = SchemaRegistry::new();
        registry.register(make_struct_with_ref("A", "B"));
        registry.register(make_struct_with_ref("B", "A"));

        let result = contract.generate(&registry);
        assert!(result.is_err());

        match result.unwrap_err() {
            ContractError::CyclicDependency(_) => {}
            _ => panic!("Expected CyclicDependency error"),
        }
    }

    #[test]
    fn test_generate_exports_only() {
        let generator = MockGenerator;
        let config = GeneratorConfig::default();
        let contract = ContractGenerator::new(generator, config);

        let mut registry = SchemaRegistry::new();
        registry.register(make_simple_struct("Public"));
        registry.register(make_simple_struct("Internal").with_export(false));

        let result = contract.generate_exports_only(&registry);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(output.contains("PublicSchema"));
        assert!(!output.contains("InternalSchema"));
    }

    #[test]
    fn test_generate_with_namespaces() {
        let generator = MockGenerator;
        let config = GeneratorConfig::default();
        let contract = ContractGenerator::new(generator, config);

        let mut registry = SchemaRegistry::new();
        registry.register(make_simple_struct("User"));
        registry.register(make_simple_struct("Post"));
        registry.register(make_simple_struct("Comment"));

        let mut namespace_map = HashMap::new();
        namespace_map.insert("Post".to_string(), "blog".to_string());
        namespace_map.insert("Comment".to_string(), "blog".to_string());

        let result = contract.generate_with_namespaces(&registry, &namespace_map);
        assert!(result.is_ok());

        let output = result.unwrap();
        // User should be at root level
        assert!(output.contains("UserSchema"));
        // Post and Comment should be in blog namespace
        assert!(output.contains("namespace blog"));
    }

    #[test]
    fn test_contract_error_display() {
        let err = ContractError::GeneratorError("test error".to_string());
        assert!(format!("{}", err).contains("test error"));
    }
}
