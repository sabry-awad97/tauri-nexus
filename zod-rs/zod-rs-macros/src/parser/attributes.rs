//! Attribute parsing using darling for ergonomic derive macro attributes.
//!
//! This module defines the attribute structures for `#[zod(...)]` attributes
//! on containers (structs/enums), fields, and variants.

use darling::{FromDeriveInput, FromField, FromMeta, FromVariant};
use syn::{Generics, Ident, Type, Visibility};

/// Container-level attributes for structs and enums.
/// Parsed from `#[zod(...)]` on the type definition.
#[derive(Debug, Clone, FromDeriveInput)]
#[darling(attributes(zod), supports(struct_any, enum_any))]
pub struct ContainerAttrs {
    /// The identifier of the type
    pub ident: Ident,

    /// Generic parameters of the type
    pub generics: Generics,

    /// Visibility of the type
    pub vis: Visibility,

    /// Rename the type in generated schema
    #[darling(default)]
    pub rename: Option<String>,

    /// Rename all fields using a case convention
    #[darling(default)]
    pub rename_all: Option<RenameRule>,

    /// Tag field name for internally tagged enums
    #[darling(default)]
    pub tag: Option<String>,

    /// Content field name for adjacently tagged enums
    #[darling(default)]
    pub content: Option<String>,

    /// Description for the schema (from #[zod(description = "...")])
    #[darling(default)]
    pub description: Option<String>,

    /// Mark as deprecated
    #[darling(default)]
    pub deprecated: bool,

    /// Exclude from contract export
    #[darling(default)]
    pub export: Option<bool>,

    /// Use strict mode (no extra properties)
    #[darling(default)]
    pub strict: bool,
}

impl ContainerAttrs {
    /// Get the schema name (renamed or original).
    pub fn schema_name(&self) -> String {
        self.rename
            .clone()
            .unwrap_or_else(|| self.ident.to_string())
    }

    /// Check if this is an internally tagged enum.
    pub fn is_internally_tagged(&self) -> bool {
        self.tag.is_some() && self.content.is_none()
    }

    /// Check if this is an adjacently tagged enum.
    pub fn is_adjacently_tagged(&self) -> bool {
        self.tag.is_some() && self.content.is_some()
    }

    /// Get the export setting (defaults to true).
    pub fn should_export(&self) -> bool {
        self.export.unwrap_or(true)
    }
}

/// Field-level attributes parsed from `#[zod(...)]` on struct fields.
///
/// Note: This struct cannot derive Default because `Type` and `Visibility`
/// don't implement Default. Use darling's FromField to parse from syn::Field.
#[derive(Debug, Clone, FromField)]
#[darling(attributes(zod))]
pub struct FieldAttrs {
    /// Field identifier (None for tuple struct fields)
    pub ident: Option<Ident>,

    /// Field type
    pub ty: Type,

    /// Field visibility
    pub vis: Visibility,

    /// Rename this field
    #[darling(default)]
    pub rename: Option<String>,

    /// Skip this field in schema
    #[darling(default)]
    pub skip: bool,

    /// Mark as optional (.optional())
    #[darling(default)]
    pub optional: bool,

    /// Mark as nullable (.nullable())
    #[darling(default)]
    pub nullable: bool,

    /// Default value expression
    #[darling(default)]
    pub default: Option<String>,

    /// Flatten nested object
    #[darling(default)]
    pub flatten: bool,

    /// Field description
    #[darling(default)]
    pub description: Option<String>,

    /// Custom Zod type override
    #[darling(default, rename = "type")]
    pub type_override: Option<String>,

    /// Validation: minimum value for numbers
    #[darling(default)]
    pub min: Option<f64>,

    /// Validation: maximum value for numbers
    #[darling(default)]
    pub max: Option<f64>,

    /// Validation: minimum length for strings/arrays
    #[darling(default)]
    pub min_length: Option<usize>,

    /// Validation: maximum length for strings/arrays
    #[darling(default)]
    pub max_length: Option<usize>,

    /// Validation: exact length for strings/arrays
    #[darling(default)]
    pub length: Option<usize>,

    /// Validation: email format
    #[darling(default)]
    pub email: bool,

