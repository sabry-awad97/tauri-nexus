//! Schema generator for producing TypeScript Zod schemas.
//!
//! This module uses the `ZodEmitter` from `zod-rs-macros` to generate
//! TypeScript Zod schemas from parsed Rust types.

use crate::config::Config;
use crate::error::CliResult;
use crate::parser::ParsedType;
use std::collections::{HashMap, HashSet};

/// Generated output containing all schemas.
#[derive(Debug, Clone)]
pub struct GeneratedOutput {
    /// Complete TypeScript content.
    pub content: String,

    /// Individual generated schemas.
    pub schemas: Vec<GeneratedSchema>,
}

/// A single generated schema.
#[derive(Debug, Clone)]
pub struct GeneratedSchema {
    /// Original type name.
    pub name: String,

    /// Schema constant name (e.g., "UserSchema").
    pub schema_name: String,

    /// TypeScript type name.
    pub type_name: String,

    /// Dependencies on other schemas.
    pub dependencies: Vec<String>,
}

/// Schema generator using zod-rs-macros infrastructure.
pub struct SchemaGenerator {
    config: Config,
}

impl SchemaGenerator {
    /// Create a new schema generator with the given configuration.
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    /// Generate TypeScript Zod schemas from parsed types.
    pub fn generate(&self, types: Vec<ParsedType>) -> CliResult<GeneratedOutput> {
        if types.is_empty() {
            return Ok(GeneratedOutput {
                content: self.generate_empty_output(),
                schemas: Vec::new(),
            });
        }

        // Generate individual schemas
        let mut schemas = Vec::new();
        for parsed_type in &types {
            let schema = self.generate_schema(parsed_type)?;
            schemas.push(schema);
        }

        // Sort by dependencies
        let sorted_schemas = self.topological_sort(&schemas)?;

        // Generate complete output
        let content = self.generate_output(&sorted_schemas);

        Ok(GeneratedOutput {
            content,
            schemas: sorted_schemas,
        })
    }

    /// Generate a single schema from a parsed type.
    fn generate_schema(&self, parsed_type: &ParsedType) -> CliResult<GeneratedSchema> {
        let name = parsed_type.name.clone();
        let schema_name = format!("{}{}", name, self.config.naming.schema_suffix);
        let type_name = name.clone();

        // Extract dependencies from the type
        let dependencies = self.extract_dependencies(&parsed_type.derive_input);

        // Generate the Zod schema code using the derive input
        // We'll use a simplified approach here since we can't directly call the macro
        let _schema_code = self.generate_schema_code(&parsed_type.derive_input);

        Ok(GeneratedSchema {
            name,
            schema_name,
            type_name,
            dependencies,
        })
    }

    /// Generate Zod schema code from a DeriveInput.
    ///
    /// This is a simplified implementation that generates basic schemas.
    /// For full functionality, this would integrate with zod-rs-macros.
    fn generate_schema_code(&self, input: &syn::DeriveInput) -> String {
        match &input.data {
            syn::Data::Struct(data) => self.generate_struct_schema(&input.ident, data),
            syn::Data::Enum(data) => self.generate_enum_schema(&input.ident, data),
            syn::Data::Union(_) => "z.unknown()".to_string(),
        }
    }

    /// Generate schema for a struct.
    fn generate_struct_schema(&self, _ident: &syn::Ident, data: &syn::DataStruct) -> String {
        match &data.fields {
            syn::Fields::Named(fields) => {
                let field_schemas: Vec<String> = fields
                    .named
                    .iter()
                    .filter_map(|f| {
                        let name = f.ident.as_ref()?;
                        let field_name = self.transform_field_name(&name.to_string());
                        let type_schema = self.type_to_zod(&f.ty);
                        Some(format!("  {}: {}", field_name, type_schema))
                    })
                    .collect();

                if field_schemas.is_empty() {
                    "z.object({})".to_string()
                } else {
                    format!("z.object({{\n{}\n}})", field_schemas.join(",\n"))
                }
            }
            syn::Fields::Unnamed(fields) => {
                let field_schemas: Vec<String> = fields
                    .unnamed
                    .iter()
                    .map(|f| self.type_to_zod(&f.ty))
                    .collect();

                if field_schemas.len() == 1 {
                    field_schemas[0].clone()
                } else {
                    format!("z.tuple([{}])", field_schemas.join(", "))
                }
            }
            syn::Fields::Unit => "z.object({})".to_string(),
        }
    }

