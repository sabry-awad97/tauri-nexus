//! Error types for RPC operations
//!
//! This module provides type-safe error handling for the RPC plugin.
//!
//! # Error Codes
//!
//! Error codes are represented by the [`RpcErrorCode`] enum, which provides
//! exhaustive variants for common error scenarios. When serialized, codes
//! are converted to SCREAMING_SNAKE_CASE strings for compatibility.
//!
//! # Example
//! ```rust,ignore
//! use tauri_plugin_rpc::{RpcError, RpcErrorCode};
//!
//! let error = RpcError::new(RpcErrorCode::NotFound, "User not found");
//! let error = RpcError::not_found("User not found"); // Convenience method
//! ```

use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;
use tracing::{debug, trace};

/// Type-safe error codes for RPC operations.
///
/// These codes categorize errors into client errors (similar to HTTP 4xx),
/// server errors (similar to HTTP 5xx), and RPC-specific errors.
///
/// When serialized to JSON, codes are converted to SCREAMING_SNAKE_CASE
/// (e.g., `NotFound` becomes `"NOT_FOUND"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
pub enum RpcErrorCode {
    // Client errors (4xx equivalent)
    /// The request was malformed or invalid
    BadRequest,
    /// Authentication is required
    Unauthorized,
    /// The authenticated user lacks permission
    Forbidden,
    /// The requested resource was not found
    NotFound,
    /// Input validation failed
    ValidationError,
    /// The request conflicts with current state
    Conflict,
    /// The request payload exceeds size limits
    PayloadTooLarge,
    /// Too many requests - rate limit exceeded
    RateLimited,

    // Server errors (5xx equivalent)
    /// An unexpected internal error occurred
    InternalError,
    /// The requested functionality is not implemented
    NotImplemented,
    /// The service is temporarily unavailable
    ServiceUnavailable,

    // RPC-specific errors
    /// The requested procedure was not found
    ProcedureNotFound,
    /// An error occurred in subscription handling
    SubscriptionError,
    /// An error occurred in middleware execution
    MiddlewareError,
    /// JSON serialization/deserialization failed
    SerializationError,
}

impl RpcErrorCode {
    /// Returns the string representation of the error code.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::BadRequest => "BAD_REQUEST",
            Self::Unauthorized => "UNAUTHORIZED",
            Self::Forbidden => "FORBIDDEN",
            Self::NotFound => "NOT_FOUND",
            Self::ValidationError => "VALIDATION_ERROR",
            Self::Conflict => "CONFLICT",
            Self::PayloadTooLarge => "PAYLOAD_TOO_LARGE",
            Self::RateLimited => "RATE_LIMITED",
            Self::InternalError => "INTERNAL_ERROR",
            Self::NotImplemented => "NOT_IMPLEMENTED",
            Self::ServiceUnavailable => "SERVICE_UNAVAILABLE",
            Self::ProcedureNotFound => "PROCEDURE_NOT_FOUND",
            Self::SubscriptionError => "SUBSCRIPTION_ERROR",
            Self::MiddlewareError => "MIDDLEWARE_ERROR",
            Self::SerializationError => "SERIALIZATION_ERROR",
        }
    }

    /// Returns true if this is a client error (4xx equivalent).
    pub fn is_client_error(&self) -> bool {
        matches!(
            self,
            Self::BadRequest
                | Self::Unauthorized
                | Self::Forbidden
                | Self::NotFound
                | Self::ValidationError
                | Self::Conflict
                | Self::PayloadTooLarge
                | Self::RateLimited
        )
    }

    /// Returns true if this is a server error (5xx equivalent).
    pub fn is_server_error(&self) -> bool {
        matches!(
            self,
            Self::InternalError | Self::NotImplemented | Self::ServiceUnavailable
        )
    }
}

