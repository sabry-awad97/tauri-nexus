//! # Tauri RPC Plugin
//!
//! A type-safe, ORPC-style RPC framework for Tauri applications.
//!
//! ## Features
//! - Builder pattern for router configuration
//! - Context injection for handlers
//! - Middleware support with async/await
//! - Nested routers with namespacing
//! - Type-safe error handling
//!
//! ## Example
//! ```rust,ignore
//! use tauri_plugin_rpc::*;
//!
//! fn create_router() -> Router<AppContext> {
//!     Router::new()
//!         .context(AppContext::default())
//!         .middleware(logging)
//!         .query("health", health_check)
//!         .merge("users", users_router())
//! }
//!
//! tauri::Builder::default()
//!     .plugin(tauri_plugin_rpc::init(create_router()))
//!     .run(tauri::generate_context!())
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
pub use config::RpcConfig;
pub use context::{Context, EmptyContext};
pub use error::{RpcError, RpcErrorCode, RpcResult};
pub use handler::Handler;
pub use middleware::{Middleware, Next, ProcedureType, Request};
pub use plugin::{
    DynRouter, init, init_with_config, validate_input_size, validate_path, validate_subscription_id,
};
pub use router::Router;
pub use subscription::{
    CancellationSignal, ChannelPublisher, Event, EventMeta, EventPublisher, EventSender,
    EventStream, EventSubscriber, SubscriptionContext, SubscriptionEvent, SubscriptionHandle,
    SubscriptionHandler, SubscriptionId, SubscriptionManager, event_channel,
    generate_subscription_id, with_event_meta,
};
pub use types::*;

/// Prelude for convenient imports
pub mod prelude {
    pub use crate::{
        ChannelPublisher,
        Context,
        EmptyContext,
        // Subscription types
        Event,
        EventMeta,
        EventPublisher,
        EventSender,
        EventStream,
        Handler,
        Middleware,
        Next,
        PaginatedResponse,
        PaginationInput,
        ProcedureType,
        Request,
        Router,
        RpcConfig,
        RpcError,
        RpcErrorCode,
        RpcResult,
        SubscriptionContext,
        SubscriptionEvent,
        SubscriptionHandler,
        SubscriptionId,
        SubscriptionManager,
        SuccessResponse,
        event_channel,
        generate_subscription_id,
        init,
        init_with_config,
        with_event_meta,
    };
}
