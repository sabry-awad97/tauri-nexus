//! Export utilities for generating TypeScript contract files.

#[cfg(not(feature = "std"))]
use alloc::{string::String, vec::Vec};

use crate::registry::SchemaRegistry;

/// Configuration for contract file generation.
#[derive(Debug, Clone)]
pub struct ExportConfig {
    /// Whether to include the Zod import statement.
    pub include_import: bool,

    /// Whether to generate type inference exports.
    pub generate_types: bool,

    /// Whether to generate JSDoc comments.
    pub generate_docs: bool,

    /// Custom preamble to add at the top of the file.
    pub preamble: Option<String>,

    /// Custom postamble to add at the bottom of the file.
    pub postamble: Option<String>,
}

impl Default for ExportConfig {
    fn default() -> Self {
        Self {
            include_import: true,
            generate_types: true,
            generate_docs: true,
            preamble: None,
            postamble: None,
        }
    }
}

impl ExportConfig {
    /// Create a new export configuration with defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set whether to include the Zod import.
    pub fn with_import(mut self, include: bool) -> Self {
        self.include_import = include;
        self
    }

    /// Set whether to generate types.
    pub fn with_types(mut self, generate: bool) -> Self {
        self.generate_types = generate;
        self
    }

    /// Set whether to generate docs.
    pub fn with_docs(mut self, generate: bool) -> Self {
        self.generate_docs = generate;
        self
    }

    /// Set custom preamble.
    pub fn with_preamble(mut self, preamble: impl Into<String>) -> Self {
        self.preamble = Some(preamble.into());
        self
    }

    /// Set custom postamble.
    pub fn with_postamble(mut self, postamble: impl Into<String>) -> Self {
        self.postamble = Some(postamble.into());
        self
    }
}

/// Generate a complete TypeScript contract file from a schema registry.
pub fn generate_contract(registry: &SchemaRegistry, config: &ExportConfig) -> String {
    let mut output = String::new();

    // Add preamble
    if let Some(preamble) = &config.preamble {
        output.push_str(preamble);
        output.push_str("\n\n");
    }

    // Add Zod import
    if config.include_import {
        output.push_str("import { z } from 'zod';\n\n");
    }

    // Get schemas in topological order
    let schemas = registry.topological_sort().unwrap_or_else(|| {
        // If there's a cycle, just iterate in arbitrary order
        registry.schemas().collect()
    });

    // Generate each schema
    for schema in schemas {
        if schema.export {
            output.push_str(&schema.to_typescript());
            output.push('\n');
        }
    }

    // Add postamble
    if let Some(postamble) = &config.postamble {
        output.push_str(postamble);
        output.push('\n');
    }

    output
}

/// Generate an index file that re-exports all schemas.
pub fn generate_index(registry: &SchemaRegistry) -> String {
    let mut output = String::new();

    output.push_str("// Auto-generated index file\n\n");

    let mut schema_names: Vec<_> = registry
        .schemas()
        .filter(|s| s.export)
        .map(|s| (&s.name, &s.type_name))
        .collect();

    schema_names.sort_by(|a, b| a.0.cmp(b.0));

    for (schema_name, type_name) in schema_names {
        output.push_str(&format!(
            "export {{ {}, {} }} from './schemas';\n",
            schema_name, type_name
        ));
    }

    output
}
