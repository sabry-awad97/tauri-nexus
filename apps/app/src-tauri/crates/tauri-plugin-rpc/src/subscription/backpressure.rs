// Backpressure handling strategies for event publishers

use serde::{Deserialize, Serialize};

/// Strategy for handling backpressure when the channel is full
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BackpressureStrategy {
    /// Drop the oldest messages when the channel is full (default)
    ///
    /// This strategy maintains the most recent messages, which is useful
    /// for real-time data where old values become stale.
    ///
    /// # Example Use Cases
    /// - Live sensor readings
    /// - Real-time price updates
    /// - UI state updates
    #[default]
    DropOldest,

    /// Drop the newest messages when the channel is full
    ///
    /// This strategy maintains the oldest messages, which is useful
    /// when message ordering and completeness are critical.
    ///
    /// # Example Use Cases
    /// - Audit logs
    /// - Transaction history
    /// - Sequential command processing
    DropNewest,

    /// Return an error when the channel is full
    ///
    /// This strategy fails fast when backpressure occurs, allowing
    /// the caller to handle the situation explicitly.
    ///
    /// # Example Use Cases
    /// - Critical notifications
    /// - Payment processing events
    /// - Security alerts
    Error,
}

impl BackpressureStrategy {
    /// Get a human-readable description of the strategy
    pub fn description(&self) -> &'static str {
        match self {
            BackpressureStrategy::DropOldest => "Drop oldest messages to maintain most recent data",
            BackpressureStrategy::DropNewest => "Drop newest messages to maintain message order",
            BackpressureStrategy::Error => "Return error when channel is full",
        }
    }
}

/// Result of a batch publish operation
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchPublishResult {
    /// Number of events successfully published
    pub success_count: usize,
    /// Number of events that failed to publish
    pub failure_count: usize,
    /// Total number of subscribers that received events
    pub total_subscribers: usize,
}

impl BatchPublishResult {
    /// Create a new batch publish result
    pub fn new(success_count: usize, failure_count: usize, total_subscribers: usize) -> Self {
        Self {
            success_count,
            failure_count,
            total_subscribers,
        }
    }

    /// Returns true if all events were published successfully
    pub fn is_complete_success(&self) -> bool {
        self.failure_count == 0 && self.success_count > 0
    }

    /// Returns true if no events were published
    pub fn is_complete_failure(&self) -> bool {
        self.success_count == 0
    }

    /// Returns true if some events succeeded and some failed
    pub fn is_partial_success(&self) -> bool {
        self.success_count > 0 && self.failure_count > 0
    }

    /// Get the total number of events attempted
    pub fn total_count(&self) -> usize {
        self.success_count + self.failure_count
    }

    /// Get the success rate as a percentage (0.0 to 1.0)
    pub fn success_rate(&self) -> f64 {
        let total = self.total_count();
        if total == 0 {
            0.0
        } else {
            self.success_count as f64 / total as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backpressure_strategy_default() {
        assert_eq!(
            BackpressureStrategy::default(),
            BackpressureStrategy::DropOldest
        );
    }

    #[test]
    fn test_backpressure_strategy_description() {
        assert!(!BackpressureStrategy::DropOldest.description().is_empty());
        assert!(!BackpressureStrategy::DropNewest.description().is_empty());
        assert!(!BackpressureStrategy::Error.description().is_empty());
    }

    #[test]
    fn test_batch_publish_result_complete_success() {
        let result = BatchPublishResult::new(10, 0, 5);
        assert!(result.is_complete_success());
        assert!(!result.is_complete_failure());
        assert!(!result.is_partial_success());
        assert_eq!(result.success_rate(), 1.0);
    }

    #[test]
    fn test_batch_publish_result_complete_failure() {
        let result = BatchPublishResult::new(0, 10, 0);
        assert!(!result.is_complete_success());
        assert!(result.is_complete_failure());
        assert!(!result.is_partial_success());
        assert_eq!(result.success_rate(), 0.0);
    }

    #[test]
    fn test_batch_publish_result_partial_success() {
        let result = BatchPublishResult::new(7, 3, 5);
        assert!(!result.is_complete_success());
        assert!(!result.is_complete_failure());
        assert!(result.is_partial_success());
        assert_eq!(result.success_rate(), 0.7);
    }

    #[test]
    fn test_batch_publish_result_total_count() {
        let result = BatchPublishResult::new(7, 3, 5);
        assert_eq!(result.total_count(), 10);
    }

    #[test]
    fn test_batch_publish_result_zero_events() {
        let result = BatchPublishResult::new(0, 0, 0);
        assert_eq!(result.success_rate(), 0.0);
        assert_eq!(result.total_count(), 0);
    }
}
