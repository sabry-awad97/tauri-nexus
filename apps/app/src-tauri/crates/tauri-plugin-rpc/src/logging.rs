//! Request/Response Logging
//!
//! Provides structured logging for RPC requests with request ID tracking,
//! timing metrics, and sensitive field redaction.
//!
//! # Example
//!
//! ```rust,ignore
//! use tauri_plugin_rpc::logging::{LogConfig, LogLevel, logging_middleware};
//!
//! let config = LogConfig::new()
//!     .with_level(LogLevel::Info)
//!     .with_timing(true)
//!     .redact_field("password")
//!     .redact_field("token");
//!
//! let router = Router::new()
//!     .middleware(logging_middleware(config))
//!     .query("users.get", get_user);
//! ```

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::middleware::{MiddlewareFn, ProcedureType, Request, from_fn};
use crate::{Context, Next};

/// Unique identifier for a request, used for tracing and correlation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RequestId(String);

impl RequestId {
    /// Creates a new unique request ID using UUID v7.
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext)).to_string())
    }

    /// Creates a request ID from an existing string.
    pub fn from_string(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Returns the request ID as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for RequestId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for RequestId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Log level for RPC logging.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    /// Trace level - most verbose, includes all details.
    Trace,
    /// Debug level - detailed information for debugging.
    Debug,
    /// Info level - general information about requests.
    #[default]
    Info,
    /// Warn level - warnings and errors only.
    Warn,
    /// Error level - errors only.
    Error,
    /// Off - no logging.
    Off,
}

impl LogLevel {
    /// Returns true if this level should log at the given level.
    pub fn should_log(&self, target: LogLevel) -> bool {
        match self {
            LogLevel::Off => false,
            LogLevel::Error => matches!(target, LogLevel::Error),
            LogLevel::Warn => matches!(target, LogLevel::Error | LogLevel::Warn),
            LogLevel::Info => matches!(target, LogLevel::Error | LogLevel::Warn | LogLevel::Info),
            LogLevel::Debug => !matches!(target, LogLevel::Trace | LogLevel::Off),
            LogLevel::Trace => target != LogLevel::Off,
        }
    }
}

/// Metadata about an RPC request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestMeta {
    /// Unique identifier for this request.
    pub request_id: RequestId,
    /// The procedure path being called.
    pub path: String,
    /// The type of procedure (query, mutation, subscription).
    pub procedure_type: ProcedureType,
    /// Timestamp when the request was received (Unix millis).
    pub timestamp: u64,
    /// Optional client identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
    /// Optional parent request ID for distributed tracing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_request_id: Option<RequestId>,
}

impl RequestMeta {
    /// Creates new request metadata.
    pub fn new(path: impl Into<String>, procedure_type: ProcedureType) -> Self {
        Self {
            request_id: RequestId::new(),
            path: path.into(),
            procedure_type,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            client_id: None,
            parent_request_id: None,
        }
    }

    /// Sets the client ID.
    pub fn with_client_id(mut self, client_id: impl Into<String>) -> Self {
        self.client_id = Some(client_id.into());
        self
    }

    /// Sets the parent request ID for distributed tracing.
    pub fn with_parent_request_id(mut self, parent_id: RequestId) -> Self {
        self.parent_request_id = Some(parent_id);
        self
    }
}

/// A structured log entry for an RPC request/response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    /// Request metadata.
    pub meta: RequestMeta,
    /// Duration of the request in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    /// Whether the request succeeded.
    pub success: bool,
    /// Error code if the request failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    /// Error message if the request failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    /// Redacted input (sensitive fields removed).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<Value>,
    /// Redacted output (sensitive fields removed).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<Value>,
}

impl LogEntry {
    /// Creates a new log entry from request metadata.
    pub fn new(meta: RequestMeta) -> Self {
        Self {
            meta,
            duration_ms: None,
            success: true,
            error_code: None,
            error_message: None,
            input: None,
            output: None,
        }
    }

    /// Sets the duration.
    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.duration_ms = Some(duration.as_millis() as u64);
        self
    }

    /// Marks the entry as an error.
    pub fn with_error(mut self, code: impl Into<String>, message: impl Into<String>) -> Self {
        self.success = false;
        self.error_code = Some(code.into());
        self.error_message = Some(message.into());
        self
    }

    /// Sets the redacted input.
    pub fn with_input(mut self, input: Value) -> Self {
        self.input = Some(input);
        self
    }

    /// Sets the redacted output.
    pub fn with_output(mut self, output: Value) -> Self {
        self.output = Some(output);
        self
    }
}

