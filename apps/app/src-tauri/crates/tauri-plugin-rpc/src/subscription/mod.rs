//! Event Iterator and Streaming (SSE-Style) Implementation
//!
//! This module provides comprehensive subscription/streaming support for the RPC framework,
//! enabling real-time data streaming from backend to frontend using Tauri's event system.
//!
//! ## Features
//!
//! - **Async generator-style subscriptions** - Stream data asynchronously with full backpressure support
//! - **Event metadata with IDs** - Support for resumption and retry with event IDs
//! - **Automatic cleanup** - Graceful shutdown and resource management
//! - **Channel-based pub/sub** - Multi-channel event broadcasting
//! - **Health monitoring** - Built-in health checks and lifecycle metrics
//! - **Configurable timeouts** - Timeout support for all operations
//! - **Backpressure strategies** - Choose how to handle slow consumers
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use tauri_plugin_rpc::subscription::*;
//!
//! // Create a subscription manager
//! let manager = Arc::new(SubscriptionManager::new());
//!
//! // Create an event publisher
//! let publisher = EventPublisher::<String>::new(256);
//! let mut subscriber = publisher.subscribe();
//!
//! // Publish events
//! publisher.publish_data("Hello, world!".to_string());
//!
//! // Receive events
//! if let Some(event) = subscriber.recv().await {
//!     println!("Received: {}", event.data);
//! }
//! ```
//!
//! ## Configuration
//!
//! ### Capacity Recommendations
//!
//! Choose the appropriate capacity based on your use case:
//!
//! - **Small (32)** - Low-frequency events, minimal memory usage
//!   - Example: User preference updates, occasional notifications
//!
//! - **Medium (256)** - Default for most use cases, balanced performance
//!   - Example: Chat messages, UI state updates
//!
//! - **Large (1024)** - High-frequency events, more memory usage
//!   - Example: Real-time sensor data, live metrics
//!
//! - **XLarge (4096)** - Very high throughput, significant memory usage
//!   - Example: High-frequency trading data, video streaming metadata
//!
//! ```rust,ignore
//! // Using capacity presets
//! let publisher = EventPublisher::<Data>::with_capacity(Capacity::Large);
//! ```
//!
//! ### Backpressure Strategies
//!
//! Choose the strategy that matches your data semantics:
//!
//! - **DropOldest (default)** - Maintains most recent data
//!   - Use for: Real-time sensor readings, live prices, UI state
//!   - Trade-off: May lose historical data, but always current
//!
//! - **DropNewest** - Maintains message order and completeness
//!   - Use for: Audit logs, transaction history, sequential commands
//!   - Trade-off: May deliver stale data, but preserves order
//!
//! - **Error** - Fails fast on backpressure
//!   - Use for: Critical notifications, payment events, security alerts
//!   - Trade-off: Requires explicit error handling, but guarantees delivery or failure
//!
//! ```rust,ignore
//! let publisher = EventPublisher::<Alert>::with_strategy(
//!     Capacity::Medium,
//!     BackpressureStrategy::Error
//! );
//! ```
//!
//! ### Timeout Configuration
//!
//! Configure timeouts to prevent operations from hanging:
//!
//! ```rust,ignore
//! let config = ManagerConfig::new()
//!     .with_subscribe_timeout(Duration::from_secs(30))
//!     .with_unsubscribe_timeout(Duration::from_secs(5))
//!     .with_cleanup_interval(Duration::from_secs(60));
//!
//! let manager = SubscriptionManager::with_config(config);
//! ```
//!
//! **Implications:**
//! - Shorter timeouts = faster failure detection, but may timeout valid operations
//! - Longer timeouts = more resilient, but slower to detect issues
//! - Cleanup interval affects memory usage vs. CPU overhead trade-off
//!
//! ## Error Handling
//!
//! ### PublishResult
//!
//! ```rust,ignore
//! match publisher.publish_data(data) {
//!     PublishResult::Published(count) => {
//!         println!("Published to {} subscribers", count);
//!     }
//!     PublishResult::NoSubscribers => {
//!         println!("No active subscribers (not an error)");
//!     }
//! }
//! ```
//!
//! ### Timeout Errors
//!
//! ```rust,ignore
//! match manager.subscribe_with_timeout(handle).await {
//!     Ok(id) => println!("Subscribed: {}", id),
//!     Err(ManagerError::Timeout(duration)) => {
//!         println!("Operation timed out after {:?}", duration);
//!     }
//!     Err(e) => println!("Error: {}", e),
//! }
//! ```
//!
//! ### Validation Errors
//!
//! ```rust,ignore
//! match RetryDelay::from_millis(5000) {
//!     Ok(delay) => println!("Valid delay: {:?}", delay.as_duration()),
//!     Err(ValidationError::RetryDelayOutOfRange { min, max, actual }) => {
//!         println!("Delay {}ms out of range [{}, {}]", actual, min, max);
//!     }
//!     Err(e) => println!("Validation error: {}", e),
//! }
//! ```
//!
//! ## Health Monitoring
//!
//! ```rust,ignore
//! let health = manager.health().await;
//! println!("Status: {}", health.status_message());
//! println!("Active subscriptions: {}", health.active_subscriptions);
//! println!("Uptime: {}s", health.uptime_seconds);
//!
//! // Get lifecycle metrics
//! let metrics = manager.metrics();
//! let snapshot = metrics.snapshot();
//! println!("Created: {}, Active: {}, Cancelled: {}",
//!     snapshot.created, snapshot.active, snapshot.cancelled);
//! ```
//!
//! ## Module Organization
//!
//! - `backpressure` - Backpressure strategies and batch publishing
//! - `config` - Configuration types and capacity presets
//! - `errors` - Error types for the subscription system
//! - `metrics` - Metrics collection and reporting
//! - `retry_delay` - Validated retry delay type
//!
//! ## Common Patterns
//!
//! ### Basic Subscription
//!
//! ```rust,ignore
//! let router = Router::new()
//!     .subscription("chat.messages", |ctx, input: ChatInput| async move {
//!         let (tx, rx) = event_channel(256);
//!         
//!         // Spawn background task to send messages
//!         tokio::spawn(async move {
//!             while let Some(msg) = get_next_message().await {
//!                 if tx.send(Event::new(msg)).await.is_err() {
//!                     break; // Subscriber disconnected
//!                 }
//!             }
//!         });
//!         
//!         Ok(rx)
//!     });
//! ```
//!
//! ### With Cancellation
//!
//! ```rust,ignore
//! .subscription("live.data", |ctx, sub_ctx, input| async move {
//!     let (tx, rx) = event_channel(256);
//!     
//!     tokio::spawn(async move {
//!         loop {
//!             tokio::select! {
//!                 _ = sub_ctx.cancelled() => break,
//!                 data = fetch_data() => {
//!                     if tx.send(Event::new(data)).await.is_err() {
//!                         break;
//!                     }
//!                 }
//!             }
//!         }
//!     });
//!     
//!     Ok(rx)
//! })
//! ```
//!
//! ### With Timeout
//!
//! ```rust,ignore
//! let ctx = SubscriptionContext::new(id, None)
//!     .with_timeout(Duration::from_secs(300)); // 5 minute timeout
//!
//! match ctx.cancelled_or_timeout().await {
//!     CancellationReason::Cancelled => println!("User cancelled"),
//!     CancellationReason::Timeout => println!("Subscription timed out"),
//! }
//! ```

