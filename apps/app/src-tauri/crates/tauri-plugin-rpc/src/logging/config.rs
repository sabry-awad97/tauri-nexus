//! Configuration types for the logging module.
//!
//! This module provides configuration types for customizing logging behavior:
//! - TracingConfig: Distributed tracing integration settings
//! - LogConfig: Main logging configuration with builder pattern
//!
//! Both types use the builder pattern for ergonomic configuration and are
//! immutable after construction to ensure thread-safety.

use super::constants::{DEFAULT_MAX_ATTRIBUTE_SIZE, DEFAULT_SLOW_THRESHOLD_MS};
use super::types::LogLevel;
use std::collections::{HashMap, HashSet};

/// Configuration for distributed tracing integration.
///
/// Controls how RPC requests are traced using OpenTelemetry-compatible spans.
#[derive(Debug, Clone)]
pub struct TracingConfig {
    /// Whether to create tracing spans for requests.
    pub create_spans: bool,
    /// Whether to record request input in span attributes.
    pub record_input: bool,
    /// Whether to record response output in span attributes.
    pub record_output: bool,
    /// Maximum size in bytes for span attributes (prevents excessive memory usage).
    pub max_attribute_size: usize,
    /// Service name for distributed tracing identification.
    pub service_name: String,
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self {
            create_spans: true,
            record_input: false,
            record_output: false,
            max_attribute_size: DEFAULT_MAX_ATTRIBUTE_SIZE,
            service_name: "tauri-rpc".to_string(),
        }
    }
}

impl TracingConfig {
    /// Creates a new tracing configuration with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets whether to create tracing spans for requests.
    pub fn with_spans(mut self, enabled: bool) -> Self {
        self.create_spans = enabled;
        self
    }

    /// Sets whether to record request input in span attributes.
    pub fn with_input_recording(mut self, enabled: bool) -> Self {
        self.record_input = enabled;
        self
    }

    /// Sets whether to record response output in span attributes.
    pub fn with_output_recording(mut self, enabled: bool) -> Self {
        self.record_output = enabled;
        self
    }

    /// Sets the maximum size for span attributes.
    pub fn with_max_attribute_size(mut self, size: usize) -> Self {
        self.max_attribute_size = size;
        self
    }

    /// Sets the service name for distributed tracing.
    pub fn with_service_name(mut self, name: impl Into<String>) -> Self {
        self.service_name = name.into();
        self
    }
}

/// Returns the default set of fields to redact.
fn default_redacted_fields() -> HashSet<String> {
    [
        "password",
        "secret",
        "token",
        "api_key",
        "apiKey",
        "authorization",
        "auth",
        "credential",
        "credentials",
        "private_key",
        "privateKey",
        "access_token",
        "accessToken",
        "refresh_token",
        "refreshToken",
        "ssn",
        "social_security",
        "credit_card",
        "creditCard",
        "card_number",
        "cardNumber",
        "cvv",
        "pin",
        "bearer",
    ]
    .into_iter()
    .map(String::from)
    .collect()
}

/// Configuration for RPC logging.
///
/// Controls all aspects of request/response logging including log levels,
/// what data to log, redaction settings, and performance thresholds.
#[derive(Debug, Clone)]
pub struct LogConfig {
    /// Global log level for all requests.
    pub level: LogLevel,
    /// Whether to log request timing information.
    pub log_timing: bool,
    /// Whether to log request input (will be redacted).
    pub log_input: bool,
    /// Whether to log response output (will be redacted).
    pub log_output: bool,
    /// Set of field names to redact from logs (case-insensitive substring matching).
    pub redacted_fields: HashSet<String>,
    /// Replacement string for redacted values.
    pub redaction_replacement: String,
    /// Whether to log successful requests.
    pub log_success: bool,
    /// Whether to log failed requests.
    pub log_errors: bool,
    /// Set of paths to exclude from logging.
    pub excluded_paths: HashSet<String>,
    /// Per-procedure log level overrides.
    pub procedure_levels: HashMap<String, LogLevel>,
    /// Optional distributed tracing configuration.
    pub tracing: Option<TracingConfig>,
    /// Optional threshold in milliseconds for slow request warnings.
    pub slow_request_threshold_ms: Option<u64>,
    /// Whether to log input/output sizes in bytes.
    pub log_sizes: bool,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: LogLevel::Info,
            log_timing: true,
            log_input: false,
            log_output: false,
            redacted_fields: default_redacted_fields(),
            redaction_replacement: "[REDACTED]".to_string(),
            log_success: true,
            log_errors: true,
            excluded_paths: HashSet::new(),
            procedure_levels: HashMap::new(),
            tracing: Some(TracingConfig::default()),
            slow_request_threshold_ms: Some(DEFAULT_SLOW_THRESHOLD_MS),
            log_sizes: true,
        }
    }
}

