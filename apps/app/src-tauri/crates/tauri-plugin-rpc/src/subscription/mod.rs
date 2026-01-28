//! Event Iterator and Streaming (SSE-Style) Implementation
//!
//! This module provides subscription/streaming support for the RPC framework,
//! enabling real-time data streaming from backend to frontend using Tauri's
//! event system.
//!
//! ## Features
//! - Async generator-style subscriptions
//! - Event metadata with IDs for resumption
//! - Automatic cleanup on disconnect
//! - Channel-based pub/sub patterns
//!
//! ## Example
//! ```rust,ignore
//! let router = Router::new()
//!     .subscription("chat.messages", |ctx, input: ChatInput| async move {
//!         let (tx, rx) = channel();
//!         // Stream messages...
//!         Ok(rx)
//!     });
//! ```

mod errors;

pub use errors::{ManagerError, ParseError, PublishResult, ValidationError};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use tokio::sync::{RwLock, broadcast, mpsc};
use uuid::Uuid;

use crate::{Context, RpcError, RpcResult};

// =============================================================================
// Subscription ID (UUID v7 Newtype)
// =============================================================================

/// A unique, time-ordered subscription identifier based on UUID v7.
///
/// UUID v7 provides:
/// - Time-ordered IDs (sortable by creation time)
/// - Cryptographically random bits for uniqueness
/// - Standard UUID format for interoperability
///
/// # Example
/// ```rust,ignore
/// let id = SubscriptionId::new();
/// println!("Subscription: {}", id); // sub_01234567-89ab-7cde-8f01-234567890abc
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SubscriptionId(Uuid);

impl SubscriptionId {
    /// Create a new subscription ID using UUID v7.
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    /// Create a subscription ID from an existing UUID.
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Get the underlying UUID.
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }

    /// Convert to the underlying UUID.
    pub fn into_uuid(self) -> Uuid {
        self.0
    }

    /// Parse a subscription ID from a string.
    ///
    /// Accepts both formats:
    /// - With prefix: "sub_01234567-89ab-7cde-8f01-234567890abc"
    /// - Without prefix: "01234567-89ab-7cde-8f01-234567890abc"
    pub fn parse(s: &str) -> Result<Self, uuid::Error> {
        let uuid_str = s.strip_prefix("sub_").unwrap_or(s);
        Uuid::parse_str(uuid_str).map(Self)
    }
}

impl Default for SubscriptionId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for SubscriptionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "sub_{}", self.0)
    }
}

impl From<Uuid> for SubscriptionId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<SubscriptionId> for String {
    fn from(id: SubscriptionId) -> Self {
        id.to_string()
    }
}

/// Generate a unique subscription ID using UUID v7.
///
/// This is a convenience function that creates a new [`SubscriptionId`].
///
/// # Example
/// ```rust,ignore
/// let id = generate_subscription_id();
/// assert!(id.to_string().starts_with("sub_"));
/// ```
pub fn generate_subscription_id() -> SubscriptionId {
    SubscriptionId::new()
}

// =============================================================================
// Event Types
// =============================================================================

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

/// Helper function to create event metadata
pub fn with_event_meta(id: impl Into<String>) -> EventMeta {
    EventMeta::with_id(id)
}

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
}

impl SubscriptionContext {
    /// Create a new subscription context
    pub fn new(subscription_id: SubscriptionId, last_event_id: Option<String>) -> Self {
        Self {
            subscription_id,
            last_event_id,
            signal: Arc::new(CancellationSignal::new()),
        }
    }

    /// Check if the subscription has been cancelled
    pub fn is_cancelled(&self) -> bool {
        self.signal.is_cancelled()
    }