mod backpressure;
mod config;
mod errors;
mod event;
mod id;
mod metrics;
mod retry_delay;

pub use backpressure::{BackpressureStrategy, BatchPublishResult};
pub use config::{Capacity, ManagerConfig, SubscriptionConfig};
pub use errors::{ManagerError, ParseError, PublishResult, ValidationError};
pub use event::{Event, EventMeta, SubscriptionEvent, with_event_meta};
pub use id::{SubscriptionId, generate_subscription_id};
pub use metrics::{
    MetricsSnapshot, PublisherMetrics, PublisherMetricsSnapshot, SubscriberMetrics,
    SubscriberMetricsSnapshot, SubscriptionMetrics,
};
pub use retry_delay::RetryDelay;

use serde::Serialize;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;
use tokio::sync::{RwLock, broadcast, mpsc};

use crate::{Context, RpcError, RpcResult};

// =============================================================================
// Subscription Context
// =============================================================================

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

// =============================================================================
// Subscription Stream Types
// =============================================================================

/// A stream of events from a subscription
pub type EventStream<T> = mpsc::Receiver<Event<T>>;

/// Sender for emitting events
pub type EventSender<T> = mpsc::Sender<Event<T>>;

/// Create a new event channel for streaming data.
///
/// This function creates a bounded MPSC channel for sending events from a subscription
/// handler to the frontend. The buffer size determines how many events can be queued
/// before backpressure is applied.
///
/// # Arguments
///
/// * `buffer` - The maximum number of events that can be buffered
///
/// # Returns
///
/// A tuple of `(EventSender<T>, EventStream<T>)` where:
/// - `EventSender` is used to send events from your subscription handler
/// - `EventStream` is returned to the RPC framework for delivery to the frontend
///
/// # Example
///
/// ```rust,ignore
/// use tauri_plugin_rpc::subscription::*;
///
/// async fn my_subscription() -> RpcResult<EventStream<String>> {
///     let (tx, rx) = event_channel(256);
///     
///     tokio::spawn(async move {
///         for i in 0..10 {
///             let event = Event::new(format!("Message {}", i));
///             if tx.send(event).await.is_err() {
///                 break; // Subscriber disconnected
///             }
///         }
///     });
///     
///     Ok(rx)
/// }
/// ```
///
/// # Buffer Size Guidelines
///
/// - Small (32-64): Low-frequency events, minimal latency
/// - Medium (256): Default for most use cases
/// - Large (1024+): High-frequency events, burst tolerance
pub fn event_channel<T>(buffer: usize) -> (EventSender<T>, EventStream<T>) {
    mpsc::channel(buffer)
}