    /// Validation: URL format
    #[darling(default)]
    pub url: bool,

    /// Validation: UUID format
    #[darling(default)]
    pub uuid: bool,

    /// Validation: CUID format
    #[darling(default)]
    pub cuid: bool,

    /// Validation: datetime format
    #[darling(default)]
    pub datetime: bool,

    /// Validation: IP address format
    #[darling(default)]
    pub ip: bool,

    /// Validation: regex pattern
    #[darling(default)]
    pub regex: Option<String>,

    /// Validation: string starts with
    #[darling(default)]
    pub starts_with: Option<String>,

    /// Validation: string ends with
    #[darling(default)]
    pub ends_with: Option<String>,

    /// Validation: positive number (> 0)
    #[darling(default)]
    pub positive: bool,

    /// Validation: negative number (< 0)
    #[darling(default)]
    pub negative: bool,

    /// Validation: non-negative number (>= 0)
    #[darling(default)]
    pub nonnegative: bool,

    /// Validation: non-positive number (<= 0)
    #[darling(default)]
    pub nonpositive: bool,

    /// Validation: integer
    #[darling(default)]
    pub int: bool,

    /// Validation: finite number
    #[darling(default)]
    pub finite: bool,

    /// Validation: non-empty array/string
    #[darling(default)]
    pub nonempty: bool,

    /// Custom validation expression
    #[darling(default)]
    pub custom: Option<String>,
}

impl FieldAttrs {
    /// Get the schema name for this field (renamed or original).
    pub fn schema_name(&self, rename_rule: Option<RenameRule>) -> String {
        // Explicit rename takes precedence
        if let Some(ref name) = self.rename {
            return name.clone();
        }

        // Apply rename_all rule if present
        if let Some(ident) = &self.ident {
            let name = ident.to_string();
            if let Some(rule) = rename_rule {
                rule.apply(&name)
            } else {
                name
            }
        } else {
            String::new()
        }
    }

    /// Check if this field has any validation rules.
    pub fn has_validation(&self) -> bool {
        self.min.is_some()
            || self.max.is_some()
            || self.min_length.is_some()
            || self.max_length.is_some()
            || self.length.is_some()
            || self.email
            || self.url
            || self.uuid
            || self.cuid
            || self.datetime
            || self.ip
            || self.regex.is_some()
            || self.starts_with.is_some()
            || self.ends_with.is_some()
            || self.positive
            || self.negative
            || self.nonnegative
            || self.nonpositive
            || self.int
            || self.finite
            || self.nonempty
            || self.custom.is_some()
    }

    /// Convert validation attributes to ValidationRule list.
    pub fn to_validation_rules(&self) -> Vec<crate::ir::ValidationRule> {
        use crate::ir::ValidationRule;
        let mut rules = Vec::new();

        // String validations
        if let Some(n) = self.min_length {
            rules.push(ValidationRule::MinLength(n));
        }
        if let Some(n) = self.max_length {
            rules.push(ValidationRule::MaxLength(n));
        }
        if let Some(n) = self.length {
            rules.push(ValidationRule::Length(n));
        }
        if self.email {
            rules.push(ValidationRule::Email);
        }
        if self.url {
            rules.push(ValidationRule::Url);
        }
        if self.uuid {
            rules.push(ValidationRule::Uuid);
        }
        if self.cuid {
            rules.push(ValidationRule::Cuid);
        }
        if self.datetime {
            rules.push(ValidationRule::Datetime);
        }
        if self.ip {
            rules.push(ValidationRule::Ip);
        }
        if let Some(ref pattern) = self.regex {
            rules.push(ValidationRule::Regex(pattern.clone()));
        }
        if let Some(ref prefix) = self.starts_with {
            rules.push(ValidationRule::StartsWith(prefix.clone()));
        }
        if let Some(ref suffix) = self.ends_with {
            rules.push(ValidationRule::EndsWith(suffix.clone()));
        }

        // Number validations
        if let Some(n) = self.min {
            rules.push(ValidationRule::Min(n));
        }
        if let Some(n) = self.max {
            rules.push(ValidationRule::Max(n));
        }
        if self.positive {
            rules.push(ValidationRule::Positive);
        }
        if self.negative {
            rules.push(ValidationRule::Negative);
        }
        if self.nonnegative {
            rules.push(ValidationRule::NonNegative);
        }
        if self.nonpositive {
            rules.push(ValidationRule::NonPositive);
        }
        if self.int {
            rules.push(ValidationRule::Int);
        }
        if self.finite {
            rules.push(ValidationRule::Finite);
        }

        // Array validations
        if self.nonempty {
            rules.push(ValidationRule::Nonempty);
        }

        // Custom validation
        if let Some(ref expr) = self.custom {
            rules.push(ValidationRule::Custom(expr.clone()));
        }

        rules
    }
}

