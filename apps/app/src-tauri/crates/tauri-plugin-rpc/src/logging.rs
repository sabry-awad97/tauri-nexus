//! Comprehensive Request/Response Logging with Tracing
//!
//! Provides structured logging for RPC requests with request ID tracking,
//! timing metrics, sensitive field redaction, and distributed tracing support.
//!
//! # Features
//!
//! - **Request ID Tracking**: UUID v7 based request IDs for correlation
//! - **Structured Logging**: JSON-compatible log entries with metadata
//! - **Sensitive Data Redaction**: Automatic redaction of passwords, tokens, etc.
//! - **Performance Metrics**: Request duration tracking with histograms
//! - **Distributed Tracing**: OpenTelemetry-compatible span context
//! - **Configurable Log Levels**: Per-procedure and global log level control
//!
//! # Example
//!
//! ```rust,ignore
//! use tauri_plugin_rpc::logging::{LogConfig, LogLevel, logging_middleware, TracingConfig};
//!
//! let config = LogConfig::new()
//!     .with_level(LogLevel::Info)
//!     .with_timing(true)
//!     .with_tracing(TracingConfig::default())
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

// =============================================================================
// Request ID
// =============================================================================

/// Unique identifier for a request, used for tracing and correlation.
///
/// Uses UUID v7 for time-ordered, sortable identifiers that are ideal
/// for distributed tracing and log correlation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RequestId(uuid::Uuid);

impl RequestId {
    /// Creates a new unique request ID using UUID v7.
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext)))
    }

    /// Creates a request ID from an existing UUID.
    pub fn from_uuid(uuid: uuid::Uuid) -> Self {
        Self(uuid)
    }

    /// Creates a request ID from an existing string.
    ///
    /// # Panics
    /// Panics if the string is not a valid UUID.
    pub fn from_string(id: impl AsRef<str>) -> Self {
        Self(uuid::Uuid::parse_str(id.as_ref()).expect("Invalid UUID string"))
    }

    /// Tries to create a request ID from a string.
    ///
    /// Returns None if the string is not a valid UUID.
    pub fn try_from_string(id: impl AsRef<str>) -> Option<Self> {
        uuid::Uuid::parse_str(id.as_ref()).ok().map(Self)
    }

    /// Returns the request ID as a UUID reference.
    pub fn as_uuid(&self) -> &uuid::Uuid {
        &self.0
    }

    /// Returns the request ID as a string.
    pub fn as_str(&self) -> String {
        self.0.to_string()
    }

    /// Returns the short form of the request ID (first 8 characters).
    pub fn short(&self) -> String {
        self.0.to_string()[..8].to_string()
    }

    /// Returns the inner UUID.
    pub fn into_uuid(self) -> uuid::Uuid {
        self.0
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

impl From<uuid::Uuid> for RequestId {
    fn from(uuid: uuid::Uuid) -> Self {
        Self(uuid)
    }
}

impl From<RequestId> for uuid::Uuid {
    fn from(id: RequestId) -> Self {
        id.0
    }
}

// =============================================================================
// Log Level
// =============================================================================

/// Log level for RPC logging.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    /// Trace level - most verbose, includes all details including input/output.
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

    /// Convert to tracing::Level
    pub fn to_tracing_level(&self) -> Option<tracing::Level> {
        match self {
            LogLevel::Trace => Some(tracing::Level::TRACE),
            LogLevel::Debug => Some(tracing::Level::DEBUG),
            LogLevel::Info => Some(tracing::Level::INFO),
            LogLevel::Warn => Some(tracing::Level::WARN),
            LogLevel::Error => Some(tracing::Level::ERROR),
            LogLevel::Off => None,
        }
    }
}

// =============================================================================
// Request Metadata
// =============================================================================

/// Metadata about an RPC request for logging and tracing.
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
    /// Optional trace ID for OpenTelemetry compatibility.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    /// Optional span ID for OpenTelemetry compatibility.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span_id: Option<String>,
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
            trace_id: None,
            span_id: None,
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

    /// Sets the trace ID for OpenTelemetry compatibility.
    pub fn with_trace_id(mut self, trace_id: impl Into<String>) -> Self {
        self.trace_id = Some(trace_id.into());
        self
    }

    /// Sets the span ID for OpenTelemetry compatibility.
    pub fn with_span_id(mut self, span_id: impl Into<String>) -> Self {
        self.span_id = Some(span_id.into());
        self
    }
}