    /// Get a future that resolves when cancelled
    pub async fn cancelled(&self) {
        self.signal.cancelled().await
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

/// Create a new event channel
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
                    let json_event = Event {
                        data: serde_json::to_value(&event.data).unwrap_or(serde_json::Value::Null),
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
}

impl SubscriptionHandle {
    /// Create a new subscription handle
    pub fn new(id: SubscriptionId, path: String, signal: Arc<CancellationSignal>) -> Self {
        Self {
            id,
            path,
            signal,
            task_handle: None,
        }
    }

    /// Set the task handle
    pub fn with_task(mut self, handle: tokio::task::JoinHandle<()>) -> Self {
        self.task_handle = Some(handle);
        self
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
    /// Active subscriptions by ID
    subscriptions: RwLock<HashMap<SubscriptionId, SubscriptionHandle>>,
    /// Task tracker for cleanup - tracks all spawned subscription tasks
    task_tracker: RwLock<tokio::task::JoinSet<()>>,
}

impl Default for SubscriptionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SubscriptionManager {
    /// Create a new subscription manager
    pub fn new() -> Self {
        tracing::trace!("SubscriptionManager created");
        Self {
            subscriptions: RwLock::new(HashMap::new()),
            task_tracker: RwLock::new(tokio::task::JoinSet::new()),
        }
    }

    /// Register a new subscription
    pub async fn subscribe(&self, handle: SubscriptionHandle) -> SubscriptionId {
        let id = handle.id;
        let path = handle.path.clone();
        self.subscriptions.write().await.insert(id, handle);

        tracing::debug!(
            subscription_id = %id,
            path = %path,
            "Subscription registered"
        );

        id
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
        let sub_count = self.subscriptions.read().await.len();
        tracing::info!(
            active_subscriptions = %sub_count,
            "SubscriptionManager shutdown initiated"
        );

        // Cancel all subscription signals first
        {
            let subs = self.subscriptions.read().await;
            for (id, handle) in subs.iter() {
                tracing::trace!(
                    subscription_id = %id,
                    "Cancelling subscription"
                );
                handle.cancel();
            }
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
        {
            let mut subs = self.subscriptions.write().await;
            subs.clear();
        }

        tracing::info!("SubscriptionManager shutdown complete");
    }

    /// Unsubscribe by ID
    pub async fn unsubscribe(&self, id: &SubscriptionId) -> bool {
        if let Some(handle) = self.subscriptions.write().await.remove(id) {
            tracing::debug!(
                subscription_id = %id,
                path = %handle.path,
                "Subscription unsubscribed"
            );
            handle.cancel();
            true
        } else {
            tracing::trace!(
                subscription_id = %id,
                "Unsubscribe called for non-existent subscription"
            );
            false
        }
    }

    /// Get subscription count
    pub async fn count(&self) -> usize {
        self.subscriptions.read().await.len()
    }

    /// Check if a subscription exists
    pub async fn exists(&self, id: &SubscriptionId) -> bool {
        self.subscriptions.read().await.contains_key(id)
    }

    /// Cancel all subscriptions
    pub async fn cancel_all(&self) {
        let mut subs = self.subscriptions.write().await;
        let count = subs.len();
        for (id, handle) in subs.drain() {
            tracing::trace!(
                subscription_id = %id,
                "Cancelling subscription"
            );
            handle.cancel();
        }
        tracing::debug!(
            cancelled_count = %count,
            "All subscriptions cancelled"
        );
    }

    /// Get all subscription IDs
    pub async fn subscription_ids(&self) -> Vec<SubscriptionId> {
        self.subscriptions.read().await.keys().copied().collect()
    }

    /// Clean up completed subscriptions
    pub async fn cleanup(&self) {
        let mut subs = self.subscriptions.write().await;
        let before_count = subs.len();
        subs.retain(|id, handle| {
            let keep = !handle.is_cancelled();
            if !keep {
                tracing::trace!(
                    subscription_id = %id,
                    "Cleaning up cancelled subscription"
                );
            }
            keep
        });
        let removed = before_count - subs.len();
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
    capacity: usize,
}

impl<T: Clone + Send + 'static> EventPublisher<T> {
    /// Create a new event publisher
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender, capacity }
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
            Ok(count) => PublishResult::Published(count),
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

    /// Subscribe to events
    pub fn subscribe(&self) -> EventSubscriber<T> {
        EventSubscriber {
            receiver: self.sender.subscribe(),
        }
    }

    /// Get the number of active subscribers
    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

impl<T: Clone + Send + 'static> Default for EventPublisher<T> {
    fn default() -> Self {
        Self::new(256)
    }
}

impl<T: Clone + Send + 'static> Clone for EventPublisher<T> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            capacity: self.capacity,
        }
    }
}