impl LogConfig {
    /// Creates a new logging configuration with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the global log level.
    pub fn with_level(mut self, level: LogLevel) -> Self {
        self.level = level;
        self
    }

    /// Sets whether to log request timing information.
    pub fn with_timing(mut self, enabled: bool) -> Self {
        self.log_timing = enabled;
        self
    }

    /// Sets whether to log request input (will be redacted).
    pub fn with_input_logging(mut self, enabled: bool) -> Self {
        self.log_input = enabled;
        self
    }

    /// Sets whether to log response output (will be redacted).
    pub fn with_output_logging(mut self, enabled: bool) -> Self {
        self.log_output = enabled;
        self
    }

    /// Adds a field name to the redaction list.
    ///
    /// Field matching is case-insensitive and uses substring matching.
    pub fn redact_field(mut self, field: impl Into<String>) -> Self {
        self.redacted_fields.insert(field.into());
        self
    }

    /// Adds multiple field names to the redaction list.
    pub fn redact_fields(mut self, fields: impl IntoIterator<Item = impl Into<String>>) -> Self {
        for field in fields {
            self.redacted_fields.insert(field.into());
        }
        self
    }

    /// Clears all redacted fields (removes default sensitive fields).
    pub fn clear_redacted_fields(mut self) -> Self {
        self.redacted_fields.clear();
        self
    }

    /// Sets the replacement string for redacted values.
    pub fn with_redaction_replacement(mut self, replacement: impl Into<String>) -> Self {
        self.redaction_replacement = replacement.into();
        self
    }

    /// Sets whether to log successful requests.
    pub fn with_success_logging(mut self, enabled: bool) -> Self {
        self.log_success = enabled;
        self
    }

    /// Sets whether to log failed requests.
    pub fn with_error_logging(mut self, enabled: bool) -> Self {
        self.log_errors = enabled;
        self
    }

    /// Excludes a path from logging.
    pub fn exclude_path(mut self, path: impl Into<String>) -> Self {
        self.excluded_paths.insert(path.into());
        self
    }

    /// Excludes multiple paths from logging.
    pub fn exclude_paths(mut self, paths: impl IntoIterator<Item = impl Into<String>>) -> Self {
        for path in paths {
            self.excluded_paths.insert(path.into());
        }
        self
    }

    /// Sets a custom log level for a specific procedure path.
    pub fn with_procedure_level(mut self, path: impl Into<String>, level: LogLevel) -> Self {
        self.procedure_levels.insert(path.into(), level);
        self
    }

    /// Enables distributed tracing with the given configuration.
    pub fn with_tracing(mut self, config: TracingConfig) -> Self {
        self.tracing = Some(config);
        self
    }

    /// Disables distributed tracing.
    pub fn without_tracing(mut self) -> Self {
        self.tracing = None;
        self
    }

    /// Sets the threshold in milliseconds for slow request warnings.
    pub fn with_slow_request_threshold(mut self, threshold_ms: u64) -> Self {
        self.slow_request_threshold_ms = Some(threshold_ms);
        self
    }

    /// Disables slow request logging.
    pub fn without_slow_request_logging(mut self) -> Self {
        self.slow_request_threshold_ms = None;
        self
    }

    /// Sets whether to log input/output sizes in bytes.
    pub fn with_size_logging(mut self, enabled: bool) -> Self {
        self.log_sizes = enabled;
        self
    }

    /// Checks if a path should be logged (not in excluded paths).
    pub fn should_log_path(&self, path: &str) -> bool {
        !self.excluded_paths.contains(path)
    }

    /// Gets the effective log level for a specific path.
    ///
    /// Returns the procedure-specific level if set, otherwise the global level.
    pub fn get_level_for_path(&self, path: &str) -> LogLevel {
        self.procedure_levels
            .get(path)
            .copied()
            .unwrap_or(self.level)
    }
}