impl fmt::Display for RpcErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// RPC error with type-safe code and message.
///
/// This struct represents errors that occur during RPC operations.
/// It uses [`RpcErrorCode`] for type-safe error categorization.
///
/// # Example
/// ```rust,ignore
/// use tauri_plugin_rpc::{RpcError, RpcErrorCode};
///
/// // Create with code and message
/// let error = RpcError::new(RpcErrorCode::NotFound, "User not found");
///
/// // Add optional details
/// let error = error.with_details(serde_json::json!({"user_id": 123}));
///
/// // Add cause for debugging
/// let error = error.with_cause("Database query returned empty result");
/// ```
#[derive(Debug, Clone, Deserialize, Error)]
#[error("[{code}] {message}")]
pub struct RpcError {
    /// Type-safe error code
    pub code: RpcErrorCode,
    /// Human-readable error message
    pub message: String,
    /// Optional additional details (JSON value)
    pub details: Option<serde_json::Value>,
    /// Optional cause for debugging (not exposed to clients in production)
    pub cause: Option<String>,
    /// Optional stack trace (only included in development mode)
    pub stack_trace: Option<String>,
}

impl RpcError {
    /// Create a new error with code and message.
    pub fn new(code: RpcErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            details: None,
            cause: None,
            stack_trace: None,
        }
    }

    /// Add details to the error.
    pub fn with_details(mut self, details: impl Serialize) -> Self {
        self.details = serde_json::to_value(details).ok();
        self
    }

    /// Add a cause string for debugging.
    pub fn with_cause(mut self, cause: impl Into<String>) -> Self {
        self.cause = Some(cause.into());
        self
    }

    /// Add a stack trace for debugging (only shown in development mode).
    pub fn with_stack_trace(mut self, stack_trace: impl Into<String>) -> Self {
        self.stack_trace = Some(stack_trace.into());
        self
    }

    /// Capture the current stack trace (only in debug builds).
    #[cfg(debug_assertions)]
    pub fn capture_stack_trace(mut self) -> Self {
        self.stack_trace = Some(format!("{:?}", std::backtrace::Backtrace::capture()));
        self
    }

    /// Capture the current stack trace (no-op in release builds).
    #[cfg(not(debug_assertions))]
    pub fn capture_stack_trace(self) -> Self {
        self
    }

    /// Sanitize error for client response (removes internal details for server errors).
    pub fn sanitize(mut self) -> Self {
        if matches!(self.code, RpcErrorCode::InternalError) {
            debug!(
                original_message = %self.message,
                "Sanitizing internal error for client response"
            );
            self.message = "An internal error occurred".to_string();
            self.details = None;
            self.cause = None;
            self.stack_trace = None;
        }
        self
    }

    /// Apply error configuration to prepare error for client response.
    pub fn apply_config(mut self, config: &ErrorConfig) -> Self {
        trace!(
            code = %self.code,
            development_mode = config.development_mode,
            has_transformer = config.transformer.is_some(),
            "Applying error configuration"
        );

        // Remove stack trace in production mode
        if !config.development_mode {
            if self.stack_trace.is_some() || self.cause.is_some() {
                trace!("Removing debug info (stack_trace, cause) for production mode");
            }
            self.stack_trace = None;
            self.cause = None;
        }

        // Sanitize internal errors in production
        if !config.development_mode && self.code.is_server_error() {
            debug!(
                original_code = %self.code,
                original_message = %self.message,
                "Sanitizing server error for production"
            );
            self.message = "An internal error occurred".to_string();
            self.details = None;
        }

        // Apply custom transformer if configured
        if let Some(transformer) = &config.transformer {
            trace!("Applying custom error transformer");
            self = transformer.transform(self);
        }

        self
    }

    // Convenience constructors

    /// Create a NOT_FOUND error.
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(RpcErrorCode::NotFound, message)
    }

    /// Create a BAD_REQUEST error.
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(RpcErrorCode::BadRequest, message)
    }

    /// Create a VALIDATION_ERROR error.
    pub fn validation(message: impl Into<String>) -> Self {
        Self::new(RpcErrorCode::ValidationError, message)
    }

    /// Create an UNAUTHORIZED error.
    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::new(RpcErrorCode::Unauthorized, message)
    }

    /// Create a FORBIDDEN error.
    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::new(RpcErrorCode::Forbidden, message)
    }

    /// Create an INTERNAL_ERROR error.
    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(RpcErrorCode::InternalError, message)
    }

    /// Create a CONFLICT error.
    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new(RpcErrorCode::Conflict, message)
    }

    /// Create a PROCEDURE_NOT_FOUND error.
    pub fn procedure_not_found(path: &str) -> Self {
        Self::new(
            RpcErrorCode::ProcedureNotFound,
            format!("Procedure '{}' not found", path),
        )
    }

    /// Create a PAYLOAD_TOO_LARGE error.
    pub fn payload_too_large(message: impl Into<String>) -> Self {
        Self::new(RpcErrorCode::PayloadTooLarge, message)
    }

    /// Create a SERIALIZATION_ERROR error.
    pub fn serialization(message: impl Into<String>) -> Self {
        Self::new(RpcErrorCode::SerializationError, message)
    }

    /// Create a MIDDLEWARE_ERROR error.
    pub fn middleware(message: impl Into<String>) -> Self {
        Self::new(RpcErrorCode::MiddlewareError, message)
    }

    /// Create a SUBSCRIPTION_ERROR error.
    pub fn subscription(message: impl Into<String>) -> Self {
        Self::new(RpcErrorCode::SubscriptionError, message)
    }

    /// Create a SERVICE_UNAVAILABLE error.
    pub fn service_unavailable(message: impl Into<String>) -> Self {
        Self::new(RpcErrorCode::ServiceUnavailable, message)
    }

    /// Create a RATE_LIMITED error.
    pub fn rate_limited(message: impl Into<String>) -> Self {
        Self::new(RpcErrorCode::RateLimited, message)
    }
}