/// Variant-level attributes for enum variants.
///
/// Note: This struct cannot derive Default because `Ident` doesn't implement Default.
/// Use darling's FromVariant to parse from syn::Variant.
#[derive(Debug, Clone, FromVariant)]
#[darling(attributes(zod))]
pub struct VariantAttrs {
    /// Variant identifier
    pub ident: Ident,

    /// Rename this variant
    #[darling(default)]
    pub rename: Option<String>,

    /// Skip this variant in schema
    #[darling(default)]
    pub skip: bool,

    /// Variant description
    #[darling(default)]
    pub description: Option<String>,

    /// Mark as deprecated
    #[darling(default)]
    pub deprecated: bool,
}

impl VariantAttrs {
    /// Get the schema name for this variant (renamed or original).
    pub fn schema_name(&self, rename_rule: Option<RenameRule>) -> String {
        // Explicit rename takes precedence
        if let Some(ref name) = self.rename {
            return name.clone();
        }

        // Apply rename_all rule if present
        let name = self.ident.to_string();
        if let Some(rule) = rename_rule {
            rule.apply(&name)
        } else {
            name
        }
    }
}

/// Rename rule for field/variant name transformation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, FromMeta)]
pub enum RenameRule {
    /// camelCase
    #[darling(rename = "camelCase")]
    CamelCase,

    /// snake_case
    #[darling(rename = "snake_case")]
    SnakeCase,

    /// PascalCase
    #[darling(rename = "PascalCase")]
    PascalCase,

    /// SCREAMING_SNAKE_CASE
    #[darling(rename = "SCREAMING_SNAKE_CASE")]
    ScreamingSnakeCase,

    /// kebab-case
    #[darling(rename = "kebab-case")]
    KebabCase,
}

