// Metrics collection for subscription module

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::Duration;

/// Metrics for subscription lifecycle tracking
#[derive(Debug, Default)]
pub struct SubscriptionMetrics {
    created: AtomicU64,
    cancelled: AtomicU64,
    completed: AtomicU64,
    active: AtomicUsize,
    total_duration_ms: AtomicU64,
}

impl SubscriptionMetrics {
    /// Create new subscription metrics
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a subscription creation
    pub fn record_created(&self) {
        self.created.fetch_add(1, Ordering::Relaxed);
        self.active.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a subscription cancellation with its duration
    pub fn record_cancelled(&self, duration: Duration) {
        self.cancelled.fetch_add(1, Ordering::Relaxed);
        self.active.fetch_sub(1, Ordering::Relaxed);
        self.total_duration_ms
            .fetch_add(duration.as_millis() as u64, Ordering::Relaxed);
    }

    /// Record a subscription completion with its duration
    pub fn record_completed(&self, duration: Duration) {
        self.completed.fetch_add(1, Ordering::Relaxed);
        self.active.fetch_sub(1, Ordering::Relaxed);
        self.total_duration_ms
            .fetch_add(duration.as_millis() as u64, Ordering::Relaxed);
    }

    /// Get a snapshot of current metrics
    pub fn snapshot(&self) -> MetricsSnapshot {
        let created = self.created.load(Ordering::Relaxed);
        let cancelled = self.cancelled.load(Ordering::Relaxed);
        let completed = self.completed.load(Ordering::Relaxed);
        let active = self.active.load(Ordering::Relaxed);
        let total_duration_ms = self.total_duration_ms.load(Ordering::Relaxed);

        let terminated = cancelled + completed;
        let avg_duration_ms = if terminated > 0 {
            total_duration_ms / terminated
        } else {
            0
        };

        MetricsSnapshot {
            created,
            cancelled,
            completed,
            active,
            avg_duration_ms,
        }
    }
}

/// Snapshot of subscription metrics at a point in time
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetricsSnapshot {
    /// Total subscriptions created
    pub created: u64,
    /// Total subscriptions cancelled
    pub cancelled: u64,
    /// Total subscriptions completed
    pub completed: u64,
    /// Currently active subscriptions
    pub active: usize,
    /// Average subscription duration in milliseconds
    pub avg_duration_ms: u64,
}

/// Metrics for event publisher
#[derive(Debug, Default)]
pub struct PublisherMetrics {
    published: AtomicU64,
    failed: AtomicU64,
    batch_published: AtomicU64,
}

impl PublisherMetrics {
    /// Create new publisher metrics
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a successful publish with subscriber count
    pub fn record_publish(&self, _subscriber_count: usize) {
        self.published.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a failed publish
    pub fn record_failed(&self) {
        self.failed.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a batch publish with event count
    pub fn record_batch(&self, count: usize) {
        self.batch_published
            .fetch_add(count as u64, Ordering::Relaxed);
    }

    /// Get a snapshot of current metrics
    pub fn snapshot(&self) -> PublisherMetricsSnapshot {
        PublisherMetricsSnapshot {
            published: self.published.load(Ordering::Relaxed),
            failed: self.failed.load(Ordering::Relaxed),
            batch_published: self.batch_published.load(Ordering::Relaxed),
        }
    }
}

/// Snapshot of publisher metrics at a point in time
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublisherMetricsSnapshot {
    /// Total events published
    pub published: u64,
    /// Total publish failures
    pub failed: u64,
    /// Total events published in batches
    pub batch_published: u64,
}

/// Metrics for event subscriber
#[derive(Debug, Default)]
pub struct SubscriberMetrics {
    received: AtomicU64,
    lagged: AtomicU64,
    lagged_messages: AtomicU64,
}

impl SubscriberMetrics {
    /// Create new subscriber metrics
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a received event
    pub fn record_received(&self) {
        self.received.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a lag event with the number of messages skipped
    pub fn record_lagged(&self, count: u64) {
        self.lagged.fetch_add(1, Ordering::Relaxed);
        self.lagged_messages.fetch_add(count, Ordering::Relaxed);
    }

    /// Get the total number of lagged messages
    pub fn lag_count(&self) -> u64 {
        self.lagged_messages.load(Ordering::Relaxed)
    }

    /// Get a snapshot of current metrics
    pub fn snapshot(&self) -> SubscriberMetricsSnapshot {
        SubscriberMetricsSnapshot {
            received: self.received.load(Ordering::Relaxed),
            lagged: self.lagged.load(Ordering::Relaxed),
            lagged_messages: self.lagged_messages.load(Ordering::Relaxed),
        }
    }
}

/// Snapshot of subscriber metrics at a point in time
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubscriberMetricsSnapshot {
    /// Total events received
    pub received: u64,
    /// Total lag events encountered
    pub lagged: u64,
    /// Total messages skipped due to lag
    pub lagged_messages: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subscription_metrics_creation() {
        let metrics = SubscriptionMetrics::new();
        metrics.record_created();
        metrics.record_created();

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.created, 2);
        assert_eq!(snapshot.active, 2);
    }

    #[test]
    fn test_subscription_metrics_cancellation() {
        let metrics = SubscriptionMetrics::new();
        metrics.record_created();
        metrics.record_cancelled(Duration::from_millis(100));

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.created, 1);
        assert_eq!(snapshot.cancelled, 1);
        assert_eq!(snapshot.active, 0);
        assert_eq!(snapshot.avg_duration_ms, 100);
    }

    #[test]
    fn test_subscription_metrics_completion() {
        let metrics = SubscriptionMetrics::new();
        metrics.record_created();
        metrics.record_completed(Duration::from_millis(200));

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.created, 1);
        assert_eq!(snapshot.completed, 1);
        assert_eq!(snapshot.active, 0);
        assert_eq!(snapshot.avg_duration_ms, 200);
    }

    #[test]
    fn test_subscription_metrics_average_duration() {
        let metrics = SubscriptionMetrics::new();
        metrics.record_created();
        metrics.record_created();
        metrics.record_cancelled(Duration::from_millis(100));
        metrics.record_completed(Duration::from_millis(300));

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.avg_duration_ms, 200); // (100 + 300) / 2
    }

    #[test]
    fn test_publisher_metrics() {
        let metrics = PublisherMetrics::new();
        metrics.record_publish(5);
        metrics.record_publish(3);
        metrics.record_failed();

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.published, 2);
        assert_eq!(snapshot.failed, 1);
    }

    #[test]
    fn test_publisher_metrics_batch() {
        let metrics = PublisherMetrics::new();
        metrics.record_batch(10);
        metrics.record_batch(5);

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.batch_published, 15);
    }

    #[test]
    fn test_subscriber_metrics() {
        let metrics = SubscriberMetrics::new();
        metrics.record_received();
        metrics.record_received();
        metrics.record_lagged(3);

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.received, 2);
        assert_eq!(snapshot.lagged, 1);
        assert_eq!(snapshot.lagged_messages, 3);
    }

    #[test]
    fn test_subscriber_lag_count() {
        let metrics = SubscriberMetrics::new();
        metrics.record_lagged(5);
        metrics.record_lagged(3);

        assert_eq!(metrics.lag_count(), 8);
    }
}
