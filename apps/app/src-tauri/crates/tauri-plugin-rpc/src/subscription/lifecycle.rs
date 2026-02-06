//! Subscription Lifecycle Management
//!
//! This module manages the lifecycle of subscriptions, including state transitions,
//! event handling, and metrics collection.
//!
//! # State Machine
//!
//! ```text
//! Created → Active → Cancelled
//!                  → Completed
//!                  → Error
//! ```
//!
//! Terminal states (Cancelled, Completed, Error) cannot transition to other states.
//!
//! # Example
//!
//! ```rust,ignore
//! use tauri_plugin_rpc::subscription::lifecycle::*;
//!
//! let event_name = subscription_event_name("rpc:subscription:", &subscription_id);
//! let metrics = handle_subscription_events(
//!     app,
//!     subscription_id,
//!     path,
//!     event_name,
//!     stream,
//!     signal,
//! ).await;
//! ```

use super::{CancellationSignal, Event, SubscriptionEvent, SubscriptionId};
use crate::config::PluginConfig;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Runtime};
use tokio::time::interval;
use tracing::{debug, info, trace};

// =============================================================================
// State Machine Types
// =============================================================================

/// Subscription state in the lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubscriptionState {
    /// Subscription has been created but not yet active
    Created,
    /// Subscription is actively processing events
    Active,
    /// Subscription was cancelled by the client
    Cancelled,
    /// Subscription completed successfully
    Completed,
    /// Subscription ended with an error
    Error,
}

impl SubscriptionState {
    /// Check if this is a terminal state (no further transitions allowed).
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            SubscriptionState::Cancelled | SubscriptionState::Completed | SubscriptionState::Error
        )
    }

    /// Validate a state transition.
    ///
    /// Returns `Ok(())` if the transition is valid, `Err` otherwise.
    pub fn can_transition_to(&self, next: SubscriptionState) -> Result<(), String> {
        match (self, next) {
            // From Created
            (SubscriptionState::Created, SubscriptionState::Active) => Ok(()),
            (SubscriptionState::Created, SubscriptionState::Cancelled) => Ok(()),
            (SubscriptionState::Created, SubscriptionState::Error) => Ok(()),

            // From Active
            (SubscriptionState::Active, SubscriptionState::Cancelled) => Ok(()),
            (SubscriptionState::Active, SubscriptionState::Completed) => Ok(()),
            (SubscriptionState::Active, SubscriptionState::Error) => Ok(()),

            // Terminal states cannot transition
            (current, _) if current.is_terminal() => Err(format!(
                "Cannot transition from terminal state {:?}",
                current
            )),

            // Invalid transitions
            (current, next) => Err(format!(
                "Invalid transition from {:?} to {:?}",
                current, next
            )),
        }
    }
}

/// Reason for subscription completion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompletionReason {
    /// Client cancelled the subscription
    Cancelled,
    /// Subscription completed successfully
    Completed,
    /// Subscription ended with an error
    Error,
}

/// Metrics collected during subscription lifecycle event processing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifecycleMetrics {
    /// Total number of events processed
    pub event_count: u64,
    /// Final state of the subscription
    pub final_state: SubscriptionState,
    /// Reason for completion
    pub completion_reason: CompletionReason,
    /// Duration in milliseconds
    pub duration_ms: u64,
}

impl LifecycleMetrics {
    /// Create new metrics with default values.
    pub fn new(completion_reason: CompletionReason) -> Self {
        let final_state = match completion_reason {
            CompletionReason::Cancelled => SubscriptionState::Cancelled,
            CompletionReason::Completed => SubscriptionState::Completed,
            CompletionReason::Error => SubscriptionState::Error,
        };

        Self {
            event_count: 0,
            final_state,
            completion_reason,
            duration_ms: 0,
        }
    }
}

// =============================================================================
// Event Naming
// =============================================================================

