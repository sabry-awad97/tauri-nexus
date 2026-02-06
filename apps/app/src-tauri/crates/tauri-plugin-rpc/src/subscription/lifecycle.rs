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
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Runtime};
use tracing::{debug, info};

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
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

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
}
