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

/// Manages active subscriptions
#[derive(Debug, Default)]
pub struct SubscriptionManager {
    /// Active subscriptions by ID
    subscriptions: RwLock<HashMap<SubscriptionId, SubscriptionHandle>>,
}

impl SubscriptionManager {
    /// Create a new subscription manager
    pub fn new() -> Self {
        Self {
            subscriptions: RwLock::new(HashMap::new()),
        }
    }

    /// Register a new subscription
    pub async fn subscribe(&self, handle: SubscriptionHandle) -> SubscriptionId {
        let id = handle.id;
        self.subscriptions.write().await.insert(id, handle);
        id
    }

    /// Unsubscribe by ID
    pub async fn unsubscribe(&self, id: &SubscriptionId) -> bool {
        if let Some(handle) = self.subscriptions.write().await.remove(id) {
            handle.cancel();
            true
        } else {
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
        for (_, handle) in subs.drain() {
            handle.cancel();
        }
    }

    /// Get all subscription IDs
    pub async fn subscription_ids(&self) -> Vec<SubscriptionId> {
        self.subscriptions.read().await.keys().copied().collect()
    }

    /// Clean up completed subscriptions
    pub async fn cleanup(&self) {
        let mut subs = self.subscriptions.write().await;
        subs.retain(|_, handle| !handle.is_cancelled());
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

    /// Publish an event to all subscribers
    pub fn publish(&self, event: Event<T>) -> Result<usize, RpcError> {
        self.sender
            .send(event)
            .map_err(|_| RpcError::internal("No active subscribers"))
    }

    /// Publish data as an event
    pub fn publish_data(&self, data: T) -> Result<usize, RpcError> {
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
    /// Receive the next event
    pub async fn recv(&mut self) -> Option<Event<T>> {
        loop {
            match self.receiver.recv().await {
                Ok(event) => return Some(event),
                Err(broadcast::error::RecvError::Lagged(_)) => continue, // Skip lagged messages
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
    pub async fn publish(&self, channel: &str, event: Event<T>) -> Result<usize, RpcError> {
        let channels = self.channels.read().await;
        if let Some(publisher) = channels.get(channel) {
            publisher.publish(event)
        } else {
            Err(RpcError::not_found(format!(
                "Channel '{}' not found",
                channel
            )))
        }
    }

    /// Publish data to a channel
    pub async fn publish_data(&self, channel: &str, data: T) -> Result<usize, RpcError> {
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
pub enum SubscriptionEvent {
    /// Data event
    Data {
        /// Event payload
        payload: Event<serde_json::Value>,
    },
    /// Error event
    Error {
        /// Error details
        payload: crate::RpcError,
    },
    /// Completion event
    Completed,
}

impl SubscriptionEvent {
    /// Create a data event
    pub fn data(payload: Event<serde_json::Value>) -> Self {
        Self::Data { payload }
    }

    /// Create an error event
    pub fn error(err: crate::RpcError) -> Self {
        Self::Error { payload: err }
    }

    /// Create a completion event
    pub fn completed() -> Self {
        Self::Completed
    }
}