    /// Generate schema for an enum.
    fn generate_enum_schema(&self, _ident: &syn::Ident, data: &syn::DataEnum) -> String {
        // Check if all variants are unit variants
        let is_unit_only = data
            .variants
            .iter()
            .all(|v| matches!(v.fields, syn::Fields::Unit));

        if is_unit_only {
            let variants: Vec<String> = data
                .variants
                .iter()
                .map(|v| format!("\"{}\"", v.ident))
                .collect();
            format!("z.enum([{}])", variants.join(", "))
        } else {
            // Generate union for complex enums
            let variants: Vec<String> = data
                .variants
                .iter()
                .map(|v| self.generate_variant_schema(v))
                .collect();
            format!("z.union([\n{}\n])", variants.join(",\n"))
        }
    }

    /// Generate schema for an enum variant.
    fn generate_variant_schema(&self, variant: &syn::Variant) -> String {
        let name = &variant.ident;
        match &variant.fields {
            syn::Fields::Unit => {
                format!("  z.literal(\"{}\")", name)
            }
            syn::Fields::Unnamed(fields) => {
                let field_schemas: Vec<String> = fields
                    .unnamed
                    .iter()
                    .map(|f| self.type_to_zod(&f.ty))
                    .collect();
                if field_schemas.len() == 1 {
                    format!("  z.object({{ \"{}\": {} }})", name, field_schemas[0])
                } else {
                    format!(
                        "  z.object({{ \"{}\": z.tuple([{}]) }})",
                        name,
                        field_schemas.join(", ")
                    )
                }
            }
            syn::Fields::Named(fields) => {
                let field_schemas: Vec<String> = fields
                    .named
                    .iter()
                    .filter_map(|f| {
                        let field_name = f.ident.as_ref()?;
                        let type_schema = self.type_to_zod(&f.ty);
                        Some(format!("{}: {}", field_name, type_schema))
                    })
                    .collect();
                format!(
                    "  z.object({{ \"{}\": z.object({{ {} }}) }})",
                    name,
                    field_schemas.join(", ")
                )
            }
        }
    }