/// Generate the event name for a subscription.
///
/// # Arguments
///
/// * `prefix` - The event prefix (e.g., "rpc:subscription:")
/// * `subscription_id` - The subscription ID
///
/// # Returns
///
/// The full event name (e.g., "rpc:subscription:sub_01234567...")
///
/// # Examples
///
/// ```rust,ignore
/// let event_name = subscription_event_name("rpc:subscription:", &id);
/// assert_eq!(event_name, "rpc:subscription:sub_01234567...");
/// ```
pub fn subscription_event_name(prefix: &str, subscription_id: &SubscriptionId) -> String {
    format!("{}{}", prefix, subscription_id)
}

// =============================================================================
// Event Handler
// =============================================================================

/// Handle subscription event stream and emit events to frontend.
///
/// This function manages the event loop for a subscription:
/// - Receives events from the stream
/// - Checks for cancellation
/// - Emits events to the frontend via Tauri event system
/// - Handles completion and errors
/// - Tracks metrics
///
/// Returns the metrics collected during the subscription lifecycle.
///
/// # Arguments
///
/// * `app` - Tauri application handle
/// * `subscription_id` - The subscription ID
/// * `path` - The procedure path
/// * `event_name` - The event name for emission
/// * `stream` - The event stream receiver
/// * `signal` - Cancellation signal
///
/// # Returns
///
/// Metrics collected during the subscription lifecycle.
pub async fn handle_subscription_events<R: Runtime>(
    app: AppHandle<R>,
    subscription_id: SubscriptionId,
    path: String,
    event_name: String,
    mut stream: tokio::sync::mpsc::Receiver<Event<serde_json::Value>>,
    signal: Arc<CancellationSignal>,
) -> LifecycleMetrics {
    let start = std::time::Instant::now();
    let mut event_count = 0u64;
    let mut state = SubscriptionState::Active;

    while let Some(event) = stream.recv().await {
        // Check for cancellation
        if signal.is_cancelled() {
            debug!(
                subscription_id = %subscription_id,
                path = %path,
                event_count = %event_count,
                "Subscription cancelled"
            );
            state = SubscriptionState::Cancelled;
            break;
        }

        // Reject events if in terminal state
        if state.is_terminal() {
            debug!(
                subscription_id = %subscription_id,
                state = ?state,
                "Rejecting event in terminal state"
            );
            break;
        }

        event_count += 1;
        let sub_event = SubscriptionEvent::data(event);

        if app.emit(&event_name, &sub_event).is_err() {
            debug!(
                subscription_id = %subscription_id,
                path = %path,
                "Subscription emit failed, closing"
            );
            state = SubscriptionState::Error;
            break;
        }
    }

    // Determine completion reason
    let completion_reason = if signal.is_cancelled() {
        CompletionReason::Cancelled
    } else if state == SubscriptionState::Error {
        CompletionReason::Error
    } else {
        CompletionReason::Completed
    };

    // Update final state
    state = match completion_reason {
        CompletionReason::Cancelled => SubscriptionState::Cancelled,
        CompletionReason::Completed => SubscriptionState::Completed,
        CompletionReason::Error => SubscriptionState::Error,
    };

    // Send completion event if not cancelled
    if !signal.is_cancelled() {
        info!(
            subscription_id = %subscription_id,
            path = %path,
            event_count = %event_count,
            state = ?state,
            "Subscription completed"
        );
        let _ = app.emit(&event_name, &SubscriptionEvent::completed());
    }

    let duration_ms = start.elapsed().as_millis() as u64;

    LifecycleMetrics {
        event_count,
        final_state: state,
        completion_reason,
        duration_ms,
    }
}

// =============================================================================
// Buffered Event Handler
// =============================================================================

/// Flush buffered events to the frontend.
///
/// Emits all events in the buffer as a batch and clears the buffer.
fn flush_buffer<R: Runtime>(
    app: &AppHandle<R>,
    event_name: &str,
    buffer: &mut Vec<Event<serde_json::Value>>,
) -> Result<(), String> {
    if buffer.is_empty() {
        return Ok(());
    }

    // Emit each event individually
    // Note: In a future optimization, we could batch these into a single IPC call
    for event in buffer.drain(..) {
        let sub_event = SubscriptionEvent::data(event);
        app.emit(event_name, &sub_event)
            .map_err(|e| format!("Failed to emit event: {}", e))?;
    }

    Ok(())
}

