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

/// Type-safe error codes for RPC operations.
///
/// These codes categorize errors into client errors (similar to HTTP 4xx),
/// server errors (similar to HTTP 5xx), and RPC-specific errors.
///
/// When serialized to JSON, codes are converted to SCREAMING_SNAKE_CASE
/// (e.g., `NotFound` becomes `"NOT_FOUND"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
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
#[derive(Debug, Clone, Serialize, Deserialize, Error)]
#[error("[{code}] {message}")]
pub struct RpcError {
    /// Type-safe error code
    pub code: RpcErrorCode,
    /// Human-readable error message
    pub message: String,
    /// Optional additional details (JSON value)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    /// Optional cause for debugging (not exposed to clients in production)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cause: Option<String>,
}

impl RpcError {
    /// Create a new error with code and message.
    pub fn new(code: RpcErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            details: None,
            cause: None,
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

    /// Sanitize error for client response (removes internal details for server errors).
    pub fn sanitize(mut self) -> Self {
        if matches!(self.code, RpcErrorCode::InternalError) {
            self.message = "An internal error occurred".to_string();
            self.details = None;
            self.cause = None;
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

/// Result type alias for RPC operations.
pub type RpcResult<T> = Result<T, RpcError>;