impl RenameRule {
    /// Apply the rename rule to a string.
    pub fn apply(&self, name: &str) -> String {
        use convert_case::{Case, Casing};

        match self {
            RenameRule::CamelCase => name.to_case(Case::Camel),
            RenameRule::SnakeCase => name.to_case(Case::Snake),
            RenameRule::PascalCase => name.to_case(Case::Pascal),
            RenameRule::ScreamingSnakeCase => name.to_case(Case::UpperSnake),
            RenameRule::KebabCase => name.to_case(Case::Kebab),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rename_rule_camel_case() {
        assert_eq!(RenameRule::CamelCase.apply("user_name"), "userName");
        assert_eq!(RenameRule::CamelCase.apply("UserName"), "userName");
        assert_eq!(RenameRule::CamelCase.apply("user-name"), "userName");
    }

    #[test]
    fn test_rename_rule_snake_case() {
        assert_eq!(RenameRule::SnakeCase.apply("userName"), "user_name");
        assert_eq!(RenameRule::SnakeCase.apply("UserName"), "user_name");
        assert_eq!(RenameRule::SnakeCase.apply("user-name"), "user_name");
    }

    #[test]
    fn test_rename_rule_pascal_case() {
        assert_eq!(RenameRule::PascalCase.apply("user_name"), "UserName");
        assert_eq!(RenameRule::PascalCase.apply("userName"), "UserName");
        assert_eq!(RenameRule::PascalCase.apply("user-name"), "UserName");
    }

    #[test]
    fn test_rename_rule_screaming_snake_case() {
        assert_eq!(
            RenameRule::ScreamingSnakeCase.apply("userName"),
            "USER_NAME"
        );
        assert_eq!(
            RenameRule::ScreamingSnakeCase.apply("user_name"),
            "USER_NAME"
        );
    }

    #[test]
    fn test_rename_rule_kebab_case() {
        assert_eq!(RenameRule::KebabCase.apply("userName"), "user-name");
        assert_eq!(RenameRule::KebabCase.apply("user_name"), "user-name");
        assert_eq!(RenameRule::KebabCase.apply("UserName"), "user-name");
    }
}

#[cfg(test)]
mod proptest_tests {
    use super::*;
    use proptest::prelude::*;

    /// Strategy for generating valid identifier-like strings.
    /// These are strings that could be valid Rust field names.
    fn arb_identifier() -> impl Strategy<Value = String> {
        // Generate identifiers with 1-3 words separated by underscores
        // Each word is 1-10 lowercase letters
        proptest::collection::vec("[a-z]{1,10}", 1..=3).prop_map(|words| words.join("_"))
    }

    /// Strategy for generating arbitrary RenameRule values.
    fn arb_rename_rule() -> impl Strategy<Value = RenameRule> {
        prop_oneof![
            Just(RenameRule::CamelCase),
            Just(RenameRule::SnakeCase),
            Just(RenameRule::PascalCase),
            Just(RenameRule::ScreamingSnakeCase),
            Just(RenameRule::KebabCase),
        ]
    }

    proptest! {
        /// **Property 7: Rename Transformation**
        ///
        /// *For any* struct with `rename_all` attribute, all field names in the
        /// generated schema SHALL follow the specified naming convention.
        ///
        /// This property verifies that:
        /// 1. Applying a rename rule produces a non-empty string
        /// 2. The output follows the expected case convention pattern
        ///
        /// **Validates: Requirements 4.2**
        #[test]
        fn prop_rename_transformation_produces_valid_output(
            name in arb_identifier(),
            rule in arb_rename_rule()
        ) {
            let result = rule.apply(&name);

            // Result should not be empty
            prop_assert!(!result.is_empty(), "Rename should produce non-empty output");

            // Verify the output follows the expected pattern for each rule
            match rule {
                RenameRule::CamelCase => {
                    // camelCase: first char lowercase, no underscores or hyphens
                    prop_assert!(
                        result.chars().next().unwrap().is_lowercase(),
                        "camelCase should start with lowercase: {}", result
                    );
                    prop_assert!(
                        !result.contains('_') && !result.contains('-'),
                        "camelCase should not contain underscores or hyphens: {}", result
                    );
                }
                RenameRule::SnakeCase => {
                    // snake_case: all lowercase, may contain underscores
                    prop_assert!(
                        result.chars().all(|c| c.is_lowercase() || c == '_'),
                        "snake_case should be all lowercase with underscores: {}", result
                    );
                }
                RenameRule::PascalCase => {
                    // PascalCase: first char uppercase, no underscores or hyphens
                    prop_assert!(
                        result.chars().next().unwrap().is_uppercase(),
                        "PascalCase should start with uppercase: {}", result
                    );
                    prop_assert!(
                        !result.contains('_') && !result.contains('-'),
                        "PascalCase should not contain underscores or hyphens: {}", result
                    );
                }
                RenameRule::ScreamingSnakeCase => {
                    // SCREAMING_SNAKE_CASE: all uppercase, may contain underscores
                    prop_assert!(
                        result.chars().all(|c| c.is_uppercase() || c == '_'),
                        "SCREAMING_SNAKE_CASE should be all uppercase with underscores: {}", result
                    );
                }
                RenameRule::KebabCase => {
                    // kebab-case: all lowercase, may contain hyphens
                    prop_assert!(
                        result.chars().all(|c| c.is_lowercase() || c == '-'),
                        "kebab-case should be all lowercase with hyphens: {}", result
                    );
                }
            }
        }

        /// Property: Rename transformation is deterministic.
        ///
        /// *For any* field name and rename rule, applying the rule twice
        /// SHALL produce the same result.
        #[test]
        fn prop_rename_transformation_is_deterministic(
            name in arb_identifier(),
            rule in arb_rename_rule()
        ) {
            let result1 = rule.apply(&name);
            let result2 = rule.apply(&name);

            prop_assert_eq!(result1, result2, "Rename should be deterministic");
        }
    }
}
