//! Metadata IR definitions.
//!
//! This module defines metadata structures for schemas and fields.
//! Metadata provides additional information that doesn't affect the
//! schema structure but is useful for documentation and tooling.

use serde::{Deserialize, Serialize};

/// Schema metadata.
///
/// Contains additional information about a schema that doesn't affect
/// the schema structure but provides documentation and hints.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SchemaMetadata {
    /// Schema description (from doc comments or attribute)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Whether the schema is deprecated
    #[serde(default)]
    pub deprecated: bool,

    /// Deprecation message explaining why and what to use instead
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deprecation_message: Option<String>,

    /// Example values for documentation
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub examples: Vec<String>,

    /// Tags for categorization
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,

    /// Version when this schema was introduced
    #[serde(skip_serializing_if = "Option::is_none")]
    pub since: Option<String>,

    /// External documentation URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub docs_url: Option<String>,
}

impl SchemaMetadata {
    /// Create metadata with a description.
    #[allow(unused)]
    pub fn with_description(description: impl Into<String>) -> Self {
        Self {
            description: Some(description.into()),
            ..Default::default()
        }
    }

    /// Set the description.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Mark as deprecated.
    #[allow(unused)]
    #[allow(clippy::wrong_self_convention)]
    pub fn as_deprecated(mut self) -> Self {
        self.deprecated = true;
        self
    }

    /// Mark as deprecated with a message.
    #[allow(unused)]
    pub fn deprecated_with_message(mut self, message: impl Into<String>) -> Self {
        self.deprecated = true;
        self.deprecation_message = Some(message.into());
        self
    }

    /// Add an example value.
    #[allow(unused)]
    pub fn with_example(mut self, example: impl Into<String>) -> Self {
        self.examples.push(example.into());
        self
    }

    /// Add multiple example values.
    #[allow(unused)]
    pub fn with_examples(mut self, examples: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.examples.extend(examples.into_iter().map(|e| e.into()));
        self
    }

    /// Add a tag.
    #[allow(unused)]
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Add multiple tags.
    #[allow(unused)]
    pub fn with_tags(mut self, tags: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.tags.extend(tags.into_iter().map(|t| t.into()));
        self
    }

    /// Set the version when this schema was introduced.
    #[allow(unused)]
    pub fn since(mut self, version: impl Into<String>) -> Self {
        self.since = Some(version.into());
        self
    }

    /// Set the external documentation URL.
    #[allow(unused)]
    pub fn docs_url(mut self, url: impl Into<String>) -> Self {
        self.docs_url = Some(url.into());
        self
    }

    /// Check if this metadata has any content.
    #[allow(unused)]
    pub fn is_empty(&self) -> bool {
        self.description.is_none()
            && !self.deprecated
            && self.deprecation_message.is_none()
            && self.examples.is_empty()
            && self.tags.is_empty()
            && self.since.is_none()
            && self.docs_url.is_none()
    }

    /// Merge another metadata into this one, preferring non-empty values from other.
    #[allow(unused)]
    pub fn merge(&mut self, other: &SchemaMetadata) {
        if other.description.is_some() {
            self.description = other.description.clone();
        }
        if other.deprecated {
            self.deprecated = true;
        }
        if other.deprecation_message.is_some() {
            self.deprecation_message = other.deprecation_message.clone();
        }
        self.examples.extend(other.examples.iter().cloned());
        self.tags.extend(other.tags.iter().cloned());
        if other.since.is_some() {
            self.since = other.since.clone();
        }
        if other.docs_url.is_some() {
            self.docs_url = other.docs_url.clone();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_metadata_default() {
        let meta = SchemaMetadata::default();
        assert!(meta.is_empty());
        assert!(meta.description.is_none());
        assert!(!meta.deprecated);
    }

    #[test]
    fn test_schema_metadata_with_description() {
        let meta = SchemaMetadata::with_description("A user account");
        assert_eq!(meta.description, Some("A user account".to_string()));
        assert!(!meta.is_empty());
    }

    #[test]
    fn test_schema_metadata_builder() {
        let meta = SchemaMetadata::default()
            .description("A user account")
            .as_deprecated()
            .with_example("{ \"id\": 1, \"name\": \"John\" }")
            .with_tag("user")
            .with_tag("account")
            .since("1.0.0");

        assert_eq!(meta.description, Some("A user account".to_string()));
        assert!(meta.deprecated);
        assert_eq!(meta.examples.len(), 1);
        assert_eq!(meta.tags, vec!["user", "account"]);
        assert_eq!(meta.since, Some("1.0.0".to_string()));
    }

    #[test]
    fn test_schema_metadata_deprecated_with_message() {
        let meta = SchemaMetadata::default().deprecated_with_message("Use UserV2 instead");

        assert!(meta.deprecated);
        assert_eq!(
            meta.deprecation_message,
            Some("Use UserV2 instead".to_string())
        );
    }

    #[test]
    fn test_schema_metadata_merge() {
        let mut base = SchemaMetadata::default()
            .description("Base description")
            .with_tag("base");

        let other = SchemaMetadata::default()
            .description("Override description")
            .as_deprecated()
            .with_tag("other");

        base.merge(&other);

        assert_eq!(base.description, Some("Override description".to_string()));
        assert!(base.deprecated);
        assert_eq!(base.tags, vec!["base", "other"]);
    }

    #[test]
    fn test_schema_metadata_is_empty() {
        assert!(SchemaMetadata::default().is_empty());
        assert!(!SchemaMetadata::with_description("test").is_empty());
        assert!(!SchemaMetadata::default().as_deprecated().is_empty());
        assert!(!SchemaMetadata::default().with_tag("test").is_empty());
    }
}
