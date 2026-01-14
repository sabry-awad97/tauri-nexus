#![warn(missing_docs)]
//! # Tauri RPC Plugin
//!
//! A production-ready, type-safe RPC framework for Tauri v2 applications.
//!
//! ## Overview
//!
//! This plugin provides an ORPC-style RPC system with:
//! - **Router-based architecture** for organizing procedures
//! - **Type-safe handlers** with context injection
//! - **Middleware support** with onion-model execution
//! - **Real-time subscriptions** with backpressure handling
//! - **Structured error handling** with typed error codes
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                      Frontend (TypeScript)                  │
//! │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
//! │  │ RPC Client  │  │ React Hooks │  │ Event Iterator      │  │
//! │  └──────┬──────┘  └──────┬──────┘  └──────────┬──────────┘  │
//! └─────────┼────────────────┼───────────────────┼──────────────┘
//!           │                │                   │
//!           ▼                ▼                   ▼
//! ┌─────────────────────────────────────────────────────────────┐
//! │                      Tauri Bridge                           │
//! │  ┌─────────────────────┐  ┌─────────────────────────────┐   │
//! │  │ invoke()            │  │ Event System                │   │
//! │  └──────────┬──────────┘  └──────────────┬──────────────┘   │
//! └─────────────┼────────────────────────────┼──────────────────┘
//!               │                            │
//!               ▼                            ▼
//! ┌─────────────────────────────────────────────────────────────┐
//! │                      Backend (Rust)                         │
//! │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
//! │  │ Plugin      │──│ Router      │──│ Middleware Stack    │  │
//! │  └─────────────┘  └──────┬──────┘  └──────────┬──────────┘  │
//! │                          │                    │             │
//! │                          ▼                    ▼             │
//! │  ┌─────────────────────────────────────────────────────┐    │
//! │  │                    Handlers                         │    │
//! │  │  ┌─────────┐  ┌───────────┐  ┌─────────────────┐    │    │
//! │  │  │ Queries │  │ Mutations │  │ Subscriptions   │    │    │
//! │  │  └─────────┘  └───────────┘  └─────────────────┘    │    │
//! │  └─────────────────────────────────────────────────────┘    │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Quick Start
//!
//! ### 1. Define Your Context
//!
//! ```rust,ignore
//! #[derive(Clone)]
//! pub struct AppContext {
//!     pub db: Arc<RwLock<Database>>,
//! }
//!
//! impl AppContext {
//!     pub fn new() -> Self {
//!         Self { db: Arc::new(RwLock::new(Database::new())) }
//!     }
//! }
//! ```
//!
//! ### 2. Create Handlers
//!
//! ```rust,ignore
//! use tauri_plugin_rpc::prelude::*;
//!
//! // Query - read-only operation
//! async fn get_user(ctx: Context<AppContext>, input: GetUserInput) -> RpcResult<User> {
//!     let db = ctx.db.read().await;
//!     db.get_user(input.id)
//!         .ok_or_else(|| RpcError::not_found("User not found"))
//! }
//!
//! // Mutation - write operation
//! async fn create_user(ctx: Context<AppContext>, input: CreateUserInput) -> RpcResult<User> {
//!     if input.name.is_empty() {
//!         return Err(RpcError::validation("Name is required"));
//!     }
//!     let mut db = ctx.db.write().await;
//!     db.create_user(&input.name, &input.email)
//! }
//!
//! // Handler with no input - use NoInput
//! async fn health_check(_ctx: Context<AppContext>, _: NoInput) -> RpcResult<String> {
//!     Ok("healthy".to_string())
//! }
//! ```
//!
//! ### 3. Build Your Router
//!
//! ```rust,ignore
//! pub fn create_router() -> Router<AppContext> {
//!     Router::new()
//!         .context(AppContext::new())
//!         .middleware(logging)
//!         .query("health", health_check)
//!         .merge("user", user_router())
//! }
//!
//! fn user_router() -> Router<AppContext> {
//!     Router::new()
//!         .context(AppContext::new())
//!         .query("get", get_user)
//!         .query("list", list_users)
//!         .mutation("create", create_user)
//! }
//! ```
//!
//! ### 4. Register the Plugin
//!
//! ```rust,ignore
//! tauri::Builder::default()
//!     .plugin(tauri_plugin_rpc::init(create_router()))
//!     .run(tauri::generate_context!())
//! ```
//!
//! ## Subscriptions
//!
//! Create real-time streaming endpoints for live data:
//!
//! ```rust,ignore
//! use tauri_plugin_rpc::prelude::*;
//! use async_stream::stream;
//! use std::pin::pin;
//! use tokio_stream::StreamExt;
//!
//! async fn counter_stream(
//!     _ctx: Context<AppContext>,
//!     sub_ctx: SubscriptionContext,
//!     input: CounterInput,
//! ) -> RpcResult<EventStream<CounterEvent>> {
//!     let (tx, rx) = event_channel(32);
//!
//!     tokio::spawn(async move {
//!         let event_stream = stream! {
//!             let mut count = input.start;
//!             let mut ticker = tokio::time::interval(
//!                 Duration::from_millis(input.interval_ms)
//!             );
//!
//!             while count < input.start + input.max_count {
//!                 ticker.tick().await;
//!                 yield Event::with_id(
//!                     CounterEvent { count, timestamp: Utc::now().to_rfc3339() },
//!                     format!("counter-{}", count)
//!                 );
//!                 count += 1;
//!             }
//!         };
//!
//!         let mut pinned = pin!(event_stream);
//!         while let Some(event) = pinned.next().await {
//!             if sub_ctx.is_cancelled() { break; }
//!             if tx.send(event).await.is_err() { break; }
//!         }
//!     });
//!
//!     Ok(rx)
//! }
//!
//! // Subscription with no input
//! async fn time_stream(
//!     _ctx: Context<AppContext>,
//!     sub_ctx: SubscriptionContext,
//!     _: NoInput,  // Accepts both {} and null from frontend
//! ) -> RpcResult<EventStream<String>> {
//!     // ...
//! }
//! ```
//!
//! ## Middleware
//!
//! Add cross-cutting concerns like logging, authentication, and rate limiting:
//!
//! ```rust,ignore
//! use tauri_plugin_rpc::middleware::{Request, Response, Next};
//!
//! async fn logging(
//!     ctx: Context<AppContext>,
//!     req: Request,
//!     next: Next<AppContext>,
//! ) -> RpcResult<Response> {
//!     let start = std::time::Instant::now();
//!     println!("→ [{:?}] {}", req.procedure_type, req.path);
//!
//!     let result = next(ctx, req.clone()).await;
//!
//!     match &result {
//!         Ok(_) => println!("← {} ({:?})", req.path, start.elapsed()),
//!         Err(e) => println!("✗ {} - {} ({:?})", req.path, e.code, start.elapsed()),
//!     }
//!
//!     result
//! }
//!
//! async fn auth(
//!     ctx: Context<AppContext>,
//!     req: Request,
//!     next: Next<AppContext>,
//! ) -> RpcResult<Response> {
//!     if req.path == "health" {
//!         return next(ctx, req).await;  // Skip auth for health check
//!     }
//!     
//!     // Validate token...
//!     next(ctx, req).await
//! }
//! ```
//!
//! ## Error Handling
//!
//! Use structured errors with typed error codes:
//!
//! ```rust,ignore
//! // Built-in error constructors
//! RpcError::not_found("User not found")
//! RpcError::bad_request("Invalid request")
//! RpcError::validation("Email is required")
//! RpcError::unauthorized("Not authenticated")
//! RpcError::forbidden("Access denied")
//! RpcError::internal("Something went wrong")
//! RpcError::conflict("User already exists")
//!
//! // With additional details
//! RpcError::validation("Invalid input")
//!     .with_details(json!({ "field": "email", "reason": "invalid format" }))
//! ```
//!
//! ## Configuration
//!
//! Customize plugin behavior:
//!
//! ```rust,ignore
//! use tauri_plugin_rpc::{RpcConfig, BackpressureStrategy};
//!
//! let config = RpcConfig::new()
//!     .with_max_input_size(1024 * 1024)  // 1MB max input
//!     .with_channel_buffer(64)           // Subscription buffer size
//!     .with_backpressure_strategy(BackpressureStrategy::DropOldest)
//!     .with_debug_logging(true);
//!
//! tauri::Builder::default()
//!     .plugin(tauri_plugin_rpc::init_with_config(router, config))
//! ```
//!
//! ## Compiled Routers
//!
//! For production, compile your router to pre-build middleware chains:
//!
//! ```rust,ignore
//! let router = Router::new()
//!     .context(AppContext::new())
//!     .middleware(logging)
//!     .middleware(auth)
//!     .query("health", health_check)
//!     .compile();  // Pre-compute middleware chains for O(1) execution
//!
//! tauri::Builder::default()
//!     .plugin(tauri_plugin_rpc::init(router))
//! ```
//!
//! ## Module Structure
//!
//! - [`Router`] - Router builder for defining procedures
//! - [`CompiledRouter`] - Optimized router with pre-built middleware chains
//! - [`Context`] - Context wrapper for dependency injection
//! - [`Handler`] - Handler trait for procedures
//! - [`middleware`] - Middleware types and execution
//! - [`subscription`] - Subscription system with events and channels
//! - [`RpcError`] - Error types and codes
//! - [`RpcConfig`] - Plugin configuration
//! - [`types`] - Common types (NoInput, SuccessResponse, etc.)
//!
//! ## Prelude
//!
//! Import everything you need with a single statement:
//!
//! ```rust,ignore
//! use tauri_plugin_rpc::prelude::*;
//! ```

