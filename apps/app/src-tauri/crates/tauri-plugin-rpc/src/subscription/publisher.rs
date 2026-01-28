//! Event publishers and subscribers for pub/sub patterns.
//!
//! This module provides broadcast-based event publishing with support for
//! multiple subscribers, backpressure handling, and metrics tracking.

use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};

use crate::RpcError;

use super::{
    BackpressureStrategy, BatchPublishResult, Capacity, Event, PublishResult, PublisherMetrics,
    SubscriberMetrics,
};

// =============================================================================
// Event Stream Types
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

// =============================================================================
// Event Subscriber
// =============================================================================

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