impl From<serde_json::Error> for RpcError {
    fn from(err: serde_json::Error) -> Self {
        Self::serialization(format!("JSON error: {}", err))
    }
}

impl From<std::io::Error> for RpcError {
    fn from(err: std::io::Error) -> Self {
        Self::internal(format!("IO error: {}", err))
    }
}

impl Serialize for RpcError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;

        let mut state = serializer.serialize_struct("RpcError", 5)?;
        state.serialize_field("code", &self.code)?;
        state.serialize_field("message", &self.message)?;

        if let Some(ref details) = self.details {
            state.serialize_field("details", details)?;
        }

        if let Some(ref cause) = self.cause {
            state.serialize_field("cause", cause)?;
        }

        if let Some(ref stack_trace) = self.stack_trace {
            state.serialize_field("stack_trace", stack_trace)?;
        }

        state.end()
    }
}

/// Result type alias for RPC operations.
pub type RpcResult<T> = Result<T, RpcError>;

// =============================================================================
// Error Configuration
// =============================================================================

/// Configuration for error handling behavior.
///
/// Controls how errors are processed before being sent to clients,
/// including development mode features and custom transformations.
///
/// # Example
/// ```rust,ignore
/// use tauri_plugin_rpc::error::{ErrorConfig, ErrorMode};
///
/// // Development mode - include stack traces
/// let dev_config = ErrorConfig::development();
///
/// // Production mode - sanitize errors
/// let prod_config = ErrorConfig::production();
///
/// // Custom configuration
/// let config = ErrorConfig::new()
///     .with_development_mode(cfg!(debug_assertions))
///     .with_transformer(MyErrorTransformer);
/// ```
#[derive(Clone)]
pub struct ErrorConfig {
    /// Whether to include development-only information (stack traces, causes)
    pub development_mode: bool,
    /// Custom error transformer
    pub transformer: Option<std::sync::Arc<dyn ErrorTransformer>>,
}

impl ErrorConfig {
    /// Create a new error configuration with default settings.
    pub fn new() -> Self {
        Self {
            development_mode: cfg!(debug_assertions),
            transformer: None,
        }
    }

    /// Create a development mode configuration.
    pub fn development() -> Self {
        Self {
            development_mode: true,
            transformer: None,
        }
    }

