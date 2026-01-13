//! Configuration management for the CLI.
//!
//! This module handles loading configuration from `zod-rs.toml` files
//! and merging with command-line arguments.

use crate::error::{CliResult, ConfigError};
use serde::Deserialize;
use std::path::{Path, PathBuf};

/// Default configuration filename.
pub const CONFIG_FILENAME: &str = "zod-rs.toml";

/// Main configuration structure.
#[derive(Debug, Default, Clone, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Output configuration.
    pub output: OutputConfig,

    /// Naming conventions.
    pub naming: NamingConfig,

    /// Feature flags.
    pub features: FeaturesConfig,
}

/// Output configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct OutputConfig {
    /// Output directory for generated files.
    pub dir: PathBuf,

    /// Output filename.
    pub file: String,

    /// Whether to generate type inference exports.
    pub generate_types: bool,

    /// Whether to generate JSDoc comments.
    pub generate_docs: bool,
}

/// Naming convention configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct NamingConfig {
    /// Field rename convention (camelCase, snake_case, etc.).
    pub rename_all: Option<String>,

    /// Suffix for schema names.
    pub schema_suffix: String,
}

/// Feature flags configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct FeaturesConfig {
    /// Enable serde attribute compatibility.
    pub serde_compat: bool,

    /// Enable chrono DateTime support.
    pub chrono: bool,

    /// Enable uuid support.
    pub uuid: bool,
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            dir: PathBuf::from("./generated"),
            file: "schemas.ts".to_string(),
            generate_types: true,
            generate_docs: true,
        }
    }
}

impl Default for NamingConfig {
    fn default() -> Self {
        Self {
            rename_all: Some("camelCase".to_string()),
            schema_suffix: "Schema".to_string(),
        }
    }
}

impl Default for FeaturesConfig {
    fn default() -> Self {
        Self {
            serde_compat: true,
            chrono: false,
            uuid: false,
        }
    }
}

/// Configuration manager for loading and merging configs.
pub struct ConfigManager;

impl ConfigManager {
    /// Load configuration from a file path.
    ///
    /// If the path is None, attempts to load from the default location.
    /// If no config file exists, returns default configuration.
    pub fn load(path: Option<&Path>) -> CliResult<Config> {
        let config_path = path
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(CONFIG_FILENAME));

        if !config_path.exists() {
            return Ok(Config::default());
        }

        let content = std::fs::read_to_string(&config_path).map_err(|e| ConfigError::Io {
            path: config_path.clone(),
            source: e,
        })?;

        let config: Config = toml::from_str(&content)
            .map_err(|e| ConfigError::invalid_toml(config_path, e.to_string()))?;

        Ok(config)
    }

    /// Merge CLI arguments into configuration.
    ///
    /// CLI arguments take precedence over config file values.
    pub fn merge_cli_args(mut config: Config, args: &CliArgs) -> Config {
        if let Some(ref output) = args.output {
            config.output.dir = output.clone();
        }

        if let Some(ref file) = args.output_file {
            config.output.file = file.clone();
        }

        if let Some(generate_types) = args.generate_types {
            config.output.generate_types = generate_types;
        }

        if let Some(generate_docs) = args.generate_docs {
            config.output.generate_docs = generate_docs;
        }

        if let Some(ref rename_all) = args.rename_all {
            config.naming.rename_all = Some(rename_all.clone());
        }

        if let Some(serde_compat) = args.serde_compat {
            config.features.serde_compat = serde_compat;
        }

        config
    }

    /// Get default configuration.
    pub fn default_config() -> Config {
        Config::default()
    }

    /// Generate default configuration file content with comments.
    pub fn default_config_content() -> &'static str {
        r#"# zod-rs configuration file
# See https://github.com/example/zod-rs for documentation

[output]
# Output directory for generated TypeScript files
dir = "./generated"

# Output file name
file = "schemas.ts"

# Whether to generate type inference exports (export type X = z.infer<typeof XSchema>)
generate_types = true

# Whether to generate JSDoc comments from Rust doc comments
generate_docs = true

[naming]
# Rename convention for fields (camelCase, snake_case, PascalCase, SCREAMING_SNAKE_CASE, kebab-case)
rename_all = "camelCase"

# Schema name suffix (e.g., UserSchema)
schema_suffix = "Schema"

[features]
# Enable serde attribute compatibility (#[serde(...)] attributes are respected)
serde_compat = true

# Enable chrono DateTime support (requires chrono feature in zod-rs)
chrono = false

# Enable uuid support (requires uuid feature in zod-rs)
uuid = false
"#
    }
}

/// CLI arguments that can override configuration.
#[derive(Debug, Default)]
pub struct CliArgs {
    /// Output directory override.
    pub output: Option<PathBuf>,

    /// Output filename override.
    pub output_file: Option<String>,

    /// Generate types override.
    pub generate_types: Option<bool>,

    /// Generate docs override.
    pub generate_docs: Option<bool>,

    /// Rename all override.
    pub rename_all: Option<String>,

    /// Serde compat override.
    pub serde_compat: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.output.dir, PathBuf::from("./generated"));
        assert_eq!(config.output.file, "schemas.ts");
        assert!(config.output.generate_types);
        assert!(config.output.generate_docs);
        assert_eq!(config.naming.rename_all, Some("camelCase".to_string()));
        assert_eq!(config.naming.schema_suffix, "Schema");
        assert!(config.features.serde_compat);
        assert!(!config.features.chrono);
        assert!(!config.features.uuid);
    }

    #[test]
    fn test_merge_cli_args_output() {
        let config = Config::default();
        let args = CliArgs {
            output: Some(PathBuf::from("./custom")),
            ..Default::default()
        };

        let merged = ConfigManager::merge_cli_args(config, &args);
        assert_eq!(merged.output.dir, PathBuf::from("./custom"));
    }

    #[test]
    fn test_merge_cli_args_preserves_unset() {
        let config = Config::default();
        let args = CliArgs::default();

        let merged = ConfigManager::merge_cli_args(config.clone(), &args);
        assert_eq!(merged.output.dir, config.output.dir);
        assert_eq!(merged.output.file, config.output.file);
    }

    #[test]
    fn test_parse_toml_config() {
        let toml = r#"
[output]
dir = "./custom-output"
file = "types.ts"
generate_types = false
generate_docs = true

[naming]
rename_all = "snake_case"
schema_suffix = "Validator"

[features]
serde_compat = false
chrono = true
uuid = true
"#;

        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.output.dir, PathBuf::from("./custom-output"));
        assert_eq!(config.output.file, "types.ts");
        assert!(!config.output.generate_types);
        assert!(config.output.generate_docs);
        assert_eq!(config.naming.rename_all, Some("snake_case".to_string()));
        assert_eq!(config.naming.schema_suffix, "Validator");
        assert!(!config.features.serde_compat);
        assert!(config.features.chrono);
        assert!(config.features.uuid);
    }
}
