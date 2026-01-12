//! Zod code emitter.
//!
//! This module implements the Zod schema code generator that transforms
//! the intermediate representation (IR) into TypeScript Zod schema code.
//!
//! # Features
//!
//! - Generates `z.object()` for structs with named fields
//! - Generates `z.enum()` for unit-only enums
//! - Generates `z.discriminatedUnion()` for tagged enums
//! - Generates `z.union()` for untagged enums
//! - Applies validation methods (`.min()`, `.max()`, `.email()`, etc.)
//! - Supports `.optional()`, `.nullable()`, `.default()`, `.describe()`
//! - Generates type inference with `z.infer<typeof Schema>`

use crate::error::GeneratorError;
use crate::generator::traits::{
    CodeGenerator, GeneratedCode, GeneratorConfig, GeneratorFeature, OutputStyle,
};
use crate::generator::zod::type_mapper::ZodTypeMapper;
use crate::ir::{
    EnumSchema, EnumTagging, FieldIR, SchemaIR, SchemaKind, StructSchema, TupleStructSchema,
    ValidationRule, VariantIR, VariantKind,
};

/// Zod schema code generator.
///
/// Implements the [`CodeGenerator`] trait to transform IR into TypeScript
/// Zod schema code.
///
/// # Example
///
/// ```rust,ignore
/// use zod_rs_macros::generator::zod::ZodEmitter;
/// use zod_rs_macros::generator::{CodeGenerator, GeneratorConfig};
/// use zod_rs_macros::ir::SchemaIR;
///
/// let emitter = ZodEmitter::new();
/// let config = GeneratorConfig::default();
/// let schema = // ... create SchemaIR ...
/// let generated = emitter.generate(&schema, &config)?;
/// println!("{}", generated.code);
/// ```
#[derive(Debug, Clone)]
pub struct ZodEmitter {
    /// Type mapper for converting TypeIR to Zod schema strings
    type_mapper: ZodTypeMapper,
}

impl Default for ZodEmitter {
    fn default() -> Self {
        Self::new()
    }
}

impl ZodEmitter {
    /// Create a new ZodEmitter with default settings.
    pub fn new() -> Self {
        Self {
            type_mapper: ZodTypeMapper::new(),
        }
    }

    /// Create a ZodEmitter with a custom type mapper.
    pub fn with_type_mapper(type_mapper: ZodTypeMapper) -> Self {
        Self { type_mapper }
    }

    // =========================================================================
    // Struct Generation (Task 10.2)
    // =========================================================================

    /// Generate Zod schema for a struct.
    fn generate_struct(
        &self,
        schema: &SchemaIR,
        s: &StructSchema,
        config: &GeneratorConfig,
    ) -> String {
        let mut fields = Vec::new();

        for field in &s.fields {
            let field_schema = self.generate_field(field, config);
            fields.push(format!(
                "{}{}: {}",
                config.indent.as_str(),
                field.schema_name,
                field_schema
            ));
        }

        let fields_str = if fields.is_empty() {
            "{}".to_string()
        } else {
            format!(
                "{{{}\n{}\n}}",
                config.line_ending.as_str(),
                fields.join(&format!(",{}", config.line_ending.as_str()))
            )
        };

        let mut result = format!("z.object({})", fields_str);

        // Apply strict or passthrough mode
        if s.strict {
            result.push_str(".strict()");
        } else if s.passthrough {
            result.push_str(".passthrough()");
        }

        // Apply description from metadata
        if config.generate_docs {
            if let Some(desc) = &schema.metadata.description {
                result.push_str(&format!(".describe(\"{}\")", escape_string(desc)));
            }
        }

        result
    }

    /// Generate Zod schema for a tuple struct.
    fn generate_tuple_struct(
        &self,
        schema: &SchemaIR,
        ts: &TupleStructSchema,
        config: &GeneratorConfig,
    ) -> String {
        if ts.fields.is_empty() {
            return "z.tuple([])".to_string();
        }

        let field_schemas: Vec<String> = ts
            .fields
            .iter()
            .map(|f| self.type_mapper.map_type(f))
            .collect();

        let mut result = format!("z.tuple([{}])", field_schemas.join(", "));

        // Apply description from metadata
        if config.generate_docs {
            if let Some(desc) = &schema.metadata.description {
                result.push_str(&format!(".describe(\"{}\")", escape_string(desc)));
            }
        }

        result
    }

    // =========================================================================
    // Field Generation (Task 10.3)
    // =========================================================================

    /// Generate Zod schema for a field.
    fn generate_field(&self, field: &FieldIR, config: &GeneratorConfig) -> String {
        let mut schema = self.type_mapper.map_type(&field.ty);

        // Apply validation rules (Task 10.4)
        for rule in &field.validation {
            schema = self.apply_validation(schema, rule);
        }

        // Apply nullable modifier
        if field.nullable {
            schema.push_str(".nullable()");
        }

        // Apply optional modifier
        if field.optional {
            schema.push_str(".optional()");
        }

        // Apply default value
        if let Some(default) = &field.default {
            schema.push_str(&format!(".default({})", default));
        }

        // Apply description
        if config.generate_docs {
            if let Some(desc) = &field.metadata.description {
                schema.push_str(&format!(".describe(\"{}\")", escape_string(desc)));
            }
        }

        schema
    }

    // =========================================================================
    // Validation Application (Task 10.4)
    // =========================================================================

