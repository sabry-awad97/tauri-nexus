//! Error types for the macro crate.
//!
//! This module defines error types for parsing and code generation,
//! providing detailed error messages with span information and suggestions.

use proc_macro2::Span;
use std::fmt;

/// Errors that can occur during parsing.
///
/// Each variant provides specific context about what went wrong,
/// enabling helpful error messages with suggestions.
#[derive(Debug, Clone)]
pub enum ParseError {
    /// An unsupported type was encountered.
    UnsupportedType {
        /// The type that is not supported.
        type_name: String,
        /// Optional span for error location.
        span: Option<Span>,
        /// Suggestions for alternatives.
        suggestions: Vec<String>,
    },

    /// Empty path in type expression.
    EmptyPath {
        /// Optional span for error location.
        span: Option<Span>,
    },

    /// Missing generic parameter for a type.
    MissingGeneric {
        /// The type that requires a generic parameter.
        type_name: &'static str,
        /// Optional span for error location.
        span: Option<Span>,
    },

    /// Invalid attribute was used.
    InvalidAttribute {
        /// The invalid attribute name.
        attribute: String,
        /// Optional span for error location.
        span: Option<Span>,
        /// Valid alternatives.
        valid_alternatives: Vec<String>,
    },

    /// Conflicting attributes were used together.
    ConflictingAttributes {
        /// First conflicting attribute.
        first: String,
        /// Second conflicting attribute.
        second: String,
        /// Optional span for error location.
        span: Option<Span>,
        /// Explanation of why they conflict.
        explanation: Option<String>,
    },

    /// Invalid regex pattern.
    InvalidRegex {
        /// The invalid pattern.
        pattern: String,
        /// The regex error message.
        error: String,
        /// Optional span for error location.
        span: Option<Span>,
    },

    /// Invalid attribute value.
    InvalidAttributeValue {
        /// The attribute name.
        attribute: String,
        /// The invalid value.
        value: String,
        /// Expected format or type.
        expected: String,
        /// Optional span for error location.
        span: Option<Span>,
    },

    /// Referenced type doesn't implement ZodSchema.
    MissingZodSchemaImpl {
        /// The type that doesn't implement ZodSchema.
        type_name: String,
        /// Optional span for error location.
        span: Option<Span>,
    },

    /// Syn parse error.
    SynError(syn::Error),

    /// Darling parse error.
    DarlingError(darling::Error),

    /// Generic error with message.
    Other {
        /// Error message.
        message: String,
        /// Optional span for error location.
        span: Option<Span>,
        /// Suggestions for fixing the error.
        suggestions: Vec<String>,
    },
}

impl ParseError {
    /// Create an unsupported type error.
    pub fn unsupported_type(type_name: impl Into<String>) -> Self {
        Self::UnsupportedType {
            type_name: type_name.into(),
            span: None,
            suggestions: Vec::new(),
        }
    }

    /// Create an empty path error.
    pub fn empty_path() -> Self {
        Self::EmptyPath { span: None }
    }

    /// Create a missing generic error.
    pub fn missing_generic(type_name: &'static str) -> Self {
        Self::MissingGeneric {
            type_name,
            span: None,
        }
    }

    /// Create an invalid attribute error.
    pub fn invalid_attribute(attribute: impl Into<String>) -> Self {
        Self::InvalidAttribute {
            attribute: attribute.into(),
            span: None,
            valid_alternatives: Vec::new(),
        }
    }

    /// Create a conflicting attributes error.
    pub fn conflicting_attributes(first: impl Into<String>, second: impl Into<String>) -> Self {
        Self::ConflictingAttributes {
            first: first.into(),
            second: second.into(),
            span: None,
            explanation: None,
        }
    }

    /// Create an invalid regex error.
    pub fn invalid_regex(pattern: impl Into<String>, error: impl Into<String>) -> Self {
        Self::InvalidRegex {
            pattern: pattern.into(),
            error: error.into(),
            span: None,
        }
    }