// =============================================================================
// Log Entry
// =============================================================================

/// A structured log entry for an RPC request/response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    /// Request metadata.
    pub meta: RequestMeta,
    /// Duration of the request in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    /// Duration in microseconds for high-precision timing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_us: Option<u64>,
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
    /// Input size in bytes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_size: Option<usize>,
    /// Output size in bytes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_size: Option<usize>,
    /// Whether the response was served from cache.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_hit: Option<bool>,
    /// Rate limit remaining after this request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit_remaining: Option<u32>,
}

impl LogEntry {
    /// Creates a new log entry from request metadata.
    pub fn new(meta: RequestMeta) -> Self {
        Self {
            meta,
            duration_ms: None,
            duration_us: None,
            success: true,
            error_code: None,
            error_message: None,
            input: None,
            output: None,
            input_size: None,
            output_size: None,
            cache_hit: None,
            rate_limit_remaining: None,
        }
    }

    /// Sets the duration.
    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.duration_ms = Some(duration.as_millis() as u64);
        self.duration_us = Some(duration.as_micros() as u64);
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
        self.input_size = Some(serde_json::to_vec(&input).map(|v| v.len()).unwrap_or(0));
        self.input = Some(input);
        self
    }

    /// Sets the redacted output.
    pub fn with_output(mut self, output: Value) -> Self {
        self.output_size = Some(serde_json::to_vec(&output).map(|v| v.len()).unwrap_or(0));
        self.output = Some(output);
        self
    }

    /// Sets the cache hit status.
    pub fn with_cache_hit(mut self, hit: bool) -> Self {
        self.cache_hit = Some(hit);
        self
    }

    /// Sets the rate limit remaining.
    pub fn with_rate_limit_remaining(mut self, remaining: u32) -> Self {
        self.rate_limit_remaining = Some(remaining);
        self
    }
}

// =============================================================================
// Tracing Configuration
// =============================================================================

/// Configuration for distributed tracing integration.
#[derive(Debug, Clone)]
pub struct TracingConfig {
    /// Whether to create spans for each request.
    pub create_spans: bool,
    /// Whether to record input as span attributes.
    pub record_input: bool,
    /// Whether to record output as span attributes.
    pub record_output: bool,
    /// Maximum size of input/output to record (bytes).
    pub max_attribute_size: usize,
    /// Service name for tracing.
    pub service_name: String,
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self {
            create_spans: true,
            record_input: false,
            record_output: false,
            max_attribute_size: 1024,
            service_name: "tauri-rpc".to_string(),
        }
    }
}

impl TracingConfig {
    /// Creates a new tracing configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets whether to create spans.
    pub fn with_spans(mut self, enabled: bool) -> Self {
        self.create_spans = enabled;
        self
    }

    /// Sets whether to record input.
    pub fn with_input_recording(mut self, enabled: bool) -> Self {
        self.record_input = enabled;
        self
    }

    /// Sets whether to record output.
    pub fn with_output_recording(mut self, enabled: bool) -> Self {
        self.record_output = enabled;
        self
    }

    /// Sets the maximum attribute size.
    pub fn with_max_attribute_size(mut self, size: usize) -> Self {
        self.max_attribute_size = size;
        self
    }

    /// Sets the service name.
    pub fn with_service_name(mut self, name: impl Into<String>) -> Self {
        self.service_name = name.into();
        self
    }
}

