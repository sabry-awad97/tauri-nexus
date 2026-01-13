//! # zod-rs-cli
//!
//! CLI library for generating TypeScript Zod schemas from Rust source files.
//!
//! This crate provides the core functionality for the `zod-rs` CLI tool,
//! including source file scanning, Rust parsing, schema generation, and file output.
//!
//! ## Architecture
//!
//! The library is organized into several modules:
//!
//! - [`config`] - Configuration management and TOML parsing
//! - [`scanner`] - Source file discovery and filtering
//! - [`parser`] - Rust source parsing and type extraction
//! - [`generator`] - Schema generation using zod-rs-macros
//! - [`writer`] - File output and dry-run support
//! - [`watcher`] - File system watching for development mode
//! - [`error`] - Error types and handling

pub mod config;
pub mod error;
pub mod generator;
pub mod parser;
pub mod scanner;
pub mod watcher;
pub mod writer;

// Re-export main types for convenience
pub use config::{Config, ConfigManager};
pub use error::{CliError, CliResult};
pub use generator::SchemaGenerator;
pub use parser::{ParsedType, RustParser};
pub use scanner::{SourceFile, SourceScanner};
pub use watcher::FileWatcher;
pub use writer::FileWriter;