    /// Create an invalid attribute value error.
    pub fn invalid_attribute_value(
        attribute: impl Into<String>,
        value: impl Into<String>,
        expected: impl Into<String>,
    ) -> Self {
        Self::InvalidAttributeValue {
            attribute: attribute.into(),
            value: value.into(),
            expected: expected.into(),
            span: None,
        }
    }

    /// Create a missing ZodSchema implementation error.
    pub fn missing_zod_schema_impl(type_name: impl Into<String>) -> Self {
        Self::MissingZodSchemaImpl {
            type_name: type_name.into(),
            span: None,
        }
    }

    /// Create a generic error with a message.
    pub fn other(message: impl Into<String>) -> Self {
        Self::Other {
            message: message.into(),
            span: None,
            suggestions: Vec::new(),
        }
    }

    /// Add span information to the error.
    pub fn with_span(mut self, span: Span) -> Self {
        match &mut self {
            Self::UnsupportedType { span: s, .. } => *s = Some(span),
            Self::EmptyPath { span: s } => *s = Some(span),
            Self::MissingGeneric { span: s, .. } => *s = Some(span),
            Self::InvalidAttribute { span: s, .. } => *s = Some(span),
            Self::ConflictingAttributes { span: s, .. } => *s = Some(span),
            Self::InvalidRegex { span: s, .. } => *s = Some(span),
            Self::InvalidAttributeValue { span: s, .. } => *s = Some(span),
            Self::MissingZodSchemaImpl { span: s, .. } => *s = Some(span),
            Self::SynError(_) => {}
            Self::DarlingError(_) => {}
            Self::Other { span: s, .. } => *s = Some(span),
        }
        self
    }

    /// Add a suggestion to the error.
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        match &mut self {
            Self::UnsupportedType { suggestions, .. } => suggestions.push(suggestion.into()),
            Self::InvalidAttribute {
                valid_alternatives, ..
            } => valid_alternatives.push(suggestion.into()),
            Self::Other { suggestions, .. } => suggestions.push(suggestion.into()),
            _ => {}
        }
        self
    }

    /// Add an explanation for conflicting attributes.
    pub fn with_explanation(mut self, explanation: impl Into<String>) -> Self {
        if let Self::ConflictingAttributes {
            explanation: exp, ..
        } = &mut self
        {
            *exp = Some(explanation.into());
        }
        self
    }

    /// Get the span associated with this error, if any.
    pub fn span(&self) -> Option<Span> {
        match self {
            Self::UnsupportedType { span, .. } => *span,
            Self::EmptyPath { span } => *span,
            Self::MissingGeneric { span, .. } => *span,
            Self::InvalidAttribute { span, .. } => *span,
            Self::ConflictingAttributes { span, .. } => *span,
            Self::InvalidRegex { span, .. } => *span,
            Self::InvalidAttributeValue { span, .. } => *span,
            Self::MissingZodSchemaImpl { span, .. } => *span,
            Self::SynError(e) => Some(e.span()),
            Self::DarlingError(_) => None,
            Self::Other { span, .. } => *span,
        }
    }

    /// Convert to a syn::Error for proc-macro error reporting.
    pub fn into_syn_error(self) -> syn::Error {
        let span = self.span().unwrap_or_else(Span::call_site);
        let message = self.to_string();
        syn::Error::new(span, message)
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedType {
                type_name,
                suggestions,
                ..
            } => {
                write!(f, "Unsupported type: `{}`", type_name)?;
                if !suggestions.is_empty() {
                    write!(f, "\n\nSuggestions:")?;
                    for suggestion in suggestions {
                        write!(f, "\n  - {}", suggestion)?;
                    }
                }
                Ok(())
            }
            Self::EmptyPath { .. } => {
                write!(f, "Empty path in type expression")
            }
            Self::MissingGeneric { type_name, .. } => {
                write!(
                    f,
                    "Missing generic parameter for `{}`\n\nExpected: `{}<T>`",
                    type_name, type_name
                )
            }
            Self::InvalidAttribute {
                attribute,
                valid_alternatives,
                ..
            } => {
                write!(f, "Invalid attribute: `{}`", attribute)?;
                if !valid_alternatives.is_empty() {
                    write!(f, "\n\nValid alternatives:")?;
                    for alt in valid_alternatives {
                        write!(f, "\n  - {}", alt)?;
                    }
                }
                Ok(())
            }
            Self::ConflictingAttributes {
                first,
                second,
                explanation,
                ..
            } => {
                write!(
                    f,
                    "Conflicting attributes: `{}` and `{}` cannot be used together",
                    first, second
                )?;
                if let Some(exp) = explanation {
                    write!(f, "\n\n{}", exp)?;
                }
                Ok(())
            }
            Self::InvalidRegex { pattern, error, .. } => {
                write!(f, "Invalid regex pattern `{}`: {}", pattern, error)
            }
            Self::InvalidAttributeValue {
                attribute,
                value,
                expected,
                ..
            } => {
                write!(
                    f,
                    "Invalid value `{}` for attribute `{}`\n\nExpected: {}",
                    value, attribute, expected
                )
            }
            Self::MissingZodSchemaImpl { type_name, .. } => {
                write!(
                    f,
                    "Type `{}` does not implement `ZodSchema`\n\n\
                     Add `#[derive(ZodSchema)]` to the type definition, or ensure it's a supported primitive type.",
                    type_name
                )
            }
            Self::SynError(e) => write!(f, "{}", e),
            Self::DarlingError(e) => write!(f, "{}", e),
            Self::Other {
                message,
                suggestions,
                ..
            } => {
                write!(f, "{}", message)?;
                if !suggestions.is_empty() {
                    write!(f, "\n\nSuggestions:")?;
                    for suggestion in suggestions {
                        write!(f, "\n  - {}", suggestion)?;
                    }
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for ParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::SynError(e) => Some(e),
            Self::DarlingError(e) => Some(e),
            _ => None,
        }
    }
}

