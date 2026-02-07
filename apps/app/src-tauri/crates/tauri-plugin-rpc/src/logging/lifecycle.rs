//! Plugin lifecycle logging functions.
//!
//! This module provides logging functions for plugin lifecycle events
//! such as initialization, shutdown, router compilation, and procedure
//! registration.

// =============================================================================
// Plugin Lifecycle Logging
// =============================================================================

/// Log plugin initialization.
///
/// This function logs the initialization of the RPC plugin with a summary
/// of the configuration. Logged at Info level.
///
/// # Example
///
/// ```rust,ignore
/// log_plugin_init("LogLevel::Info, Tracing: enabled, Redaction: 5 fields");
/// ```
pub fn log_plugin_init(config_summary: &str) {
    tracing::info!(
        config = %config_summary,
        "RPC plugin initialized"
    );
}

/// Log plugin shutdown.
///
/// This function logs the shutdown of the RPC plugin with the count of
/// active subscriptions that will be terminated. Logged at Info level.
///
/// # Example
///
/// ```rust,ignore
/// log_plugin_shutdown(3);
/// ```
pub fn log_plugin_shutdown(active_subscriptions: usize) {
    tracing::info!(
        active_subscriptions = %active_subscriptions,
        "RPC plugin shutting down"
    );
}

/// Log router compilation.
///
/// This function logs the completion of router compilation with counts
/// of registered procedures and subscriptions. Logged at Debug level.
///
/// # Example
///
/// ```rust,ignore
/// log_router_compiled(15, 3);
/// ```
pub fn log_router_compiled(procedure_count: usize, subscription_count: usize) {
    tracing::debug!(
        procedure_count = %procedure_count,
        subscription_count = %subscription_count,
        "Router compiled"
    );
}

/// Log procedure registration.
///
/// This function logs the registration of a single procedure with its
/// path and type (query, mutation, or subscription). Logged at Trace level.
///
/// # Example
///
/// ```rust,ignore
/// log_procedure_registered("users.get", "query");
/// ```
pub fn log_procedure_registered(path: &str, procedure_type: &str) {
    tracing::trace!(
        path = %path,
        procedure_type = %procedure_type,
        "Procedure registered"
    );
}
