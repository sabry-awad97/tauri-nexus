//! Event types for subscription streaming

use serde::{Deserialize, Serialize};

/// Event with optional metadata for streaming
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Event<T> {
    /// The event data
    pub data: T,
    /// Optional event ID for resumption
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Optional retry interval in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry: Option<u64>,
}

impl<T> Event<T> {
    /// Create a new event with just data
    pub fn new(data: T) -> Self {
        Self {
            data,
            id: None,
            retry: None,
        }
    }

    /// Create an event with an ID
    pub fn with_id(data: T, id: impl Into<String>) -> Self {
        Self {
            data,
            id: Some(id.into()),
            retry: None,
        }
    }

    /// Add metadata to an event
    pub fn with_meta(mut self, meta: EventMeta) -> Self {
        self.id = meta.id;
        self.retry = meta.retry;
        self
    }
}

/// Event metadata for SSE-style streaming
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventMeta {
    /// Event ID for resumption (Last-Event-ID)
    pub id: Option<String>,
    /// Retry interval in milliseconds
    pub retry: Option<u64>,
}

impl EventMeta {
    /// Create new metadata with an ID
    pub fn with_id(id: impl Into<String>) -> Self {
        Self {
            id: Some(id.into()),
            retry: None,
        }
    }

    /// Create metadata with retry interval
    pub fn with_retry(retry: u64) -> Self {
        Self {
            id: None,
            retry: Some(retry),
        }
    }
}

/// Helper function to create event metadata with an ID.
///
/// This is a convenience function for creating `EventMeta` with an event ID,
/// which is useful for implementing resumable subscriptions (similar to SSE's Last-Event-ID).
///
/// # Arguments
///
/// * `id` - The event ID (can be any type that converts to String)
///
/// # Returns
///
/// An `EventMeta` struct with the specified ID
///
/// # Example
///
/// ```rust,ignore
/// use tauri_plugin_rpc::subscription::*;
///
/// let event = Event::new("data".to_string())
///     .with_meta(with_event_meta("event-123"));
///
/// // Client can use this ID to resume from this point
/// assert_eq!(event.id, Some("event-123".to_string()));
/// ```
///
/// # Use Cases
///
/// - **Resumable streams**: Client can reconnect and resume from last received event
/// - **Deduplication**: Client can detect and skip duplicate events
/// - **Ordering**: Client can detect out-of-order delivery
pub fn with_event_meta(id: impl Into<String>) -> EventMeta {
    EventMeta::with_id(id)
}

/// Event sent to frontend via Tauri event system
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
#[non_exhaustive]
pub enum SubscriptionEvent {
    /// Data event
    Data {
        /// Event payload
        payload: Event<serde_json::Value>,
    },
    /// Error event with optional retry hint
    Error {
        /// Error details
        payload: crate::RpcError,
        /// Suggested retry delay in milliseconds (None = don't retry)
        #[serde(rename = "retryAfterMs", skip_serializing_if = "Option::is_none")]
        retry_after_ms: Option<u64>,
    },
    /// Completion event
    Completed,
}

impl SubscriptionEvent {
    /// Create a data event
    pub fn data(payload: Event<serde_json::Value>) -> Self {
        Self::Data { payload }
    }

    /// Create an error event without retry hint (non-recoverable error)
    pub fn error(err: crate::RpcError) -> Self {
        Self::Error {
            payload: err,
            retry_after_ms: None,
        }
    }

    /// Create an error event with retry metadata.
    ///
    /// Use this for recoverable errors where the client should retry after
    /// the specified delay.
    ///
    /// # Arguments
    /// * `err` - The error that occurred
    /// * `retry_after_ms` - Suggested retry delay in milliseconds
    ///
    /// # Example
    /// ```rust,ignore
    /// let event = SubscriptionEvent::error_with_retry(
    ///     RpcError::service_unavailable("Server busy"),
    ///     5000, // Retry after 5 seconds
    /// );
    /// ```
    pub fn error_with_retry(err: crate::RpcError, retry_after_ms: u64) -> Self {
        Self::Error {
            payload: err,
            retry_after_ms: Some(retry_after_ms),
        }
    }

    /// Create an error event without retry (non-recoverable).
    ///
    /// Use this for errors where retrying would not help, such as
    /// authentication failures or validation errors.
    ///
    /// # Example
    /// ```rust,ignore
    /// let event = SubscriptionEvent::error_no_retry(
    ///     RpcError::unauthorized("Invalid token"),
    /// );
    /// ```
    pub fn error_no_retry(err: crate::RpcError) -> Self {
        Self::Error {
            payload: err,
            retry_after_ms: None,
        }
    }

    /// Create a completion event
    pub fn completed() -> Self {
        Self::Completed
    }
}