impl From<syn::Error> for ParseError {
    fn from(error: syn::Error) -> Self {
        Self::SynError(error)
    }
}

impl From<darling::Error> for ParseError {
    fn from(error: darling::Error) -> Self {
        Self::DarlingError(error)
    }
}

/// Errors that can occur during code generation.
#[derive(Debug, Clone)]
pub enum GeneratorError {
    /// Circular dependency detected in schema references.
    CircularDependency {
        /// The cycle path (list of type names forming the cycle).
        cycle: Vec<String>,
    },

    /// Unknown type reference in schema.
    UnknownTypeReference {
        /// The unknown type name.
        type_name: String,
        /// The schema that references it.
        referencing_schema: Option<String>,
    },

    /// Feature not supported by this generator.
    UnsupportedFeature {
        /// The unsupported feature name.
        feature: String,
        /// The generator that doesn't support it.
        generator: Option<String>,
    },

    /// IO error during file operations.
    IoError(String),

    /// Generic error with message.
    Other {
        /// Error message.
        message: String,
        /// The schema name that caused the error.
        schema_name: Option<String>,
    },
}

impl GeneratorError {
    /// Create a circular dependency error.
    pub fn circular_dependency(cycle: Vec<String>) -> Self {
        Self::CircularDependency { cycle }
    }

    /// Create an unknown type reference error.
    pub fn unknown_type_reference(type_name: impl Into<String>) -> Self {
        Self::UnknownTypeReference {
            type_name: type_name.into(),
            referencing_schema: None,
        }
    }

    /// Create an unsupported feature error.
    pub fn unsupported_feature(feature: impl Into<String>) -> Self {
        Self::UnsupportedFeature {
            feature: feature.into(),
            generator: None,
        }
    }

    /// Create an IO error.
    pub fn io_error(error: impl Into<String>) -> Self {
        Self::IoError(error.into())
    }

    /// Create a generic error with a message.
    pub fn other(message: impl Into<String>) -> Self {
        Self::Other {
            message: message.into(),
            schema_name: None,
        }
    }