    /// Create a production mode configuration.
    pub fn production() -> Self {
        Self {
            development_mode: false,
            transformer: None,
        }
    }

    /// Set development mode.
    pub fn with_development_mode(mut self, enabled: bool) -> Self {
        self.development_mode = enabled;
        self
    }

    /// Set a custom error transformer.
    pub fn with_transformer<T: ErrorTransformer + 'static>(mut self, transformer: T) -> Self {
        self.transformer = Some(std::sync::Arc::new(transformer));
        self
    }
}

impl Default for ErrorConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for ErrorConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ErrorConfig")
            .field("development_mode", &self.development_mode)
            .field("transformer", &self.transformer.is_some())
            .finish()
    }
}

// =============================================================================
// Error Transformer
// =============================================================================

/// Trait for custom error transformation.
///
/// Implement this trait to customize how errors are processed before
/// being sent to clients. This can be used for logging, metrics,
/// error mapping, or adding additional context.
///
/// # Example
/// ```rust,ignore
/// use tauri_plugin_rpc::error::{ErrorTransformer, RpcError};
///
/// struct LoggingTransformer;
///
/// impl ErrorTransformer for LoggingTransformer {
///     fn transform(&self, error: RpcError) -> RpcError {
///         eprintln!("RPC Error: [{:?}] {}", error.code, error.message);
///         error
///     }
/// }
/// ```
pub trait ErrorTransformer: Send + Sync {
    /// Transform an error before it's sent to the client.
    fn transform(&self, error: RpcError) -> RpcError;
}

/// A no-op error transformer that passes errors through unchanged.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoOpTransformer;

impl ErrorTransformer for NoOpTransformer {
    fn transform(&self, error: RpcError) -> RpcError {
        error
    }
}

/// An error transformer that logs errors before passing them through.
#[derive(Debug, Clone, Copy, Default)]
pub struct LoggingTransformer;

impl ErrorTransformer for LoggingTransformer {
    fn transform(&self, error: RpcError) -> RpcError {
        tracing::error!(
            code = %error.code,
            message = %error.message,
            "RPC error occurred"
        );
        error
    }
}

/// An error transformer that maps specific error codes to different codes.
pub struct ErrorCodeMapper {
    mappings: std::collections::HashMap<RpcErrorCode, RpcErrorCode>,
}

impl ErrorCodeMapper {
    /// Create a new error code mapper.
    pub fn new() -> Self {
        Self {
            mappings: std::collections::HashMap::new(),
        }
    }

    /// Add a mapping from one error code to another.
    pub fn map(mut self, from: RpcErrorCode, to: RpcErrorCode) -> Self {
        self.mappings.insert(from, to);
        self
    }
}

impl Default for ErrorCodeMapper {
    fn default() -> Self {
        Self::new()
    }
}

impl ErrorTransformer for ErrorCodeMapper {
    fn transform(&self, mut error: RpcError) -> RpcError {
        if let Some(&new_code) = self.mappings.get(&error.code) {
            debug!(
                from_code = %error.code,
                to_code = %new_code,
                "Mapping error code"
            );
            error.code = new_code;
        }
        error
    }
}

/// Compose multiple error transformers into a single transformer.
pub struct ComposedTransformer {
    transformers: Vec<std::sync::Arc<dyn ErrorTransformer>>,
}

impl ComposedTransformer {
    /// Create a new composed transformer.
    pub fn new() -> Self {
        Self {
            transformers: Vec::new(),
        }
    }

    /// Add a transformer to the composition.
    pub fn with_transformer<T: ErrorTransformer + 'static>(mut self, transformer: T) -> Self {
        self.transformers.push(std::sync::Arc::new(transformer));
        self
    }
}

impl Default for ComposedTransformer {
    fn default() -> Self {
        Self::new()
    }
}

