//! Schema Registry for collecting and managing schemas.
//!
//! The SchemaRegistry collects all schemas, tracks dependencies between them,
//! and provides topological sorting for correct output ordering.

use std::collections::{HashMap, HashSet, VecDeque};

use crate::ir::{
    EnumSchema, FieldIR, SchemaIR, SchemaKind, StructSchema, TupleStructSchema, TypeIR, TypeKind,
    VariantIR, VariantKind,
};

/// Error type for cycle detection.
#[derive(Debug, Clone, PartialEq)]
pub struct CycleError {
    /// The cycles detected in the dependency graph.
    pub cycles: Vec<Vec<String>>,
}

impl std::fmt::Display for CycleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Circular dependencies detected: ")?;
        for (i, cycle) in self.cycles.iter().enumerate() {
            if i > 0 {
                write!(f, "; ")?;
            }
            write!(f, "{}", cycle.join(" -> "))?;
        }
        Ok(())
    }
}

impl std::error::Error for CycleError {}

/// Registry that collects all schemas for contract generation.
///
/// The registry stores schemas by name, tracks dependencies between them,
/// and provides methods for topological sorting and cycle detection.
#[derive(Debug, Clone, Default)]
pub struct SchemaRegistry {
    /// All registered schemas by name
    schemas: HashMap<String, SchemaIR>,

    /// Dependency graph: schema name -> set of schema names it depends on
    dependencies: HashMap<String, HashSet<String>>,

    /// Schemas marked for export
    exports: HashSet<String>,
}

impl SchemaRegistry {
    /// Create a new empty schema registry.
    pub fn new() -> Self {
        Self {
            schemas: HashMap::new(),
            dependencies: HashMap::new(),
            exports: HashSet::new(),
        }
    }

    /// Register a schema in the registry.
    ///
    /// This extracts dependencies from the schema and updates the dependency graph.
    pub fn register(&mut self, schema: SchemaIR) {
        let name = schema.name.clone();
        let deps = Self::extract_dependencies(&schema);

        if schema.export {
            self.exports.insert(name.clone());
        }

        self.dependencies.insert(name.clone(), deps);
        self.schemas.insert(name, schema);
    }

    /// Get a schema by name.
    pub fn get(&self, name: &str) -> Option<&SchemaIR> {
        self.schemas.get(name)
    }

    /// Check if a schema is registered.
    pub fn contains(&self, name: &str) -> bool {
        self.schemas.contains_key(name)
    }

    /// Get all registered schema names.
    pub fn schema_names(&self) -> impl Iterator<Item = &String> {
        self.schemas.keys()
    }

    /// Get the number of registered schemas.
    pub fn len(&self) -> usize {
        self.schemas.len()
    }