    /// Apply a validation rule to a schema string.
    fn apply_validation(&self, mut schema: String, rule: &ValidationRule) -> String {
        match rule {
            // String validations
            ValidationRule::MinLength(n) => schema.push_str(&format!(".min({})", n)),
            ValidationRule::MaxLength(n) => schema.push_str(&format!(".max({})", n)),
            ValidationRule::Length(n) => schema.push_str(&format!(".length({})", n)),
            ValidationRule::Email => schema.push_str(".email()"),
            ValidationRule::Url => schema.push_str(".url()"),
            ValidationRule::Uuid => schema.push_str(".uuid()"),
            ValidationRule::Cuid => schema.push_str(".cuid()"),
            ValidationRule::Cuid2 => schema.push_str(".cuid2()"),
            ValidationRule::Ulid => schema.push_str(".ulid()"),
            ValidationRule::Regex(pattern) => {
                schema.push_str(&format!(".regex(/{}/))", escape_regex(pattern)))
            }
            ValidationRule::StartsWith(prefix) => {
                schema.push_str(&format!(".startsWith(\"{}\")", escape_string(prefix)))
            }
            ValidationRule::EndsWith(suffix) => {
                schema.push_str(&format!(".endsWith(\"{}\")", escape_string(suffix)))
            }
            ValidationRule::Includes(substring) => {
                schema.push_str(&format!(".includes(\"{}\")", escape_string(substring)))
            }
            ValidationRule::Datetime => schema.push_str(".datetime()"),
            ValidationRule::Ip => schema.push_str(".ip()"),
            ValidationRule::Ipv4 => schema.push_str(".ip({ version: \"v4\" })"),
            ValidationRule::Ipv6 => schema.push_str(".ip({ version: \"v6\" })"),
            ValidationRule::Emoji => schema.push_str(".emoji()"),
            ValidationRule::Trim => schema.push_str(".trim()"),
            ValidationRule::ToLowerCase => schema.push_str(".toLowerCase()"),
            ValidationRule::ToUpperCase => schema.push_str(".toUpperCase()"),

            // Number validations
            ValidationRule::Min(n) => schema.push_str(&format!(".min({})", n)),
            ValidationRule::Max(n) => schema.push_str(&format!(".max({})", n)),
            ValidationRule::GreaterThan(n) => schema.push_str(&format!(".gt({})", n)),
            ValidationRule::LessThan(n) => schema.push_str(&format!(".lt({})", n)),
            ValidationRule::Positive => schema.push_str(".positive()"),
            ValidationRule::Negative => schema.push_str(".negative()"),
            ValidationRule::NonNegative => schema.push_str(".nonnegative()"),
            ValidationRule::NonPositive => schema.push_str(".nonpositive()"),
            ValidationRule::Int => schema.push_str(".int()"),
            ValidationRule::Finite => schema.push_str(".finite()"),
            ValidationRule::Safe => schema.push_str(".safe()"),
            ValidationRule::MultipleOf(n) => schema.push_str(&format!(".multipleOf({})", n)),

            // Array validations
            ValidationRule::MinItems(n) => schema.push_str(&format!(".min({})", n)),
            ValidationRule::MaxItems(n) => schema.push_str(&format!(".max({})", n)),
            ValidationRule::ItemsLength(n) => schema.push_str(&format!(".length({})", n)),
            ValidationRule::Nonempty => schema.push_str(".nonempty()"),

            // Custom validations
            ValidationRule::Custom(expr) => schema.push_str(&format!(".refine({})", expr)),
            ValidationRule::Refine {
                expression,
                message,
            } => {
                if let Some(msg) = message {
                    schema.push_str(&format!(
                        ".refine({}, {{ message: \"{}\" }})",
                        expression,
                        escape_string(msg)
                    ));
                } else {
                    schema.push_str(&format!(".refine({})", expression));
                }
            }
            ValidationRule::Transform(expr) => schema.push_str(&format!(".transform({})", expr)),
            ValidationRule::SuperRefine(expr) => {
                schema.push_str(&format!(".superRefine({})", expr))
            }
        }
        schema
    }

    // =========================================================================
    // Enum Generation (Task 10.5)
    // =========================================================================

    /// Generate Zod schema for an enum.
    fn generate_enum(&self, schema: &SchemaIR, e: &EnumSchema, config: &GeneratorConfig) -> String {
        let mut result = if e.is_unit_only {
            // Simple z.enum for unit-only enums
            self.generate_unit_enum(e)
        } else {
            // Discriminated union or union for data enums
            match &e.tagging {
                EnumTagging::Internal { tag } => self.generate_internal_tagged_enum(e, tag),
                EnumTagging::Adjacent { tag, content } => {
                    self.generate_adjacent_tagged_enum(e, tag, content)
                }
                EnumTagging::External => self.generate_external_tagged_enum(e),
                EnumTagging::Untagged => self.generate_untagged_enum(e),
            }
        };

        // Apply description from metadata
        if config.generate_docs {
            if let Some(desc) = &schema.metadata.description {
                result.push_str(&format!(".describe(\"{}\")", escape_string(desc)));
            }
        }

        result
    }

    /// Generate z.enum() for unit-only enums.
    fn generate_unit_enum(&self, e: &EnumSchema) -> String {
        let variants: Vec<String> = e
            .variants
            .iter()
            .map(|v| format!("\"{}\"", v.schema_name))
            .collect();
        format!("z.enum([{}])", variants.join(", "))
    }

    /// Generate z.discriminatedUnion() for internally tagged enums.
    fn generate_internal_tagged_enum(&self, e: &EnumSchema, tag: &str) -> String {
        let variants: Vec<String> = e
            .variants
            .iter()
            .map(|v| self.generate_variant_internal(v, tag))
            .collect();
        format!(
            "z.discriminatedUnion(\"{}\", [\n{}\n])",
            tag,
            variants.join(",\n")
        )
    }

    /// Generate variant schema for internally tagged enum.
    fn generate_variant_internal(&self, variant: &VariantIR, tag: &str) -> String {
        match &variant.kind {
            VariantKind::Unit => {
                format!(
                    "  z.object({{ {}: z.literal(\"{}\") }})",
                    tag, variant.schema_name
                )
            }
            VariantKind::Tuple(fields) => {
                if fields.len() == 1 {
                    // Single-element tuple: merge with tag
                    let field_schema = self.type_mapper.map_type(&fields[0]);
                    format!(
                        "  z.object({{ {}: z.literal(\"{}\"), value: {} }})",
                        tag, variant.schema_name, field_schema
                    )
                } else {
                    // Multi-element tuple: use array
                    let field_schemas: Vec<String> = fields
                        .iter()
                        .map(|f| self.type_mapper.map_type(f))
                        .collect();
                    format!(
                        "  z.object({{ {}: z.literal(\"{}\"), value: z.tuple([{}]) }})",
                        tag,
                        variant.schema_name,
                        field_schemas.join(", ")
                    )
                }
            }
            VariantKind::Struct(fields) => {
                let field_schemas: Vec<String> = fields
                    .iter()
                    .map(|f| {
                        format!(
                            "{}: {}",
                            f.schema_name,
                            self.generate_field(f, &GeneratorConfig::default())
                        )
                    })
                    .collect();
                format!(
                    "  z.object({{ {}: z.literal(\"{}\"), {} }})",
                    tag,
                    variant.schema_name,
                    field_schemas.join(", ")
                )
            }
        }
    }

    /// Generate z.discriminatedUnion() for adjacently tagged enums.
    fn generate_adjacent_tagged_enum(&self, e: &EnumSchema, tag: &str, content: &str) -> String {
        let variants: Vec<String> = e
            .variants
            .iter()
            .map(|v| self.generate_variant_adjacent(v, tag, content))
            .collect();
        format!(
            "z.discriminatedUnion(\"{}\", [\n{}\n])",
            tag,
            variants.join(",\n")
        )
    }

    /// Generate variant schema for adjacently tagged enum.
    fn generate_variant_adjacent(&self, variant: &VariantIR, tag: &str, content: &str) -> String {
        match &variant.kind {
            VariantKind::Unit => {
                format!(
                    "  z.object({{ {}: z.literal(\"{}\") }})",
                    tag, variant.schema_name
                )
            }
            VariantKind::Tuple(fields) => {
                let content_schema = if fields.len() == 1 {
                    self.type_mapper.map_type(&fields[0])
                } else {
                    let field_schemas: Vec<String> = fields
                        .iter()
                        .map(|f| self.type_mapper.map_type(f))
                        .collect();
                    format!("z.tuple([{}])", field_schemas.join(", "))
                };
                format!(
                    "  z.object({{ {}: z.literal(\"{}\"), {}: {} }})",
                    tag, variant.schema_name, content, content_schema
                )
            }
            VariantKind::Struct(fields) => {
                let field_schemas: Vec<String> = fields
                    .iter()
                    .map(|f| {
                        format!(
                            "{}: {}",
                            f.schema_name,
                            self.generate_field(f, &GeneratorConfig::default())
                        )
                    })
                    .collect();
                format!(
                    "  z.object({{ {}: z.literal(\"{}\"), {}: z.object({{ {} }}) }})",
                    tag,
                    variant.schema_name,
                    content,
                    field_schemas.join(", ")
                )
            }
        }
    }

