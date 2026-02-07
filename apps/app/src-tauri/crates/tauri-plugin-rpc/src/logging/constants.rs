//! Constants and default values for the logging module.
//!
//! This module centralizes all magic constants and default values used
//! throughout the logging system. This makes it easy to adjust defaults
//! and ensures consistency across the codebase.

/// Length of the short request ID format (first N characters of UUID).
///
/// Short IDs are useful for compact log output while still maintaining
/// uniqueness for correlation within a reasonable time window.
pub const SHORT_ID_LENGTH: usize = 8;

/// Default maximum size for tracing span attributes in bytes.
///
/// Large attributes can impact tracing performance. This limit prevents
/// excessive memory usage and ensures efficient span processing.
pub const DEFAULT_MAX_ATTRIBUTE_SIZE: usize = 1024;

/// Default threshold for slow request logging in milliseconds.
///
/// Requests exceeding this duration will trigger a warning log.
/// Set to 0 to disable slow request logging.
pub const DEFAULT_SLOW_THRESHOLD_MS: u64 = 1000;

/// Default replacement string for redacted sensitive fields.
///
/// This string replaces sensitive values in logs to prevent data leakage
/// while maintaining log structure for debugging.
pub const DEFAULT_REDACTION_REPLACEMENT: &str = "[REDACTED]";

/// Default list of sensitive field names to redact.
///
/// These fields are commonly used for sensitive data and will be
/// automatically redacted unless explicitly configured otherwise.
/// Field matching is case-insensitive and uses substring matching.
pub const DEFAULT_SENSITIVE_FIELDS: &[&str] = &[
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
];