// =============================================================================
// Log Configuration
// =============================================================================

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
    /// Per-procedure log level overrides.
    pub procedure_levels: std::collections::HashMap<String, LogLevel>,
    /// Tracing configuration.
    pub tracing: Option<TracingConfig>,
    /// Whether to log slow requests (above threshold).
    pub slow_request_threshold_ms: Option<u64>,
    /// Whether to include input/output size in logs.
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
            procedure_levels: std::collections::HashMap::new(),
            tracing: Some(TracingConfig::default()),
            slow_request_threshold_ms: Some(1000), // 1 second
            log_sizes: true,
        }
    }
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

    /// Sets a log level for a specific procedure.
    pub fn with_procedure_level(mut self, path: impl Into<String>, level: LogLevel) -> Self {
        self.procedure_levels.insert(path.into(), level);
        self
    }

    /// Sets the tracing configuration.
    pub fn with_tracing(mut self, config: TracingConfig) -> Self {
        self.tracing = Some(config);
        self
    }

    /// Disables tracing.
    pub fn without_tracing(mut self) -> Self {
        self.tracing = None;
        self
    }

    /// Sets the slow request threshold in milliseconds.
    pub fn with_slow_request_threshold(mut self, threshold_ms: u64) -> Self {
        self.slow_request_threshold_ms = Some(threshold_ms);
        self
    }

    /// Disables slow request logging.
    pub fn without_slow_request_logging(mut self) -> Self {
        self.slow_request_threshold_ms = None;
        self
    }

    /// Enables or disables size logging.
    pub fn with_size_logging(mut self, enabled: bool) -> Self {
        self.log_sizes = enabled;
        self
    }

    /// Returns true if the given path should be logged.
    pub fn should_log_path(&self, path: &str) -> bool {
        !self.excluded_paths.contains(path)
    }

    /// Gets the effective log level for a procedure.
    pub fn get_level_for_path(&self, path: &str) -> LogLevel {
        self.procedure_levels
            .get(path)
            .copied()
            .unwrap_or(self.level)
    }
}

// =============================================================================
// Redaction
// =============================================================================

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

/// Truncates a string value if it exceeds the maximum size.
#[allow(dead_code)]
fn truncate_value(value: &Value, max_size: usize) -> Value {
    let serialized = serde_json::to_string(value).unwrap_or_default();
    if serialized.len() <= max_size {
        value.clone()
    } else {
        Value::String(format!("[truncated: {} bytes]", serialized.len()))
    }
}

// =============================================================================
// Logger Trait
// =============================================================================

/// A logger that can be used to emit log entries.
pub trait Logger: Send + Sync {
    /// Logs a request/response entry.
    fn log(&self, entry: &LogEntry, level: LogLevel);

    /// Logs a slow request warning.
    fn log_slow_request(&self, entry: &LogEntry, threshold_ms: u64) {
        let duration_ms = entry.duration_ms.unwrap_or(0);
        tracing::warn!(
            request_id = %entry.meta.request_id,
            path = %entry.meta.path,
            procedure_type = %entry.meta.procedure_type,
            duration_ms = %duration_ms,
            threshold_ms = %threshold_ms,
            "Slow RPC request detected"
        );
    }

    /// Logs the start of a request (for tracing).
    fn log_request_start(&self, meta: &RequestMeta) {
        tracing::debug!(
            request_id = %meta.request_id,
            path = %meta.path,
            procedure_type = %meta.procedure_type,
            "RPC request started"
        );
    }
}

/// Default logger that uses the tracing crate with structured fields.
#[derive(Debug, Clone, Default)]
pub struct TracingLogger;