    /// Generate z.union() for externally tagged enums.
    fn generate_external_tagged_enum(&self, e: &EnumSchema) -> String {
        let variants: Vec<String> = e
            .variants
            .iter()
            .map(|v| self.generate_variant_external(v))
            .collect();
        format!("z.union([\n{}\n])", variants.join(",\n"))
    }

    /// Generate variant schema for externally tagged enum.
    fn generate_variant_external(&self, variant: &VariantIR) -> String {
        match &variant.kind {
            VariantKind::Unit => {
                format!("  z.literal(\"{}\")", variant.schema_name)
            }
            VariantKind::Tuple(fields) => {
                let content_schema = if fields.len() == 1 {
                    self.type_mapper.map_type(&fields[0])
                } else {
                    let field_schemas: Vec<String> = fields
                        .iter()
                        .map(|f| self.type_mapper.map_type(f))
                        .collect();
                    format!("z.tuple([{}])", field_schemas.join(", "))
                };
                format!(
                    "  z.object({{ \"{}\": {} }})",
                    variant.schema_name, content_schema
                )
            }
            VariantKind::Struct(fields) => {
                let field_schemas: Vec<String> = fields
                    .iter()
                    .map(|f| {
                        format!(
                            "{}: {}",
                            f.schema_name,
                            self.generate_field(f, &GeneratorConfig::default())
                        )
                    })
                    .collect();
                format!(
                    "  z.object({{ \"{}\": z.object({{ {} }}) }})",
                    variant.schema_name,
                    field_schemas.join(", ")
                )
            }
        }
    }

    /// Generate z.union() for untagged enums.
    fn generate_untagged_enum(&self, e: &EnumSchema) -> String {
        let variants: Vec<String> = e
            .variants
            .iter()
            .map(|v| self.generate_variant_untagged(v))
            .collect();
        format!("z.union([\n{}\n])", variants.join(",\n"))
    }

    /// Generate variant schema for untagged enum.
    fn generate_variant_untagged(&self, variant: &VariantIR) -> String {
        match &variant.kind {
            VariantKind::Unit => {
                format!("  z.literal(\"{}\")", variant.schema_name)
            }
            VariantKind::Tuple(fields) => {
                if fields.len() == 1 {
                    format!("  {}", self.type_mapper.map_type(&fields[0]))
                } else {
                    let field_schemas: Vec<String> = fields
                        .iter()
                        .map(|f| self.type_mapper.map_type(f))
                        .collect();
                    format!("  z.tuple([{}])", field_schemas.join(", "))
                }
            }
            VariantKind::Struct(fields) => {
                let field_schemas: Vec<String> = fields
                    .iter()
                    .map(|f| {
                        format!(
                            "{}: {}",
                            f.schema_name,
                            self.generate_field(f, &GeneratorConfig::default())
                        )
                    })
                    .collect();
                format!("  z.object({{ {} }})", field_schemas.join(", "))
            }
        }
    }

    // =========================================================================
    // Helper Methods
    // =========================================================================

    /// Extract dependencies from a schema.
    fn extract_dependencies(&self, schema: &SchemaIR) -> Vec<String> {
        let mut deps = Vec::new();
        self.collect_schema_deps(&schema.kind, &mut deps);
        deps
    }

    /// Collect type dependencies from a schema kind.
    fn collect_schema_deps(&self, kind: &SchemaKind, deps: &mut Vec<String>) {
        match kind {
            SchemaKind::Struct(s) => {
                for field in &s.fields {
                    self.collect_type_deps(&field.ty, deps);
                }
            }
            SchemaKind::TupleStruct(ts) => {
                for field in &ts.fields {
                    self.collect_type_deps(field, deps);
                }
            }
            SchemaKind::Enum(e) => {
                for variant in &e.variants {
                    match &variant.kind {
                        VariantKind::Tuple(fields) => {
                            for field in fields {
                                self.collect_type_deps(field, deps);
                            }
                        }
                        VariantKind::Struct(fields) => {
                            for field in fields {
                                self.collect_type_deps(&field.ty, deps);
                            }
                        }
                        VariantKind::Unit => {}
                    }
                }
            }
            SchemaKind::Alias(ty) => {
                self.collect_type_deps(ty, deps);
            }
            SchemaKind::UnitStruct => {}
        }
    }

    /// Collect type dependencies from a type IR.
    fn collect_type_deps(&self, ty: &crate::ir::TypeIR, deps: &mut Vec<String>) {
        use crate::ir::TypeKind;

        match &ty.kind {
            TypeKind::Reference { name, generics } => {
                if !deps.contains(name) {
                    deps.push(name.clone());
                }
                for generic in generics {
                    self.collect_type_deps(generic, deps);
                }
            }
            TypeKind::Array(inner) | TypeKind::Set(inner) | TypeKind::Optional(inner) => {
                self.collect_type_deps(inner, deps);
            }
            TypeKind::Tuple(elements) => {
                for elem in elements {
                    self.collect_type_deps(elem, deps);
                }
            }
            TypeKind::Record { key, value } => {
                self.collect_type_deps(key, deps);
                self.collect_type_deps(value, deps);
            }
            TypeKind::Union(types) | TypeKind::Intersection(types) => {
                for t in types {
                    self.collect_type_deps(t, deps);
                }
            }
            _ => {}
        }
    }

    /// Generate the schema string for a schema IR (without export wrapper).
    pub fn generate_schema_string(&self, schema: &SchemaIR) -> String {
        let config = GeneratorConfig::default();
        match &schema.kind {
            SchemaKind::Struct(s) => self.generate_struct(schema, s, &config),
            SchemaKind::TupleStruct(ts) => self.generate_tuple_struct(schema, ts, &config),
            SchemaKind::UnitStruct => "z.object({})".to_string(),
            SchemaKind::Enum(e) => self.generate_enum(schema, e, &config),
            SchemaKind::Alias(ty) => self.type_mapper.map_type(ty),
        }
    }
}

// =============================================================================
// CodeGenerator Implementation (Tasks 10.1, 10.6, 10.7)
// =============================================================================

