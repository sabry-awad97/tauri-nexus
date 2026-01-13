//! Serde attribute compatibility.
//!
//! This module handles parsing serde attributes when the `serde-compat` feature is enabled.
//! It allows zod-rs to respect existing serde configuration, with zod attributes taking
//! precedence when both are present.

use syn::Attribute;

use super::RenameRule;

/// Serde container attributes extracted from `#[serde(...)]`.
#[derive(Debug, Clone, Default)]
pub struct SerdeContainerAttrs {
    /// Rename the type
    pub rename: Option<String>,

    /// Rename all fields using a case convention
    pub rename_all: Option<RenameRule>,

    /// Tag field name for internally tagged enums
    pub tag: Option<String>,

    /// Content field name for adjacently tagged enums
    pub content: Option<String>,

    /// Whether the enum is untagged
    pub untagged: bool,
}

impl SerdeContainerAttrs {
    /// Parse serde attributes from a list of attributes.
    pub fn from_attrs(attrs: &[Attribute]) -> Self {
        let mut result = Self::default();

        for attr in attrs {
            if !attr.path().is_ident("serde") {
                continue;
            }

            let _ = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("rename") {
                    if let Some(value) = parse_string_value(&meta)? {
                        result.rename = Some(value);
                    }
                } else if meta.path.is_ident("rename_all") {
                    if let Some(value) = parse_string_value(&meta)? {
                        result.rename_all = parse_rename_rule(&value);
                    }
                } else if meta.path.is_ident("tag") {
                    if let Some(value) = parse_string_value(&meta)? {
                        result.tag = Some(value);
                    }
                } else if meta.path.is_ident("content") {
                    if let Some(value) = parse_string_value(&meta)? {
                        result.content = Some(value);
                    }
                } else if meta.path.is_ident("untagged") {
                    result.untagged = true;
                }
                Ok(())
            });
        }

        result
    }
}

/// Serde field attributes extracted from `#[serde(...)]`.
#[derive(Debug, Clone, Default)]
pub struct SerdeFieldAttrs {
    /// Rename this field
    pub rename: Option<String>,

    /// Skip this field
    pub skip: bool,

    /// Skip serializing this field
    pub skip_serializing: bool,

    /// Skip deserializing this field
    pub skip_deserializing: bool,

    /// Field has a default value
    pub default: bool,

    /// Flatten nested object
    pub flatten: bool,
}

impl SerdeFieldAttrs {
    /// Parse serde attributes from a list of attributes.
    pub fn from_attrs(attrs: &[Attribute]) -> Self {
        let mut result = Self::default();

        for attr in attrs {
            if !attr.path().is_ident("serde") {
                continue;
            }

            let _ = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("rename") {
                    if let Some(value) = parse_string_value(&meta)? {
                        result.rename = Some(value);
                    }
                } else if meta.path.is_ident("skip") {
                    result.skip = true;
                } else if meta.path.is_ident("skip_serializing") {
                    result.skip_serializing = true;
                } else if meta.path.is_ident("skip_deserializing") {
                    result.skip_deserializing = true;
                } else if meta.path.is_ident("default") {
                    result.default = true;
                } else if meta.path.is_ident("flatten") {
                    result.flatten = true;
                }
                Ok(())
            });
        }

        result
    }

    /// Check if this field should be skipped entirely.
    /// For schema generation, we skip if the field is skipped for serialization OR deserialization,
    /// since the schema represents the data contract.
    pub fn should_skip(&self) -> bool {
        self.skip || self.skip_serializing || self.skip_deserializing
    }
}

/// Serde variant attributes extracted from `#[serde(...)]`.
#[derive(Debug, Clone, Default)]
pub struct SerdeVariantAttrs {
    /// Rename this variant
    pub rename: Option<String>,

    /// Skip this variant
    pub skip: bool,

    /// Skip serializing this variant
    pub skip_serializing: bool,

    /// Skip deserializing this variant
    pub skip_deserializing: bool,
}

impl SerdeVariantAttrs {
    /// Parse serde attributes from a list of attributes.
    pub fn from_attrs(attrs: &[Attribute]) -> Self {
        let mut result = Self::default();

        for attr in attrs {
            if !attr.path().is_ident("serde") {
                continue;
            }

            let _ = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("rename") {
                    if let Some(value) = parse_string_value(&meta)? {
                        result.rename = Some(value);
                    }
                } else if meta.path.is_ident("skip") {
                    result.skip = true;
                } else if meta.path.is_ident("skip_serializing") {
                    result.skip_serializing = true;
                } else if meta.path.is_ident("skip_deserializing") {
                    result.skip_deserializing = true;
                }
                Ok(())
            });
        }

        result
    }

    /// Check if this variant should be skipped entirely.
    pub fn should_skip(&self) -> bool {
        self.skip || (self.skip_serializing && self.skip_deserializing)
    }
}

