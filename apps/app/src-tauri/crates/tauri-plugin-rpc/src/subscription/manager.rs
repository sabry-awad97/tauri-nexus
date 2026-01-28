//! Subscription manager for tracking and managing active subscriptions.
//!
//! This module provides the `SubscriptionManager` which tracks all active subscriptions
//! and their associated background tasks, enabling graceful cleanup and shutdown.

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use super::{CancellationSignal, ManagerConfig, ManagerError, SubscriptionId, SubscriptionMetrics};

// =============================================================================
// Health Status
// =============================================================================

/// Health status of the subscription manager
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HealthStatus {
    /// Number of active subscriptions
    pub active_subscriptions: usize,
    /// Number of tracked tasks
    pub active_tasks: usize,
    /// Number of completed tasks
    pub completed_tasks: usize,
    /// Manager uptime in seconds
    pub uptime_seconds: u64,
}

impl HealthStatus {
    /// Check if the manager is healthy
    pub fn is_healthy(&self) -> bool {
        // Manager is healthy if it's operational (has been created)
        // We could add more sophisticated health checks here
        true
    }

    /// Get a human-readable status message
    pub fn status_message(&self) -> String {
        format!(
            "Active: {} subscriptions, {} tasks | Completed: {} tasks | Uptime: {}s",
            self.active_subscriptions, self.active_tasks, self.completed_tasks, self.uptime_seconds
        )
    }
}

// =============================================================================
// Subscription Handle
// =============================================================================

/// Handle to an active subscription
#[derive(Debug)]
pub struct SubscriptionHandle {
    /// Unique subscription ID
    pub id: SubscriptionId,
    /// Path of the subscription procedure
    pub path: String,
    /// Cancellation signal
    signal: Arc<CancellationSignal>,
    /// Task handle for cleanup
    task_handle: Option<tokio::task::JoinHandle<()>>,
    /// Creation timestamp for duration tracking
    created_at: std::time::Instant,
}

impl SubscriptionHandle {
    /// Create a new subscription handle
    pub fn new(id: SubscriptionId, path: String, signal: Arc<CancellationSignal>) -> Self {
        Self {
            id,
            path,
            signal,
            task_handle: None,
            created_at: std::time::Instant::now(),
        }
    }

    /// Create a builder for constructing a subscription handle
    ///
    /// # Example
    /// ```rust,ignore
    /// let handle = SubscriptionHandle::builder()
    ///     .id(subscription_id)
    ///     .path("chat.messages")
    ///     .signal(signal)
    ///     .task(task_handle)
    ///     .build();
    /// ```
    pub fn builder() -> SubscriptionHandleBuilder {
        SubscriptionHandleBuilder::new()
    }

    /// Set the task handle
    pub fn with_task(mut self, handle: tokio::task::JoinHandle<()>) -> Self {
        self.task_handle = Some(handle);
        self
    }

    /// Get the duration since creation
    pub fn duration(&self) -> Duration {
        self.created_at.elapsed()
    }

    /// Cancel the subscription
    pub fn cancel(&self) {
        self.signal.cancel();
    }

    /// Check if cancelled
    pub fn is_cancelled(&self) -> bool {
        self.signal.is_cancelled()
    }
}

impl Drop for SubscriptionHandle {
    fn drop(&mut self) {
        self.signal.cancel();
        if let Some(handle) = self.task_handle.take() {
            handle.abort();
        }
    }
}

// =============================================================================
// Subscription Handle Builder
// =============================================================================

/// Builder for constructing a SubscriptionHandle
#[derive(Debug)]
pub struct SubscriptionHandleBuilder {
    id: Option<SubscriptionId>,
    path: Option<String>,
    signal: Option<Arc<CancellationSignal>>,
    task_handle: Option<tokio::task::JoinHandle<()>>,
}

impl SubscriptionHandleBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            id: None,
            path: None,
            signal: None,
            task_handle: None,
        }
    }

    /// Set the subscription ID
    pub fn id(mut self, id: SubscriptionId) -> Self {
        self.id = Some(id);
        self
    }

    /// Set the subscription path
    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    /// Set the cancellation signal
    pub fn signal(mut self, signal: Arc<CancellationSignal>) -> Self {
        self.signal = Some(signal);
        self
    }

    /// Set the task handle
    pub fn task(mut self, handle: tokio::task::JoinHandle<()>) -> Self {
        self.task_handle = Some(handle);
        self
    }

    /// Build the subscription handle
    ///
    /// # Panics
    /// Panics if required fields (id, path, signal) are not set
    pub fn build(self) -> SubscriptionHandle {
        SubscriptionHandle {
            id: self.id.expect("id is required"),
            path: self.path.expect("path is required"),
            signal: self.signal.expect("signal is required"),
            task_handle: self.task_handle,
            created_at: std::time::Instant::now(),
        }
    }
}