    /// Check if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.schemas.is_empty()
    }

    /// Get schemas marked for export.
    pub fn exports(&self) -> &HashSet<String> {
        &self.exports
    }

    /// Check if a schema is marked for export.
    pub fn is_exported(&self, name: &str) -> bool {
        self.exports.contains(name)
    }

    /// Get the dependencies of a schema.
    pub fn get_dependencies(&self, name: &str) -> Option<&HashSet<String>> {
        self.dependencies.get(name)
    }

    /// Get schemas in topologically sorted order.
    ///
    /// Returns schemas ordered so that dependencies come before dependents.
    /// Returns an error if circular dependencies are detected.
    pub fn sorted_schemas(&self) -> Result<Vec<&SchemaIR>, CycleError> {
        let order = self.topological_sort()?;
        Ok(order
            .iter()
            .filter_map(|name| self.schemas.get(name))
            .collect())
    }

    /// Detect circular dependencies in the registry.
    ///
    /// Returns a list of cycles, where each cycle is a list of schema names
    /// forming a circular dependency.
    pub fn detect_cycles(&self) -> Vec<Vec<String>> {
        let mut cycles = Vec::new();
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut path = Vec::new();

        for name in self.schemas.keys() {
            if !visited.contains(name) {
                self.dfs_cycles(name, &mut visited, &mut rec_stack, &mut path, &mut cycles);
            }
        }

        cycles
    }

    /// Topological sort using Kahn's algorithm.
    ///
    /// Returns schema names in dependency order (dependencies first).
    fn topological_sort(&self) -> Result<Vec<String>, CycleError> {
        // in_degree[X] = number of schemas that X depends on (that are registered)
        let mut in_degree: HashMap<&String, usize> = HashMap::new();

        // Initialize in-degrees
        for name in self.schemas.keys() {
            in_degree.insert(name, 0);
        }

        // Calculate in-degrees: count how many registered dependencies each schema has
        for (name, deps) in &self.dependencies {
            let registered_deps = deps
                .iter()
                .filter(|d| self.schemas.contains_key(*d))
                .count();
            if let Some(degree) = in_degree.get_mut(name) {
                *degree = registered_deps;
            }
        }

        // Find nodes with no dependencies (in_degree == 0)
        let mut queue: VecDeque<&String> = VecDeque::new();
        for (name, &degree) in &in_degree {
            if degree == 0 {
                queue.push_back(*name);
            }
        }

        // Process queue
        let mut result = Vec::new();
        while let Some(name) = queue.pop_front() {
            result.push(name.clone());

            // For each schema that depends on this one, decrement its in-degree
            for (other_name, deps) in &self.dependencies {
                if deps.contains(name) && self.schemas.contains_key(other_name) {
                    if let Some(degree) = in_degree.get_mut(other_name) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push_back(other_name);
                        }
                    }
                }
            }
        }

        // If we didn't process all schemas, there's a cycle
        if result.len() != self.schemas.len() {
            return Err(CycleError {
                cycles: self.detect_cycles(),
            });
        }

        Ok(result)
    }

    /// DFS helper for cycle detection.
    fn dfs_cycles(
        &self,
        node: &String,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
        path: &mut Vec<String>,
        cycles: &mut Vec<Vec<String>>,
    ) {
        visited.insert(node.clone());
        rec_stack.insert(node.clone());
        path.push(node.clone());

        if let Some(deps) = self.dependencies.get(node) {
            for dep in deps {
                // Only consider dependencies that are registered
                if !self.schemas.contains_key(dep) {
                    continue;
                }

                if !visited.contains(dep) {
                    self.dfs_cycles(dep, visited, rec_stack, path, cycles);
                } else if rec_stack.contains(dep) {
                    // Found a cycle - extract it from the path
                    if let Some(start_idx) = path.iter().position(|n| n == dep) {
                        let mut cycle: Vec<String> = path[start_idx..].to_vec();
                        cycle.push(dep.clone()); // Complete the cycle
                        cycles.push(cycle);
                    }
                }
            }
        }

        path.pop();
        rec_stack.remove(node);
    }

    /// Extract type references from a schema.
    ///
    /// Returns the set of schema names that this schema depends on.
    fn extract_dependencies(schema: &SchemaIR) -> HashSet<String> {
        let mut deps = HashSet::new();
        Self::collect_schema_deps(&schema.kind, &mut deps);
        deps
    }

    /// Collect dependencies from a schema kind.
    fn collect_schema_deps(kind: &SchemaKind, deps: &mut HashSet<String>) {
        match kind {
            SchemaKind::Struct(s) => Self::collect_struct_deps(s, deps),
            SchemaKind::TupleStruct(ts) => Self::collect_tuple_struct_deps(ts, deps),
            SchemaKind::Enum(e) => Self::collect_enum_deps(e, deps),
            SchemaKind::Alias(ty) => Self::collect_type_deps(ty, deps),
            SchemaKind::UnitStruct => {}
        }
    }

    /// Collect dependencies from a struct schema.
    fn collect_struct_deps(s: &StructSchema, deps: &mut HashSet<String>) {
        for field in &s.fields {
            Self::collect_field_deps(field, deps);
        }
    }

    /// Collect dependencies from a tuple struct schema.
    fn collect_tuple_struct_deps(ts: &TupleStructSchema, deps: &mut HashSet<String>) {
        for ty in &ts.fields {
            Self::collect_type_deps(ty, deps);
        }
    }

    /// Collect dependencies from an enum schema.
    fn collect_enum_deps(e: &EnumSchema, deps: &mut HashSet<String>) {
        for variant in &e.variants {
            Self::collect_variant_deps(variant, deps);
        }
    }

    /// Collect dependencies from a field.
    fn collect_field_deps(field: &FieldIR, deps: &mut HashSet<String>) {
        Self::collect_type_deps(&field.ty, deps);
    }

    /// Collect dependencies from a variant.
    fn collect_variant_deps(variant: &VariantIR, deps: &mut HashSet<String>) {
        match &variant.kind {
            VariantKind::Unit => {}
            VariantKind::Tuple(fields) => {
                for ty in fields {
                    Self::collect_type_deps(ty, deps);
                }
            }
            VariantKind::Struct(fields) => {
                for field in fields {
                    Self::collect_field_deps(field, deps);
                }
            }
        }
    }

    /// Collect dependencies from a type.
    fn collect_type_deps(ty: &TypeIR, deps: &mut HashSet<String>) {
        match &ty.kind {
            TypeKind::Reference { name, generics } => {
                deps.insert(name.clone());
                for g in generics {
                    Self::collect_type_deps(g, deps);
                }
            }
            TypeKind::Array(inner) => Self::collect_type_deps(inner, deps),
            TypeKind::Optional(inner) => Self::collect_type_deps(inner, deps),
            TypeKind::Set(inner) => Self::collect_type_deps(inner, deps),
            TypeKind::Tuple(elements) => {
                for el in elements {
                    Self::collect_type_deps(el, deps);
                }
            }
            TypeKind::Record { key, value } => {
                Self::collect_type_deps(key, deps);
                Self::collect_type_deps(value, deps);
            }
            TypeKind::Union(types) | TypeKind::Intersection(types) => {
                for t in types {
                    Self::collect_type_deps(t, deps);
                }
            }
            // Primitive types have no dependencies
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{FieldIR, SchemaIR, SchemaKind, StructSchema, TypeIR, TypeKind};

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
    fn test_registry_new() {
        let registry = SchemaRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_registry_register() {
        let mut registry = SchemaRegistry::new();
        let schema = make_simple_struct("User");

        registry.register(schema);

        assert!(!registry.is_empty());
        assert_eq!(registry.len(), 1);
        assert!(registry.contains("User"));
        assert!(registry.get("User").is_some());
    }

    #[test]
    fn test_registry_exports() {
        let mut registry = SchemaRegistry::new();

        let schema1 = make_simple_struct("User");
        let schema2 = make_simple_struct("Internal").with_export(false);

        registry.register(schema1);
        registry.register(schema2);

        assert!(registry.is_exported("User"));
        assert!(!registry.is_exported("Internal"));
        assert_eq!(registry.exports().len(), 1);
    }

    #[test]
    fn test_registry_dependencies() {
        let mut registry = SchemaRegistry::new();

        let user = make_simple_struct("User");
        let post = make_struct_with_ref("Post", "User");

        registry.register(user);
        registry.register(post);

        let post_deps = registry.get_dependencies("Post").unwrap();
        assert!(post_deps.contains("User"));

        let user_deps = registry.get_dependencies("User").unwrap();
        assert!(user_deps.is_empty());
    }

    #[test]
    fn test_topological_sort_simple() {
        let mut registry = SchemaRegistry::new();

        // User has no dependencies
        let user = make_simple_struct("User");
        // Post depends on User
        let post = make_struct_with_ref("Post", "User");

        registry.register(post);
        registry.register(user);

        let sorted = registry.sorted_schemas().unwrap();
        let names: Vec<_> = sorted.iter().map(|s| s.name.as_str()).collect();

        // User should come before Post
        let user_idx = names.iter().position(|&n| n == "User").unwrap();
        let post_idx = names.iter().position(|&n| n == "Post").unwrap();
        assert!(user_idx < post_idx);
    }

    #[test]
    fn test_topological_sort_chain() {
        let mut registry = SchemaRegistry::new();

        // A -> B -> C (C depends on B, B depends on A)
        let a = make_simple_struct("A");
        let b = make_struct_with_ref("B", "A");
        let c = make_struct_with_ref("C", "B");

        // Register in reverse order
        registry.register(c);
        registry.register(b);
        registry.register(a);

        let sorted = registry.sorted_schemas().unwrap();
        let names: Vec<_> = sorted.iter().map(|s| s.name.as_str()).collect();

        let a_idx = names.iter().position(|&n| n == "A").unwrap();
        let b_idx = names.iter().position(|&n| n == "B").unwrap();
        let c_idx = names.iter().position(|&n| n == "C").unwrap();

        assert!(a_idx < b_idx);
        assert!(b_idx < c_idx);
    }

    #[test]
    fn test_cycle_detection_simple() {
        let mut registry = SchemaRegistry::new();

        // A -> B -> A (circular)
        let a = make_struct_with_ref("A", "B");
        let b = make_struct_with_ref("B", "A");

        registry.register(a);
        registry.register(b);

        let cycles = registry.detect_cycles();
        assert!(!cycles.is_empty());
    }

    #[test]
    fn test_cycle_detection_three_way() {
        let mut registry = SchemaRegistry::new();

        // A -> B -> C -> A (circular)
        let a = make_struct_with_ref("A", "B");
        let b = make_struct_with_ref("B", "C");
        let c = make_struct_with_ref("C", "A");

        registry.register(a);
        registry.register(b);
        registry.register(c);

        let cycles = registry.detect_cycles();
        assert!(!cycles.is_empty());
    }

    #[test]
    fn test_sorted_schemas_with_cycle_returns_error() {
        let mut registry = SchemaRegistry::new();

        let a = make_struct_with_ref("A", "B");
        let b = make_struct_with_ref("B", "A");

        registry.register(a);
        registry.register(b);

        let result = registry.sorted_schemas();
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(!err.cycles.is_empty());
    }

    #[test]
    fn test_no_cycles_in_acyclic_graph() {
        let mut registry = SchemaRegistry::new();

        let a = make_simple_struct("A");
        let b = make_struct_with_ref("B", "A");
        let c = make_struct_with_ref("C", "A");

        registry.register(a);
        registry.register(b);
        registry.register(c);

        let cycles = registry.detect_cycles();
        assert!(cycles.is_empty());
    }

    #[test]
    fn test_external_dependency_ignored() {
        let mut registry = SchemaRegistry::new();

        // Post depends on User, but User is not registered
        let post = make_struct_with_ref("Post", "User");
        registry.register(post);

        // Should not fail - external dependencies are ignored
        let sorted = registry.sorted_schemas().unwrap();
        assert_eq!(sorted.len(), 1);
    }

    #[test]
    fn test_schema_names() {
        let mut registry = SchemaRegistry::new();

        registry.register(make_simple_struct("A"));
        registry.register(make_simple_struct("B"));
        registry.register(make_simple_struct("C"));

        let names: HashSet<_> = registry.schema_names().cloned().collect();
        assert!(names.contains("A"));
        assert!(names.contains("B"));
        assert!(names.contains("C"));
        assert_eq!(names.len(), 3);
    }

    #[test]
    fn test_cycle_error_display() {
        let err = CycleError {
            cycles: vec![vec!["A".to_string(), "B".to_string(), "A".to_string()]],
        };
        let display = format!("{}", err);
        assert!(display.contains("A -> B -> A"));
    }
}

#[cfg(test)]
mod proptest_tests {
    use super::*;
    use crate::ir::{FieldIR, SchemaIR, SchemaKind, StructSchema, TypeIR, TypeKind};
    use proptest::prelude::*;

    /// Generate a valid schema name (alphanumeric, starting with uppercase)
    fn arb_schema_name() -> impl Strategy<Value = String> {
        "[A-Z][a-zA-Z0-9]{0,9}".prop_map(|s| s.to_string())
    }

    /// Generate a simple struct schema with no dependencies
    fn make_simple_struct(name: &str) -> SchemaIR {
        SchemaIR::new(
            name,
            SchemaKind::Struct(StructSchema::new(vec![FieldIR::new(
                "id",
                TypeIR::new(TypeKind::String),
            )])),
        )
    }

    /// Generate a struct schema that depends on another schema
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

    /// **Property 11: Circular Reference Detection**
    ///
    /// *For any* set of schemas with circular dependencies, the SchemaRegistry
    /// SHALL detect all cycles and report them.
    ///
    /// This property tests that:
    /// 1. When schemas form a cycle (A -> B -> A), detect_cycles returns non-empty
    /// 2. When schemas form a cycle, sorted_schemas returns an error
    /// 3. When schemas don't form a cycle, detect_cycles returns empty
    ///
    /// **Validates: Requirements 8.2**
    ///
    /// **Feature: zod-schema-macro, Property 11: Circular Reference Detection**
    #[test]
    fn prop_circular_reference_detection() {
        proptest!(|(
            name_a in arb_schema_name(),
            name_b in arb_schema_name().prop_filter("different from a", |b| b.len() > 0),
            name_c in arb_schema_name().prop_filter("different from a and b", |c| c.len() > 0),
        )| {
            // Ensure unique names
            let name_b = if name_b == name_a { format!("{}X", name_b) } else { name_b };
            let name_c = if name_c == name_a || name_c == name_b { format!("{}Y", name_c) } else { name_c };

            // Test 1: Two-way cycle (A -> B -> A)
            {
                let mut registry = SchemaRegistry::new();
                let a = make_struct_with_ref(&name_a, &name_b);
                let b = make_struct_with_ref(&name_b, &name_a);
                registry.register(a);
                registry.register(b);

                let cycles = registry.detect_cycles();
                prop_assert!(!cycles.is_empty(), "Two-way cycle should be detected");

                let result = registry.sorted_schemas();
                prop_assert!(result.is_err(), "sorted_schemas should fail with cycle");
            }

            // Test 2: Three-way cycle (A -> B -> C -> A)
            {
                let mut registry = SchemaRegistry::new();
                let a = make_struct_with_ref(&name_a, &name_b);
                let b = make_struct_with_ref(&name_b, &name_c);
                let c = make_struct_with_ref(&name_c, &name_a);
                registry.register(a);
                registry.register(b);
                registry.register(c);

                let cycles = registry.detect_cycles();
                prop_assert!(!cycles.is_empty(), "Three-way cycle should be detected");

                let result = registry.sorted_schemas();
                prop_assert!(result.is_err(), "sorted_schemas should fail with cycle");
            }

            // Test 3: No cycle (A <- B <- C, linear chain)
            {
                let mut registry = SchemaRegistry::new();
                let a = make_simple_struct(&name_a);
                let b = make_struct_with_ref(&name_b, &name_a);
                let c = make_struct_with_ref(&name_c, &name_b);
                registry.register(a);
                registry.register(b);
                registry.register(c);

                let cycles = registry.detect_cycles();
                prop_assert!(cycles.is_empty(), "Linear chain should have no cycles");

                let result = registry.sorted_schemas();
                prop_assert!(result.is_ok(), "sorted_schemas should succeed without cycle");
            }
        });
    }

    /// **Property 12: Topological Sort Correctness**
    ///
    /// *For any* set of schemas without circular dependencies, the topological sort
    /// SHALL return schemas in an order where dependencies come before dependents.
    ///
    /// This property tests that:
    /// 1. All schemas are included in the sorted output
    /// 2. For each schema, all its dependencies appear before it in the sorted order
    ///
    /// **Validates: Requirements 8.3**
    ///
    /// **Feature: zod-schema-macro, Property 12: Topological Sort Correctness**
    #[test]
    fn prop_topological_sort_correctness() {
        proptest!(|(
            names in prop::collection::vec(arb_schema_name(), 2..6)
        )| {
            // Ensure unique names
            let mut unique_names: Vec<String> = Vec::new();
            for (i, name) in names.iter().enumerate() {
                let mut unique_name = name.clone();
                while unique_names.contains(&unique_name) {
                    unique_name = format!("{}{}", name, i);
                }
                unique_names.push(unique_name);
            }

            // Create a linear dependency chain: first has no deps, each subsequent depends on previous
            let mut registry = SchemaRegistry::new();

            // First schema has no dependencies
            registry.register(make_simple_struct(&unique_names[0]));

            // Each subsequent schema depends on the previous one
            for i in 1..unique_names.len() {
                registry.register(make_struct_with_ref(&unique_names[i], &unique_names[i - 1]));
            }

            // Get sorted schemas
            let sorted = registry.sorted_schemas();
            prop_assert!(sorted.is_ok(), "Acyclic graph should sort successfully");

            let sorted = sorted.unwrap();
            let sorted_names: Vec<&str> = sorted.iter().map(|s| s.name.as_str()).collect();

            // Property 1: All schemas are included
            prop_assert_eq!(sorted_names.len(), unique_names.len(), "All schemas should be in output");

            // Property 2: Dependencies come before dependents
            for i in 1..unique_names.len() {
                let dep_name = &unique_names[i - 1];
                let dependent_name = &unique_names[i];

                let dep_idx = sorted_names.iter().position(|&n| n == dep_name);
                let dependent_idx = sorted_names.iter().position(|&n| n == dependent_name);

                prop_assert!(dep_idx.is_some(), "Dependency {} should be in sorted output", dep_name);
                prop_assert!(dependent_idx.is_some(), "Dependent {} should be in sorted output", dependent_name);

                prop_assert!(
                    dep_idx.unwrap() < dependent_idx.unwrap(),
                    "Dependency {} (idx {}) should come before dependent {} (idx {})",
                    dep_name, dep_idx.unwrap(), dependent_name, dependent_idx.unwrap()
                );
            }
        });
    }
}