impl ErrorTransformer for ComposedTransformer {
    fn transform(&self, mut error: RpcError) -> RpcError {
        trace!(
            transformer_count = self.transformers.len(),
            "Applying composed error transformers"
        );
        for (i, transformer) in self.transformers.iter().enumerate() {
            trace!(transformer_index = i, "Applying transformer");
            error = transformer.transform(error);
        }
        error
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_config_development() {
        let config = ErrorConfig::development();
        assert!(config.development_mode);
    }

    #[test]
    fn test_error_config_production() {
        let config = ErrorConfig::production();
        assert!(!config.development_mode);
    }

    #[test]
    fn test_error_apply_config_production_removes_stack_trace() {
        let error = RpcError::internal("test error")
            .with_stack_trace("stack trace here")
            .with_cause("some cause");

        let config = ErrorConfig::production();
        let sanitized = error.apply_config(&config);

        assert!(sanitized.stack_trace.is_none());
        assert!(sanitized.cause.is_none());
        assert_eq!(sanitized.message, "An internal error occurred");
    }

    #[test]
    fn test_error_apply_config_development_keeps_stack_trace() {
        let error = RpcError::internal("test error")
            .with_stack_trace("stack trace here")
            .with_cause("some cause");

        let config = ErrorConfig::development();
        let result = error.apply_config(&config);

        assert!(result.stack_trace.is_some());
        assert!(result.cause.is_some());
        assert_eq!(result.message, "test error");
    }

    #[test]
    fn test_error_apply_config_client_error_not_sanitized() {
        let error = RpcError::not_found("User not found").with_cause("Database lookup failed");

        let config = ErrorConfig::production();
        let result = error.apply_config(&config);

        // Client errors keep their message
        assert_eq!(result.message, "User not found");
        // But cause is still removed in production
        assert!(result.cause.is_none());
    }

    #[test]
    fn test_noop_transformer() {
        let error = RpcError::not_found("test");
        let transformer = NoOpTransformer;
        let result = transformer.transform(error.clone());

        assert_eq!(result.code, error.code);
        assert_eq!(result.message, error.message);
    }

    #[test]
    fn test_error_code_mapper() {
        let error = RpcError::not_found("test");
        let mapper = ErrorCodeMapper::new().map(RpcErrorCode::NotFound, RpcErrorCode::BadRequest);

        let result = mapper.transform(error);
        assert_eq!(result.code, RpcErrorCode::BadRequest);
    }

    #[test]
    fn test_composed_transformer() {
        struct AddPrefixTransformer;
        impl ErrorTransformer for AddPrefixTransformer {
            fn transform(&self, mut error: RpcError) -> RpcError {
                error.message = format!("PREFIX: {}", error.message);
                error
            }
        }

        struct AddSuffixTransformer;
        impl ErrorTransformer for AddSuffixTransformer {
            fn transform(&self, mut error: RpcError) -> RpcError {
                error.message = format!("{} :SUFFIX", error.message);
                error
            }
        }

        let error = RpcError::not_found("test");
        let composed = ComposedTransformer::new()
            .with_transformer(AddPrefixTransformer)
            .with_transformer(AddSuffixTransformer);

        let result = composed.transform(error);
        assert_eq!(result.message, "PREFIX: test :SUFFIX");
    }

    #[test]
    fn test_error_with_stack_trace() {
        let error = RpcError::internal("test").with_stack_trace("at function_a\nat function_b");

        assert!(error.stack_trace.is_some());
        assert!(error.stack_trace.unwrap().contains("function_a"));
    }

    #[test]
    fn test_error_config_with_transformer() {
        struct TestTransformer;
        impl ErrorTransformer for TestTransformer {
            fn transform(&self, mut error: RpcError) -> RpcError {
                error.message = "transformed".to_string();
                error
            }
        }

        let error = RpcError::not_found("original");
        let config = ErrorConfig::development().with_transformer(TestTransformer);

        let result = error.apply_config(&config);
        assert_eq!(result.message, "transformed");
    }
}

// =============================================================================
// Property-Based Tests
// =============================================================================

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    // Strategy for generating error codes
    fn error_code_strategy() -> impl Strategy<Value = RpcErrorCode> {
        prop_oneof![
            Just(RpcErrorCode::BadRequest),
            Just(RpcErrorCode::Unauthorized),
            Just(RpcErrorCode::Forbidden),
            Just(RpcErrorCode::NotFound),
            Just(RpcErrorCode::ValidationError),
            Just(RpcErrorCode::Conflict),
            Just(RpcErrorCode::PayloadTooLarge),
            Just(RpcErrorCode::RateLimited),
            Just(RpcErrorCode::InternalError),
            Just(RpcErrorCode::NotImplemented),
            Just(RpcErrorCode::ServiceUnavailable),
            Just(RpcErrorCode::ProcedureNotFound),
            Just(RpcErrorCode::SubscriptionError),
            Just(RpcErrorCode::MiddlewareError),
            Just(RpcErrorCode::SerializationError),
        ]
    }

