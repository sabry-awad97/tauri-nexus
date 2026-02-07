use crate::logging::config::LogConfig;
use crate::logging::logger::{Logger, TracingLogger};
use crate::logging::redaction::redact_value;
use crate::logging::types::{LogEntry, LogLevel, RequestMeta};
use crate::middleware::{MiddlewareFn, Request, from_fn};
use crate::{Context, Next, RpcError};
use serde_json::Value;
use std::sync::Arc;
use std::time::{Duration, Instant};

// =============================================================================
// Helper Functions
// =============================================================================

/// Serializes and redacts input in a single pass.
///
/// This function optimizes the common case where we need both the size and
/// the redacted value by serializing once and reusing the result.
///
/// Returns: (size, redacted_value)
/// - size: Some(len) if size logging is enabled, None otherwise
/// - redacted_value: Some(Value) if input logging is enabled, None otherwise
fn serialize_and_redact(
    input: &Value,
    config: &LogConfig,
    request_id: &str,
) -> (Option<usize>, Option<Value>) {
    // Early return if neither size nor input logging is enabled
    if !config.log_input && !config.log_sizes {
        return (None, None);
    }

    // Serialize once for both size and logging
    let serialized = match serde_json::to_vec(input) {
        Ok(vec) => vec,
        Err(e) => {
            tracing::warn!(
                request_id = %request_id,
                error = %e,
                "Failed to serialize request input for logging"
            );

            // Return placeholder values on error
            let size = if config.log_sizes { Some(0) } else { None };
            let redacted = if config.log_input {
                Some(Value::String("[serialization error]".to_string()))
            } else {
                None
            };
            return (size, redacted);
        }
    };

    let size = if config.log_sizes {
        Some(serialized.len())
    } else {
        None
    };

    let redacted = if config.log_input {
        Some(redact_value(input, config))
    } else {
        None
    };

    (size, redacted)
}

/// Adds response data to a log entry with error handling.
///
/// This function handles output serialization with proper error handling,
/// reusing serialization for both size and logging when possible.
fn add_response_to_entry(
    entry: &mut LogEntry,
    response: &Value,
    config: &LogConfig,
    request_id: &str,
) {
    // Early return if neither output nor size logging is enabled
    if !config.log_output && !config.log_sizes {
        return;
    }

    // If we need both output and size, serialize once
    if config.log_output && config.log_sizes {
        match serde_json::to_vec(response) {
            Ok(vec) => {
                entry.output_size = Some(vec.len());
                entry.output = Some(redact_value(response, config));
            }
            Err(e) => {
                tracing::warn!(
                    request_id = %request_id,
                    error = %e,
                    "Failed to serialize response output for logging"
                );
                entry.output_size = Some(0);
                entry.output = Some(Value::String("[serialization error]".to_string()));
            }
        }
    } else if config.log_output {
        // Only need redacted output
        entry.output = Some(redact_value(response, config));
    } else if config.log_sizes {
        // Only need size
        match serde_json::to_vec(response) {
            Ok(vec) => {
                entry.output_size = Some(vec.len());
            }
            Err(e) => {
                tracing::warn!(
                    request_id = %request_id,
                    error = %e,
                    "Failed to serialize response output for size calculation"
                );
                entry.output_size = Some(0);
            }
        }
    }
}

/// Executes the next middleware with an optional tracing span.
///
/// This function conditionally creates a tracing span based on configuration,
/// avoiding the overhead when spans are disabled.
async fn execute_with_optional_span<Ctx>(
    config: &LogConfig,
    request_id: &str,
    req: &Request,
    ctx: Context<Ctx>,
    next: Next<Ctx>,
) -> Result<Value, RpcError>
where
    Ctx: Clone + Send + Sync + 'static,
{
    if config
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
    }
}

/// Builds a log entry from request metadata and configuration.
///
/// This function consolidates entry construction logic to avoid duplication.
fn build_log_entry(
    meta: RequestMeta,
    config: &LogConfig,
    duration: Duration,
    input_size: Option<usize>,
    redacted_input: Option<Value>,
) -> LogEntry {
    let mut entry = LogEntry::new(meta);

    if config.log_timing {
        entry = entry.with_duration(duration);
    }

    if let Some(input) = redacted_input {
        entry = entry.with_input(input);
    } else if let Some(size) = input_size {
        entry.input_size = Some(size);
    }

    entry
}

/// Helper function to determine if a request should be logged as slow.
pub fn should_log_slow_request(config: &LogConfig, duration: &Duration) -> bool {
    config
        .slow_request_threshold_ms
        .map(|threshold| duration.as_millis() as u64 > threshold)
        .unwrap_or(false)
}

// =============================================================================
// Middleware Functions
// =============================================================================

/// Creates a logging middleware with the given configuration.
///
/// This middleware logs RPC requests and responses with structured fields,
/// timing information, and sensitive data redaction. It uses a single-pass
/// serialization approach to minimize overhead.
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
///
/// This allows you to use a different logger implementation (e.g., JsonLogger,
/// MetricsLogger) instead of the default TracingLogger.
///
/// # Example
///
/// ```rust,ignore
/// use tauri_plugin_rpc::logging::{LogConfig, JsonLogger, logging_middleware_with_logger};
///
/// let config = LogConfig::new().with_timing(true);
/// let logger = JsonLogger;
///
/// let router = Router::new()
///     .middleware(logging_middleware_with_logger(config, logger))
///     .query("users.get", get_user);
/// ```
pub fn logging_middleware_with_logger<Ctx, L>(config: LogConfig, logger: L) -> MiddlewareFn<Ctx>
where
    Ctx: Clone + Send + Sync + 'static,
    L: Logger + Clone + 'static,
{
    // Clone Arc once at the start to minimize cloning in the hot path
    let config = Arc::new(config);
    let logger = Arc::new(logger);

    let middleware = move |ctx: Context<Ctx>, req: Request, next: Next<Ctx>| {
        // Clone Arc references for this request
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
            let request_id_str = request_id.to_string();
            let effective_level = config.get_level_for_path(&req.path);

            // Log request start at debug level
            if effective_level.should_log(LogLevel::Debug) {
                logger.log_request_start(&meta).await;
            }

            let start = Instant::now();

            // Serialize and redact input once for both size and logging
            let (input_size, redacted_input) =
                serialize_and_redact(&req.input, &config, &request_id_str);

            // Execute the request with optional tracing span
            let result =
                execute_with_optional_span(&config, &request_id_str, &req, ctx, next).await;

            let duration = start.elapsed();

            // Build log entry with common fields
            let mut entry = build_log_entry(meta, &config, duration, input_size, redacted_input);

            match &result {
                Ok(response) => {
                    if config.log_success {
                        // Add response data with error handling
                        add_response_to_entry(&mut entry, response, &config, &request_id_str);

                        logger.log(&entry, effective_level).await;
                    }

                    // Check for slow request
                    if should_log_slow_request(&config, &duration) {
                        logger
                            .log_slow_request(&entry, config.slow_request_threshold_ms.unwrap())
                            .await;
                    }
                }
                Err(error) => {
                    if config.log_errors {
                        entry = entry.with_error(format!("{:?}", error.code), &error.message);
                        logger.log(&entry, LogLevel::Warn).await;
                    }
                }
            }

            result
        }
    };

    from_fn(middleware)
}