/// Configuration for RPC logging.
#[derive(Debug, Clone)]
pub struct LogConfig {
    /// Log level for requests.
    pub level: LogLevel,
    /// Whether to log timing information.
    pub log_timing: bool,
    /// Whether to log input data (after redaction).
    pub log_input: bool,
    /// Whether to log output data (after redaction).
    pub log_output: bool,
    /// Fields to redact from logs.
    pub redacted_fields: HashSet<String>,
    /// Replacement string for redacted values.
    pub redaction_replacement: String,
    /// Whether to log successful requests.
    pub log_success: bool,
    /// Whether to log failed requests.
    pub log_errors: bool,
    /// Paths to exclude from logging.
    pub excluded_paths: HashSet<String>,
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
        }
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
    ]
    .into_iter()
    .map(String::from)
    .collect()
}

impl LogConfig {
    /// Creates a new log configuration with defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the log level.
    pub fn with_level(mut self, level: LogLevel) -> Self {
        self.level = level;
        self
    }

    /// Enables or disables timing logging.
    pub fn with_timing(mut self, enabled: bool) -> Self {
        self.log_timing = enabled;
        self
    }

    /// Enables or disables input logging.
    pub fn with_input_logging(mut self, enabled: bool) -> Self {
        self.log_input = enabled;
        self
    }

    /// Enables or disables output logging.
    pub fn with_output_logging(mut self, enabled: bool) -> Self {
        self.log_output = enabled;
        self
    }

    /// Adds a field to redact.
    pub fn redact_field(mut self, field: impl Into<String>) -> Self {
        self.redacted_fields.insert(field.into());
        self
    }

    /// Adds multiple fields to redact.
    pub fn redact_fields(mut self, fields: impl IntoIterator<Item = impl Into<String>>) -> Self {
        for field in fields {
            self.redacted_fields.insert(field.into());
        }
        self
    }

    /// Clears all redacted fields.
    pub fn clear_redacted_fields(mut self) -> Self {
        self.redacted_fields.clear();
        self
    }

    /// Sets the redaction replacement string.
    pub fn with_redaction_replacement(mut self, replacement: impl Into<String>) -> Self {
        self.redaction_replacement = replacement.into();
        self
    }

    /// Enables or disables logging of successful requests.
    pub fn with_success_logging(mut self, enabled: bool) -> Self {
        self.log_success = enabled;
        self
    }

    /// Enables or disables logging of failed requests.
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

    /// Returns true if the given path should be logged.
    pub fn should_log_path(&self, path: &str) -> bool {
        !self.excluded_paths.contains(path)
    }
}

/// Redacts sensitive fields from a JSON value.
pub fn redact_value(value: &Value, config: &LogConfig) -> Value {
    match value {
        Value::Object(map) => {
            let mut redacted = serde_json::Map::new();
            for (key, val) in map {
                let lower_key = key.to_lowercase();
                if config
                    .redacted_fields
                    .iter()
                    .any(|f| lower_key.contains(&f.to_lowercase()))
                {
                    redacted.insert(
                        key.clone(),
                        Value::String(config.redaction_replacement.clone()),
                    );
                } else {
                    redacted.insert(key.clone(), redact_value(val, config));
                }
            }
            Value::Object(redacted)
        }
        Value::Array(arr) => Value::Array(arr.iter().map(|v| redact_value(v, config)).collect()),
        other => other.clone(),
    }
}

/// A logger that can be used to emit log entries.
pub trait Logger: Send + Sync {
    /// Logs a request/response entry.
    fn log(&self, entry: &LogEntry, level: LogLevel);
}

/// Default logger that uses the tracing crate.
#[derive(Debug, Clone, Default)]
pub struct TracingLogger;

impl Logger for TracingLogger {
    fn log(&self, entry: &LogEntry, level: LogLevel) {
        let request_id = entry.meta.request_id.as_str();
        let path = &entry.meta.path;
        let procedure_type = format!("{:?}", entry.meta.procedure_type);
        let duration_ms = entry.duration_ms.unwrap_or(0);

        if entry.success {
            match level {
                LogLevel::Trace => {
                    tracing::trace!(
                        request_id = %request_id,
                        path = %path,
                        procedure_type = %procedure_type,
                        duration_ms = %duration_ms,
                        "RPC request completed"
                    );
                }
                LogLevel::Debug => {
                    tracing::debug!(
                        request_id = %request_id,
                        path = %path,
                        procedure_type = %procedure_type,
                        duration_ms = %duration_ms,
                        "RPC request completed"
                    );
                }
                LogLevel::Info => {
                    tracing::info!(
                        request_id = %request_id,
                        path = %path,
                        procedure_type = %procedure_type,
                        duration_ms = %duration_ms,
                        "RPC request completed"
                    );
                }
                _ => {}
            }
        } else {
            let error_code = entry.error_code.as_deref().unwrap_or("UNKNOWN");
            let error_message = entry.error_message.as_deref().unwrap_or("");

            // Errors are always logged as warnings per user requirement
            tracing::warn!(
                request_id = %request_id,
                path = %path,
                procedure_type = %procedure_type,
                duration_ms = %duration_ms,
                error_code = %error_code,
                error_message = %error_message,
                "RPC request failed"
            );
        }
    }
}

