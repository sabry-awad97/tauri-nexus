//! Core types for the logging module.
//!
//! This module defines the fundamental types used throughout the logging system:
//! - RequestId: Unique identifier for request correlation
//! - LogLevel: Configurable log levels with ordering
//! - RequestMeta: Metadata about an RPC request
//! - LogEntry: Complete log entry with all request/response data
//!
//! All types use the builder pattern for ergonomic construction and are
//! designed to be serializable for structured logging.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

use super::constants::SHORT_ID_LENGTH;
use crate::middleware::ProcedureType;

/// Unique identifier for a request, used for tracing and correlation.
///
/// Uses UUID v7 for time-ordered, sortable identifiers that are ideal
/// for distributed tracing and log correlation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RequestId(uuid::Uuid);

impl RequestId {
    /// Creates a new unique request ID using UUID v7.
    ///
    /// UUID v7 includes a timestamp component, making IDs sortable by creation time.
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext)))
    }

    /// Returns the short form of the request ID (first N characters).
    ///
    /// Useful for compact log output while maintaining uniqueness within
    /// a reasonable time window.
    pub fn short(&self) -> String {
        let full = self.0.to_string();
        full.chars().take(SHORT_ID_LENGTH).collect()
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

impl std::str::FromStr for RequestId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        uuid::Uuid::parse_str(s).map(Self)
    }
}

impl TryFrom<&str> for RequestId {
    type Error = uuid::Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        s.parse()
    }
}

impl TryFrom<String> for RequestId {
    type Error = uuid::Error;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        s.parse()
    }
}

impl AsRef<uuid::Uuid> for RequestId {
    fn as_ref(&self) -> &uuid::Uuid {
        &self.0
    }
}

/// Log level for RPC logging.
///
/// Controls the verbosity of logging output. Levels are ordered from
/// most verbose (Trace) to least verbose (Off).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    /// Most verbose level, logs everything including detailed traces.
    Trace,
    /// Debug information useful for development.
    Debug,
    /// General informational messages (default).
    #[default]
    Info,
    /// Warning messages for potentially problematic situations.
    Warn,
    /// Error messages for failures.
    Error,
    /// Logging disabled.
    Off,
}

impl LogLevel {
    /// Checks if this log level should log messages at the target level.
    ///
    /// Returns true if the target level is equal to or more severe than this level.
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

    /// Converts this log level to a tracing::Level.
    ///
    /// Returns None for LogLevel::Off since tracing doesn't have an "off" level.
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

/// Metadata about an RPC request for logging and tracing.
///
/// This struct captures all relevant information about an RPC request
/// including identifiers, timing, and distributed tracing context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestMeta {
    /// Unique identifier for this request.
    pub request_id: RequestId,
    /// RPC procedure path (e.g., "users.get").
    pub path: String,
    /// Type of procedure (Query, Mutation, or Subscription).
    pub procedure_type: ProcedureType,
    /// Unix timestamp in milliseconds when the request was created.
    pub timestamp: u64,
    /// Optional client identifier for tracking requests by client.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
    /// Optional parent request ID for nested/chained requests.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_request_id: Option<RequestId>,
    /// Optional distributed tracing trace ID (OpenTelemetry compatible).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    /// Optional distributed tracing span ID (OpenTelemetry compatible).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span_id: Option<String>,
}

impl RequestMeta {
    /// Creates new request metadata with the given path and procedure type.
    ///
    /// Automatically generates a new request ID and captures the current timestamp.
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

    /// Sets the client identifier for this request.
    pub fn with_client_id(mut self, client_id: impl Into<String>) -> Self {
        self.client_id = Some(client_id.into());
        self
    }

    /// Sets the parent request ID for nested/chained requests.
    pub fn with_parent_request_id(mut self, parent_id: RequestId) -> Self {
        self.parent_request_id = Some(parent_id);
        self
    }

    /// Sets the distributed tracing trace ID.
    pub fn with_trace_id(mut self, trace_id: impl Into<String>) -> Self {
        self.trace_id = Some(trace_id.into());
        self
    }

    /// Sets the distributed tracing span ID.
    pub fn with_span_id(mut self, span_id: impl Into<String>) -> Self {
        self.span_id = Some(span_id.into());
        self
    }
}

/// A structured log entry for an RPC request/response.
///
/// This struct captures all relevant information about an RPC request
/// and its response, including timing, input/output data, errors, and
/// additional metadata like cache hits and rate limiting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    /// Request metadata (ID, path, type, timestamp, etc.).
    pub meta: RequestMeta,
    /// Request duration in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    /// Request duration in microseconds (for high-precision timing).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_us: Option<u64>,
    /// Whether the request completed successfully.
    pub success: bool,
    /// Error code if the request failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    /// Error message if the request failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    /// Request input (potentially redacted).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<Value>,
    /// Response output (potentially redacted).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<Value>,
    /// Size of the request input in bytes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_size: Option<usize>,
    /// Size of the response output in bytes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_size: Option<usize>,
    /// Whether the response was served from cache.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_hit: Option<bool>,
    /// Remaining rate limit quota for the client.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit_remaining: Option<u32>,
}

impl LogEntry {
    /// Creates a new log entry with the given request metadata.
    ///
    /// All optional fields are initialized to None, and success is set to true.
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

    /// Sets the request duration.
    ///
    /// Automatically calculates both milliseconds and microseconds.
    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.duration_ms = Some(duration.as_millis() as u64);
        self.duration_us = Some(duration.as_micros() as u64);
        self
    }

    /// Marks the request as failed with the given error code and message.
    ///
    /// Automatically sets success to false.
    pub fn with_error(mut self, code: impl Into<String>, message: impl Into<String>) -> Self {
        self.success = false;
        self.error_code = Some(code.into());
        self.error_message = Some(message.into());
        self
    }

    /// Sets the request input and calculates its size.
    pub fn with_input(mut self, input: Value) -> Self {
        self.input_size = Some(serde_json::to_vec(&input).map(|v| v.len()).unwrap_or(0));
        self.input = Some(input);
        self
    }

    /// Sets the response output and calculates its size.
    pub fn with_output(mut self, output: Value) -> Self {
        self.output_size = Some(serde_json::to_vec(&output).map(|v| v.len()).unwrap_or(0));
        self.output = Some(output);
        self
    }

    /// Sets whether the response was served from cache.
    pub fn with_cache_hit(mut self, hit: bool) -> Self {
        self.cache_hit = Some(hit);
        self
    }

    /// Sets the remaining rate limit quota for the client.
    pub fn with_rate_limit_remaining(mut self, remaining: u32) -> Self {
        self.rate_limit_remaining = Some(remaining);
        self
    }
}