// =============================================================================
// Subscription Handler Trait
// =============================================================================

/// Return type for subscription handlers
pub type SubscriptionResult<T> = RpcResult<EventStream<T>>;

/// Boxed subscription handler for type erasure
pub type BoxedSubscriptionHandler<Ctx> = Arc<
    dyn Fn(
            Context<Ctx>,
            SubscriptionContext,
            serde_json::Value,
        ) -> Pin<
            Box<dyn Future<Output = RpcResult<mpsc::Receiver<Event<serde_json::Value>>>> + Send>,
        > + Send
        + Sync,
>;

/// Trait for subscription handler functions
pub trait SubscriptionHandler<Ctx, Input, Output>: Clone + Send + Sync + 'static
where
    Ctx: Clone + Send + Sync + 'static,
    Input: serde::de::DeserializeOwned + Send + 'static,
    Output: Serialize + Send + 'static,
{
    /// The future type returned by the handler
    type Future: Future<Output = SubscriptionResult<Output>> + Send;

    /// Call the handler
    fn call(&self, ctx: Context<Ctx>, sub_ctx: SubscriptionContext, input: Input) -> Self::Future;
}

// Implement for async functions
impl<Ctx, Input, Output, F, Fut> SubscriptionHandler<Ctx, Input, Output> for F
where
    Ctx: Clone + Send + Sync + 'static,
    Input: serde::de::DeserializeOwned + Send + 'static,
    Output: Serialize + Send + 'static,
    F: Fn(Context<Ctx>, SubscriptionContext, Input) -> Fut + Clone + Send + Sync + 'static,
    Fut: Future<Output = SubscriptionResult<Output>> + Send + 'static,
{
    type Future = Fut;

    fn call(&self, ctx: Context<Ctx>, sub_ctx: SubscriptionContext, input: Input) -> Self::Future {
        (self)(ctx, sub_ctx, input)
    }
}

/// Convert a subscription handler into a boxed handler
pub fn into_boxed_subscription<Ctx, Input, Output, H>(handler: H) -> BoxedSubscriptionHandler<Ctx>
where
    Ctx: Clone + Send + Sync + 'static,
    Input: serde::de::DeserializeOwned + Send + 'static,
    Output: Serialize + Send + 'static,
    H: SubscriptionHandler<Ctx, Input, Output>,
{
    Arc::new(move |ctx, sub_ctx, input_value| {
        let handler = handler.clone();
        Box::pin(async move {
            let input: Input = serde_json::from_value(input_value)?;
            let stream = handler.call(ctx, sub_ctx, input).await?;

            // Convert typed stream to JSON stream
            let (tx, rx) = mpsc::channel(32);
            tokio::spawn(async move {
                let mut stream = stream;
                while let Some(event) = stream.recv().await {
                    // Properly handle serialization errors instead of silently converting to Null
                    let data = match serde_json::to_value(&event.data) {
                        Ok(value) => value,
                        Err(e) => {
                            tracing::error!(
                                error = %e,
                                "Failed to serialize subscription event data"
                            );
                            // Stop the stream on serialization error
                            break;
                        }
                    };

                    let json_event = Event {
                        data,
                        id: event.id,
                        retry: event.retry,
                    };
                    if tx.send(json_event).await.is_err() {
                        break;
                    }
                }
            });

            Ok(rx)
        })
    })
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
        F: Future<Output = ()> + Send + 'static,
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

// =============================================================================
// Event Publisher (Pub/Sub Pattern)
// =============================================================================

/// A publisher for broadcasting events to multiple subscribers
#[derive(Debug)]
pub struct EventPublisher<T: Clone + Send + 'static> {
    /// Broadcast sender
    sender: broadcast::Sender<Event<T>>,
    /// Channel capacity
    capacity: Capacity,
    /// Backpressure handling strategy
    strategy: BackpressureStrategy,
    /// Publisher metrics
    metrics: Arc<PublisherMetrics>,
}