    /// Add schema name context.
    pub fn with_schema(mut self, name: impl Into<String>) -> Self {
        match &mut self {
            Self::UnknownTypeReference {
                referencing_schema, ..
            } => *referencing_schema = Some(name.into()),
            Self::Other { schema_name, .. } => *schema_name = Some(name.into()),
            _ => {}
        }
        self
    }

    /// Add generator name context.
    pub fn with_generator(mut self, name: impl Into<String>) -> Self {
        if let Self::UnsupportedFeature { generator, .. } = &mut self {
            *generator = Some(name.into());
        }
        self
    }
}

impl fmt::Display for GeneratorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CircularDependency { cycle } => {
                write!(f, "Circular dependency detected: ")?;
                for (i, name) in cycle.iter().enumerate() {
                    if i > 0 {
                        write!(f, " -> ")?;
                    }
                    write!(f, "{}", name)?;
                }
                if let Some(first) = cycle.first() {
                    write!(f, " -> {}", first)?;
                }
                Ok(())
            }
            Self::UnknownTypeReference {
                type_name,
                referencing_schema,
            } => {
                write!(f, "Unknown type reference: `{}`", type_name)?;
                if let Some(schema) = referencing_schema {
                    write!(f, " (referenced from `{}`)", schema)?;
                }
                Ok(())
            }
            Self::UnsupportedFeature { feature, generator } => {
                write!(f, "Unsupported feature: `{}`", feature)?;
                if let Some(gen) = generator {
                    write!(f, " (not supported by {} generator)", gen)?;
                }
                Ok(())
            }
            Self::IoError(msg) => write!(f, "IO error: {}", msg),
            Self::Other {
                message,
                schema_name,
            } => {
                if let Some(name) = schema_name {
                    write!(f, "Error generating schema `{}`: {}", name, message)
                } else {
                    write!(f, "{}", message)
                }
            }
        }
    }
}

impl std::error::Error for GeneratorError {}