impl Default for SubscriptionHandleBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Subscription Manager
// =============================================================================

/// Manages active subscriptions with task tracking for graceful cleanup.
///
/// The `SubscriptionManager` tracks all active subscriptions and their associated
/// background tasks. It provides methods for spawning tracked subscription tasks
/// and graceful shutdown.
///
/// # Example
/// ```rust,ignore
/// let manager = SubscriptionManager::new();
///
/// // Spawn a tracked subscription task
/// let id = manager.spawn_subscription(subscription_id, async move {
///     // Subscription logic here
/// }).await;
///
/// // Graceful shutdown - cancels all subscriptions and waits for tasks
/// manager.shutdown().await;
/// ```
#[derive(Debug)]
pub struct SubscriptionManager {
    /// Active subscriptions by ID (using DashMap for better concurrent performance)
    subscriptions: dashmap::DashMap<SubscriptionId, SubscriptionHandle>,
    /// Task tracker for cleanup - tracks all spawned subscription tasks
    task_tracker: RwLock<tokio::task::JoinSet<()>>,
    /// Counter for completed tasks (for monitoring memory leaks)
    completed_tasks: Arc<std::sync::atomic::AtomicUsize>,
    /// Configuration for manager operations
    config: ManagerConfig,
    /// Creation time for uptime calculation
    created_at: std::time::Instant,
    /// Subscription lifecycle metrics
    metrics: Arc<SubscriptionMetrics>,
}

impl Default for SubscriptionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SubscriptionManager {
    /// Create a new subscription manager with default configuration
    pub fn new() -> Self {
        Self::with_config(ManagerConfig::default())
    }

    /// Create a new subscription manager with custom configuration
    pub fn with_config(config: ManagerConfig) -> Self {
        tracing::trace!("SubscriptionManager created with config");
        Self {
            subscriptions: dashmap::DashMap::new(),
            task_tracker: RwLock::new(tokio::task::JoinSet::new()),
            completed_tasks: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            config,
            created_at: std::time::Instant::now(),
            metrics: Arc::new(SubscriptionMetrics::new()),
        }
    }

    /// Register a new subscription
    pub fn subscribe(&self, handle: SubscriptionHandle) -> SubscriptionId {
        let id = handle.id;
        let path = handle.path.clone();

        // Record subscription creation in metrics
        self.metrics.record_created();

        self.subscriptions.insert(id, handle);

        tracing::info!(
            subscription_id = %id,
            path = %path,
            "Subscription registered"
        );

        id
    }

    /// Register a new subscription with timeout.
    ///
    /// This method attempts to register a subscription within the configured timeout.
    /// If the operation takes longer than the timeout, it returns a `ManagerError::Timeout`.
    ///
    /// # Arguments
    /// * `handle` - The subscription handle to register
    ///
    /// # Returns
    /// * `Ok(SubscriptionId)` - The subscription was registered successfully
    /// * `Err(ManagerError::Timeout)` - The operation timed out
    ///
    /// # Example
    /// ```rust,ignore
    /// match manager.subscribe_with_timeout(handle).await {
    ///     Ok(id) => println!("Subscribed: {}", id),
    ///     Err(ManagerError::Timeout(_)) => println!("Subscription timed out"),
    ///     Err(e) => println!("Error: {}", e),
    /// }
    /// ```
    pub async fn subscribe_with_timeout(
        &self,
        handle: SubscriptionHandle,
    ) -> Result<SubscriptionId, ManagerError> {
        let timeout = self.config.subscribe_timeout;

        tokio::time::timeout(timeout, async move {
            // The actual subscribe operation is synchronous, but we wrap it
            // in an async block to support timeout
            Ok(self.subscribe(handle))
        })
        .await
        .map_err(|_| ManagerError::Timeout(timeout))?
    }

    /// Spawn a tracked subscription task.
    ///
    /// This method spawns a background task and tracks it in the `JoinSet` for
    /// graceful cleanup during shutdown. The task will be automatically aborted
    /// when `shutdown()` is called.
    ///
    /// # Arguments
    /// * `id` - The subscription ID for this task
    /// * `future` - The async task to spawn
    ///
    /// # Returns
    /// The subscription ID that was passed in
    ///
    /// # Example
    /// ```rust,ignore
    /// let id = manager.spawn_subscription(subscription_id, async move {
    ///     while !signal.is_cancelled() {
    ///         // Process subscription events
    ///     }
    /// }).await;
    /// ```
    pub async fn spawn_subscription<F>(&self, id: SubscriptionId, future: F) -> SubscriptionId
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        let mut tracker = self.task_tracker.write().await;
        tracker.spawn(future);

        tracing::trace!(
            subscription_id = %id,
            "Subscription task spawned"
        );