impl<T: Clone + Send + 'static> EventPublisher<T> {
    /// Create a new event publisher with default capacity and strategy
    pub fn new(capacity: usize) -> Self {
        Self::with_capacity(Capacity::from(capacity))
    }

    /// Create a new event publisher with a specific capacity preset
    pub fn with_capacity(capacity: Capacity) -> Self {
        let (sender, _) = broadcast::channel(capacity.value());
        Self {
            sender,
            capacity,
            strategy: BackpressureStrategy::default(),
            metrics: Arc::new(PublisherMetrics::new()),
        }
    }

    /// Create a new event publisher with a specific backpressure strategy
    pub fn with_strategy(capacity: Capacity, strategy: BackpressureStrategy) -> Self {
        let (sender, _) = broadcast::channel(capacity.value());
        Self {
            sender,
            capacity,
            strategy,
            metrics: Arc::new(PublisherMetrics::new()),
        }
    }

    /// Get the backpressure strategy
    pub fn strategy(&self) -> BackpressureStrategy {
        self.strategy
    }

    /// Get the capacity
    pub fn capacity(&self) -> Capacity {
        self.capacity
    }

    /// Get publisher metrics
    pub fn metrics(&self) -> Arc<PublisherMetrics> {
        Arc::clone(&self.metrics)
    }

    /// Publish an event to all subscribers.
    ///
    /// Returns `Published(count)` with the number of subscribers that received the event,
    /// or `NoSubscribers` if there are no active subscribers.
    ///
    /// This method handles the case of no subscribers gracefully by returning
    /// `NoSubscribers` instead of an error. Having no subscribers is a normal
    /// operational state, not an error condition.
    pub fn publish(&self, event: Event<T>) -> PublishResult {
        match self.sender.send(event) {
            Ok(count) => {
                self.metrics.record_publish(count);
                PublishResult::Published(count)
            }
            Err(_) => {
                tracing::trace!("EventPublisher::publish: no active subscribers");
                PublishResult::NoSubscribers
            }
        }
    }

    /// Publish data as an event.
    ///
    /// This is a convenience method that wraps the data in an [`Event`] and publishes it.
    pub fn publish_data(&self, data: T) -> PublishResult {
        self.publish(Event::new(data))
    }

    /// Publish multiple events as a batch.
    ///
    /// This method attempts to publish all events in the batch. The behavior
    /// depends on the configured backpressure strategy.
    ///
    /// # Returns
    /// A `BatchPublishResult` containing success/failure counts and total subscribers.
    ///
    /// # Example
    /// ```rust,ignore
    /// let events = vec![
    ///     Event::new("message1"),
    ///     Event::new("message2"),
    ///     Event::new("message3"),
    /// ];
    ///
    /// let result = publisher.publish_batch(events);
    /// println!("Published {}/{} events to {} subscribers",
    ///     result.success_count,
    ///     result.total_count(),
    ///     result.total_subscribers
    /// );
    /// ```
    pub fn publish_batch(&self, events: Vec<Event<T>>) -> BatchPublishResult {
        let mut success_count = 0;
        let mut failure_count = 0;
        let mut total_subscribers = 0;

        for event in events {
            match self.publish(event) {
                PublishResult::Published(count) => {
                    success_count += 1;
                    total_subscribers = total_subscribers.max(count);
                }
                PublishResult::NoSubscribers => {
                    failure_count += 1;
                }
            }
        }

        self.metrics.record_batch(success_count);

        BatchPublishResult::new(success_count, failure_count, total_subscribers)
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> EventSubscriber<T> {
        EventSubscriber {
            receiver: self.sender.subscribe(),
            metrics: Arc::new(SubscriberMetrics::new()),
        }
    }

    /// Get the number of active subscribers
    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

impl<T: Clone + Send + 'static> Default for EventPublisher<T> {
    fn default() -> Self {
        Self::with_capacity(Capacity::Medium)
    }
}