impl From<std::io::Error> for GeneratorError {
    fn from(error: std::io::Error) -> Self {
        Self::IoError(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ParseError tests

    #[test]
    fn test_unsupported_type_error() {
        let error = ParseError::unsupported_type("FnOnce");
        let msg = error.to_string();
        assert!(msg.contains("Unsupported type"));
        assert!(msg.contains("FnOnce"));
    }

    #[test]
    fn test_unsupported_type_with_suggestions() {
        let error = ParseError::unsupported_type("FnOnce")
            .with_suggestion("Use a concrete type instead")
            .with_suggestion("Consider using a closure wrapper");
        let msg = error.to_string();
        assert!(msg.contains("Suggestions:"));
        assert!(msg.contains("Use a concrete type instead"));
        assert!(msg.contains("Consider using a closure wrapper"));
    }

    #[test]
    fn test_empty_path_error() {
        let error = ParseError::empty_path();
        let msg = error.to_string();
        assert!(msg.contains("Empty path"));
    }

    #[test]
    fn test_missing_generic_error() {
        let error = ParseError::missing_generic("Option");
        let msg = error.to_string();
        assert!(msg.contains("Missing generic parameter"));
        assert!(msg.contains("Option"));
        assert!(msg.contains("Option<T>"));
    }

    #[test]
    fn test_invalid_attribute_error() {
        let error = ParseError::invalid_attribute("unknwon")
            .with_suggestion("rename")
            .with_suggestion("skip")
            .with_suggestion("optional");
        let msg = error.to_string();
        assert!(msg.contains("Invalid attribute"));
        assert!(msg.contains("unknwon"));
        assert!(msg.contains("Valid alternatives:"));
        assert!(msg.contains("rename"));
    }

    #[test]
    fn test_conflicting_attributes_error() {
        let error = ParseError::conflicting_attributes("skip", "rename")
            .with_explanation("A skipped field cannot be renamed");
        let msg = error.to_string();
        assert!(msg.contains("Conflicting attributes"));
        assert!(msg.contains("skip"));
        assert!(msg.contains("rename"));
        assert!(msg.contains("A skipped field cannot be renamed"));
    }

    #[test]
    fn test_invalid_regex_error() {
        let error = ParseError::invalid_regex("[invalid", "unclosed bracket");
        let msg = error.to_string();
        assert!(msg.contains("Invalid regex pattern"));
        assert!(msg.contains("[invalid"));
        assert!(msg.contains("unclosed bracket"));
    }

    #[test]
    fn test_invalid_attribute_value_error() {
        let error = ParseError::invalid_attribute_value("min", "abc", "a number");
        let msg = error.to_string();
        assert!(msg.contains("Invalid value"));
        assert!(msg.contains("abc"));
        assert!(msg.contains("min"));
        assert!(msg.contains("a number"));
    }

    #[test]
    fn test_missing_zod_schema_impl_error() {
        let error = ParseError::missing_zod_schema_impl("CustomType");
        let msg = error.to_string();
        assert!(msg.contains("CustomType"));
        assert!(msg.contains("ZodSchema"));
        assert!(msg.contains("#[derive(ZodSchema)]"));
    }

    #[test]
    fn test_other_error() {
        let error = ParseError::other("Something went wrong").with_suggestion("Try again");
        let msg = error.to_string();
        assert!(msg.contains("Something went wrong"));
        assert!(msg.contains("Try again"));
    }

    #[test]
    fn test_parse_error_with_span() {
        let error = ParseError::unsupported_type("FnOnce").with_span(Span::call_site());
        assert!(error.span().is_some());
    }

    #[test]
    fn test_parse_error_into_syn_error() {
        let error = ParseError::unsupported_type("FnOnce");
        let syn_error = error.into_syn_error();
        assert!(syn_error.to_string().contains("FnOnce"));
    }

    #[test]
    fn test_parse_error_from_syn_error() {
        let syn_error = syn::Error::new(Span::call_site(), "test error");
        let parse_error: ParseError = syn_error.into();
        assert!(matches!(parse_error, ParseError::SynError(_)));
        assert!(parse_error.to_string().contains("test error"));
    }

    // GeneratorError tests

    #[test]
    fn test_circular_dependency_error() {
        let error = GeneratorError::circular_dependency(vec![
            "A".to_string(),
            "B".to_string(),
            "C".to_string(),
        ]);
        let msg = error.to_string();
        assert!(msg.contains("Circular dependency"));
        assert!(msg.contains("A -> B -> C -> A"));
    }

    #[test]
    fn test_unknown_type_reference_error() {
        let error = GeneratorError::unknown_type_reference("UnknownType").with_schema("MyStruct");
        let msg = error.to_string();
        assert!(msg.contains("Unknown type reference"));
        assert!(msg.contains("UnknownType"));
        assert!(msg.contains("MyStruct"));
    }

    #[test]
    fn test_unsupported_feature_error() {
        let error = GeneratorError::unsupported_feature("transforms").with_generator("Zod");
        let msg = error.to_string();
        assert!(msg.contains("Unsupported feature"));
        assert!(msg.contains("transforms"));
        assert!(msg.contains("Zod"));
    }

    #[test]
    fn test_io_error() {
        let error = GeneratorError::io_error("file not found");
        let msg = error.to_string();
        assert!(msg.contains("IO error"));
        assert!(msg.contains("file not found"));
    }

    #[test]
    fn test_generator_other_error() {
        let error = GeneratorError::other("Generation failed").with_schema("UserSchema");
        let msg = error.to_string();
        assert!(msg.contains("Generation failed"));
        assert!(msg.contains("UserSchema"));
    }

    #[test]
    fn test_generator_error_from_io_error() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let gen_error: GeneratorError = io_error.into();
        assert!(matches!(gen_error, GeneratorError::IoError(_)));
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    /// Generate arbitrary type names for testing.
    fn arb_type_name() -> impl Strategy<Value = String> {
        prop::string::string_regex("[A-Z][a-zA-Z0-9_]{0,20}")
            .unwrap()
            .prop_filter("non-empty", |s| !s.is_empty())
    }

    /// Generate arbitrary attribute names for testing.
    fn arb_attribute_name() -> impl Strategy<Value = String> {
        prop::string::string_regex("[a-z][a-z_]{0,15}")
            .unwrap()
            .prop_filter("non-empty", |s| !s.is_empty())
    }

    /// Generate arbitrary error messages for testing.
    fn arb_error_message() -> impl Strategy<Value = String> {
        // Ensure message always starts with a letter and is at least 5 characters
        prop::string::string_regex("[a-zA-Z][a-zA-Z0-9 ]{4,49}")
            .unwrap()
            .prop_filter("has letters", |s| s.chars().any(|c| c.is_alphabetic()))
    }

    /// Generate arbitrary ParseError variants.
    fn arb_parse_error() -> impl Strategy<Value = ParseError> {
        prop_oneof![
            arb_type_name().prop_map(ParseError::unsupported_type),
            Just(ParseError::empty_path()),
            prop::sample::select(vec!["Option", "Vec", "HashMap", "Result"])
                .prop_map(ParseError::missing_generic),
            arb_attribute_name().prop_map(ParseError::invalid_attribute),
            (arb_attribute_name(), arb_attribute_name())
                .prop_map(|(a, b)| ParseError::conflicting_attributes(a, b)),
            (arb_error_message(), arb_error_message())
                .prop_map(|(p, e)| ParseError::invalid_regex(p, e)),
            (
                arb_attribute_name(),
                arb_error_message(),
                arb_error_message()
            )
                .prop_map(|(a, v, e)| ParseError::invalid_attribute_value(a, v, e)),
            arb_type_name().prop_map(ParseError::missing_zod_schema_impl),
            arb_error_message().prop_map(ParseError::other),
        ]
    }

    /// Generate arbitrary GeneratorError variants.
    fn arb_generator_error() -> impl Strategy<Value = GeneratorError> {
        prop_oneof![
            prop::collection::vec(arb_type_name(), 2..5)
                .prop_map(GeneratorError::circular_dependency),
            arb_type_name().prop_map(GeneratorError::unknown_type_reference),
            arb_error_message().prop_map(GeneratorError::unsupported_feature),
            arb_error_message().prop_map(GeneratorError::io_error),
            arb_error_message().prop_map(GeneratorError::other),
        ]
    }

    proptest! {
        /// **Property 14: Error Message Quality**
        ///
        /// *For any* invalid input (unsupported type, invalid attribute), the macro SHALL
        /// produce an error message containing the source location and a description of the problem.
        ///
        /// This property test validates:
        /// - All ParseError variants produce non-empty error messages
        /// - Error messages contain descriptive information about the problem
        /// - Error messages are human-readable (contain alphabetic characters)
        ///
        /// **Validates: Requirements 11.1, 11.2**
        ///
        /// **Feature: zod-schema-macro, Property 14: Error Message Quality**
        #[test]
        fn property_14_parse_error_message_quality(error in arb_parse_error()) {
            let message = error.to_string();

            // Error message must be non-empty
            prop_assert!(!message.is_empty(), "Error message should not be empty");

            // Error message must contain some alphabetic characters (human-readable)
            prop_assert!(
                message.chars().any(|c| c.is_alphabetic()),
                "Error message should contain alphabetic characters: {}",
                message
            );

            // Error message should be reasonably sized (not too short)
            prop_assert!(
                message.len() >= 5,
                "Error message should be at least 5 characters: {}",
                message
            );
        }

        /// **Property 14: Error Message Quality (Generator Errors)**
        ///
        /// *For any* generator error, the error message SHALL contain a description of the problem.
        ///
        /// **Validates: Requirements 11.1, 11.2**
        ///
        /// **Feature: zod-schema-macro, Property 14: Error Message Quality**
        #[test]
        fn property_14_generator_error_message_quality(error in arb_generator_error()) {
            let message = error.to_string();

            // Error message must be non-empty
            prop_assert!(!message.is_empty(), "Error message should not be empty");

            // Error message must contain some alphabetic characters (human-readable)
            prop_assert!(
                message.chars().any(|c| c.is_alphabetic()),
                "Error message should contain alphabetic characters: {}",
                message
            );

            // Error message should be reasonably sized (not too short)
            prop_assert!(
                message.len() >= 5,
                "Error message should be at least 5 characters: {}",
                message
            );
        }

        /// **Property 14: Error Message Quality (Span Preservation)**
        ///
        /// *For any* ParseError with a span attached, the span SHALL be preserved
        /// and accessible for error reporting.
        ///
        /// **Validates: Requirements 11.1, 11.2**
        ///
        /// **Feature: zod-schema-macro, Property 14: Error Message Quality**
        #[test]
        fn property_14_span_preservation(error in arb_parse_error()) {
            // Attach a span to the error
            let error_with_span = error.with_span(Span::call_site());

            // Span should be preserved
            prop_assert!(
                error_with_span.span().is_some(),
                "Span should be preserved after with_span()"
            );

            // Converting to syn::Error should preserve the message
            let syn_error = error_with_span.into_syn_error();
            let syn_message = syn_error.to_string();

            prop_assert!(
                !syn_message.is_empty(),
                "syn::Error message should not be empty"
            );
        }

        /// **Property 14: Error Message Quality (Suggestions)**
        ///
        /// *For any* error that supports suggestions, adding suggestions SHALL
        /// include them in the error message.
        ///
        /// **Validates: Requirements 11.6**
        ///
        /// **Feature: zod-schema-macro, Property 14: Error Message Quality**
        #[test]
        fn property_14_suggestions_included(
            type_name in arb_type_name(),
            suggestion in arb_error_message()
        ) {
            // Test UnsupportedType with suggestion
            let error = ParseError::unsupported_type(&type_name)
                .with_suggestion(&suggestion);
            let message = error.to_string();

            prop_assert!(
                message.contains(&type_name),
                "Error message should contain the type name: {} not in {}",
                type_name,
                message
            );

            prop_assert!(
                message.contains(&suggestion),
                "Error message should contain the suggestion: {} not in {}",
                suggestion,
                message
            );

            prop_assert!(
                message.contains("Suggestion"),
                "Error message should indicate suggestions are present: {}",
                message
            );
        }

        /// **Property 14: Error Message Quality (Conflicting Attributes Explanation)**
        ///
        /// *For any* conflicting attributes error with an explanation, the explanation
        /// SHALL be included in the error message.
        ///
        /// **Validates: Requirements 11.3**
        ///
        /// **Feature: zod-schema-macro, Property 14: Error Message Quality**
        #[test]
        fn property_14_conflicting_attributes_explanation(
            first in arb_attribute_name(),
            second in arb_attribute_name(),
            explanation in arb_error_message()
        ) {
            let error = ParseError::conflicting_attributes(&first, &second)
                .with_explanation(&explanation);
            let message = error.to_string();

            prop_assert!(
                message.contains(&first),
                "Error message should contain first attribute: {} not in {}",
                first,
                message
            );

            prop_assert!(
                message.contains(&second),
                "Error message should contain second attribute: {} not in {}",
                second,
                message
            );

            prop_assert!(
                message.contains(&explanation),
                "Error message should contain explanation: {} not in {}",
                explanation,
                message
            );
        }

        /// **Property 14: Error Message Quality (Circular Dependency Path)**
        ///
        /// *For any* circular dependency error, the error message SHALL contain
        /// all types in the cycle path.
        ///
        /// **Validates: Requirements 11.7**
        ///
        /// **Feature: zod-schema-macro, Property 14: Error Message Quality**
        #[test]
        fn property_14_circular_dependency_path(
            cycle in prop::collection::vec(arb_type_name(), 2..5)
        ) {
            let error = GeneratorError::circular_dependency(cycle.clone());
            let message = error.to_string();

            // All types in the cycle should be mentioned
            for type_name in &cycle {
                prop_assert!(
                    message.contains(type_name),
                    "Error message should contain cycle member: {} not in {}",
                    type_name,
                    message
                );
            }

            // Should indicate it's a circular dependency
            prop_assert!(
                message.to_lowercase().contains("circular"),
                "Error message should mention 'circular': {}",
                message
            );
        }
    }
}