    // Strategy for generating error messages
    fn message_strategy() -> impl Strategy<Value = String> {
        "[a-zA-Z0-9 ]{1,50}".prop_map(|s| s)
    }

    proptest! {
        /// Property: Error serialization always produces valid JSON or fallback string
        #[test]
        fn prop_error_serialization_always_succeeds(
            code in error_code_strategy(),
            message in message_strategy(),
        ) {
            let error = RpcError::new(code, message);
            let serialized = serde_json::to_string(&error).unwrap_or_else(|_| error.to_string());

            // Should always produce a non-empty string
            prop_assert!(!serialized.is_empty());

            // Should be valid JSON or contain the error message
            prop_assert!(
                serde_json::from_str::<serde_json::Value>(&serialized).is_ok()
                || serialized.contains(&error.message)
            );
        }

        /// Property 7: Error Mode Behavior
        /// In production mode, server errors should be sanitized and stack traces removed.
        /// In development mode, all error information should be preserved.
        #[test]
        fn prop_production_mode_sanitizes_server_errors(
            code in error_code_strategy(),
            message in message_strategy(),
        ) {
            let error = RpcError::new(code, message.clone())
                .with_stack_trace("stack trace")
                .with_cause("some cause");

            let config = ErrorConfig::production();
            let result = error.apply_config(&config);

            // Stack trace and cause should always be removed in production
            prop_assert!(result.stack_trace.is_none(), "Stack trace should be removed in production");
            prop_assert!(result.cause.is_none(), "Cause should be removed in production");

            // Server errors should have sanitized message
            if code.is_server_error() {
                prop_assert_eq!(result.message, "An internal error occurred");
            } else {
                // Client errors keep their original message
                prop_assert_eq!(result.message, message);
            }
        }

        /// Property: Development mode preserves all error information
        #[test]
        fn prop_development_mode_preserves_all_info(
            code in error_code_strategy(),
            message in message_strategy(),
        ) {
            let stack_trace = "at function_a\nat function_b";
            let cause = "database connection failed";

            let error = RpcError::new(code, message.clone())
                .with_stack_trace(stack_trace)
                .with_cause(cause);

            let config = ErrorConfig::development();
            let result = error.apply_config(&config);

            // All information should be preserved in development mode
            prop_assert_eq!(result.message, message);
            prop_assert_eq!(result.stack_trace.as_deref(), Some(stack_trace));
            prop_assert_eq!(result.cause.as_deref(), Some(cause));
        }

        /// Property: Error code classification is consistent
        #[test]
        fn prop_error_code_classification_consistent(code in error_code_strategy()) {
            // An error code should be either client error, server error, or neither
            // but never both
            let is_client = code.is_client_error();
            let is_server = code.is_server_error();

            prop_assert!(
                !(is_client && is_server),
                "Error code cannot be both client and server error"
            );
        }

        /// Property: Error transformers are applied correctly
        #[test]
        fn prop_transformer_applied_in_config(
            code in error_code_strategy(),
            message in message_strategy(),
        ) {
            struct PrefixTransformer;
            impl ErrorTransformer for PrefixTransformer {
                fn transform(&self, mut error: RpcError) -> RpcError {
                    error.message = format!("TRANSFORMED: {}", error.message);
                    error
                }
            }

            let error = RpcError::new(code, message.clone());
            let config = ErrorConfig::development()
                .with_transformer(PrefixTransformer);

            let result = error.apply_config(&config);

            // For client errors, the transformer should be applied
            if code.is_client_error() {
                prop_assert!(result.message.starts_with("TRANSFORMED: "));
            }
        }

        /// Property: Composed transformers execute in order
        #[test]
        fn prop_composed_transformers_execute_in_order(
            message in message_strategy(),
        ) {
            struct AddATransformer;
            impl ErrorTransformer for AddATransformer {
                fn transform(&self, mut error: RpcError) -> RpcError {
                    error.message = format!("{}A", error.message);
                    error
                }
            }

            struct AddBTransformer;
            impl ErrorTransformer for AddBTransformer {
                fn transform(&self, mut error: RpcError) -> RpcError {
                    error.message = format!("{}B", error.message);
                    error
                }
            }

            let error = RpcError::not_found(message.clone());
            let composed = ComposedTransformer::new()
                .with_transformer(AddATransformer)
                .with_transformer(AddBTransformer);

            let result = composed.transform(error);

            // Should be message + A + B (in order)
            prop_assert_eq!(result.message, format!("{}AB", message));
        }
    }

