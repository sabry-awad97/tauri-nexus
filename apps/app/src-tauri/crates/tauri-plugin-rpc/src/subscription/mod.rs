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
//! - `context` - Subscription context and cancellation signals
//! - `errors` - Error types for the subscription system
//! - `event` - Event types and metadata
//! - `handler` - Subscription handler trait and conversions
//! - `id` - Subscription ID types and generation
//! - `manager` - Subscription manager and health status
//! - `metrics` - Metrics collection and reporting
//! - `publisher` - Event publishers and subscribers
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
mod context;
mod errors;
mod event;
mod handler;
mod id;
mod lifecycle;
mod manager;
mod metrics;
mod publisher;
mod retry_delay;

pub use backpressure::{BackpressureStrategy, BatchPublishResult};
pub use config::{Capacity, ManagerConfig, SubscriptionConfig};
pub use context::{CancellationReason, CancellationSignal, SubscriptionContext};
pub use errors::{ManagerError, ParseError, PublishResult, ValidationError};
pub use event::{Event, EventMeta, SubscriptionEvent, with_event_meta};
pub use handler::{
    BoxedSubscriptionHandler, SubscriptionHandler, SubscriptionResult, into_boxed_subscription,
};
pub use id::{SubscriptionId, generate_subscription_id};
pub use lifecycle::{
    CompletionReason, LifecycleMetrics, SubscriptionState, handle_subscription_events,
    subscription_event_name,
};
pub use manager::{
    HealthStatus, ShutdownResult, SubscriptionHandle, SubscriptionHandleBuilder,
    SubscriptionManager,
};
pub use metrics::{
    MetricsSnapshot, PublisherMetrics, PublisherMetricsSnapshot, SubscriberMetrics,
    SubscriberMetricsSnapshot, SubscriptionMetrics,
};
pub use publisher::{
    ChannelPublisher, EventPublisher, EventSender, EventStream, EventSubscriber, event_channel,
};
pub use retry_delay::RetryDelay;

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    mod integration;
    mod unit;
}
