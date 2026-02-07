//! Event types and specialized logging functions for RPC events.
//!
//! This module provides event enums and logging functions for various
//! RPC-related events like subscriptions, caching, rate limiting, and
//! authentication.

// =============================================================================
// Subscription Events
// =============================================================================

/// Subscription lifecycle events for logging.
///
/// These events track the lifecycle of a subscription from start to completion
/// or cancellation, including any events emitted and errors encountered.
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

/// Log a subscription lifecycle event.
///
/// This function logs subscription events at appropriate levels:
/// - Started/Cancelled/Completed: Info level
/// - EventEmitted: Trace level
/// - Error: Warn level
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

// =============================================================================
// Batch Request Logging
// =============================================================================

/// Log a batch request.
///
/// This function logs the completion of a batch request with statistics
/// about the batch size, success/error counts, and total duration.
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

// =============================================================================
// Cache Events
// =============================================================================

/// Cache events for logging.
///
/// These events track cache operations including hits, misses, sets,
/// invalidations, and expirations.
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

/// Log a cache event.
///
/// This function logs cache events at appropriate levels:
/// - Hit/Miss/Invalidated: Debug level
/// - Set/Expired: Trace level
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

// =============================================================================
// Rate Limit Events
// =============================================================================

/// Rate limit events for logging.
///
/// These events track rate limiting decisions including allowed requests
/// and rate limit violations.
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

/// Log a rate limit event.
///
/// This function logs rate limit events at appropriate levels:
/// - Allowed: Trace level
/// - Limited: Warn level
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

// =============================================================================
// Authentication Events
// =============================================================================

/// Authentication events for logging.
///
/// These events track authentication and authorization decisions including
/// successful authentication, missing authentication, authorization checks,
/// and access denials.
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

/// Log an authentication event.
///
/// This function logs authentication events at appropriate levels:
/// - Authenticated/Unauthenticated: Debug level
/// - Authorized: Trace level
/// - Forbidden: Warn level
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