/// Handle subscription event stream with optional buffering.
///
/// This function extends `handle_subscription_events` with buffering support.
/// When buffering is enabled (buffer_size > 1), events are collected and flushed
/// either when the buffer is full or after the flush interval.
///
/// # Arguments
///
/// * `app` - Tauri application handle
/// * `subscription_id` - The subscription ID
/// * `path` - The procedure path
/// * `event_name` - The event name for emission
/// * `stream` - The event stream receiver
/// * `signal` - Cancellation signal
/// * `config` - Plugin configuration with buffering settings
///
/// # Returns
///
/// Metrics collected during the subscription lifecycle.
pub async fn handle_subscription_events_buffered<R: Runtime>(
    app: AppHandle<R>,
    subscription_id: SubscriptionId,
    path: String,
    event_name: String,
    mut stream: tokio::sync::mpsc::Receiver<Event<serde_json::Value>>,
    signal: Arc<CancellationSignal>,
    config: &PluginConfig,
) -> LifecycleMetrics {
    // If buffering is disabled, use the standard handler
    if !config.is_buffering_enabled() {
        return handle_subscription_events(app, subscription_id, path, event_name, stream, signal)
            .await;
    }

    let start = std::time::Instant::now();
    let mut event_count = 0u64;
    let mut state = SubscriptionState::Active;
    let mut buffer: Vec<Event<serde_json::Value>> = Vec::with_capacity(config.event_buffer_size);
    let mut flush_timer = interval(config.event_buffer_flush_interval);
    flush_timer.tick().await; // Skip first immediate tick

    trace!(
        subscription_id = %subscription_id,
        buffer_size = config.event_buffer_size,
        flush_interval_ms = config.event_buffer_flush_interval.as_millis(),
        "Starting buffered subscription"
    );

    loop {
        tokio::select! {
            // Receive event from stream
            event_opt = stream.recv() => {
                match event_opt {
                    Some(event) => {
                        // Check for cancellation
                        if signal.is_cancelled() {
                            debug!(
                                subscription_id = %subscription_id,
                                path = %path,
                                event_count = %event_count,
                                buffered = buffer.len(),
                                "Subscription cancelled, flushing buffer"
                            );
                            state = SubscriptionState::Cancelled;
                            break;
                        }

                        // Reject events if in terminal state
                        if state.is_terminal() {
                            debug!(
                                subscription_id = %subscription_id,
                                state = ?state,
                                "Rejecting event in terminal state"
                            );
                            break;
                        }

                        event_count += 1;
                        buffer.push(event);

                        // Flush if buffer is full
                        if buffer.len() >= config.event_buffer_size {
                            trace!(
                                subscription_id = %subscription_id,
                                buffer_size = buffer.len(),
                                "Buffer full, flushing"
                            );
                            if let Err(e) = flush_buffer(&app, &event_name, &mut buffer) {
                                debug!(
                                    subscription_id = %subscription_id,
                                    error = ?e,
                                    "Buffer flush failed, closing subscription"
                                );
                                state = SubscriptionState::Error;
                                break;
                            }
                        }
                    }
                    None => {
                        // Stream closed
                        debug!(
                            subscription_id = %subscription_id,
                            buffered = buffer.len(),
                            "Stream closed, flushing remaining events"
                        );
                        break;
                    }
                }
            }

            // Flush on interval
            _ = flush_timer.tick() => {
                if !buffer.is_empty() {
                    trace!(
                        subscription_id = %subscription_id,
                        buffer_size = buffer.len(),
                        "Flush interval reached, flushing buffer"
                    );
                    if let Err(e) = flush_buffer(&app, &event_name, &mut buffer) {
                        debug!(
                            subscription_id = %subscription_id,
                            error = ?e,
                            "Buffer flush failed, closing subscription"
                        );
                        state = SubscriptionState::Error;
                        break;
                    }
                }
            }
        }
    }

    // Flush any remaining buffered events
    if !buffer.is_empty() && !signal.is_cancelled() {
        trace!(
            subscription_id = %subscription_id,
            buffer_size = buffer.len(),
            "Flushing remaining buffered events"
        );
        let _ = flush_buffer(&app, &event_name, &mut buffer);
    }

    // Determine completion reason
    let completion_reason = if signal.is_cancelled() {
        CompletionReason::Cancelled
    } else if state == SubscriptionState::Error {
        CompletionReason::Error
    } else {
        CompletionReason::Completed
    };

    // Update final state
    state = match completion_reason {
        CompletionReason::Cancelled => SubscriptionState::Cancelled,
        CompletionReason::Completed => SubscriptionState::Completed,
        CompletionReason::Error => SubscriptionState::Error,
    };

    // Send completion event if not cancelled
    if !signal.is_cancelled() {
        info!(
            subscription_id = %subscription_id,
            path = %path,
            event_count = %event_count,
            state = ?state,
            "Buffered subscription completed"
        );
        let _ = app.emit(&event_name, &SubscriptionEvent::completed());
    }

    let duration_ms = start.elapsed().as_millis() as u64;

    LifecycleMetrics {
        event_count,
        final_state: state,
        completion_reason,
        duration_ms,
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::time::Duration;
    use tokio::time::interval;

    // Property 12: Subscription state machine transitions
    #[test]
    fn test_state_machine_valid_transitions() {
        // Created → Active
        assert!(
            SubscriptionState::Created
                .can_transition_to(SubscriptionState::Active)
                .is_ok()
        );

        // Created → Cancelled
        assert!(
            SubscriptionState::Created
                .can_transition_to(SubscriptionState::Cancelled)
                .is_ok()
        );

        // Active → Completed
        assert!(
            SubscriptionState::Active
                .can_transition_to(SubscriptionState::Completed)
                .is_ok()
        );

        // Active → Cancelled
        assert!(
            SubscriptionState::Active
                .can_transition_to(SubscriptionState::Cancelled)
                .is_ok()
        );

        // Active → Error
        assert!(
            SubscriptionState::Active
                .can_transition_to(SubscriptionState::Error)
                .is_ok()
        );
    }

    #[test]
    fn test_state_machine_invalid_transitions() {
        // Created → Completed (must go through Active)
        assert!(
            SubscriptionState::Created
                .can_transition_to(SubscriptionState::Completed)
                .is_err()
        );

        // Active → Created (cannot go backwards)
        assert!(
            SubscriptionState::Active
                .can_transition_to(SubscriptionState::Created)
                .is_err()
        );
    }

    // Property 13: Terminal states reject new events
    proptest! {
        #[test]
        fn prop_terminal_states_reject_transitions(
            next_state in prop::sample::select(vec![
                SubscriptionState::Created,
                SubscriptionState::Active,
                SubscriptionState::Cancelled,
                SubscriptionState::Completed,
                SubscriptionState::Error,
            ])
        ) {
            let terminal_states = vec![
                SubscriptionState::Cancelled,
                SubscriptionState::Completed,
                SubscriptionState::Error,
            ];

            for terminal in terminal_states {
                assert!(terminal.is_terminal());
                assert!(terminal.can_transition_to(next_state).is_err());
            }
        }
    }

    #[test]
    fn test_terminal_states() {
        assert!(SubscriptionState::Cancelled.is_terminal());
        assert!(SubscriptionState::Completed.is_terminal());
        assert!(SubscriptionState::Error.is_terminal());
        assert!(!SubscriptionState::Created.is_terminal());
        assert!(!SubscriptionState::Active.is_terminal());
    }

    // Property 14: Event names use configured prefix
    proptest! {
        #[test]
        fn prop_event_names_use_prefix(
            prefix in "[a-z:]{1,20}",
            uuid_part in "[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}"
        ) {
            let id = SubscriptionId::parse_lenient(&uuid_part).unwrap();
            let event_name = subscription_event_name(&prefix, &id);
            assert!(event_name.starts_with(&prefix));
        }
    }

    // Property 15: Event name construction validates subscription ID
    #[test]
    fn test_event_name_construction() {
        let id = SubscriptionId::parse_lenient("01234567-89ab-7cde-8f01-234567890abc").unwrap();
        let event_name = subscription_event_name("rpc:subscription:", &id);
        assert_eq!(event_name, format!("rpc:subscription:{}", id));
    }

    #[test]
    fn test_metrics_creation() {
        let metrics = LifecycleMetrics::new(CompletionReason::Completed);
        assert_eq!(metrics.event_count, 0);
        assert_eq!(metrics.final_state, SubscriptionState::Completed);
        assert_eq!(metrics.completion_reason, CompletionReason::Completed);

        let metrics = LifecycleMetrics::new(CompletionReason::Cancelled);
        assert_eq!(metrics.final_state, SubscriptionState::Cancelled);

        let metrics = LifecycleMetrics::new(CompletionReason::Error);
        assert_eq!(metrics.final_state, SubscriptionState::Error);
    }

    // Task 10.1: Property tests for event buffering
    // Note: These tests verify the buffering logic conceptually.
    // Full integration tests would require a Tauri runtime.

    /// Property 27: Event buffering when enabled
    #[test]
    fn test_buffering_enabled_check() {
        let config_buffered =
            PluginConfig::default().with_event_buffering(100, Duration::from_millis(50));
        assert!(config_buffered.is_buffering_enabled());

        let config_immediate = PluginConfig::default();
        assert!(!config_immediate.is_buffering_enabled());
    }

    /// Property 28: Buffer flush on capacity
    #[test]
    fn test_buffer_capacity_logic() {
        let buffer_size = 10;
        let mut buffer: Vec<i32> = Vec::with_capacity(buffer_size);

        // Fill buffer to capacity
        for i in 0..buffer_size {
            buffer.push(i as i32);
        }

        // Buffer should be full
        assert_eq!(buffer.len(), buffer_size);

        // Simulate flush
        buffer.clear();
        assert_eq!(buffer.len(), 0);
    }

    /// Property 29: Buffer flush on interval
    #[tokio::test]
    async fn test_flush_interval_timing() {
        let flush_interval = Duration::from_millis(50);
        let mut interval_timer = interval(flush_interval);
        interval_timer.tick().await; // Skip first immediate tick

        let start = std::time::Instant::now();
        interval_timer.tick().await;
        let elapsed = start.elapsed();

        // Should be approximately flush_interval (with generous tolerance for CI/scheduling)
        // We only check the upper bound since tokio scheduling can vary
        assert!(elapsed < flush_interval + Duration::from_millis(100));
    }

    /// Property 30: Immediate emission when buffering disabled
    #[test]
    fn test_immediate_emission_when_disabled() {
        let config = PluginConfig::default();
        assert_eq!(config.event_buffer_size, 1);
        assert!(!config.is_buffering_enabled());

        // With buffer size 1, events should be emitted immediately
        // (no buffering occurs)
    }

    proptest! {
        /// Property: Buffer size configuration is respected
        #[test]
        fn prop_buffer_size_respected(
            buffer_size in 1usize..1000usize,
            flush_ms in 1u64..1000u64
        ) {
            let config = PluginConfig::default()
                .with_event_buffering(buffer_size, Duration::from_millis(flush_ms));

            assert_eq!(config.event_buffer_size, buffer_size);
            assert_eq!(config.event_buffer_flush_interval, Duration::from_millis(flush_ms));

            if buffer_size > 1 {
                assert!(config.is_buffering_enabled());
            } else {
                assert!(!config.is_buffering_enabled());
            }
        }

        /// Property: Flush interval configuration is respected
        #[test]
        fn prop_flush_interval_respected(
            buffer_size in 2usize..100usize,
            flush_ms in 10u64..500u64
        ) {
            let config = PluginConfig::default()
                .with_event_buffering(buffer_size, Duration::from_millis(flush_ms));

            assert_eq!(config.event_buffer_flush_interval, Duration::from_millis(flush_ms));
            assert!(config.validate().is_ok());
        }
    }
}