impl Logger for TracingLogger {
    fn log(&self, entry: &LogEntry, level: LogLevel) {
        let request_id = entry.meta.request_id.to_string();
        let path = &entry.meta.path;
        let procedure_type = format!("{}", entry.meta.procedure_type);
        let duration_ms = entry.duration_ms.unwrap_or(0);
        let duration_us = entry.duration_us.unwrap_or(0);

        if entry.success {
            match level {
                LogLevel::Trace => {
                    tracing::trace!(
                        request_id = %request_id,
                        path = %path,
                        procedure_type = %procedure_type,
                        duration_ms = %duration_ms,
                        duration_us = %duration_us,
                        input_size = ?entry.input_size,
                        output_size = ?entry.output_size,
                        cache_hit = ?entry.cache_hit,
                        "RPC request completed"
                    );
                }
                LogLevel::Debug => {
                    tracing::debug!(
                        request_id = %request_id,
                        path = %path,
                        procedure_type = %procedure_type,
                        duration_ms = %duration_ms,
                        input_size = ?entry.input_size,
                        output_size = ?entry.output_size,
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

// =============================================================================
// JSON Logger
// =============================================================================

/// A logger that outputs JSON-formatted log entries.
#[derive(Debug, Clone, Default)]
pub struct JsonLogger;

impl Logger for JsonLogger {
    fn log(&self, entry: &LogEntry, _level: LogLevel) {
        if let Ok(json) = serde_json::to_string(entry) {
            if entry.success {
                tracing::info!(target: "rpc_json", "{}", json);
            } else {
                tracing::warn!(target: "rpc_json", "{}", json);
            }
        }
    }
}

// =============================================================================
// Metrics Logger
// =============================================================================

/// A logger that also records metrics for monitoring.
#[derive(Debug, Clone)]
pub struct MetricsLogger {
    inner: TracingLogger,
}

impl Default for MetricsLogger {
    fn default() -> Self {
        Self::new()
    }
}

impl MetricsLogger {
    /// Creates a new metrics logger.
    pub fn new() -> Self {
        Self {
            inner: TracingLogger,
        }
    }
}

impl Logger for MetricsLogger {
    fn log(&self, entry: &LogEntry, level: LogLevel) {
        // Log using the inner logger
        self.inner.log(entry, level);

        // Record metrics using tracing events that can be captured by metrics layers
        let duration_us = entry.duration_us.unwrap_or(0);
        let path = &entry.meta.path;
        let procedure_type = format!("{}", entry.meta.procedure_type);

        tracing::trace!(
            target: "rpc_metrics",
            metric_type = "counter",
            metric_name = "rpc_requests_total",
            value = 1u64,
            path = %path,
            procedure_type = %procedure_type,
            success = %entry.success,
            "RPC request metric"
        );

        tracing::trace!(
            target: "rpc_metrics",
            metric_type = "histogram",
            metric_name = "rpc_request_duration_us",
            value = duration_us,
            path = %path,
            procedure_type = %procedure_type,
            "RPC duration metric"
        );

        if let Some(input_size) = entry.input_size {
            tracing::trace!(
                target: "rpc_metrics",
                metric_type = "histogram",
                metric_name = "rpc_request_input_bytes",
                value = input_size,
                path = %path,
                "RPC input size metric"
            );
        }

        if let Some(output_size) = entry.output_size {
            tracing::trace!(
                target: "rpc_metrics",
                metric_type = "histogram",
                metric_name = "rpc_response_output_bytes",
                value = output_size,
                path = %path,
                "RPC output size metric"
            );
        }

        if !entry.success {
            let error_code = entry.error_code.as_deref().unwrap_or("UNKNOWN");
            tracing::trace!(
                target: "rpc_metrics",
                metric_type = "counter",
                metric_name = "rpc_errors_total",
                value = 1u64,
                path = %path,
                error_code = %error_code,
                "RPC error metric"
            );
        }
    }
}

// =============================================================================
// Logging Middleware
// =============================================================================

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

    let middleware = move |ctx: Context<Ctx>, req: Request, next: Next<Ctx>| {
        let config = Arc::clone(&config);
        let logger = Arc::clone(&logger);

        async move {
            // Check if logging is disabled or path is excluded
            if config.level == LogLevel::Off || !config.should_log_path(&req.path) {
                return next(ctx, req).await;
            }

            // Create request metadata
            let meta = RequestMeta::new(&req.path, req.procedure_type);
            let request_id = meta.request_id;
            let effective_level = config.get_level_for_path(&req.path);

            // Log request start at debug level
            if effective_level.should_log(LogLevel::Debug) {
                logger.log_request_start(&meta);
            }

            let start = Instant::now();

            // Calculate input size if needed
            let input_size = if config.log_sizes {
                Some(serde_json::to_vec(&req.input).map(|v| v.len()).unwrap_or(0))
            } else {
                None
            };

            // Log input if enabled
            let redacted_input = if config.log_input {
                Some(redact_value(&req.input, &config))
            } else {
                None
            };

            // Execute the request with tracing span if configured
            let result = if config
                .tracing
                .as_ref()
                .map(|t| t.create_spans)
                .unwrap_or(false)
            {
                let span = tracing::info_span!(
                    "rpc_request",
                    request_id = %request_id,
                    path = %req.path,
                    procedure_type = %req.procedure_type,
                    otel.name = %format!("RPC {}", req.path),
                    otel.kind = "server",
                );
                let _guard = span.enter();
                next(ctx, req.clone()).await
            } else {
                next(ctx, req.clone()).await
            };

            let duration = start.elapsed();

            // Build log entry
            let mut entry = LogEntry::new(meta);

            if config.log_timing {
                entry = entry.with_duration(duration);
            }

            if let Some(input) = redacted_input {
                entry = entry.with_input(input);
            } else if let Some(size) = input_size {
                entry.input_size = Some(size);
            }

            match &result {
                Ok(response) => {
                    if config.log_success {
                        if config.log_output {
                            entry = entry.with_output(redact_value(response, &config));
                        } else if config.log_sizes {
                            entry.output_size =
                                Some(serde_json::to_vec(response).map(|v| v.len()).unwrap_or(0));
                        }
                        logger.log(&entry, effective_level);
                    }

                    // Check for slow request
                    if let Some(threshold) = config.slow_request_threshold_ms
                        && duration.as_millis() as u64 > threshold
                    {
                        logger.log_slow_request(&entry, threshold);
                    }
                }
                Err(error) => {
                    if config.log_errors {
                        entry = entry.with_error(format!("{:?}", error.code), &error.message);
                        logger.log(&entry, LogLevel::Warn);
                    }
                }
            }

            result
        }
    };
    from_fn(middleware)
}

// =============================================================================
// Specialized Logging Functions
// =============================================================================

/// Log a subscription lifecycle event.
pub fn log_subscription_event(subscription_id: &str, path: &str, event: SubscriptionLogEvent) {
    match event {
        SubscriptionLogEvent::Started => {
            tracing::info!(
                subscription_id = %subscription_id,
                path = %path,
                "Subscription started"
            );
        }
        SubscriptionLogEvent::EventEmitted { event_id } => {
            tracing::trace!(
                subscription_id = %subscription_id,
                path = %path,
                event_id = ?event_id,
                "Subscription event emitted"
            );
        }
        SubscriptionLogEvent::Cancelled => {
            tracing::info!(
                subscription_id = %subscription_id,
                path = %path,
                "Subscription cancelled"
            );
        }
        SubscriptionLogEvent::Completed => {
            tracing::info!(
                subscription_id = %subscription_id,
                path = %path,
                "Subscription completed"
            );
        }
        SubscriptionLogEvent::Error { code, message } => {
            tracing::warn!(
                subscription_id = %subscription_id,
                path = %path,
                error_code = %code,
                error_message = %message,
                "Subscription error"
            );
        }
    }
}

/// Subscription lifecycle events for logging.
#[derive(Debug, Clone)]
pub enum SubscriptionLogEvent {
    /// Subscription was started.
    Started,
    /// An event was emitted.
    EventEmitted {
        /// Optional event ID for the emitted event.
        event_id: Option<String>,
    },
    /// Subscription was cancelled by client.
    Cancelled,
    /// Subscription completed normally.
    Completed,
    /// Subscription encountered an error.
    Error {
        /// Error code.
        code: String,
        /// Error message.
        message: String,
    },
}

/// Log a batch request.
pub fn log_batch_request(
    request_id: &str,
    batch_size: usize,
    success_count: usize,
    error_count: usize,
    duration_ms: u64,
) {
    tracing::info!(
        request_id = %request_id,
        batch_size = %batch_size,
        success_count = %success_count,
        error_count = %error_count,
        duration_ms = %duration_ms,
        "Batch request completed"
    );
}

/// Log a cache event.
pub fn log_cache_event(path: &str, event: CacheLogEvent) {
    match event {
        CacheLogEvent::Hit => {
            tracing::debug!(path = %path, "Cache hit");
        }
        CacheLogEvent::Miss => {
            tracing::debug!(path = %path, "Cache miss");
        }
        CacheLogEvent::Set { ttl_ms } => {
            tracing::trace!(path = %path, ttl_ms = %ttl_ms, "Cache entry set");
        }
        CacheLogEvent::Invalidated { pattern } => {
            tracing::debug!(path = %path, pattern = %pattern, "Cache invalidated");
        }
        CacheLogEvent::Expired => {
            tracing::trace!(path = %path, "Cache entry expired");
        }
    }
}

/// Cache events for logging.
#[derive(Debug, Clone)]
pub enum CacheLogEvent {
    /// Cache hit.
    Hit,
    /// Cache miss.
    Miss,
    /// Cache entry was set.
    Set {
        /// Time-to-live in milliseconds.
        ttl_ms: u64,
    },
    /// Cache was invalidated.
    Invalidated {
        /// Pattern used for invalidation.
        pattern: String,
    },
    /// Cache entry expired.
    Expired,
}

/// Log a rate limit event.
pub fn log_rate_limit_event(path: &str, client_id: &str, event: RateLimitLogEvent) {
    match event {
        RateLimitLogEvent::Allowed { remaining } => {
            tracing::trace!(
                path = %path,
                client_id = %client_id,
                remaining = %remaining,
                "Rate limit check passed"
            );
        }
        RateLimitLogEvent::Limited { retry_after_ms } => {
            tracing::warn!(
                path = %path,
                client_id = %client_id,
                retry_after_ms = %retry_after_ms,
                "Rate limit exceeded"
            );
        }
    }
}

/// Rate limit events for logging.
#[derive(Debug, Clone)]
pub enum RateLimitLogEvent {
    /// Request was allowed.
    Allowed {
        /// Remaining requests in the current window.
        remaining: u32,
    },
    /// Request was rate limited.
    Limited {
        /// Time in milliseconds until the rate limit resets.
        retry_after_ms: u64,
    },
}

/// Log an authentication event.
pub fn log_auth_event(request_id: &str, path: &str, event: AuthLogEvent) {
    match event {
        AuthLogEvent::Authenticated { user_id } => {
            tracing::debug!(
                request_id = %request_id,
                path = %path,
                user_id = %user_id,
                "User authenticated"
            );
        }
        AuthLogEvent::Unauthenticated => {
            tracing::debug!(
                request_id = %request_id,
                path = %path,
                "Authentication required but not provided"
            );
        }
        AuthLogEvent::Authorized { user_id } => {
            tracing::trace!(
                request_id = %request_id,
                path = %path,
                user_id = %user_id,
                "User authorized"
            );
        }
        AuthLogEvent::Forbidden {
            user_id,
            required_roles,
        } => {
            tracing::warn!(
                request_id = %request_id,
                path = %path,
                user_id = %user_id,
                required_roles = ?required_roles,
                "Access forbidden - insufficient roles"
            );
        }
    }
}

/// Authentication events for logging.
#[derive(Debug, Clone)]
pub enum AuthLogEvent {
    /// User was authenticated.
    Authenticated {
        /// The authenticated user's ID.
        user_id: String,
    },
    /// Request was unauthenticated.
    Unauthenticated,
    /// User was authorized.
    Authorized {
        /// The authorized user's ID.
        user_id: String,
    },
    /// User was forbidden (authenticated but lacks roles).
    Forbidden {
        /// The user's ID.
        user_id: String,
        /// The roles that were required.
        required_roles: Vec<String>,
    },
}

// =============================================================================
// Plugin Lifecycle Logging
// =============================================================================

/// Log plugin initialization.
pub fn log_plugin_init(config_summary: &str) {
    tracing::info!(
        config = %config_summary,
        "RPC plugin initialized"
    );
}

/// Log plugin shutdown.
pub fn log_plugin_shutdown(active_subscriptions: usize) {
    tracing::info!(
        active_subscriptions = %active_subscriptions,
        "RPC plugin shutting down"
    );
}

/// Log router compilation.
pub fn log_router_compiled(procedure_count: usize, subscription_count: usize) {
    tracing::debug!(
        procedure_count = %procedure_count,
        subscription_count = %subscription_count,
        "Router compiled"
    );
}

/// Log procedure registration.
pub fn log_procedure_registered(path: &str, procedure_type: &str) {
    tracing::trace!(
        path = %path,
        procedure_type = %procedure_type,
        "Procedure registered"
    );
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_request_id_uniqueness() {
        let ids: Vec<RequestId> = (0..100).map(|_| RequestId::new()).collect();
        let unique: HashSet<_> = ids.iter().map(|id| id.to_string()).collect();
        assert_eq!(ids.len(), unique.len());
    }

    #[test]
    fn test_request_id_from_string() {
        let id = RequestId::from_string("550e8400-e29b-41d4-a716-446655440000");
        assert_eq!(id.to_string(), "550e8400-e29b-41d4-a716-446655440000");
    }

    #[test]
    fn test_request_id_display() {
        let id = RequestId::from_string("550e8400-e29b-41d4-a716-446655440000");
        assert_eq!(format!("{}", id), "550e8400-e29b-41d4-a716-446655440000");
    }

    #[test]
    fn test_request_id_short() {
        let id = RequestId::from_string("12345678-1234-4567-89ab-123456789abc");
        assert_eq!(id.short(), "12345678");
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
        let parent = RequestId::from_string("550e8400-e29b-41d4-a716-446655440000");
        let meta = RequestMeta::new("test", ProcedureType::Query).with_parent_request_id(parent);
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
        assert!(entry.duration_us.is_some());
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
    fn test_log_config_procedure_level() {
        let config = LogConfig::new()
            .with_level(LogLevel::Info)
            .with_procedure_level("debug.test", LogLevel::Debug);

        assert_eq!(config.get_level_for_path("normal"), LogLevel::Info);
        assert_eq!(config.get_level_for_path("debug.test"), LogLevel::Debug);
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
        assert_eq!(redacted["user"]["auth"], "[REDACTED]");
    }

    #[test]
    fn test_redact_value_nested_deep() {
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

    #[test]
    fn test_tracing_config_defaults() {
        let config = TracingConfig::default();
        assert!(config.create_spans);
        assert!(!config.record_input);
        assert!(!config.record_output);
        assert_eq!(config.max_attribute_size, 1024);
        assert_eq!(config.service_name, "tauri-rpc");
    }

    #[test]
    fn test_tracing_config_builder() {
        let config = TracingConfig::new()
            .with_spans(false)
            .with_input_recording(true)
            .with_output_recording(true)
            .with_max_attribute_size(2048)
            .with_service_name("my-app");

        assert!(!config.create_spans);
        assert!(config.record_input);
        assert!(config.record_output);
        assert_eq!(config.max_attribute_size, 2048);
        assert_eq!(config.service_name, "my-app");
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
            let unique: std::collections::HashSet<_> = ids.iter().copied().collect();
            prop_assert_eq!(ids.len(), unique.len());
        }

        /// Property: Request IDs should be valid UUIDs.
        #[test]
        fn prop_request_id_is_valid_uuid(_seed in 0u64..1000) {
            let id = RequestId::new();
            // The UUID is already valid internally, just verify it can be converted to string
            let uuid_str = id.to_string();
            let parsed = uuid::Uuid::parse_str(&uuid_str);
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

        /// Property: Procedure-specific log levels should override global level.
        #[test]
        fn prop_procedure_level_overrides_global(
            path in "[a-z]{3,10}",
            global_level_idx in 0usize..5,
            proc_level_idx in 0usize..5
        ) {
            let levels = [LogLevel::Trace, LogLevel::Debug, LogLevel::Info, LogLevel::Warn, LogLevel::Error];
            let global_level = levels[global_level_idx];
            let proc_level = levels[proc_level_idx];

            let config = LogConfig::new()
                .with_level(global_level)
                .with_procedure_level(path.clone(), proc_level);

            prop_assert_eq!(config.get_level_for_path(&path), proc_level);
            prop_assert_eq!(config.get_level_for_path("other"), global_level);
        }
    }
}
