use crate::logging::types::{LogEntry, LogLevel, RequestMeta};
use async_trait::async_trait;

// =============================================================================
// Logger Trait
// =============================================================================

/// A logger that can be used to emit log entries.
///
/// This trait provides async methods for logging RPC requests and responses.
/// Default implementations are provided for optional hooks like slow request
/// logging and request start logging.
#[async_trait]
pub trait Logger: Send + Sync {
    /// Logs a request/response entry.
    async fn log(&self, entry: &LogEntry, level: LogLevel);

    /// Logs a slow request warning.
    ///
    /// Default implementation logs a warning with structured fields.
    async fn log_slow_request(&self, entry: &LogEntry, threshold_ms: u64) {
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
    ///
    /// Default implementation logs a debug message with request metadata.
    async fn log_request_start(&self, meta: &RequestMeta) {
        tracing::debug!(
            request_id = %meta.request_id,
            path = %meta.path,
            procedure_type = %meta.procedure_type,
            "RPC request started"
        );
    }
}

// =============================================================================
// TracingLogger
// =============================================================================

/// Default logger that uses the tracing crate with structured fields.
///
/// This logger emits structured log events at different levels based on
/// the request outcome and configured log level. It includes detailed
/// information like duration, input/output sizes, and cache hits.
#[derive(Debug, Clone, Default)]
pub struct TracingLogger;

#[async_trait]
impl Logger for TracingLogger {
    async fn log(&self, entry: &LogEntry, level: LogLevel) {
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
// JsonLogger
// =============================================================================

/// A logger that outputs JSON-formatted log entries.
///
/// This logger serializes the entire LogEntry to JSON and emits it as a
/// structured log event. Useful for log aggregation systems that expect
/// JSON-formatted logs.
#[derive(Debug, Clone, Default)]
pub struct JsonLogger;

#[async_trait]
impl Logger for JsonLogger {
    async fn log(&self, entry: &LogEntry, _level: LogLevel) {
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
// MetricsLogger
// =============================================================================

/// A logger that also records metrics for monitoring.
///
/// This logger delegates to TracingLogger for standard logging and additionally
/// emits metric events that can be captured by metrics layers. It records:
/// - Request counters (total requests, success/failure)
/// - Duration histograms
/// - Input/output size histograms
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

    /// Records metrics for a log entry.
    ///
    /// This is extracted as a separate method to eliminate duplicate logic
    /// and make it easier to test metrics recording independently.
    fn record_metrics(&self, entry: &LogEntry) {
        let duration_us = entry.duration_us.unwrap_or(0);
        let path = &entry.meta.path;
        let procedure_type = format!("{}", entry.meta.procedure_type);

        // Record request counter
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

        // Record duration histogram
        tracing::trace!(
            target: "rpc_metrics",
            metric_type = "histogram",
            metric_name = "rpc_request_duration_us",
            value = duration_us,
            path = %path,
            procedure_type = %procedure_type,
            "RPC duration metric"
        );

        // Record input size histogram
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

        // Record output size histogram
        if let Some(output_size) = entry.output_size {
            tracing::trace!(
                target: "rpc_metrics",
                metric_type = "histogram",
                metric_name = "rpc_request_output_bytes",
                value = output_size,
                path = %path,
                "RPC output size metric"
            );
        }

        // Record cache hit metric
        if let Some(cache_hit) = entry.cache_hit {
            tracing::trace!(
                target: "rpc_metrics",
                metric_type = "counter",
                metric_name = "rpc_cache_hits_total",
                value = if cache_hit { 1u64 } else { 0u64 },
                path = %path,
                "RPC cache hit metric"
            );
        }
    }
}

#[async_trait]
impl Logger for MetricsLogger {
    async fn log(&self, entry: &LogEntry, level: LogLevel) {
        // Log using the inner logger
        self.inner.log(entry, level).await;

        // Record metrics
        self.record_metrics(entry);
    }
}

// =============================================================================
// MockLogger (for testing)
// =============================================================================

#[cfg(test)]
use std::sync::{Arc, Mutex};

/// A mock logger that captures log entries for testing.
///
/// This logger stores all logged entries in memory and provides methods
/// to retrieve and clear them. Useful for unit tests that need to verify
/// logging behavior.
#[cfg(test)]
#[derive(Debug, Clone, Default)]
pub struct MockLogger {
    entries: Arc<Mutex<Vec<(LogEntry, LogLevel)>>>,
}

#[cfg(test)]
impl MockLogger {
    /// Creates a new mock logger.
    pub fn new() -> Self {
        Self {
            entries: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Returns all captured log entries.
    pub fn entries(&self) -> Vec<(LogEntry, LogLevel)> {
        self.entries.lock().unwrap().clone()
    }

    /// Clears all captured log entries.
    pub fn clear(&self) {
        self.entries.lock().unwrap().clear();
    }

    /// Returns the number of captured log entries.
    pub fn len(&self) -> usize {
        self.entries.lock().unwrap().len()
    }

    /// Returns true if no entries have been captured.
    pub fn is_empty(&self) -> bool {
        self.entries.lock().unwrap().is_empty()
    }
}

#[cfg(test)]
#[async_trait]
impl Logger for MockLogger {
    async fn log(&self, entry: &LogEntry, level: LogLevel) {
        self.entries.lock().unwrap().push((entry.clone(), level));
    }
}
