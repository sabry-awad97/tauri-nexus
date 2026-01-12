//! Error types for the macro crate.
//!
//! This module defines error types for parsing and code generation.

use proc_macro2::Span;
use std::fmt;

/// Error that occurred during parsing.
#[derive(Debug, Clone)]
pub struct ParseError {
    /// Error message.
    pub message: String,
    /// Source span for error reporting.
    pub span: Option<Span>,
    /// Suggestions for fixing the error.
    pub suggestions: Vec<String>,
}

impl ParseError {
    /// Create a new parse error.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            span: None,
            suggestions: Vec::new(),
        }
    }

    /// Add span information.
    pub fn with_span(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }

    /// Add a suggestion.
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestions.push(suggestion.into());
        self
    }

    /// Convert to a syn::Error for proc-macro error reporting.
    pub fn into_syn_error(self) -> syn::Error {
        let span = self.span.unwrap_or_else(Span::call_site);
        let mut error = syn::Error::new(span, &self.message);

        for suggestion in self.suggestions {
            error.combine(syn::Error::new(span, format!("suggestion: {}", suggestion)));
        }

        error
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)?;
        if !self.suggestions.is_empty() {
            write!(f, "\nSuggestions:")?;
            for suggestion in &self.suggestions {
                write!(f, "\n  - {}", suggestion)?;
            }
        }
        Ok(())
    }
}

impl std::error::Error for ParseError {}

/// Error that occurred during code generation.
#[derive(Debug, Clone)]
pub struct GeneratorError {
    /// Error message.
    pub message: String,
    /// The schema name that caused the error.
    pub schema_name: Option<String>,
}

impl GeneratorError {
    /// Create a new generator error.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            schema_name: None,
        }
    }

    /// Add schema name context.
    pub fn with_schema(mut self, name: impl Into<String>) -> Self {
        self.schema_name = Some(name.into());
        self
    }
}

impl fmt::Display for GeneratorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(name) = &self.schema_name {
            write!(f, "Error generating schema '{}': {}", name, self.message)
        } else {
            write!(f, "{}", self.message)
        }
    }
}

impl std::error::Error for GeneratorError {}