pub mod auth;
pub mod batch;
pub mod cache;
mod config;
mod context;
mod error;
mod handler;
pub mod logging;
pub mod middleware;
mod plugin;
pub mod procedure;
pub mod rate_limit;
mod router;
pub mod schema;
pub mod subscription;
pub mod types;
pub mod validation;

#[cfg(test)]
mod tests;

// Public API
pub use auth::{
    AlwaysAuthProvider, AuthConfig, AuthProvider, AuthResult, AuthRule, AuthorizationResult,
    NoAuthProvider, auth_middleware, auth_with_config, requires_roles,
};
pub use batch::{
    BatchConfig, BatchRequest, BatchResponse, BatchResult, BatchResultData, SingleRequest,
};
pub use cache::{
    Cache, CacheConfig, CacheEntry, CacheStats, cache_middleware, generate_cache_key,
    invalidation_middleware,
};
pub use config::{BackpressureStrategy, ConfigValidationError, RpcConfig};
pub use context::{Context, EmptyContext};
pub use error::{
    ComposedTransformer, ErrorCodeMapper, ErrorConfig, ErrorTransformer, LoggingTransformer,
    NoOpTransformer, RpcError, RpcErrorCode, RpcResult,
};
pub use handler::Handler;
pub use logging::{
    AuthLogEvent, CacheLogEvent, JsonLogger, LogConfig, LogEntry, LogLevel, Logger, MetricsLogger,
    RateLimitLogEvent, RequestId, RequestMeta, SubscriptionLogEvent, TracingConfig, TracingLogger,
    log_auth_event, log_batch_request, log_cache_event, log_plugin_init, log_plugin_shutdown,
    log_procedure_registered, log_rate_limit_event, log_router_compiled, log_subscription_event,
    logging_middleware, logging_middleware_with_logger, redact_value,
};
pub use middleware::{Middleware, MiddlewareFn, Next, ProcedureType, Request, from_fn};
pub use plugin::{
    DynRouter, SubscribeRequest, SubscriptionFuture, init, init_with_config, validate_input_size,
    validate_path, validate_subscription_id,
};
pub use procedure::{
    ContextTransformedBuilder, ContextTransformedTypedBuilder, ContextTransformedValidatedBuilder,
    ContextTransformer, ProcedureBuilder, RegisteredProcedure, ValidatedProcedureBuilder,
};
pub use rate_limit::{
    RateLimit, RateLimitConfig, RateLimitStrategy, RateLimitUsage, RateLimiter,
    rate_limit_middleware,
};
pub use router::{
    CompiledRouter, ContextTransformedChain, ContextTransformedTypedChain,
    ContextTransformedValidatedChain, ProcedureChain, Router, TypedProcedureChain,
    ValidatedProcedureChain,
};
pub use schema::{
    OpenApiComponents, OpenApiInfo, OpenApiMediaType, OpenApiOperation, OpenApiPathItem,
    OpenApiRequestBody, OpenApiResponse, OpenApiSchema, ProcedureMeta, ProcedureSchema,
    ProcedureTypeSchema, RouterSchema, SchemaBuilder, TypeSchema,
};
pub use subscription::{
    CancellationSignal, ChannelPublisher, Event, EventMeta, EventPublisher, EventSender,
    EventStream, EventSubscriber, SubscriptionContext, SubscriptionEvent, SubscriptionHandle,
    SubscriptionHandler, SubscriptionId, SubscriptionManager, event_channel,
    generate_subscription_id, with_event_meta,
};
pub use types::*;
pub use validation::{FieldError, Validate, ValidationResult, ValidationRules};