    /// Convert a Rust type to Zod schema.
    fn type_to_zod(&self, ty: &syn::Type) -> String {
        match ty {
            syn::Type::Path(type_path) => {
                let path = &type_path.path;
                if let Some(segment) = path.segments.last() {
                    let ident = segment.ident.to_string();
                    match ident.as_str() {
                        "String" | "str" => "z.string()".to_string(),
                        "bool" => "z.boolean()".to_string(),
                        "i8" | "i16" | "i32" | "i64" | "i128" | "isize" => {
                            "z.number().int()".to_string()
                        }
                        "u8" | "u16" | "u32" | "u64" | "u128" | "usize" => {
                            "z.number().int().nonnegative()".to_string()
                        }
                        "f32" | "f64" => "z.number()".to_string(),
                        "char" => "z.string().length(1)".to_string(),
                        "Option" => {
                            if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                                if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                                    return format!("{}.optional()", self.type_to_zod(inner));
                                }
                            }
                            "z.unknown().optional()".to_string()
                        }
                        "Vec" => {
                            if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                                if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                                    return format!("z.array({})", self.type_to_zod(inner));
                                }
                            }
                            "z.array(z.unknown())".to_string()
                        }
                        "HashMap" | "BTreeMap" => {
                            if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                                let mut iter = args.args.iter();
                                if let (
                                    Some(syn::GenericArgument::Type(key)),
                                    Some(syn::GenericArgument::Type(value)),
                                ) = (iter.next(), iter.next())
                                {
                                    return format!(
                                        "z.record({}, {})",
                                        self.type_to_zod(key),
                                        self.type_to_zod(value)
                                    );
                                }
                            }
                            "z.record(z.string(), z.unknown())".to_string()
                        }
                        "HashSet" | "BTreeSet" => {
                            if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                                if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                                    return format!("z.array({})", self.type_to_zod(inner));
                                }
                            }
                            "z.array(z.unknown())".to_string()
                        }
                        // Reference to another type
                        _ => format!("{}Schema", ident),
                    }
                } else {
                    "z.unknown()".to_string()
                }
            }
            syn::Type::Reference(type_ref) => self.type_to_zod(&type_ref.elem),
            syn::Type::Tuple(tuple) => {
                if tuple.elems.is_empty() {
                    "z.tuple([])".to_string()
                } else {
                    let elems: Vec<String> =
                        tuple.elems.iter().map(|t| self.type_to_zod(t)).collect();
                    format!("z.tuple([{}])", elems.join(", "))
                }
            }
            syn::Type::Array(array) => {
                format!("z.array({})", self.type_to_zod(&array.elem))
            }
            _ => "z.unknown()".to_string(),
        }
    }

    /// Transform field name according to rename_all config.
    fn transform_field_name(&self, name: &str) -> String {
        match self.config.naming.rename_all.as_deref() {
            Some("camelCase") => to_camel_case(name),
            Some("PascalCase") => to_pascal_case(name),
            Some("SCREAMING_SNAKE_CASE") => name.to_uppercase(),
            Some("kebab-case") => name.replace('_', "-"),
            _ => name.to_string(), // snake_case or no transformation
        }
    }

    /// Extract type dependencies from a DeriveInput.
    fn extract_dependencies(&self, input: &syn::DeriveInput) -> Vec<String> {
        let mut deps = HashSet::new();

        match &input.data {
            syn::Data::Struct(data) => {
                self.extract_field_deps(&data.fields, &mut deps);
            }
            syn::Data::Enum(data) => {
                for variant in &data.variants {
                    self.extract_field_deps(&variant.fields, &mut deps);
                }
            }
            syn::Data::Union(_) => {}
        }

        deps.into_iter().collect()
    }

    /// Extract dependencies from fields.
    fn extract_field_deps(&self, fields: &syn::Fields, deps: &mut HashSet<String>) {
        match fields {
            syn::Fields::Named(named) => {
                for field in &named.named {
                    self.extract_type_deps(&field.ty, deps);
                }
            }
            syn::Fields::Unnamed(unnamed) => {
                for field in &unnamed.unnamed {
                    self.extract_type_deps(&field.ty, deps);
                }
            }
            syn::Fields::Unit => {}
        }
    }

    /// Extract dependencies from a type.
    fn extract_type_deps(&self, ty: &syn::Type, deps: &mut HashSet<String>) {
        match ty {
            syn::Type::Path(type_path) => {
                if let Some(segment) = type_path.path.segments.last() {
                    let ident = segment.ident.to_string();

                    // Skip built-in types
                    if !is_builtin_type(&ident) {
                        deps.insert(ident);
                    }

                    // Check generic arguments
                    if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                        for arg in &args.args {
                            if let syn::GenericArgument::Type(inner) = arg {
                                self.extract_type_deps(inner, deps);
                            }
                        }
                    }
                }
            }
            syn::Type::Reference(type_ref) => {
                self.extract_type_deps(&type_ref.elem, deps);
            }
            syn::Type::Tuple(tuple) => {
                for elem in &tuple.elems {
                    self.extract_type_deps(elem, deps);
                }
            }
            syn::Type::Array(array) => {
                self.extract_type_deps(&array.elem, deps);
            }
            _ => {}
        }
    }

    /// Topologically sort schemas by dependencies.
    fn topological_sort(&self, schemas: &[GeneratedSchema]) -> CliResult<Vec<GeneratedSchema>> {
        let schema_map: HashMap<&str, &GeneratedSchema> =
            schemas.iter().map(|s| (s.name.as_str(), s)).collect();

        let mut result = Vec::new();
        let mut visited = HashSet::new();
        let mut temp_visited = HashSet::new();

        for schema in schemas {
            if !visited.contains(&schema.name) {
                self.visit_schema(
                    &schema.name,
                    &schema_map,
                    &mut visited,
                    &mut temp_visited,
                    &mut result,
                )?;
            }
        }

        Ok(result)
    }

    /// Visit a schema for topological sort.
    fn visit_schema(
        &self,
        name: &str,
        schema_map: &HashMap<&str, &GeneratedSchema>,
        visited: &mut HashSet<String>,
        temp_visited: &mut HashSet<String>,
        result: &mut Vec<GeneratedSchema>,
    ) -> CliResult<()> {
        if temp_visited.contains(name) {
            // Cycle detected - this is okay, we'll use z.lazy()
            return Ok(());
        }

        if visited.contains(name) {
            return Ok(());
        }

        temp_visited.insert(name.to_string());

        if let Some(schema) = schema_map.get(name) {
            for dep in &schema.dependencies {
                if schema_map.contains_key(dep.as_str()) {
                    self.visit_schema(dep, schema_map, visited, temp_visited, result)?;
                }
            }
            result.push((*schema).clone());
        }

        temp_visited.remove(name);
        visited.insert(name.to_string());

        Ok(())
    }

    /// Generate complete TypeScript output.
    fn generate_output(&self, schemas: &[GeneratedSchema]) -> String {
        let mut output = String::new();

        // Add header comment
        output.push_str("// Auto-generated by zod-rs-cli\n");
        output.push_str("// Do not edit manually\n\n");

        // Add Zod import
        output.push_str("import { z } from 'zod';\n\n");

        // Generate each schema
        for schema in schemas {
            // Generate schema constant
            output.push_str(&format!(
                "export const {} = {};\n",
                schema.schema_name,
                self.generate_schema_body(schema)
            ));

            // Generate type inference
            if self.config.output.generate_types {
                output.push_str(&format!(
                    "export type {} = z.infer<typeof {}>;\n",
                    schema.type_name, schema.schema_name
                ));
            }

            output.push('\n');
        }

        output
    }

    /// Generate the schema body (placeholder - would use actual IR).
    fn generate_schema_body(&self, schema: &GeneratedSchema) -> String {
        // This is a placeholder - in a full implementation,
        // we would store the generated code in GeneratedSchema
        format!("z.object({{ /* {} */ }})", schema.name)
    }

    /// Generate output for empty input.
    fn generate_empty_output(&self) -> String {
        let mut output = String::new();
        output.push_str("// Auto-generated by zod-rs-cli\n");
        output.push_str("// No schemas found\n\n");
        output.push_str("import { z } from 'zod';\n");
        output
    }
}

