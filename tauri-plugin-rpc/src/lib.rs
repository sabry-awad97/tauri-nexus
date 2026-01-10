//! # Tauri RPC Plugin
//!
//! A type-safe, ORPC-style RPC framework for Tauri applications.
//!
//! ## Features
//!
//! - **Builder pattern** for router configuration
//! - **Context injection** for handlers with dependency injection
//! - **Middleware support** with async/await and onion-model execution
//! - **Nested routers** with namespacing for modular organization
//! - **Type-safe error handling** with structured error codes
//! - **Subscriptions** for real-time streaming with backpressure handling
//! - **Compiled routers** for optimized middleware chain execution
//! - **Configuration** for customizing plugin behavior
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use tauri_plugin_rpc::prelude::*;
//!
//! // Define your application context
//! #[derive(Clone, Default)]
//! struct AppContext {
//!     // Your services here
//! }
//!
//! // Create handlers
//! async fn health_check(_ctx: Context<AppContext>, _: ()) -> RpcResult<String> {
//!     Ok("healthy".to_string())
//! }
//!
//! // Build the router
//! fn create_router() -> Router<AppContext> {
//!     Router::new()
//!         .context(AppContext::default())
//!         .query("health", health_check)
//! }
//!
//! // Initialize the plugin
//! tauri::Builder::default()
//!     .plugin(tauri_plugin_rpc::init(create_router()))
//!     .run(tauri::generate_context!())
//! ```
//!
//! ## Configuration
//!
//! Customize plugin behavior with [`RpcConfig`]:
//!
//! ```rust,ignore
//! use tauri_plugin_rpc::{RpcConfig, BackpressureStrategy};
//!
//! let config = RpcConfig::new()
//!     .with_max_input_size(512 * 1024)  // 512KB max input
//!     .with_channel_buffer(64)           // Larger subscription buffers
//!     .with_backpressure_strategy(BackpressureStrategy::DropOldest)
//!     .with_debug_logging(true);
//!
//! tauri::Builder::default()
//!     .plugin(tauri_plugin_rpc::init_with_config(create_router(), config))
//!     .run(tauri::generate_context!())
//! ```
//!
//! ## Compiled Routers
//!
//! For optimized performance, compile your router to pre-build middleware chains:
//!
//! ```rust,ignore
//! let router = Router::new()
//!     .context(AppContext::default())
//!     .middleware(logging)
//!     .middleware(auth)
//!     .query("health", health_check)
//!     .compile();  // Pre-compute middleware chains
//!
//! tauri::Builder::default()
//!     .plugin(tauri_plugin_rpc::init(router))
//!     .run(tauri::generate_context!())
//! ```
//!
//! ## Error Handling
//!
//! Use [`RpcError`] with type-safe [`RpcErrorCode`] variants:
//!
//! ```rust,ignore
//! use tauri_plugin_rpc::{RpcError, RpcErrorCode};
//!
//! async fn get_user(ctx: Context<AppContext>, id: u32) -> RpcResult<User> {
//!     ctx.db.get_user(id)
//!         .ok_or_else(|| RpcError::not_found(format!("User {} not found", id)))
//! }
//! ```
//!
//! ## Subscriptions
//!
//! Create real-time streaming endpoints:
//!
//! ```rust,ignore
//! use tauri_plugin_rpc::subscription::{event_channel, Event, EventStream, SubscriptionContext};
//!
//! async fn counter_stream(
//!     _ctx: Context<AppContext>,
//!     sub_ctx: SubscriptionContext,
//!     _input: (),
//! ) -> RpcResult<EventStream<i32>> {
//!     let (tx, rx) = event_channel(32);
//!     
//!     tokio::spawn(async move {
//!         let mut count = 0;
//!         while !sub_ctx.is_cancelled() {
//!             tx.send(Event::new(count)).await.ok();
//!             count += 1;
//!             tokio::time::sleep(Duration::from_secs(1)).await;
//!         }
//!     });
//!     
//!     Ok(rx)
//! }
//! ```

mod config;
mod context;
mod error;
mod handler;
pub mod middleware;
mod plugin;
mod router;
pub mod subscription;
pub mod types;

#[cfg(test)]
mod tests;

// Public API
pub use config::{BackpressureStrategy, ConfigValidationError, RpcConfig};
pub use context::{Context, EmptyContext};
pub use error::{RpcError, RpcErrorCode, RpcResult};
pub use handler::Handler;
pub use middleware::{Middleware, Next, ProcedureType, Request};
pub use plugin::{
    DynRouter, SubscribeRequest, SubscriptionFuture, init, init_with_config, validate_input_size,
    validate_path, validate_subscription_id,
};
pub use router::{CompiledRouter, Router};
pub use subscription::{
    CancellationSignal, ChannelPublisher, Event, EventMeta, EventPublisher, EventSender,
    EventStream, EventSubscriber, SubscriptionContext, SubscriptionEvent, SubscriptionHandle,
    SubscriptionHandler, SubscriptionId, SubscriptionManager, event_channel,
    generate_subscription_id, with_event_meta,
};
pub use types::*;

/// Prelude for convenient imports
///
/// Import everything you need with a single use statement:
///
/// ```rust,ignore
/// use tauri_plugin_rpc::prelude::*;
/// ```
pub mod prelude {
    pub use crate::{
        // Configuration
        BackpressureStrategy,
        // Subscription types
        ChannelPublisher,
        // Router
        CompiledRouter,
        ConfigValidationError,
        // Context
        Context,
        EmptyContext,
        Event,
        EventMeta,
        EventPublisher,
        EventSender,
        EventStream,
        // Handler
        Handler,
        // Middleware
        Middleware,
        Next,
        // Common types
        NoInput,
        PaginatedResponse,
        PaginationInput,
        ProcedureType,
        Request,
        Router,
        RpcConfig,
        // Error handling
        RpcError,
        RpcErrorCode,
        RpcResult,
        SubscriptionContext,
        SubscriptionEvent,
        SubscriptionHandler,
        SubscriptionId,
        SubscriptionManager,
        SuccessResponse,
        // Functions
        event_channel,
        generate_subscription_id,
        init,
        init_with_config,
        with_event_meta,
    };
}
