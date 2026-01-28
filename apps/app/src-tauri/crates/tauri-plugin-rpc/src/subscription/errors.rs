// Error types for subscription module

use thiserror::Error;

/// Error parsing a subscription ID
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// Subscription ID must start with 'sub_' prefix
    #[error("Subscription ID must start with 'sub_' prefix")]
    MissingPrefix,

    /// Invalid UUID format
    #[error("Invalid UUID format: {0}")]
    InvalidUuid(#[from] uuid::Error),
}

/// Error validating input values
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ValidationError {
    /// Retry delay is out of valid range
    #[error("Retry delay must be between {min}ms and {max}ms, got {actual}ms")]
    RetryDelayOutOfRange {
        /// Minimum allowed value in milliseconds
        min: u64,
        /// Maximum allowed value in milliseconds
        max: u64,
        /// Actual value provided in milliseconds
        actual: u64,
    },

    /// Generic validation error
    #[error("Validation failed: {0}")]
    Invalid(String),
}

/// Error from subscription manager operations
#[derive(Debug, Error, Clone)]
pub enum ManagerError {
    /// Operation timed out
    #[error("Operation timed out after {0:?}")]
    Timeout(std::time::Duration),

    /// Subscription not found
    #[error("Subscription not found: {0}")]
    NotFound(crate::subscription::SubscriptionId),

    /// Manager is shutting down
    #[error("Manager is shutting down")]
    ShuttingDown,
}

/// Result of publishing an event
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PublishResult {
    /// Event was published to N subscribers
    Published(usize),

    /// No subscribers are currently listening (not an error)
    NoSubscribers,
}

impl PublishResult {
    /// Returns true if the event was published to at least one subscriber
    pub fn is_published(&self) -> bool {
        matches!(self, PublishResult::Published(_))
    }

    /// Returns the number of subscribers, or 0 if no subscribers
    pub fn subscriber_count(&self) -> usize {
        match self {
            PublishResult::Published(count) => *count,
            PublishResult::NoSubscribers => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_publish_result_is_published() {
        assert!(PublishResult::Published(5).is_published());
        assert!(!PublishResult::NoSubscribers.is_published());
    }

    #[test]
    fn test_publish_result_subscriber_count() {
        assert_eq!(PublishResult::Published(5).subscriber_count(), 5);
        assert_eq!(PublishResult::NoSubscribers.subscriber_count(), 0);
    }

    #[test]
    fn test_validation_error_display() {
        let err = ValidationError::RetryDelayOutOfRange {
            min: 1,
            max: 3600000,
            actual: 5000000,
        };
        let msg = err.to_string();
        assert!(msg.contains("1ms"));
        assert!(msg.contains("3600000ms"));
        assert!(msg.contains("5000000ms"));
    }
}