impl CodeGenerator for ZodEmitter {
    fn id(&self) -> &'static str {
        "zod"
    }

    fn name(&self) -> &'static str {
        "Zod Schema Generator"
    }

    fn file_extension(&self) -> &'static str {
        "ts"
    }

    fn generate(
        &self,
        schema: &SchemaIR,
        config: &GeneratorConfig,
    ) -> Result<GeneratedCode, GeneratorError> {
        let code = match &schema.kind {
            SchemaKind::Struct(s) => self.generate_struct(schema, s, config),
            SchemaKind::TupleStruct(ts) => self.generate_tuple_struct(schema, ts, config),
            SchemaKind::UnitStruct => "z.object({})".to_string(),
            SchemaKind::Enum(e) => self.generate_enum(schema, e, config),
            SchemaKind::Alias(ty) => self.type_mapper.map_type(ty),
        };

        let schema_name = format!("{}Schema", schema.name);
        let type_name = schema.name.clone();

        let full_code = match config.output_style {
            OutputStyle::ConstExport => {
                let mut result = String::new();

                // Add JSDoc comment if enabled
                if config.generate_docs {
                    if let Some(desc) = &schema.metadata.description {
                        result.push_str(&format!("/** {} */\n", desc));
                    }
                    if schema.metadata.deprecated {
                        if let Some(msg) = &schema.metadata.deprecation_message {
                            result.push_str(&format!("/** @deprecated {} */\n", msg));
                        } else {
                            result.push_str("/** @deprecated */\n");
                        }
                    }
                }

                // Generate schema export
                result.push_str(&format!("export const {} = {};\n", schema_name, code));

                // Generate type inference (Task 10.7)
                if config.generate_types {
                    result.push_str(&format!(
                        "export type {} = z.infer<typeof {}>;\n",
                        type_name, schema_name
                    ));
                }

                result
            }
            OutputStyle::FunctionExport => {
                let mut result = String::new();

                if config.generate_docs {
                    if let Some(desc) = &schema.metadata.description {
                        result.push_str(&format!("/** {} */\n", desc));
                    }
                }

                result.push_str(&format!(
                    "export function {}() {{ return {}; }}\n",
                    schema_name, code
                ));

                if config.generate_types {
                    result.push_str(&format!(
                        "export type {} = z.infer<ReturnType<typeof {}>>;\n",
                        type_name, schema_name
                    ));
                }

                result
            }
            OutputStyle::Declaration => {
                let mut result = String::new();

                if config.generate_docs {
                    if let Some(desc) = &schema.metadata.description {
                        result.push_str(&format!("/** {} */\n", desc));
                    }
                }

                result.push_str(&format!("const {} = {};\n", schema_name, code));

                if config.generate_types {
                    result.push_str(&format!(
                        "type {} = z.infer<typeof {}>;\n",
                        type_name, schema_name
                    ));
                }

                result
            }
            OutputStyle::Inline => code,
        };

        Ok(GeneratedCode {
            code: full_code,
            type_name,
            schema_name,
            dependencies: self.extract_dependencies(schema),
        })
    }

    /// Generate preamble (imports) for the output file (Task 10.6).
    fn generate_preamble(
        &self,
        _schemas: &[&SchemaIR],
        _config: &GeneratorConfig,
    ) -> Result<String, GeneratorError> {
        Ok("import { z } from 'zod';\n\n".to_string())
    }

    /// Generate postamble (exports) for the output file (Task 10.6).
    fn generate_postamble(
        &self,
        schemas: &[&SchemaIR],
        _config: &GeneratorConfig,
    ) -> Result<String, GeneratorError> {
        let exports: Vec<String> = schemas
            .iter()
            .filter(|s| s.export)
            .map(|s| format!("  {}Schema", s.name))
            .collect();

        if exports.is_empty() {
            return Ok(String::new());
        }

        Ok(format!(
            "\nexport const schemas = {{\n{}\n}};\n",
            exports.join(",\n")
        ))
    }

    fn supports_feature(&self, feature: GeneratorFeature) -> bool {
        matches!(
            feature,
            GeneratorFeature::DiscriminatedUnions
                | GeneratorFeature::Refinements
                | GeneratorFeature::Transforms
                | GeneratorFeature::Coercion
                | GeneratorFeature::Lazy
                | GeneratorFeature::StrictMode
                | GeneratorFeature::PassthroughMode
                | GeneratorFeature::Nullable
                | GeneratorFeature::Optional
                | GeneratorFeature::DefaultValues
                | GeneratorFeature::Descriptions
                | GeneratorFeature::Deprecation
        )
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Escape a string for use in JavaScript/TypeScript.
fn escape_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

/// Escape a regex pattern for use in JavaScript.
fn escape_regex(pattern: &str) -> String {
    // For regex patterns, we don't escape the pattern itself,
    // but we need to handle forward slashes
    pattern.replace('/', "\\/")
}

// =============================================================================
// Unit Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{
        EnumSchema, EnumTagging, FieldIR, FieldMetadata, SchemaIR, SchemaKind, StructSchema,
        TupleStructSchema, TypeIR, TypeKind, ValidationRule, VariantIR, VariantKind,
    };

    fn emitter() -> ZodEmitter {
        ZodEmitter::new()
    }

    fn config() -> GeneratorConfig {
        GeneratorConfig::default()
    }

    // =========================================================================
    // Task 10.1: ZodEmitter struct implementing CodeGenerator
    // =========================================================================

    #[test]
    fn test_emitter_id() {
        assert_eq!(emitter().id(), "zod");
    }

    #[test]
    fn test_emitter_name() {
        assert_eq!(emitter().name(), "Zod Schema Generator");
    }

    #[test]
    fn test_emitter_file_extension() {
        assert_eq!(emitter().file_extension(), "ts");
    }

    #[test]
    fn test_emitter_supports_features() {
        let e = emitter();
        assert!(e.supports_feature(GeneratorFeature::DiscriminatedUnions));
        assert!(e.supports_feature(GeneratorFeature::Refinements));
        assert!(e.supports_feature(GeneratorFeature::Lazy));
        assert!(e.supports_feature(GeneratorFeature::StrictMode));
        assert!(e.supports_feature(GeneratorFeature::Nullable));
        assert!(e.supports_feature(GeneratorFeature::Optional));
        assert!(e.supports_feature(GeneratorFeature::Descriptions));
        assert!(!e.supports_feature(GeneratorFeature::Generics));
        assert!(!e.supports_feature(GeneratorFeature::CircularReferences));
    }

    // =========================================================================
    // Task 10.2: Struct schema generation
    // =========================================================================

    #[test]
    fn test_generate_empty_struct() {
        let schema = SchemaIR::new("Empty", SchemaKind::Struct(StructSchema::new(vec![])));
        let result = emitter().generate(&schema, &config()).unwrap();
        assert!(result.code.contains("z.object({})"));
    }

    #[test]
    fn test_generate_struct_with_fields() {
        let fields = vec![
            FieldIR::new("id", TypeIR::new(TypeKind::signed_int(64))),
            FieldIR::new("name", TypeIR::new(TypeKind::String)),
        ];
        let schema = SchemaIR::new("User", SchemaKind::Struct(StructSchema::new(fields)));
        let result = emitter().generate(&schema, &config()).unwrap();
        assert!(result.code.contains("z.object("));
        assert!(result.code.contains("id: z.number().int()"));
        assert!(result.code.contains("name: z.string()"));
    }

    #[test]
    fn test_generate_struct_strict() {
        let schema = SchemaIR::new(
            "Strict",
            SchemaKind::Struct(StructSchema::new(vec![]).with_strict(true)),
        );
        let result = emitter().generate(&schema, &config()).unwrap();
        assert!(result.code.contains(".strict()"));
    }

    #[test]
    fn test_generate_struct_passthrough() {
        let schema = SchemaIR::new(
            "Passthrough",
            SchemaKind::Struct(StructSchema::new(vec![]).with_passthrough(true)),
        );
        let result = emitter().generate(&schema, &config()).unwrap();
        assert!(result.code.contains(".passthrough()"));
    }

    // =========================================================================
    // Task 10.3: Field schema generation
    // =========================================================================

    #[test]
    fn test_generate_field_optional() {
        let field = FieldIR::new("email", TypeIR::new(TypeKind::String)).with_optional(true);
        let schema = SchemaIR::new("User", SchemaKind::Struct(StructSchema::new(vec![field])));
        let result = emitter().generate(&schema, &config()).unwrap();
        assert!(result.code.contains("email: z.string().optional()"));
    }

    #[test]
    fn test_generate_field_nullable() {
        let field = FieldIR::new("email", TypeIR::new(TypeKind::String)).with_nullable(true);
        let schema = SchemaIR::new("User", SchemaKind::Struct(StructSchema::new(vec![field])));
        let result = emitter().generate(&schema, &config()).unwrap();
        assert!(result.code.contains("email: z.string().nullable()"));
    }

    #[test]
    fn test_generate_field_default() {
        let field = FieldIR::new("count", TypeIR::new(TypeKind::signed_int(32))).with_default("0");
        let schema = SchemaIR::new(
            "Counter",
            SchemaKind::Struct(StructSchema::new(vec![field])),
        );
        let result = emitter().generate(&schema, &config()).unwrap();
        assert!(result.code.contains(".default(0)"));
    }

    #[test]
    fn test_generate_field_description() {
        let field = FieldIR::new("email", TypeIR::new(TypeKind::String))
            .with_metadata(FieldMetadata::with_description("User email address"));
        let schema = SchemaIR::new("User", SchemaKind::Struct(StructSchema::new(vec![field])));
        let result = emitter().generate(&schema, &config()).unwrap();
        assert!(result.code.contains(".describe(\"User email address\")"));
    }

    // =========================================================================
    // Task 10.4: Validation method application
    // =========================================================================

    #[test]
    fn test_validation_string_min_max() {
        let field = FieldIR::new("name", TypeIR::new(TypeKind::String))
            .add_validation(ValidationRule::MinLength(1))
            .add_validation(ValidationRule::MaxLength(100));
        let schema = SchemaIR::new("User", SchemaKind::Struct(StructSchema::new(vec![field])));
        let result = emitter().generate(&schema, &config()).unwrap();
        assert!(result.code.contains(".min(1)"));
        assert!(result.code.contains(".max(100)"));
    }

    #[test]
    fn test_validation_email() {
        let field = FieldIR::new("email", TypeIR::new(TypeKind::String))
            .add_validation(ValidationRule::Email);
        let schema = SchemaIR::new("User", SchemaKind::Struct(StructSchema::new(vec![field])));
        let result = emitter().generate(&schema, &config()).unwrap();
        assert!(result.code.contains(".email()"));
    }

    #[test]
    fn test_validation_url() {
        let field = FieldIR::new("website", TypeIR::new(TypeKind::String))
            .add_validation(ValidationRule::Url);
        let schema = SchemaIR::new("User", SchemaKind::Struct(StructSchema::new(vec![field])));
        let result = emitter().generate(&schema, &config()).unwrap();
        assert!(result.code.contains(".url()"));
    }

    #[test]
    fn test_validation_uuid() {
        let field =
            FieldIR::new("id", TypeIR::new(TypeKind::String)).add_validation(ValidationRule::Uuid);
        let schema = SchemaIR::new("User", SchemaKind::Struct(StructSchema::new(vec![field])));
        let result = emitter().generate(&schema, &config()).unwrap();
        assert!(result.code.contains(".uuid()"));
    }

    #[test]
    fn test_validation_regex() {
        let field = FieldIR::new("code", TypeIR::new(TypeKind::String))
            .add_validation(ValidationRule::Regex("^[A-Z]{3}$".to_string()));
        let schema = SchemaIR::new("Code", SchemaKind::Struct(StructSchema::new(vec![field])));
        let result = emitter().generate(&schema, &config()).unwrap();
        assert!(result.code.contains(".regex(/^[A-Z]{3}$/))"));
    }

    #[test]
    fn test_validation_number_min_max() {
        let field = FieldIR::new("age", TypeIR::new(TypeKind::signed_int(32)))
            .add_validation(ValidationRule::Min(0.0))
            .add_validation(ValidationRule::Max(150.0));
        let schema = SchemaIR::new("User", SchemaKind::Struct(StructSchema::new(vec![field])));
        let result = emitter().generate(&schema, &config()).unwrap();
        assert!(result.code.contains(".min(0)"));
        assert!(result.code.contains(".max(150)"));
    }

    #[test]
    fn test_validation_positive() {
        let field = FieldIR::new("count", TypeIR::new(TypeKind::signed_int(32)))
            .add_validation(ValidationRule::Positive);
        let schema = SchemaIR::new(
            "Counter",
            SchemaKind::Struct(StructSchema::new(vec![field])),
        );
        let result = emitter().generate(&schema, &config()).unwrap();
        assert!(result.code.contains(".positive()"));
    }

    #[test]
    fn test_validation_int() {
        let field =
            FieldIR::new("count", TypeIR::new(TypeKind::Float)).add_validation(ValidationRule::Int);
        let schema = SchemaIR::new(
            "Counter",
            SchemaKind::Struct(StructSchema::new(vec![field])),
        );
        let result = emitter().generate(&schema, &config()).unwrap();
        assert!(result.code.contains(".int()"));
    }

    #[test]
    fn test_validation_nonempty() {
        let inner = TypeIR::new(TypeKind::String);
        let field = FieldIR::new("items", TypeIR::new(TypeKind::Array(Box::new(inner))))
            .add_validation(ValidationRule::Nonempty);
        let schema = SchemaIR::new("List", SchemaKind::Struct(StructSchema::new(vec![field])));
        let result = emitter().generate(&schema, &config()).unwrap();
        assert!(result.code.contains(".nonempty()"));
    }

    // =========================================================================
    // Task 10.5: Enum schema generation
    // =========================================================================

    #[test]
    fn test_generate_unit_enum() {
        let variants = vec![
            VariantIR::unit("Active"),
            VariantIR::unit("Inactive"),
            VariantIR::unit("Pending"),
        ];
        let schema = SchemaIR::new("Status", SchemaKind::Enum(EnumSchema::new(variants)));
        let result = emitter().generate(&schema, &config()).unwrap();
        assert!(result
            .code
            .contains("z.enum([\"Active\", \"Inactive\", \"Pending\"])"));
    }

    #[test]
    fn test_generate_internal_tagged_enum() {
        let variants = vec![
            VariantIR::unit("None"),
            VariantIR::tuple("Some", vec![TypeIR::new(TypeKind::String)]),
        ];
        let mut enum_schema = EnumSchema::new(variants);
        enum_schema.tagging = EnumTagging::internal("type");
        enum_schema.is_unit_only = false;

        let schema = SchemaIR::new("Option", SchemaKind::Enum(enum_schema));
        let result = emitter().generate(&schema, &config()).unwrap();
        assert!(result.code.contains("z.discriminatedUnion(\"type\""));
        assert!(result.code.contains("z.literal(\"None\")"));
        assert!(result.code.contains("z.literal(\"Some\")"));
    }

    #[test]
    fn test_generate_adjacent_tagged_enum() {
        let variants = vec![
            VariantIR::unit("Empty"),
            VariantIR::tuple("Value", vec![TypeIR::new(TypeKind::signed_int(32))]),
        ];
        let mut enum_schema = EnumSchema::new(variants);
        enum_schema.tagging = EnumTagging::adjacent("tag", "content");
        enum_schema.is_unit_only = false;

        let schema = SchemaIR::new("Container", SchemaKind::Enum(enum_schema));
        let result = emitter().generate(&schema, &config()).unwrap();
        assert!(result.code.contains("z.discriminatedUnion(\"tag\""));
        assert!(result.code.contains("content:"));
    }

    #[test]
    fn test_generate_external_tagged_enum() {
        let variants = vec![
            VariantIR::unit("None"),
            VariantIR::tuple("Some", vec![TypeIR::new(TypeKind::String)]),
        ];
        let mut enum_schema = EnumSchema::new(variants);
        enum_schema.tagging = EnumTagging::External;
        enum_schema.is_unit_only = false;

        let schema = SchemaIR::new("Option", SchemaKind::Enum(enum_schema));
        let result = emitter().generate(&schema, &config()).unwrap();
        assert!(result.code.contains("z.union(["));
    }

    #[test]
    fn test_generate_untagged_enum() {
        let variants = vec![
            VariantIR::tuple("String", vec![TypeIR::new(TypeKind::String)]),
            VariantIR::tuple("Number", vec![TypeIR::new(TypeKind::signed_int(32))]),
        ];
        let mut enum_schema = EnumSchema::new(variants);
        enum_schema.tagging = EnumTagging::Untagged;
        enum_schema.is_unit_only = false;

        let schema = SchemaIR::new("Value", SchemaKind::Enum(enum_schema));
        let result = emitter().generate(&schema, &config()).unwrap();
        assert!(result.code.contains("z.union(["));
        assert!(result.code.contains("z.string()"));
        assert!(result.code.contains("z.number().int()"));
    }

    // =========================================================================
    // Task 10.6: Preamble and postamble generation
    // =========================================================================

    #[test]
    fn test_generate_preamble() {
        let preamble = emitter().generate_preamble(&[], &config()).unwrap();
        assert_eq!(preamble, "import { z } from 'zod';\n\n");
    }

    #[test]
    fn test_generate_postamble() {
        let schema1 = SchemaIR::new("User", SchemaKind::Struct(StructSchema::new(vec![])));
        let schema2 = SchemaIR::new("Post", SchemaKind::Struct(StructSchema::new(vec![])));
        let schemas: Vec<&SchemaIR> = vec![&schema1, &schema2];

        let postamble = emitter().generate_postamble(&schemas, &config()).unwrap();
        assert!(postamble.contains("export const schemas = {"));
        assert!(postamble.contains("UserSchema"));
        assert!(postamble.contains("PostSchema"));
    }

    #[test]
    fn test_generate_postamble_empty() {
        let postamble = emitter().generate_postamble(&[], &config()).unwrap();
        assert!(postamble.is_empty());
    }

    // =========================================================================
    // Task 10.7: Type inference generation
    // =========================================================================

    #[test]
    fn test_generate_type_inference() {
        let schema = SchemaIR::new("User", SchemaKind::Struct(StructSchema::new(vec![])));
        let result = emitter().generate(&schema, &config()).unwrap();
        assert!(result
            .code
            .contains("export type User = z.infer<typeof UserSchema>"));
    }

    #[test]
    fn test_generate_without_type_inference() {
        let schema = SchemaIR::new("User", SchemaKind::Struct(StructSchema::new(vec![])));
        let config = GeneratorConfig::default().with_generate_types(false);
        let result = emitter().generate(&schema, &config).unwrap();
        assert!(!result.code.contains("z.infer"));
    }

    // =========================================================================
    // Output style tests
    // =========================================================================

    #[test]
    fn test_output_style_const_export() {
        let schema = SchemaIR::new("User", SchemaKind::Struct(StructSchema::new(vec![])));
        let config = GeneratorConfig::default().with_output_style(OutputStyle::ConstExport);
        let result = emitter().generate(&schema, &config).unwrap();
        assert!(result.code.contains("export const UserSchema ="));
    }

    #[test]
    fn test_output_style_function_export() {
        let schema = SchemaIR::new("User", SchemaKind::Struct(StructSchema::new(vec![])));
        let config = GeneratorConfig::default().with_output_style(OutputStyle::FunctionExport);
        let result = emitter().generate(&schema, &config).unwrap();
        assert!(result.code.contains("export function UserSchema()"));
    }

    #[test]
    fn test_output_style_declaration() {
        let schema = SchemaIR::new("User", SchemaKind::Struct(StructSchema::new(vec![])));
        let config = GeneratorConfig::default().with_output_style(OutputStyle::Declaration);
        let result = emitter().generate(&schema, &config).unwrap();
        assert!(result.code.contains("const UserSchema ="));
        assert!(!result.code.contains("export const"));
    }

    #[test]
    fn test_output_style_inline() {
        let schema = SchemaIR::new("User", SchemaKind::Struct(StructSchema::new(vec![])));
        let config = GeneratorConfig::default().with_output_style(OutputStyle::Inline);
        let result = emitter().generate(&schema, &config).unwrap();
        assert!(result.code.starts_with("z.object("));
        assert!(!result.code.contains("export"));
        assert!(!result.code.contains("const"));
    }

    // =========================================================================
    // Helper function tests
    // =========================================================================

    #[test]
    fn test_escape_string() {
        assert_eq!(escape_string("hello"), "hello");
        assert_eq!(escape_string("hello\nworld"), "hello\\nworld");
        assert_eq!(escape_string("say \"hi\""), "say \\\"hi\\\"");
        assert_eq!(escape_string("tab\there"), "tab\\there");
    }

    #[test]
    fn test_escape_regex() {
        assert_eq!(escape_regex("^[a-z]+$"), "^[a-z]+$");
        assert_eq!(escape_regex("a/b/c"), "a\\/b\\/c");
    }

    // =========================================================================
    // Tuple struct tests
    // =========================================================================

    #[test]
    fn test_generate_tuple_struct() {
        let fields = vec![
            TypeIR::new(TypeKind::String),
            TypeIR::new(TypeKind::signed_int(32)),
        ];
        let schema = SchemaIR::new(
            "Point",
            SchemaKind::TupleStruct(TupleStructSchema::new(fields)),
        );
        let result = emitter().generate(&schema, &config()).unwrap();
        assert!(result
            .code
            .contains("z.tuple([z.string(), z.number().int()])"));
    }

    #[test]
    fn test_generate_empty_tuple_struct() {
        let schema = SchemaIR::new(
            "Unit",
            SchemaKind::TupleStruct(TupleStructSchema::new(vec![])),
        );
        let result = emitter().generate(&schema, &config()).unwrap();
        assert!(result.code.contains("z.tuple([])"));
    }

    // =========================================================================
    // Unit struct tests
    // =========================================================================

    #[test]
    fn test_generate_unit_struct() {
        let schema = SchemaIR::new("Unit", SchemaKind::UnitStruct);
        let result = emitter().generate(&schema, &config()).unwrap();
        assert!(result.code.contains("z.object({})"));
    }

    // =========================================================================
    // Dependency extraction tests
    // =========================================================================

    #[test]
    fn test_extract_dependencies() {
        let field = FieldIR::new(
            "user",
            TypeIR::new(TypeKind::Reference {
                name: "User".to_string(),
                generics: vec![],
            }),
        );
        let schema = SchemaIR::new("Post", SchemaKind::Struct(StructSchema::new(vec![field])));
        let result = emitter().generate(&schema, &config()).unwrap();
        assert!(result.dependencies.contains(&"User".to_string()));
    }
}

