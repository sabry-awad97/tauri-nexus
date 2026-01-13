//! Schema registry for collecting and managing generated schemas.

#[cfg(not(feature = "std"))]
use alloc::{collections::BTreeMap, string::String, vec::Vec};

#[cfg(feature = "std")]
use std::collections::HashMap;

use crate::types::TypeSchema;

/// A registry for collecting all generated schemas.
///
/// The registry tracks schemas and their dependencies, allowing for
/// topological sorting and cycle detection.
#[derive(Debug, Default)]
pub struct SchemaRegistry {
    #[cfg(feature = "std")]
    schemas: HashMap<String, TypeSchema>,
    #[cfg(not(feature = "std"))]
    schemas: BTreeMap<String, TypeSchema>,
}

impl SchemaRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a schema.
    pub fn register(&mut self, schema: TypeSchema) {
        self.schemas.insert(schema.name.clone(), schema);
    }

    /// Get a schema by name.
    pub fn get(&self, name: &str) -> Option<&TypeSchema> {
        self.schemas.get(name)
    }

    /// Get all registered schemas.
    pub fn schemas(&self) -> impl Iterator<Item = &TypeSchema> {
        self.schemas.values()
    }

    /// Get the number of registered schemas.
    pub fn len(&self) -> usize {
        self.schemas.len()
    }

    /// Check if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.schemas.is_empty()
    }

    /// Get schemas sorted in topological order (dependencies first).
    ///
    /// Returns `None` if there's a circular dependency.
    pub fn topological_sort(&self) -> Option<Vec<&TypeSchema>> {
        let mut result = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut temp_visited = std::collections::HashSet::new();

        for name in self.schemas.keys() {
            if !visited.contains(name)
                && !self.visit(name, &mut visited, &mut temp_visited, &mut result)
            {
                return None; // Cycle detected
            }
        }

        result.reverse();
        Some(result)
    }

    fn visit<'a>(
        &'a self,
        name: &str,
        visited: &mut std::collections::HashSet<String>,
        temp_visited: &mut std::collections::HashSet<String>,
        result: &mut Vec<&'a TypeSchema>,
    ) -> bool {
        if temp_visited.contains(name) {
            return false; // Cycle detected
        }
        if visited.contains(name) {
            return true; // Already processed
        }

        temp_visited.insert(name.to_string());

        if let Some(schema) = self.schemas.get(name) {
            for dep in &schema.dependencies {
                if !self.visit(dep, visited, temp_visited, result) {
                    return false;
                }
            }
            result.push(schema);
        }

        temp_visited.remove(name);
        visited.insert(name.to_string());
        true
    }

    /// Detect circular dependencies and return the cycle path if found.
    pub fn detect_cycles(&self) -> Option<Vec<String>> {
        let mut visited = std::collections::HashSet::new();
        let mut path = Vec::new();

        for name in self.schemas.keys() {
            if !visited.contains(name) {
                if let Some(cycle) = self.find_cycle(name, &mut visited, &mut path) {
                    return Some(cycle);
                }
            }
        }

        None
    }

    fn find_cycle(
        &self,
        name: &str,
        visited: &mut std::collections::HashSet<String>,
        path: &mut Vec<String>,
    ) -> Option<Vec<String>> {
        if path.contains(&name.to_string()) {
            // Found a cycle, extract it
            let start = path.iter().position(|n| n == name).unwrap();
            let mut cycle: Vec<String> = path[start..].to_vec();
            cycle.push(name.to_string());
            return Some(cycle);
        }

        if visited.contains(name) {
            return None;
        }

        path.push(name.to_string());

        if let Some(schema) = self.schemas.get(name) {
            for dep in &schema.dependencies {
                if let Some(cycle) = self.find_cycle(dep, visited, path) {
                    return Some(cycle);
                }
            }
        }

        path.pop();
        visited.insert(name.to_string());
        None
    }
}