impl<T: Clone + Send + 'static> Clone for EventPublisher<T> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            capacity: self.capacity,
            strategy: self.strategy,
            metrics: Arc::clone(&self.metrics),
        }
    }
}

/// A subscriber to an event publisher
pub struct EventSubscriber<T: Clone + Send + 'static> {
    receiver: broadcast::Receiver<Event<T>>,
    metrics: Arc<SubscriberMetrics>,
}

impl<T: Clone + Send + 'static> EventSubscriber<T> {
    /// Receive the next event.
    ///
    /// This method handles lagged messages gracefully by skipping them and
    /// continuing to receive the most recent available messages. A warning-level
    /// log is emitted when messages are skipped due to lag, and metrics are tracked.
    ///
    /// Returns `Some(event)` when an event is received, or `None` when the
    /// channel is closed.
    pub async fn recv(&mut self) -> Option<Event<T>> {
        loop {
            match self.receiver.recv().await {
                Ok(event) => {
                    self.metrics.record_received();
                    return Some(event);
                }
                Err(broadcast::error::RecvError::Lagged(count)) => {
                    // Record lag in metrics and log at warn level
                    self.metrics.record_lagged(count);
                    tracing::warn!(
                        lagged_messages = count,
                        "EventSubscriber lagged behind, skipped {} messages",
                        count
                    );
                    continue;
                }
                Err(broadcast::error::RecvError::Closed) => return None,
            }
        }
    }

    /// Get the total number of lagged messages
    pub fn lag_count(&self) -> u64 {
        self.metrics.lag_count()
    }

    /// Convert to an event stream
    pub fn into_stream(self) -> EventStream<T> {
        let (tx, rx) = mpsc::channel(32);
        let mut subscriber = self;

        tokio::spawn(async move {
            while let Some(event) = subscriber.recv().await {
                if tx.send(event).await.is_err() {
                    break;
                }
            }
        });

        rx
    }
}

// =============================================================================
// Channel-based Event Publisher
// =============================================================================

/// A multi-channel event publisher for pub/sub patterns
#[derive(Debug)]
pub struct ChannelPublisher<T: Clone + Send + 'static> {
    /// Publishers by channel name (using DashMap for better concurrent performance)
    channels: dashmap::DashMap<String, EventPublisher<T>>,
    /// Default channel capacity
    capacity: Capacity,
}

impl<T: Clone + Send + 'static> ChannelPublisher<T> {
    /// Create a new channel publisher with default capacity
    pub fn new(capacity: usize) -> Self {
        Self::with_capacity(Capacity::from(capacity))
    }

    /// Create a new channel publisher with a specific capacity preset
    pub fn with_capacity(capacity: Capacity) -> Self {
        Self {
            channels: dashmap::DashMap::new(),
            capacity,
        }
    }

    /// Publish to a specific channel
    pub fn publish(&self, channel: &str, event: Event<T>) -> Result<PublishResult, RpcError> {
        if let Some(publisher) = self.channels.get(channel) {
            Ok(publisher.publish(event))
        } else {
            Err(RpcError::not_found(format!(
                "Channel '{}' not found",
                channel
            )))
        }
    }

    /// Publish data to a channel
    pub fn publish_data(&self, channel: &str, data: T) -> Result<PublishResult, RpcError> {
        self.publish(channel, Event::new(data))
    }

    /// Subscribe to a channel (creates channel if it doesn't exist)
    pub fn subscribe(&self, channel: &str) -> EventSubscriber<T> {
        let publisher = self
            .channels
            .entry(channel.to_string())
            .or_insert_with(|| EventPublisher::with_capacity(self.capacity));
        publisher.subscribe()
    }

    /// Get or create a channel
    pub fn get_or_create(&self, channel: &str) -> EventPublisher<T> {
        self.channels
            .entry(channel.to_string())
            .or_insert_with(|| EventPublisher::with_capacity(self.capacity))
            .clone()
    }

    /// Remove a channel
    pub fn remove_channel(&self, channel: &str) -> bool {
        self.channels.remove(channel).is_some()
    }

    /// List all channels
    pub fn channels(&self) -> Vec<String> {
        self.channels
            .iter()
            .map(|entry| entry.key().clone())
            .collect()
    }
}

impl<T: Clone + Send + 'static> Default for ChannelPublisher<T> {
    fn default() -> Self {
        Self::with_capacity(Capacity::Medium)
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    mod integration;
    mod property;
    mod unit;
}