// =============================================================================
// Property-Based Tests
// =============================================================================

#[cfg(test)]
mod proptest_tests {
    use super::*;
    use crate::ir::{
        EnumSchema, FieldIR, FieldMetadata, SchemaIR, SchemaKind, StructSchema, TypeIR, TypeKind,
        ValidationRule, VariantIR,
    };
    use proptest::prelude::*;

    /// Strategy for generating arbitrary validation rules.
    fn arb_string_validation() -> impl Strategy<Value = ValidationRule> {
        prop_oneof![
            (1usize..100).prop_map(ValidationRule::MinLength),
            (1usize..100).prop_map(ValidationRule::MaxLength),
            (1usize..100).prop_map(ValidationRule::Length),
            Just(ValidationRule::Email),
            Just(ValidationRule::Url),
            Just(ValidationRule::Uuid),
            Just(ValidationRule::Cuid),
            Just(ValidationRule::Datetime),
            Just(ValidationRule::Ip),
            Just(ValidationRule::Trim),
        ]
    }

    fn arb_number_validation() -> impl Strategy<Value = ValidationRule> {
        prop_oneof![
            (-1000.0f64..1000.0).prop_map(ValidationRule::Min),
            (-1000.0f64..1000.0).prop_map(ValidationRule::Max),
            Just(ValidationRule::Positive),
            Just(ValidationRule::Negative),
            Just(ValidationRule::NonNegative),
            Just(ValidationRule::NonPositive),
            Just(ValidationRule::Int),
            Just(ValidationRule::Finite),
        ]
    }