/// Parse a string value from a meta item like `rename = "value"`.
fn parse_string_value(meta: &syn::meta::ParseNestedMeta) -> syn::Result<Option<String>> {
    let value: syn::LitStr = meta.value()?.parse()?;
    Ok(Some(value.value()))
}

/// Parse a serde rename rule string into our RenameRule enum.
fn parse_rename_rule(s: &str) -> Option<RenameRule> {
    match s {
        "camelCase" => Some(RenameRule::CamelCase),
        "snake_case" => Some(RenameRule::SnakeCase),
        "PascalCase" => Some(RenameRule::PascalCase),
        "SCREAMING_SNAKE_CASE" => Some(RenameRule::ScreamingSnakeCase),
        "kebab-case" => Some(RenameRule::KebabCase),
        _ => None,
    }
}

/// Merge zod attributes with serde attributes, with zod taking precedence.
#[allow(unused)]
pub fn merge_container_attrs(
    zod_rename: Option<String>,
    zod_rename_all: Option<RenameRule>,
    zod_tag: Option<String>,
    zod_content: Option<String>,
    serde: &SerdeContainerAttrs,
) -> (
    Option<String>,
    Option<RenameRule>,
    Option<String>,
    Option<String>,
) {
    (
        zod_rename.or_else(|| serde.rename.clone()),
        zod_rename_all.or(serde.rename_all),
        zod_tag.or_else(|| serde.tag.clone()),
        zod_content.or_else(|| serde.content.clone()),
    )
}

/// Merge zod field attributes with serde field attributes, with zod taking precedence.
#[allow(unused)]
pub fn merge_field_attrs(
    zod_rename: Option<String>,
    zod_skip: bool,
    zod_flatten: bool,
    serde: &SerdeFieldAttrs,
) -> (Option<String>, bool, bool, bool) {
    (
        zod_rename.or_else(|| serde.rename.clone()),
        zod_skip || serde.should_skip(),
        zod_flatten || serde.flatten,
        serde.default, // serde default makes field optional
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_rename_rule() {
        assert_eq!(parse_rename_rule("camelCase"), Some(RenameRule::CamelCase));
        assert_eq!(parse_rename_rule("snake_case"), Some(RenameRule::SnakeCase));
        assert_eq!(
            parse_rename_rule("PascalCase"),
            Some(RenameRule::PascalCase)
        );
        assert_eq!(
            parse_rename_rule("SCREAMING_SNAKE_CASE"),
            Some(RenameRule::ScreamingSnakeCase)
        );
        assert_eq!(parse_rename_rule("kebab-case"), Some(RenameRule::KebabCase));
        assert_eq!(parse_rename_rule("unknown"), None);
    }

    #[test]
    fn test_serde_field_attrs_should_skip() {
        let mut attrs = SerdeFieldAttrs::default();
        assert!(!attrs.should_skip());

        attrs.skip = true;
        assert!(attrs.should_skip());

        attrs.skip = false;
        attrs.skip_serializing = true;
        assert!(attrs.should_skip()); // skip_serializing alone triggers skip

        attrs.skip_serializing = false;
        attrs.skip_deserializing = true;
        assert!(attrs.should_skip()); // skip_deserializing alone triggers skip

        attrs.skip_deserializing = false;
        assert!(!attrs.should_skip()); // neither set, should not skip
    }

    #[test]
    fn test_merge_container_attrs() {
        let serde = SerdeContainerAttrs {
            rename: Some("SerdeUser".to_string()),
            rename_all: Some(RenameRule::CamelCase),
            tag: Some("type".to_string()),
            content: None,
            untagged: false,
        };

        // Zod takes precedence
        let (rename, rename_all, tag, _content) =
            merge_container_attrs(Some("ZodUser".to_string()), None, None, None, &serde);
        assert_eq!(rename, Some("ZodUser".to_string()));
        assert_eq!(rename_all, Some(RenameRule::CamelCase)); // from serde
        assert_eq!(tag, Some("type".to_string())); // from serde

        // Serde used when zod not specified
        let (rename, _, _, _) = merge_container_attrs(None, None, None, None, &serde);
        assert_eq!(rename, Some("SerdeUser".to_string()));
    }

    #[test]
    fn test_merge_field_attrs() {
        let serde = SerdeFieldAttrs {
            rename: Some("serde_name".to_string()),
            skip: false,
            skip_serializing: false,
            skip_deserializing: false,
            default: true,
            flatten: true,
        };

        // Zod takes precedence
        let (rename, skip, flatten, has_default) =
            merge_field_attrs(Some("zod_name".to_string()), false, false, &serde);
        assert_eq!(rename, Some("zod_name".to_string()));
        assert!(!skip);
        assert!(flatten); // from serde
        assert!(has_default); // from serde

        // Serde used when zod not specified
        let (rename, _, _, _) = merge_field_attrs(None, false, false, &serde);
        assert_eq!(rename, Some("serde_name".to_string()));
    }
}