    // Property: Error serialization never panics
    proptest! {
        #[test]
        fn prop_error_serialization_never_panics(
            message in ".*",
            _code_num in 400u16..600u16
        ) {
            // Create various error types
            let errors = vec![
                RpcError::not_found(&message),
                RpcError::bad_request(&message),
                RpcError::internal(&message),
                RpcError::validation(&message),
            ];

            for error in errors {
                // Should never panic
                let _ = serde_json::to_string(&error).unwrap_or_else(|_| error.to_string());
            }
        }
    }

    // Property 11: Error codes and messages are preserved
    #[test]
    fn test_error_codes_and_messages_preserved() {
        let test_cases = vec![
            (
                RpcError::not_found("User not found"),
                "NOT_FOUND",
                "User not found",
            ),
            (
                RpcError::bad_request("Invalid input"),
                "BAD_REQUEST",
                "Invalid input",
            ),
            (
                RpcError::validation("Email required"),
                "VALIDATION_ERROR",
                "Email required",
            ),
            (
                RpcError::internal("Server error"),
                "INTERNAL_ERROR",
                "Server error",
            ),
        ];

        for (error, expected_code, expected_message) in test_cases {
            let serialized = serde_json::to_string(&error).unwrap_or_else(|_| error.to_string());

            // Check if it's JSON
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&serialized) {
                // Verify code and message in JSON
                assert_eq!(json["code"].as_str().unwrap(), expected_code);
                assert_eq!(json["message"].as_str().unwrap(), expected_message);
            } else {
                // Fallback format should contain code and message
                assert!(
                    serialized.contains(expected_code) || serialized.contains(expected_message)
                );
            }
        }
    }

    // Unit test: Error serialization
    #[test]
    fn test_error_serialization() {
        // Test with a normal error (should use JSON)
        let error = RpcError::not_found("Test error");
        let serialized = serde_json::to_string(&error).unwrap();

        // Should be valid JSON
        assert!(serde_json::from_str::<serde_json::Value>(&serialized).is_ok());

        // Verify structure
        let json: serde_json::Value = serde_json::from_str(&serialized).unwrap();
        assert_eq!(json["code"], "NOT_FOUND");
        assert_eq!(json["message"], "Test error");
    }

    #[test]
    fn test_error_with_details() {
        let error = RpcError::validation("Invalid input")
            .with_details(serde_json::json!({"field": "email", "reason": "invalid format"}));

        let serialized = serde_json::to_string(&error).unwrap();

        // Should be valid JSON
        let json: serde_json::Value = serde_json::from_str(&serialized).unwrap();
        assert_eq!(json["code"], "VALIDATION_ERROR");
        assert_eq!(json["message"], "Invalid input");
        assert_eq!(json["details"]["field"], "email");
        assert_eq!(json["details"]["reason"], "invalid format");
    }
}