    fn arb_array_validation() -> impl Strategy<Value = ValidationRule> {
        prop_oneof![
            (1usize..100).prop_map(ValidationRule::MinItems),
            (1usize..100).prop_map(ValidationRule::MaxItems),
            Just(ValidationRule::Nonempty),
        ]
    }

    /// Strategy for generating a list of validation rules.
    fn arb_validation_rules(max_rules: usize) -> impl Strategy<Value = Vec<ValidationRule>> {
        prop_oneof![
            proptest::collection::vec(arb_string_validation(), 0..max_rules),
            proptest::collection::vec(arb_number_validation(), 0..max_rules),
            proptest::collection::vec(arb_array_validation(), 0..max_rules),
        ]
    }

    /// Strategy for generating field names.
    fn arb_field_name() -> impl Strategy<Value = String> {
        "[a-z][a-zA-Z0-9]{0,15}".prop_map(|s| s)
    }

    proptest! {
        /// **Property 6: Validation Attribute Chaining**
        ///
        /// *For any* field with multiple validation attributes, the generated Zod schema
        /// SHALL contain all validation methods in a valid chaining order.
        ///
        /// **Validates: Requirements 4.7-4.14, 4.20**
        ///
        /// **Feature: zod-schema-macro, Property 6: Validation Attribute Chaining**
        #[test]
        fn prop_validation_chaining(
            field_name in arb_field_name(),
            validations in arb_validation_rules(5)
        ) {
            let emitter = ZodEmitter::new();
            let config = GeneratorConfig::default();

            // Create a field with the validations
            let mut field = FieldIR::new(&field_name, TypeIR::new(TypeKind::String));
            field.validation = validations.clone();

            // Create a schema with this field
            let schema = SchemaIR::new(
                "TestStruct",
                SchemaKind::Struct(StructSchema::new(vec![field])),
            );

            // Generate the schema
            let result = emitter.generate(&schema, &config).unwrap();

            // Verify all validation methods are present in the output
            for validation in &validations {
                let expected_method = match validation {
                    ValidationRule::MinLength(n) => format!(".min({})", n),
                    ValidationRule::MaxLength(n) => format!(".max({})", n),
                    ValidationRule::Length(n) => format!(".length({})", n),
                    ValidationRule::Email => ".email()".to_string(),
                    ValidationRule::Url => ".url()".to_string(),
                    ValidationRule::Uuid => ".uuid()".to_string(),
                    ValidationRule::Cuid => ".cuid()".to_string(),
                    ValidationRule::Datetime => ".datetime()".to_string(),
                    ValidationRule::Ip => ".ip()".to_string(),
                    ValidationRule::Trim => ".trim()".to_string(),
                    ValidationRule::Min(n) => format!(".min({})", n),
                    ValidationRule::Max(n) => format!(".max({})", n),
                    ValidationRule::Positive => ".positive()".to_string(),
                    ValidationRule::Negative => ".negative()".to_string(),
                    ValidationRule::NonNegative => ".nonnegative()".to_string(),
                    ValidationRule::NonPositive => ".nonpositive()".to_string(),
                    ValidationRule::Int => ".int()".to_string(),
                    ValidationRule::Finite => ".finite()".to_string(),
                    ValidationRule::MinItems(n) => format!(".min({})", n),
                    ValidationRule::MaxItems(n) => format!(".max({})", n),
                    ValidationRule::Nonempty => ".nonempty()".to_string(),
                    _ => continue, // Skip other validations for this test
                };

                prop_assert!(
                    result.code.contains(&expected_method),
                    "Generated code should contain validation method '{}' but got:\n{}",
                    expected_method,
                    result.code
                );
            }

            // Verify the output is syntactically valid (basic check)
            // All validation methods should be chained (no standalone calls)
            prop_assert!(
                !result.code.contains("z.string()z.") && !result.code.contains("()z."),
                "Validation methods should be properly chained"
            );
        }
    }