/// Prelude for convenient imports
///
/// Import everything you need with a single use statement:
///
/// ```rust,ignore
/// use tauri_plugin_rpc::prelude::*;
/// ```
pub mod prelude {
    pub use crate::{
        // Auth
        AlwaysAuthProvider,
        AuthConfig,
        // Logging
        AuthLogEvent,
        AuthProvider,
        AuthResult,
        AuthRule,
        AuthorizationResult,
        // Configuration
        BackpressureStrategy,
        // Batch processing
        BatchConfig,
        BatchRequest,
        BatchResponse,
        BatchResult,
        // Cache
        Cache,
        CacheConfig,
        CacheEntry,
        CacheLogEvent,
        CacheStats,
        // Subscription types
        ChannelPublisher,
        // Router
        CompiledRouter,
        ComposedTransformer,
        ConfigValidationError,
        // Context
        Context,
        // Context transformation (ProcedureBuilder)
        ContextTransformedBuilder,
        // Context transformation (Router)
        ContextTransformedChain,
        ContextTransformedTypedBuilder,
        ContextTransformedTypedChain,
        ContextTransformedValidatedBuilder,
        ContextTransformedValidatedChain,
        EmptyContext,
        ErrorCodeMapper,
        ErrorConfig,
        ErrorTransformer,
        Event,
        EventMeta,
        EventPublisher,
        EventSender,
        EventStream,
        // Validation
        FieldError,
        // Handler
        Handler,
        JsonLogger,
        LogConfig,
        LogEntry,
        LogLevel,
        Logger,
        LoggingTransformer,
        MetricsLogger,
        // Middleware
        Middleware,
        Next,
        NoAuthProvider,
        // Common types
        NoInput,
        NoOpTransformer,
        // Schema
        OpenApiSchema,
        PaginatedResponse,
        PaginationInput,
        // Procedure Builder
        ProcedureBuilder,
        ProcedureChain,
        ProcedureMeta,
        ProcedureSchema,
        ProcedureType,
        ProcedureTypeSchema,
        // Rate limiting
        RateLimit,
        RateLimitConfig,
        RateLimitLogEvent,
        RateLimitStrategy,
        RateLimitUsage,
        RateLimiter,
        RegisteredProcedure,
        Request,
        RequestId,
        RequestMeta,
        Router,
        RouterSchema,
        RpcConfig,
        // Error handling
        RpcError,
        RpcErrorCode,
        RpcResult,
        SchemaBuilder,
        SingleRequest,
        SubscriptionContext,
        SubscriptionEvent,
        SubscriptionHandler,
        SubscriptionId,
        SubscriptionLogEvent,
        SubscriptionManager,
        SuccessResponse,
        TracingConfig,
        TracingLogger,
        TypeSchema,
        TypedProcedureChain,
        Validate,
        ValidatedProcedureBuilder,
        ValidatedProcedureChain,
        ValidationResult,
        ValidationRules,
        // Functions
        auth_middleware,
        auth_with_config,
        cache_middleware,
        event_channel,
        generate_cache_key,
        generate_subscription_id,
        init,
        init_with_config,
        invalidation_middleware,
        log_auth_event,
        log_batch_request,
        log_cache_event,
        log_plugin_init,
        log_plugin_shutdown,
        log_procedure_registered,
        log_rate_limit_event,
        log_router_compiled,
        log_subscription_event,
        logging_middleware,
        logging_middleware_with_logger,
        rate_limit_middleware,
        redact_value,
        requires_roles,
        with_event_meta,
    };
}
