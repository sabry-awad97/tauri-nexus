//! Error types for the CLI.
//!
//! This module defines all error types used throughout the CLI,
//! providing detailed error messages with context for debugging.

use std::path::PathBuf;
use thiserror::Error;

/// Result type alias for CLI operations.
pub type CliResult<T> = Result<T, CliError>;

/// Main error type for CLI operations.
#[derive(Debug, Error)]
pub enum CliError {
    /// Error during source file scanning.
    #[error("Failed to scan directory: {0}")]
    Scan(#[from] ScanError),

    /// Error during Rust source parsing.
    #[error("Failed to parse source file: {0}")]
    Parse(#[from] ParseError),

    /// Error during schema generation.
    #[error("Failed to generate schemas: {0}")]
    Generate(#[from] GenerateError),

    /// Error loading configuration.
    #[error("Failed to load configuration: {0}")]
    Config(#[from] ConfigError),

    /// Error writing output files.
    #[error("Failed to write output: {0}")]
    Write(#[from] WriteError),

    /// Error during file watching.
    #[error("Watch error: {0}")]
    Watch(#[from] WatchError),

    /// Validation failed (schemas out of date).
    #[error("Validation failed: {0}")]
    Validation(String),

    /// Generic IO error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Error during source file scanning.
#[derive(Debug, Error)]
pub enum ScanError {
    /// Directory does not exist.
    #[error("Directory not found: {path}")]
    DirectoryNotFound { path: PathBuf },

    /// No Rust files found in directory.
    #[error("No Rust files found in: {path}")]
    NoRustFiles { path: PathBuf },

    /// Invalid filter pattern.
    #[error("Invalid filter pattern '{pattern}': {message}")]
    InvalidPattern { pattern: String, message: String },

    /// IO error during scanning.
    #[error("IO error scanning {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Error from ignore crate walker.
    #[error("Walk error: {0}")]
    Walk(#[from] ignore::Error),
}

/// Error during Rust source parsing.
#[derive(Debug, Error)]
pub enum ParseError {
    /// Syntax error in Rust source.
    #[error("Syntax error in {file}:{line}:{column}: {message}")]
    Syntax {
        file: PathBuf,
        line: usize,
        column: usize,
        message: String,
    },

    /// Invalid attribute on type.
    #[error("Invalid attribute in {file}:{line}: {message}")]
    Attribute {
        file: PathBuf,
        line: usize,
        message: String,
    },

    /// Unsupported type encountered.
    #[error("Unsupported type in {file}:{line}: {type_name}")]
    UnsupportedType {
        file: PathBuf,
        line: usize,
        type_name: String,
    },

    /// IO error reading file.
    #[error("Failed to read {file}: {source}")]
    Io {
        file: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Multiple parse errors collected.
    #[error("Multiple parse errors:\n{}", format_errors(.0))]
    Multiple(Vec<ParseError>),
}

/// Error during schema generation.
#[derive(Debug, Error)]
pub enum GenerateError {
    /// Circular dependency detected.
    #[error("Circular dependency detected: {}", .cycle.join(" -> "))]
    CircularDependency { cycle: Vec<String> },

    /// Missing dependency.
    #[error("Type '{type_name}' references unknown type '{dependency}'")]
    MissingDependency {
        type_name: String,
        dependency: String,
    },

    /// Internal generation error.
    #[error("Generation error for '{type_name}': {message}")]
    Internal { type_name: String, message: String },
}

/// Error loading configuration.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// Config file not found.
    #[error("Configuration file not found: {path}")]
    NotFound { path: PathBuf },

    /// Invalid TOML syntax.
    #[error("Invalid TOML in {path}: {message}")]
    InvalidToml { path: PathBuf, message: String },

    /// Invalid configuration value.
    #[error("Invalid configuration value for '{key}': {message}")]
    InvalidValue { key: String, message: String },

    /// IO error reading config.
    #[error("Failed to read config {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

/// Error writing output files.
#[derive(Debug, Error)]
pub enum WriteError {
    /// Failed to create directory.
    #[error("Failed to create directory {path}: {source}")]
    CreateDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Failed to write file.
    #[error("Failed to write file {path}: {source}")]
    WriteFile {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

/// Error during file watching.
#[derive(Debug, Error)]
pub enum WatchError {
    /// Failed to initialize watcher.
    #[error("Failed to initialize file watcher: {0}")]
    Init(String),

    /// Error from notify crate.
    #[error("Watch notification error: {0}")]
    Notify(String),
}

/// Format multiple errors for display.
fn format_errors(errors: &[ParseError]) -> String {
    errors
        .iter()
        .enumerate()
        .map(|(i, e)| format!("  {}. {}", i + 1, e))
        .collect::<Vec<_>>()
        .join("\n")
}

impl ParseError {
    /// Create a syntax error with location information.
    pub fn syntax(file: PathBuf, line: usize, column: usize, message: impl Into<String>) -> Self {
        Self::Syntax {
            file,
            line,
            column,
            message: message.into(),
        }
    }

    /// Create an attribute error.
    pub fn attribute(file: PathBuf, line: usize, message: impl Into<String>) -> Self {
        Self::Attribute {
            file,
            line,
            message: message.into(),
        }
    }

    /// Create an unsupported type error.
    pub fn unsupported_type(file: PathBuf, line: usize, type_name: impl Into<String>) -> Self {
        Self::UnsupportedType {
            file,
            line,
            type_name: type_name.into(),
        }
    }
}

impl ScanError {
    /// Create a directory not found error.
    pub fn not_found(path: PathBuf) -> Self {
        Self::DirectoryNotFound { path }
    }

    /// Create a no Rust files error.
    pub fn no_rust_files(path: PathBuf) -> Self {
        Self::NoRustFiles { path }
    }

    /// Create an invalid pattern error.
    pub fn invalid_pattern(pattern: impl Into<String>, message: impl Into<String>) -> Self {
        Self::InvalidPattern {
            pattern: pattern.into(),
            message: message.into(),
        }
    }
}

impl ConfigError {
    /// Create a not found error.
    pub fn not_found(path: PathBuf) -> Self {
        Self::NotFound { path }
    }

    /// Create an invalid TOML error.
    pub fn invalid_toml(path: PathBuf, message: impl Into<String>) -> Self {
        Self::InvalidToml {
            path,
            message: message.into(),
        }
    }

    /// Create an invalid value error.
    pub fn invalid_value(key: impl Into<String>, message: impl Into<String>) -> Self {
        Self::InvalidValue {
            key: key.into(),
            message: message.into(),
        }
    }
}