    // **Property 8: Skip Attribute Exclusion**
    //
    // *For any* field marked with `#[zod(skip)]`, the field SHALL NOT appear
    // in the generated schema output.
    //
    // Note: This property tests the emitter's behavior when given a schema
    // that has already had skipped fields removed by the parser. The emitter
    // should faithfully reproduce only the fields present in the IR.
    //
    // **Validates: Requirements 4.3**
    //
    // **Feature: zod-schema-macro, Property 8: Skip Attribute Exclusion**
    proptest! {
        #[test]
        fn prop_skip_attribute_exclusion(
            included_fields in proptest::collection::vec("[a-z]{4,10}".prop_map(|s| s), 1..5),
            skipped_field_name in "[a-z]{4,10}".prop_map(|s| s)
        ) {
            // Ensure skipped field name is different from included fields
            // and not a substring of any included field
            prop_assume!(!included_fields.contains(&skipped_field_name));
            prop_assume!(!included_fields.iter().any(|f| f.contains(&skipped_field_name)));
            prop_assume!(!included_fields.iter().any(|f| skipped_field_name.contains(f)));

            let emitter = ZodEmitter::new();
            let config = GeneratorConfig::default();

            // Create fields for the schema (only included fields, simulating parser behavior)
            let fields: Vec<FieldIR> = included_fields
                .iter()
                .map(|name| FieldIR::new(name, TypeIR::new(TypeKind::String)))
                .collect();

            let schema = SchemaIR::new(
                "TestStruct",
                SchemaKind::Struct(StructSchema::new(fields)),
            );

            let result = emitter.generate(&schema, &config).unwrap();

            // Verify all included fields are present (using exact field pattern)
            for field_name in &included_fields {
                let field_pattern = format!("{}: z.string()", field_name);
                prop_assert!(
                    result.code.contains(&field_pattern),
                    "Included field '{}' should be in output",
                    field_name
                );
            }

            // Verify skipped field is NOT present as a field definition
            // Use exact field pattern to avoid substring false positives
            let skipped_pattern = format!("{}: z.string()", skipped_field_name);
            prop_assert!(
                !result.code.contains(&skipped_pattern),
                "Skipped field '{}' should NOT be in output",
                skipped_field_name
            );
        }
    }