        id
    }

    /// Graceful shutdown - cancel all subscriptions and wait for tasks to complete.
    ///
    /// This method:
    /// 1. Cancels all active subscription signals
    /// 2. Waits for all tracked tasks to complete (with cancellation)
    /// 3. Clears the subscription registry
    ///
    /// # Example
    /// ```rust,ignore
    /// // During application shutdown
    /// manager.shutdown().await;
    /// ```
    pub async fn shutdown(&self) {
        let sub_count = self.count();
        tracing::info!(
            active_subscriptions = %sub_count,
            "SubscriptionManager shutdown initiated"
        );

        // Cancel all subscription signals first
        for entry in self.subscriptions.iter() {
            let (id, handle) = entry.pair();
            tracing::trace!(
                subscription_id = %id,
                "Cancelling subscription"
            );
            handle.cancel();
        }

        // Abort all tracked tasks and wait for them to complete
        {
            let mut tracker = self.task_tracker.write().await;
            let task_count = tracker.len();
            tracing::debug!(
                task_count = %task_count,
                "Aborting tracked tasks"
            );
            tracker.abort_all();
            while tracker.join_next().await.is_some() {
                // Wait for all tasks to complete
            }
        }

        // Clear the subscriptions
        self.subscriptions.clear();

        tracing::info!("SubscriptionManager shutdown complete");
    }

    /// Unsubscribe by ID
    pub fn unsubscribe(&self, id: &SubscriptionId) -> bool {
        if let Some((_, handle)) = self.subscriptions.remove(id) {
            let duration = handle.duration();

            tracing::info!(
                subscription_id = %id,
                path = %handle.path,
                duration_ms = %duration.as_millis(),
                reason = "unsubscribed",
                "Subscription cancelled"
            );

            handle.cancel();

            // Record cancellation in metrics
            self.metrics.record_cancelled(duration);

            true
        } else {
            tracing::trace!(
                subscription_id = %id,
                "Unsubscribe called for non-existent subscription"
            );
            false
        }
    }

    /// Unsubscribe by ID with timeout.
    ///
    /// This method attempts to unsubscribe within the configured timeout.
    /// If the operation takes longer than the timeout, it returns a `ManagerError::Timeout`.
    ///
    /// # Arguments
    /// * `id` - The subscription ID to unsubscribe
    ///
    /// # Returns
    /// * `Ok(true)` - The subscription was found and unsubscribed
    /// * `Ok(false)` - The subscription was not found
    /// * `Err(ManagerError::Timeout)` - The operation timed out
    ///
    /// # Example
    /// ```rust,ignore
    /// match manager.unsubscribe_with_timeout(&id).await {
    ///     Ok(true) => println!("Unsubscribed successfully"),
    ///     Ok(false) => println!("Subscription not found"),
    ///     Err(ManagerError::Timeout(_)) => println!("Unsubscribe timed out"),
    ///     Err(e) => println!("Error: {}", e),
    /// }
    /// ```
    pub async fn unsubscribe_with_timeout(
        &self,
        id: &SubscriptionId,
    ) -> Result<bool, ManagerError> {
        let timeout = self.config.unsubscribe_timeout;
        let id_copy = *id;

        tokio::time::timeout(timeout, async move {
            // The actual unsubscribe operation is synchronous, but we wrap it
            // in an async block to support timeout
            Ok(self.unsubscribe(&id_copy))
        })
        .await
        .map_err(|_| ManagerError::Timeout(timeout))?
    }

    /// Get subscription count
    pub fn count(&self) -> usize {
        self.subscriptions.len()
    }

    /// Check if a subscription exists
    pub fn exists(&self, id: &SubscriptionId) -> bool {
        self.subscriptions.contains_key(id)
    }

    /// Cancel all subscriptions
    pub fn cancel_all(&self) {
        let count = self.subscriptions.len();

        // Iterate, cancel all, and record metrics
        for entry in self.subscriptions.iter() {
            let (id, handle) = entry.pair();
            let duration = handle.duration();

            tracing::trace!(
                subscription_id = %id,
                duration_ms = %duration.as_millis(),
                "Cancelling subscription"
            );

            handle.cancel();

            // Record cancellation in metrics
            self.metrics.record_cancelled(duration);
        }

        // Clear all subscriptions
        self.subscriptions.clear();

        tracing::info!(
            cancelled_count = %count,
            reason = "cancel_all",
            "All subscriptions cancelled"
        );
    }

    /// Get all subscription IDs
    pub fn subscription_ids(&self) -> Vec<SubscriptionId> {
        self.subscriptions
            .iter()
            .map(|entry| *entry.key())
            .collect()
    }

    /// Clean up completed subscriptions
    pub fn cleanup(&self) {
        let _before_count = self.subscriptions.len();

        // Collect IDs of cancelled subscriptions
        let to_remove: Vec<SubscriptionId> = self
            .subscriptions
            .iter()
            .filter(|entry| entry.value().is_cancelled())
            .map(|entry| {
                let id = *entry.key();
                tracing::trace!(
                    subscription_id = %id,
                    "Cleaning up cancelled subscription"
                );
                id
            })
            .collect();

        // Remove them
        for id in &to_remove {
            self.subscriptions.remove(id);
        }

        let removed = to_remove.len();
        if removed > 0 {
            tracing::debug!(
                removed_count = %removed,
                "Subscription cleanup complete"
            );
        }
    }

    /// Get the number of tracked tasks (for testing/debugging)
    pub async fn task_count(&self) -> usize {
        self.task_tracker.read().await.len()
    }

    /// Get the number of completed tasks that haven't been cleaned up yet
    pub fn completed_task_count(&self) -> usize {
        self.completed_tasks
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Clean up completed tasks from the JoinSet.
    ///
    /// This method polls the JoinSet for completed tasks and removes them,
    /// preventing memory leaks from accumulated finished tasks.
    ///
    /// Returns the number of tasks that were cleaned up.
    ///
    /// # Example
    /// ```rust,ignore
    /// let removed = manager.cleanup_completed().await;
    /// tracing::debug!("Cleaned up {} completed tasks", removed);
    /// ```
    pub async fn cleanup_completed(&self) -> usize {
        let mut tracker = self.task_tracker.write().await;
        let mut removed = 0;

        // Poll for completed tasks without blocking
        while let Some(result) = tracker.try_join_next() {
            match result {
                Ok(_) => {
                    removed += 1;
                    self.completed_tasks
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
                Err(e) if e.is_cancelled() => {
                    // Task was cancelled, count it as completed
                    removed += 1;
                    self.completed_tasks
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
                Err(e) if e.is_panic() => {
                    // Task panicked, log and count it
                    tracing::error!("Subscription task panicked");
                    removed += 1;
                    self.completed_tasks
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
                Err(_) => {
                    // Other error, count it
                    removed += 1;
                    self.completed_tasks
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
            }
        }

        if removed > 0 {
            tracing::debug!(removed_count = removed, "Cleaned up completed tasks");
        }

        removed
    }

    /// Start periodic cleanup of completed tasks.
    ///
    /// Spawns a background task that periodically calls `cleanup_completed()`
    /// to prevent memory leaks from accumulated finished tasks. Uses the
    /// cleanup interval from the manager configuration.
    ///
    /// This method requires the manager to be wrapped in an Arc.
    ///
    /// Returns a JoinHandle that can be used to stop the cleanup task.
    ///
    /// # Example
    /// ```rust,ignore
    /// let manager = Arc::new(SubscriptionManager::new());
    /// let cleanup_handle = manager.start_periodic_cleanup();
    /// // Later, to stop cleanup:
    /// cleanup_handle.abort();
    /// ```
    pub fn start_periodic_cleanup(self: &Arc<Self>) -> tokio::task::JoinHandle<()> {
        let manager = Arc::clone(self);
        let interval_duration = manager.config.cleanup_interval;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(interval_duration);
            loop {
                interval.tick().await;
                let removed = manager.cleanup_completed().await;
                if removed > 0 {
                    tracing::trace!(
                        removed_count = removed,
                        "Periodic cleanup removed completed tasks"
                    );
                }
            }
        })
    }

    /// Get the current health status of the subscription manager.
    ///
    /// Returns a `HealthStatus` struct containing information about active
    /// subscriptions, tasks, and uptime.
    ///
    /// # Example
    /// ```rust,ignore
    /// let health = manager.health().await;
    /// println!("Manager status: {}", health.status_message());
    /// println!("Active subscriptions: {}", health.active_subscriptions);
    /// println!("Uptime: {}s", health.uptime_seconds);
    /// ```
    pub async fn health(&self) -> HealthStatus {
        let active_subscriptions = self.subscriptions.len();
        let active_tasks = self.task_tracker.read().await.len();
        let completed_tasks = self
            .completed_tasks
            .load(std::sync::atomic::Ordering::Relaxed);
        let uptime_seconds = self.created_at.elapsed().as_secs();

        HealthStatus {
            active_subscriptions,
            active_tasks,
            completed_tasks,
            uptime_seconds,
        }
    }

    /// Get the subscription lifecycle metrics.
    ///
    /// Returns an Arc to the metrics, allowing for efficient sharing and monitoring.
    ///
    /// # Example
    /// ```rust,ignore
    /// let metrics = manager.metrics();
    /// let snapshot = metrics.snapshot();
    /// println!("Created: {}, Active: {}", snapshot.created, snapshot.active);
    /// ```
    pub fn metrics(&self) -> Arc<SubscriptionMetrics> {
        Arc::clone(&self.metrics)
    }
}