/// A subscriber to an event publisher
pub struct EventSubscriber<T: Clone + Send + 'static> {
    receiver: broadcast::Receiver<Event<T>>,
}

impl<T: Clone + Send + 'static> EventSubscriber<T> {
    /// Receive the next event.
    ///
    /// This method handles lagged messages gracefully by skipping them and
    /// continuing to receive the most recent available messages. A trace-level
    /// log is emitted when messages are skipped due to lag.
    ///
    /// Returns `Some(event)` when an event is received, or `None` when the
    /// channel is closed.
    pub async fn recv(&mut self) -> Option<Event<T>> {
        loop {
            match self.receiver.recv().await {
                Ok(event) => return Some(event),
                Err(broadcast::error::RecvError::Lagged(count)) => {
                    // Skip lagged messages and continue receiving
                    tracing::trace!("EventSubscriber lagged behind, skipped {} messages", count);
                    continue;
                }
                Err(broadcast::error::RecvError::Closed) => return None,
            }
        }
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
    /// Publishers by channel name
    channels: RwLock<HashMap<String, EventPublisher<T>>>,
    /// Default channel capacity
    capacity: usize,
}

impl<T: Clone + Send + 'static> ChannelPublisher<T> {
    /// Create a new channel publisher
    pub fn new(capacity: usize) -> Self {
        Self {
            channels: RwLock::new(HashMap::new()),
            capacity,
        }
    }

    /// Publish to a specific channel
    pub async fn publish(&self, channel: &str, event: Event<T>) -> Result<PublishResult, RpcError> {
        let channels = self.channels.read().await;
        if let Some(publisher) = channels.get(channel) {
            Ok(publisher.publish(event))
        } else {
            Err(RpcError::not_found(format!(
                "Channel '{}' not found",
                channel
            )))
        }
    }

    /// Publish data to a channel
    pub async fn publish_data(&self, channel: &str, data: T) -> Result<PublishResult, RpcError> {
        self.publish(channel, Event::new(data)).await
    }

    /// Subscribe to a channel (creates channel if it doesn't exist)
    pub async fn subscribe(&self, channel: &str) -> EventSubscriber<T> {
        let mut channels = self.channels.write().await;
        let publisher = channels
            .entry(channel.to_string())
            .or_insert_with(|| EventPublisher::new(self.capacity));
        publisher.subscribe()
    }

    /// Get or create a channel
    pub async fn get_or_create(&self, channel: &str) -> EventPublisher<T> {
        let mut channels = self.channels.write().await;
        channels
            .entry(channel.to_string())
            .or_insert_with(|| EventPublisher::new(self.capacity))
            .clone()
    }

    /// Remove a channel
    pub async fn remove_channel(&self, channel: &str) -> bool {
        self.channels.write().await.remove(channel).is_some()
    }

    /// List all channels
    pub async fn channels(&self) -> Vec<String> {
        self.channels.read().await.keys().cloned().collect()
    }
}

impl<T: Clone + Send + 'static> Default for ChannelPublisher<T> {
    fn default() -> Self {
        Self::new(256)
    }
}

// =============================================================================
// Subscription Event Types (for Tauri events)
// =============================================================================

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

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    mod integration;
    mod property;
    mod unit;
}