    /// Strategy for generating arbitrary primitive TypeKind values.
    fn arb_primitive_type_kind() -> impl Strategy<Value = TypeKind> {
        prop_oneof![
            Just(TypeKind::String),
            Just(TypeKind::Boolean),
            Just(TypeKind::Char),
            (
                any::<bool>(),
                prop_oneof![
                    Just(Some(8u8)),
                    Just(Some(16u8)),
                    Just(Some(32u8)),
                    Just(Some(64u8)),
                    Just(None),
                ]
            )
                .prop_map(|(signed, bits)| TypeKind::Integer { signed, bits }),
            Just(TypeKind::Float),
            Just(TypeKind::Uuid),
            Just(TypeKind::DateTime),
        ]
    }

    /// Strategy for generating arbitrary TypeIR values.
    fn arb_type_ir() -> impl Strategy<Value = TypeIR> {
        arb_primitive_type_kind().prop_map(|kind| TypeIR::new(kind))
    }

    /// Strategy for generating arbitrary FieldIR values.
    fn arb_field_ir() -> impl Strategy<Value = FieldIR> {
        (
            arb_field_name(),
            arb_type_ir(),
            any::<bool>(),
            any::<bool>(),
        )
            .prop_map(|(name, ty, optional, nullable)| {
                FieldIR::new(name, ty)
                    .with_optional(optional)
                    .with_nullable(nullable)
            })
    }

    /// Strategy for generating arbitrary StructSchema values.
    fn arb_struct_schema() -> impl Strategy<Value = StructSchema> {
        proptest::collection::vec(arb_field_ir(), 0..5).prop_map(|fields| StructSchema::new(fields))
    }

    /// Strategy for generating schema names.
    fn arb_schema_name() -> impl Strategy<Value = String> {
        "[A-Z][a-zA-Z0-9]{0,15}".prop_map(|s| s)
    }

    /// **Property 13: Generated TypeScript Validity**
    ///
    /// *For any* SchemaIR, the generated TypeScript code SHALL be syntactically valid
    /// (parseable by a TypeScript parser).
    ///
    /// This property verifies basic syntactic validity by checking:
    /// - Balanced parentheses, brackets, and braces
    /// - Proper string quoting
    /// - Valid identifier names
    /// - Proper method chaining syntax
    ///
    /// **Validates: Requirements 5.1**
    ///
    /// **Feature: zod-schema-macro, Property 13: Generated TypeScript Validity**
    proptest! {
        #[test]
        fn prop_generated_typescript_validity(
            schema_name in arb_schema_name(),
            struct_schema in arb_struct_schema()
        ) {
            let emitter = ZodEmitter::new();
            let config = GeneratorConfig::default();

            let schema = SchemaIR::new(&schema_name, SchemaKind::Struct(struct_schema));
            let result = emitter.generate(&schema, &config).unwrap();

            // Check balanced parentheses
            let open_parens = result.code.matches('(').count();
            let close_parens = result.code.matches(')').count();
            prop_assert_eq!(
                open_parens, close_parens,
                "Parentheses should be balanced in:\n{}",
                result.code
            );

            // Check balanced brackets
            let open_brackets = result.code.matches('[').count();
            let close_brackets = result.code.matches(']').count();
            prop_assert_eq!(
                open_brackets, close_brackets,
                "Brackets should be balanced in:\n{}",
                result.code
            );

            // Check balanced braces
            let open_braces = result.code.matches('{').count();
            let close_braces = result.code.matches('}').count();
            prop_assert_eq!(
                open_braces, close_braces,
                "Braces should be balanced in:\n{}",
                result.code
            );

            // Check that the schema name appears in the output
            let expected_schema_name = format!("{}Schema", schema_name);
            prop_assert!(
                result.code.contains(&expected_schema_name),
                "Schema name '{}' should appear in output:\n{}",
                expected_schema_name,
                result.code
            );

            // Check that z.object is present for struct schemas
            prop_assert!(
                result.code.contains("z.object("),
                "Struct schema should contain z.object():\n{}",
                result.code
            );

            // Check for valid export syntax
            prop_assert!(
                result.code.contains("export const") || result.code.contains("export function"),
                "Should have valid export syntax:\n{}",
                result.code
            );

            // Check that type inference is generated
            prop_assert!(
                result.code.contains("z.infer<typeof"),
                "Should have type inference:\n{}",
                result.code
            );

            // Check no double dots (invalid method chaining)
            prop_assert!(
                !result.code.contains(".."),
                "Should not have double dots (invalid chaining):\n{}",
                result.code
            );

            // Check no empty method calls like ()()
            prop_assert!(
                !result.code.contains("()()"),
                "Should not have empty consecutive method calls:\n{}",
                result.code
            );
        }
    }
}