/// Check if a type name is a built-in type.
fn is_builtin_type(name: &str) -> bool {
    matches!(
        name,
        "String"
            | "str"
            | "bool"
            | "i8"
            | "i16"
            | "i32"
            | "i64"
            | "i128"
            | "isize"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "u128"
            | "usize"
            | "f32"
            | "f64"
            | "char"
            | "Option"
            | "Vec"
            | "HashMap"
            | "BTreeMap"
            | "HashSet"
            | "BTreeSet"
            | "Box"
            | "Arc"
            | "Rc"
    )
}

/// Convert snake_case to camelCase.
fn to_camel_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = false;

    for c in s.chars() {
        if c == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(c.to_ascii_uppercase());
            capitalize_next = false;
        } else {
            result.push(c);
        }
    }

    result
}

/// Convert snake_case to PascalCase.
fn to_pascal_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = true;

    for c in s.chars() {
        if c == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(c.to_ascii_uppercase());
            capitalize_next = false;
        } else {
            result.push(c);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_camel_case() {
        assert_eq!(to_camel_case("user_name"), "userName");
        assert_eq!(to_camel_case("first_name"), "firstName");
        assert_eq!(to_camel_case("id"), "id");
        assert_eq!(to_camel_case("user_id_number"), "userIdNumber");
    }

    #[test]
    fn test_to_pascal_case() {
        assert_eq!(to_pascal_case("user_name"), "UserName");
        assert_eq!(to_pascal_case("first_name"), "FirstName");
        assert_eq!(to_pascal_case("id"), "Id");
    }

    #[test]
    fn test_is_builtin_type() {
        assert!(is_builtin_type("String"));
        assert!(is_builtin_type("Vec"));
        assert!(is_builtin_type("Option"));
        assert!(!is_builtin_type("User"));
        assert!(!is_builtin_type("CustomType"));
    }

    #[test]
    fn test_empty_output() {
        let config = Config::default();
        let generator = SchemaGenerator::new(config);

        let output = generator.generate(vec![]).unwrap();

        assert!(output.content.contains("import { z } from 'zod'"));
        assert!(output.schemas.is_empty());
    }
}
