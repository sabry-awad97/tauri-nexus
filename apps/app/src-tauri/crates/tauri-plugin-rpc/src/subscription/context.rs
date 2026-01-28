//! Subscription context and cancellation types

use crate::subscription::SubscriptionId;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

/// Context provided to subscription handlers
#[derive(Debug, Clone)]
pub struct SubscriptionContext {
    /// Unique subscription ID
    pub subscription_id: SubscriptionId,
    /// Last event ID for resumption (from client)
    pub last_event_id: Option<String>,
    /// Cancellation signal
    signal: Arc<CancellationSignal>,
    /// Optional timeout for the subscription
    timeout: Option<Duration>,
}

/// Reason for subscription cancellation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancellationReason {
    /// Subscription was explicitly cancelled
    Cancelled,
    /// Subscription timed out
    Timeout,
}

impl SubscriptionContext {
    /// Create a new subscription context
    pub fn new(subscription_id: SubscriptionId, last_event_id: Option<String>) -> Self {
        Self {
            subscription_id,
            last_event_id,
            signal: Arc::new(CancellationSignal::new()),
            timeout: None,
        }
    }

    /// Set a timeout for this subscription
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Check if the subscription has been cancelled
    pub fn is_cancelled(&self) -> bool {
        self.signal.is_cancelled()
    }

    /// Get a future that resolves when cancelled
    pub async fn cancelled(&self) {
        self.signal.cancelled().await
    }

    /// Get a future that resolves when either cancelled or timed out.
    ///
    /// Returns the reason for completion (Cancelled or Timeout).
    ///
    /// # Example
    /// ```rust,ignore
    /// let ctx = SubscriptionContext::new(id, None)
    ///     .with_timeout(Duration::from_secs(30));
    ///
    /// match ctx.cancelled_or_timeout().await {
    ///     CancellationReason::Cancelled => println!("Subscription cancelled"),
    ///     CancellationReason::Timeout => println!("Subscription timed out"),
    /// }
    /// ```
    pub async fn cancelled_or_timeout(&self) -> CancellationReason {
        if let Some(timeout) = self.timeout {
            tokio::select! {
                _ = self.signal.cancelled() => CancellationReason::Cancelled,
                _ = tokio::time::sleep(timeout) => CancellationReason::Timeout,
            }
        } else {
            self.signal.cancelled().await;
            CancellationReason::Cancelled
        }
    }

    /// Get the cancellation signal for cloning
    pub fn signal(&self) -> Arc<CancellationSignal> {
        self.signal.clone()
    }
}

/// Cancellation signal for subscriptions
#[derive(Debug)]
pub struct CancellationSignal {
    cancelled: std::sync::atomic::AtomicBool,
    notify: tokio::sync::Notify,
}

impl CancellationSignal {
    /// Create a new cancellation signal
    pub fn new() -> Self {
        Self {
            cancelled: std::sync::atomic::AtomicBool::new(false),
            notify: tokio::sync::Notify::new(),
        }
    }

    /// Cancel the signal
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
        self.notify.notify_waiters();
    }

    /// Check if cancelled
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    /// Wait until cancelled
    pub async fn cancelled(&self) {
        if self.is_cancelled() {
            return;
        }
        self.notify.notified().await;
    }
}

impl Default for CancellationSignal {
    fn default() -> Self {
        Self::new()
    }
}