/// Creates a logging middleware with the given configuration.
///
/// # Example
///
/// ```rust,ignore
/// use tauri_plugin_rpc::logging::{LogConfig, logging_middleware};
///
/// let config = LogConfig::new()
///     .with_timing(true)
///     .redact_field("password");
///
/// let router = Router::new()
///     .middleware(logging_middleware(config))
///     .query("users.get", get_user);
/// ```
pub fn logging_middleware<Ctx>(config: LogConfig) -> MiddlewareFn<Ctx>
where
    Ctx: Clone + Send + Sync + 'static,
{
    logging_middleware_with_logger(config, TracingLogger)
}

/// Creates a logging middleware with a custom logger.
pub fn logging_middleware_with_logger<Ctx, L>(config: LogConfig, logger: L) -> MiddlewareFn<Ctx>
where
    Ctx: Clone + Send + Sync + 'static,
    L: Logger + Clone + 'static,
{
    let config = Arc::new(config);
    let logger = Arc::new(logger);

    from_fn(move |ctx: Context<Ctx>, req: Request, next: Next<Ctx>| {
        let config = Arc::clone(&config);
        let logger = Arc::clone(&logger);

        async move {
            // Check if logging is disabled or path is excluded
            if config.level == LogLevel::Off || !config.should_log_path(&req.path) {
                return next(ctx, req).await;
            }

            // Create request metadata
            let meta = RequestMeta::new(&req.path, req.procedure_type);
            let request_id = meta.request_id.clone();
            let start = Instant::now();

            // Log input if enabled
            let redacted_input = if config.log_input {
                Some(redact_value(&req.input, &config))
            } else {
                None
            };

            // Execute the request
            let result = next(ctx, req).await;
            let duration = start.elapsed();

            // Build log entry
            let mut entry = LogEntry::new(meta).with_duration(duration);

            if let Some(input) = redacted_input {
                entry = entry.with_input(input);
            }

            match &result {
                Ok(response) => {
                    if config.log_success {
                        if config.log_output {
                            entry = entry.with_output(redact_value(response, &config));
                        }
                        logger.log(&entry, config.level);
                    }
                }
                Err(error) => {
                    if config.log_errors {
                        entry = entry.with_error(format!("{:?}", error.code), &error.message);
                        logger.log(&entry, LogLevel::Warn);
                    }
                }
            }

            // Return the request ID in the response for correlation
            // (The request ID is logged but not added to response to avoid changing the API)
            let _ = request_id; // Suppress unused warning

            result
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_request_id_uniqueness() {
        let ids: Vec<RequestId> = (0..100).map(|_| RequestId::new()).collect();
        let unique: HashSet<_> = ids.iter().map(|id| id.as_str()).collect();
        assert_eq!(ids.len(), unique.len());
    }

    #[test]
    fn test_request_id_from_string() {
        let id = RequestId::from_string("test-id-123");
        assert_eq!(id.as_str(), "test-id-123");
    }

    #[test]
    fn test_request_id_display() {
        let id = RequestId::from_string("display-test");
        assert_eq!(format!("{}", id), "display-test");
    }

    #[test]
    fn test_log_level_should_log() {
        assert!(LogLevel::Info.should_log(LogLevel::Error));
        assert!(LogLevel::Info.should_log(LogLevel::Warn));
        assert!(LogLevel::Info.should_log(LogLevel::Info));
        assert!(!LogLevel::Info.should_log(LogLevel::Debug));
        assert!(!LogLevel::Info.should_log(LogLevel::Trace));

        assert!(!LogLevel::Off.should_log(LogLevel::Error));
        assert!(LogLevel::Trace.should_log(LogLevel::Error));
    }

    #[test]
    fn test_request_meta_creation() {
        let meta = RequestMeta::new("users.get", ProcedureType::Query);
        assert_eq!(meta.path, "users.get");
        assert_eq!(meta.procedure_type, ProcedureType::Query);
        assert!(meta.timestamp > 0);
        assert!(meta.client_id.is_none());
        assert!(meta.parent_request_id.is_none());
    }

    #[test]
    fn test_request_meta_with_client_id() {
        let meta = RequestMeta::new("test", ProcedureType::Mutation).with_client_id("client-123");
        assert_eq!(meta.client_id, Some("client-123".to_string()));
    }

    #[test]
    fn test_request_meta_with_parent_id() {
        let parent = RequestId::from_string("parent-id");
        let meta =
            RequestMeta::new("test", ProcedureType::Query).with_parent_request_id(parent.clone());
        assert_eq!(meta.parent_request_id, Some(parent));
    }

    #[test]
    fn test_log_entry_creation() {
        let meta = RequestMeta::new("test", ProcedureType::Query);
        let entry = LogEntry::new(meta);
        assert!(entry.success);
        assert!(entry.duration_ms.is_none());
        assert!(entry.error_code.is_none());
    }

    #[test]
    fn test_log_entry_with_duration() {
        let meta = RequestMeta::new("test", ProcedureType::Query);
        let entry = LogEntry::new(meta).with_duration(Duration::from_millis(150));
        assert_eq!(entry.duration_ms, Some(150));
    }

    #[test]
    fn test_log_entry_with_error() {
        let meta = RequestMeta::new("test", ProcedureType::Query);
        let entry = LogEntry::new(meta).with_error("NOT_FOUND", "User not found");
        assert!(!entry.success);
        assert_eq!(entry.error_code, Some("NOT_FOUND".to_string()));
        assert_eq!(entry.error_message, Some("User not found".to_string()));
    }

    #[test]
    fn test_log_config_defaults() {
        let config = LogConfig::new();
        assert_eq!(config.level, LogLevel::Info);
        assert!(config.log_timing);
        assert!(!config.log_input);
        assert!(!config.log_output);
        assert!(config.log_success);
        assert!(config.log_errors);
        assert!(config.redacted_fields.contains("password"));
        assert!(config.redacted_fields.contains("token"));
    }

    #[test]
    fn test_log_config_builder() {
        let config = LogConfig::new()
            .with_level(LogLevel::Debug)
            .with_timing(false)
            .with_input_logging(true)
            .with_output_logging(true)
            .redact_field("custom_secret")
            .exclude_path("health");

        assert_eq!(config.level, LogLevel::Debug);
        assert!(!config.log_timing);
        assert!(config.log_input);
        assert!(config.log_output);
        assert!(config.redacted_fields.contains("custom_secret"));
        assert!(config.excluded_paths.contains("health"));
    }

    #[test]
    fn test_log_config_clear_redacted_fields() {
        let config = LogConfig::new().clear_redacted_fields();
        assert!(config.redacted_fields.is_empty());
    }

    #[test]
    fn test_log_config_should_log_path() {
        let config = LogConfig::new().exclude_path("health").exclude_path("ping");

        assert!(!config.should_log_path("health"));
        assert!(!config.should_log_path("ping"));
        assert!(config.should_log_path("users.get"));
    }

    #[test]
    fn test_redact_value_simple() {
        let config = LogConfig::new();
        let input = json!({
            "username": "john",
            "password": "secret123"
        });

        let redacted = redact_value(&input, &config);
        assert_eq!(redacted["username"], "john");
        assert_eq!(redacted["password"], "[REDACTED]");
    }

    #[test]
    fn test_redact_value_nested() {
        let config = LogConfig::new();
        let input = json!({
            "user": {
                "name": "john",
                "auth": {
                    "password": "secret",
                    "api_key": "key123"
                }
            }
        });

        let redacted = redact_value(&input, &config);
        assert_eq!(redacted["user"]["name"], "john");
        // "auth" is redacted because it matches the "auth" field in default redacted fields
        assert_eq!(redacted["user"]["auth"], "[REDACTED]");
    }

    #[test]
    fn test_redact_value_nested_deep() {
        // Use a config without "auth" to test deep nesting
        let config = LogConfig::new()
            .clear_redacted_fields()
            .redact_field("password")
            .redact_field("api_key");
        let input = json!({
            "user": {
                "name": "john",
                "settings": {
                    "password": "secret",
                    "api_key": "key123"
                }
            }
        });

        let redacted = redact_value(&input, &config);
        assert_eq!(redacted["user"]["name"], "john");
        assert_eq!(redacted["user"]["settings"]["password"], "[REDACTED]");
        assert_eq!(redacted["user"]["settings"]["api_key"], "[REDACTED]");
    }

    #[test]
    fn test_redact_value_array() {
        let config = LogConfig::new();
        let input = json!([
            {"name": "john", "token": "abc"},
            {"name": "jane", "token": "xyz"}
        ]);

        let redacted = redact_value(&input, &config);
        assert_eq!(redacted[0]["name"], "john");
        assert_eq!(redacted[0]["token"], "[REDACTED]");
        assert_eq!(redacted[1]["name"], "jane");
        assert_eq!(redacted[1]["token"], "[REDACTED]");
    }

    #[test]
    fn test_redact_value_case_insensitive() {
        let config = LogConfig::new();
        let input = json!({
            "PASSWORD": "secret1",
            "Password": "secret2",
            "user_password": "secret3"
        });

        let redacted = redact_value(&input, &config);
        assert_eq!(redacted["PASSWORD"], "[REDACTED]");
        assert_eq!(redacted["Password"], "[REDACTED]");
        assert_eq!(redacted["user_password"], "[REDACTED]");
    }

    #[test]
    fn test_redact_value_custom_replacement() {
        let config = LogConfig::new().with_redaction_replacement("***");
        let input = json!({"password": "secret"});

        let redacted = redact_value(&input, &config);
        assert_eq!(redacted["password"], "***");
    }

    #[test]
    fn test_redact_value_primitives_unchanged() {
        let config = LogConfig::new();

        assert_eq!(redact_value(&json!(42), &config), json!(42));
        assert_eq!(redact_value(&json!("hello"), &config), json!("hello"));
        assert_eq!(redact_value(&json!(true), &config), json!(true));
        assert_eq!(redact_value(&json!(null), &config), json!(null));
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;
    use serde_json::json;

    proptest! {
        /// Property: All generated request IDs should be unique.
        #[test]
        fn prop_request_id_uniqueness(count in 10usize..100) {
            let ids: Vec<RequestId> = (0..count).map(|_| RequestId::new()).collect();
            let unique: std::collections::HashSet<_> = ids.iter().map(|id| id.as_str()).collect();
            prop_assert_eq!(ids.len(), unique.len());
        }

        /// Property: Request IDs should be valid UUIDs.
        #[test]
        fn prop_request_id_is_valid_uuid(_seed in 0u64..1000) {
            let id = RequestId::new();
            let parsed = uuid::Uuid::parse_str(id.as_str());
            prop_assert!(parsed.is_ok(), "Request ID should be a valid UUID");
        }

        /// Property: Redaction should preserve non-sensitive fields.
        #[test]
        fn prop_redaction_preserves_non_sensitive(
            name in "[a-z]{3,10}",
            age in 1i32..100
        ) {
            let config = LogConfig::new();
            let input = json!({
                "name": name.clone(),
                "age": age
            });

            let redacted = redact_value(&input, &config);
            prop_assert_eq!(&redacted["name"], &json!(name));
            prop_assert_eq!(&redacted["age"], &json!(age));
        }

        /// Property: Redaction should always redact sensitive fields.
        #[test]
        fn prop_redaction_always_redacts_sensitive(
            password in "[a-zA-Z0-9]{5,20}",
            token in "[a-zA-Z0-9]{10,30}"
        ) {
            let config = LogConfig::new();
            let input = json!({
                "password": password,
                "token": token,
                "api_key": "some-key"
            });

            let redacted = redact_value(&input, &config);
            prop_assert_eq!(&redacted["password"], &json!("[REDACTED]"));
            prop_assert_eq!(&redacted["token"], &json!("[REDACTED]"));
            prop_assert_eq!(&redacted["api_key"], &json!("[REDACTED]"));
        }

        /// Property: Log level ordering should be consistent.
        #[test]
        fn prop_log_level_ordering_consistent(level_idx in 0usize..5) {
            let levels = [LogLevel::Trace, LogLevel::Debug, LogLevel::Info, LogLevel::Warn, LogLevel::Error];
            let level = levels[level_idx];

            // Error level should always be logged (except for Off)
            if level != LogLevel::Off {
                prop_assert!(level.should_log(LogLevel::Error));
            }

            // Off should never log anything
            prop_assert!(!LogLevel::Off.should_log(level));
        }

        /// Property: Excluded paths should never be logged.
        #[test]
        fn prop_excluded_paths_not_logged(
            path in "[a-z]{3,10}"
        ) {
            let config = LogConfig::new().exclude_path(path.clone());
            prop_assert!(!config.should_log_path(&path));
        }
    }
}
