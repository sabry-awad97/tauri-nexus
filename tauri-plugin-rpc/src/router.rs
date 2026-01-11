//! Router implementation with builder pattern
//!
//! This module provides the [`Router`] and [`CompiledRouter`] types for building
//! and executing RPC procedure handlers.
//!
//! # Router
//!
//! The [`Router`] type uses a builder pattern to configure procedures and middleware:
//!
//! ```rust,ignore
//! let router = Router::new()
//!     .context(AppContext::default())
//!     .middleware(logging)
//!     .query("health", health_handler)
//!     .mutation("create", create_handler)
//!     .subscription("events", events_handler)
//!     .merge("users", users_router());
//! ```
//!
//! # Compiled Router
//!
//! For optimized performance, compile the router to pre-build middleware chains:
//!
//! ```rust,ignore
//! let compiled = router.compile();
//! // Middleware chains are now pre-computed for O(1) execution
//! ```
//!
//! Both `Router` and `CompiledRouter` implement [`DynRouter`] and can be passed
//! to [`init`](crate::init) or [`init_with_config`](crate::init_with_config).

use crate::{
    Context, EmptyContext, RpcError, RpcResult,
    batch::{BatchConfig, BatchRequest, BatchResponse, BatchResult},
    handler::{BoxedHandler, Handler, into_boxed, into_boxed_validated},
    middleware::{MiddlewareFn, Next, ProcedureType, Request, Response},
    plugin::DynRouter,
    subscription::{
        BoxedSubscriptionHandler, Event, SubscriptionContext, SubscriptionHandler,
        into_boxed_subscription,
    },
    validation::Validate,
};
use serde::{Serialize, de::DeserializeOwned};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Procedure definition
enum Procedure<Ctx: Clone + Send + Sync + 'static> {
    /// Query or Mutation procedure
    Handler {
        handler: BoxedHandler<Ctx>,
        procedure_type: ProcedureType,
    },
    /// Subscription procedure
    Subscription {
        handler: BoxedSubscriptionHandler<Ctx>,
    },
}

// =============================================================================
// Compiled Router
// =============================================================================

/// Pre-compiled middleware chain for a procedure
struct CompiledChain<Ctx: Clone + Send + Sync + 'static> {
    /// The final handler wrapped with all middleware
    chain: Next<Ctx>,
    /// Procedure type metadata
    procedure_type: ProcedureType,
}

/// A compiled router with pre-computed middleware chains for optimized execution.
///
/// This struct is created by calling `Router::compile()` and provides O(1) lookup
/// for procedure calls with pre-built middleware chains, eliminating per-request
/// chain construction overhead.
///
/// # Example
/// ```rust,ignore
/// let router = Router::new()
///     .context(AppContext::default())
///     .middleware(logging)
///     .query("health", health_handler)
///     .compile();
///
/// // Use compiled router for optimized calls
/// let result = router.call("health", json!(null)).await;
/// ```
pub struct CompiledRouter<Ctx: Clone + Send + Sync + 'static> {
    /// Application context
    context: Option<Ctx>,
    /// Pre-compiled middleware chains by path (for queries/mutations)
    compiled_chains: HashMap<String, CompiledChain<Ctx>>,
    /// Subscription handlers (subscriptions don't use middleware chains)
    subscriptions: HashMap<String, BoxedSubscriptionHandler<Ctx>>,
}

impl<Ctx: Clone + Send + Sync + 'static> CompiledRouter<Ctx> {
    /// Get the context reference
    pub fn get_context(&self) -> Option<&Ctx> {
        self.context.as_ref()
    }

    /// List all registered procedure paths
    pub fn procedures(&self) -> Vec<String> {
        let mut paths: Vec<_> = self
            .compiled_chains
            .keys()
            .chain(self.subscriptions.keys())
            .cloned()
            .collect();
        paths.sort();
        paths
    }

    /// Check if a path is a subscription
    pub fn is_subscription(&self, path: &str) -> bool {
        self.subscriptions.contains_key(path)
    }

    /// Call a procedure by path using pre-compiled middleware chain
    pub async fn call(&self, path: &str, input: serde_json::Value) -> RpcResult<serde_json::Value> {
        // Check if it's a subscription first
        if self.subscriptions.contains_key(path) {
            return Err(RpcError::bad_request(
                "Cannot call subscription procedure with 'call'. Use 'subscribe' instead.",
            ));
        }

        let compiled = self
            .compiled_chains
            .get(path)
            .ok_or_else(|| RpcError::procedure_not_found(path))?;

        let ctx = Context::new(
            self.context
                .clone()
                .ok_or_else(|| RpcError::internal("Router context not initialized"))?,
        );

        let request = Request {
            path: path.to_string(),
            procedure_type: compiled.procedure_type.clone(),
            input,
        };

        // Use pre-compiled chain directly - no per-request chain building
        (compiled.chain.clone())(ctx, request).await
    }

    /// Subscribe to a streaming procedure
    pub async fn subscribe(
        &self,
        path: &str,
        input: serde_json::Value,
        sub_ctx: SubscriptionContext,
    ) -> RpcResult<mpsc::Receiver<Event<serde_json::Value>>> {
        let handler = self
            .subscriptions
            .get(path)
            .ok_or_else(|| RpcError::procedure_not_found(path))?;

        // Check if it's actually a subscription
        if self.compiled_chains.contains_key(path) {
            return Err(RpcError::bad_request(
                "Cannot subscribe to non-subscription procedure. Use 'call' instead.",
            ));
        }

        let ctx = Context::new(
            self.context
                .clone()
                .ok_or_else(|| RpcError::internal("Router context not initialized"))?,
        );

        (handler)(ctx, sub_ctx, input).await
    }

    /// Execute a batch of RPC calls in parallel.
    ///
    /// This method processes multiple procedure calls in a single operation,
    /// executing them in parallel (if configured) and returning results in
    /// the same order as the input requests.
    ///
    /// # Arguments
    ///
    /// * `batch` - The batch request containing multiple procedure calls
    /// * `config` - Configuration for batch processing (max size, parallel execution)
    ///
    /// # Returns
    ///
    /// A `BatchResponse` containing results for each request in order.
    /// Individual failures do not affect other requests in the batch.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let batch = BatchRequest::new()
    ///     .add("1", "user.get", json!({"id": 1}))
    ///     .add("2", "user.list", json!(null));
    ///
    /// let config = BatchConfig::default();
    /// let response = router.call_batch(batch, &config).await?;
    ///
    /// for result in response.results {
    ///     println!("{}: {:?}", result.id, result.result);
    /// }
    /// ```
    pub async fn call_batch(
        &self,
        batch: BatchRequest,
        config: &BatchConfig,
    ) -> RpcResult<BatchResponse> {
        // Validate batch against configuration
        batch.validate(config)?;

        if config.parallel_execution {
            // Execute all requests in parallel using futures::join_all pattern
            let futures: Vec<_> = batch
                .requests
                .iter()
                .map(|req| {
                    let id = req.id.clone();
                    let path = req.path.clone();
                    let input = req.input.clone();
                    async move {
                        match self.call(&path, input).await {
                            Ok(data) => BatchResult::success(id, data),
                            Err(error) => BatchResult::error(id, error),
                        }
                    }
                })
                .collect();

            let results = futures::future::join_all(futures).await;
            Ok(BatchResponse::new(results))
        } else {
            // Execute requests sequentially
            let mut results = Vec::with_capacity(batch.requests.len());
            for req in batch.requests {
                let result = match self.call(&req.path, req.input).await {
                    Ok(data) => BatchResult::success(req.id, data),
                    Err(error) => BatchResult::error(req.id, error),
                };
                results.push(result);
            }
            Ok(BatchResponse::new(results))
        }
    }
}

// Implement DynRouter for CompiledRouter
impl<Ctx: Clone + Send + Sync + 'static> DynRouter for CompiledRouter<Ctx> {
    fn call<'a>(
        &'a self,
        path: &'a str,
        input: serde_json::Value,
    ) -> Pin<Box<dyn Future<Output = RpcResult<serde_json::Value>> + Send + 'a>> {
        Box::pin(async move { CompiledRouter::call(self, path, input).await })
    }

    fn procedures(&self) -> Vec<String> {
        CompiledRouter::procedures(self)
    }

    fn is_subscription(&self, path: &str) -> bool {
        CompiledRouter::is_subscription(self, path)
    }

    fn subscribe<'a>(
        &'a self,
        path: &'a str,
        input: serde_json::Value,
        ctx: SubscriptionContext,
    ) -> Pin<
        Box<dyn Future<Output = RpcResult<mpsc::Receiver<Event<serde_json::Value>>>> + Send + 'a>,
    > {
        Box::pin(async move { CompiledRouter::subscribe(self, path, input, ctx).await })
    }
}

/// Type-safe router with builder pattern
///
/// # Example
/// ```rust,ignore
/// let router = Router::new()
///     .context(AppContext::default())
///     .middleware(logging)
///     .query("health", health_handler)
///     .mutation("create", create_handler)
///     .subscription("events", events_handler)
///     .merge("users", users_router());
/// ```
pub struct Router<Ctx: Clone + Send + Sync + 'static = EmptyContext> {
    context: Option<Ctx>,
    procedures: HashMap<String, Procedure<Ctx>>,
    middleware: Vec<MiddlewareFn<Ctx>>,
    prefix: String,
}

impl Default for Router<EmptyContext> {
    fn default() -> Self {
        Self::new()
    }
}

impl Router<EmptyContext> {
    /// Create a new router without context
    pub fn new() -> Self {
        Self {
            context: None,
            procedures: HashMap::new(),
            middleware: Vec::new(),
            prefix: String::new(),
        }
    }
}

impl<Ctx: Clone + Send + Sync + 'static> Router<Ctx> {
    /// Set the context for this router
    ///
    /// The context is passed to all handlers and middleware.
    pub fn context<NewCtx: Clone + Send + Sync + 'static>(self, ctx: NewCtx) -> Router<NewCtx> {
        Router {
            context: Some(ctx),
            procedures: HashMap::new(),
            middleware: Vec::new(),
            prefix: self.prefix,
        }
    }

    /// Add middleware to the router
    ///
    /// Middleware is executed in the order it's added.
    pub fn middleware<F, Fut>(mut self, f: F) -> Self
    where
        F: Fn(Context<Ctx>, Request, Next<Ctx>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = RpcResult<Response>> + Send + 'static,
    {
        self.middleware
            .push(Arc::new(move |ctx, req, next| Box::pin(f(ctx, req, next))));
        self
    }

    /// Add a query procedure (read-only operation)
    pub fn query<N, Input, Output, H>(mut self, name: N, handler: H) -> Self
    where
        N: Into<String>,
        Input: DeserializeOwned + Send + 'static,
        Output: Serialize + Send + 'static,
        H: Handler<Ctx, Input, Output>,
    {
        let full_path = self.make_path(&name.into());
        self.procedures.insert(
            full_path,
            Procedure::Handler {
                handler: into_boxed(handler),
                procedure_type: ProcedureType::Query,
            },
        );
        self
    }

    /// Add a mutation procedure (write operation)
    pub fn mutation<N, Input, Output, H>(mut self, name: N, handler: H) -> Self
    where
        N: Into<String>,
        Input: DeserializeOwned + Send + 'static,
        Output: Serialize + Send + 'static,
        H: Handler<Ctx, Input, Output>,
    {
        let full_path = self.make_path(&name.into());
        self.procedures.insert(
            full_path,
            Procedure::Handler {
                handler: into_boxed(handler),
                procedure_type: ProcedureType::Mutation,
            },
        );
        self
    }

    /// Add a query procedure with automatic input validation.
    ///
    /// The input type must implement the `Validate` trait. Validation is
    /// performed automatically before the handler is called. If validation
    /// fails, a `VALIDATION_ERROR` is returned with field-level details.
    ///
    /// # Example
    /// ```rust,ignore
    /// use tauri_plugin_rpc::prelude::*;
    ///
    /// #[derive(Deserialize)]
    /// struct GetUserInput {
    ///     id: String,
    /// }
    ///
    /// impl Validate for GetUserInput {
    ///     fn validate(&self) -> ValidationResult {
    ///         ValidationRules::new()
    ///             .required("id", &self.id)
    ///             .build()
    ///     }
    /// }
    ///
    /// let router = Router::new()
    ///     .context(AppContext::new())
    ///     .query_validated("get_user", get_user_handler);
    /// ```
    pub fn query_validated<N, Input, Output, H>(mut self, name: N, handler: H) -> Self
    where
        N: Into<String>,
        Input: DeserializeOwned + Validate + Send + 'static,
        Output: Serialize + Send + 'static,
        H: Handler<Ctx, Input, Output>,
    {
        let full_path = self.make_path(&name.into());
        self.procedures.insert(
            full_path,
            Procedure::Handler {
                handler: into_boxed_validated(handler),
                procedure_type: ProcedureType::Query,
            },
        );
        self
    }

    /// Add a mutation procedure with automatic input validation.
    ///
    /// The input type must implement the `Validate` trait. Validation is
    /// performed automatically before the handler is called. If validation
    /// fails, a `VALIDATION_ERROR` is returned with field-level details.
    ///
    /// # Example
    /// ```rust,ignore
    /// use tauri_plugin_rpc::prelude::*;
    ///
    /// #[derive(Deserialize)]
    /// struct CreateUserInput {
    ///     name: String,
    ///     email: String,
    /// }
    ///
    /// impl Validate for CreateUserInput {
    ///     fn validate(&self) -> ValidationResult {
    ///         ValidationRules::new()
    ///             .required("name", &self.name)
    ///             .email("email", &self.email)
    ///             .build()
    ///     }
    /// }
    ///
    /// let router = Router::new()
    ///     .context(AppContext::new())
    ///     .mutation_validated("create_user", create_user_handler);
    /// ```
    pub fn mutation_validated<N, Input, Output, H>(mut self, name: N, handler: H) -> Self
    where
        N: Into<String>,
        Input: DeserializeOwned + Validate + Send + 'static,
        Output: Serialize + Send + 'static,
        H: Handler<Ctx, Input, Output>,
    {
        let full_path = self.make_path(&name.into());
        self.procedures.insert(
            full_path,
            Procedure::Handler {
                handler: into_boxed_validated(handler),
                procedure_type: ProcedureType::Mutation,
            },
        );
        self
    }

    /// Add a subscription procedure (streaming)
    ///
    /// # Example
    /// ```rust,ignore
    /// router.subscription("events", |ctx, sub_ctx, input: EventInput| async move {
    ///     let (tx, rx) = event_channel(32);
    ///     
    ///     tokio::spawn(async move {
    ///         loop {
    ///             if sub_ctx.is_cancelled() {
    ///                 break;
    ///             }
    ///             tx.send(Event::new(data)).await.ok();
    ///         }
    ///     });
    ///     
    ///     Ok(rx)
    /// })
    /// ```
    pub fn subscription<N, Input, Output, H>(mut self, name: N, handler: H) -> Self
    where
        N: Into<String>,
        Input: DeserializeOwned + Send + 'static,
        Output: Serialize + Send + 'static,
        H: SubscriptionHandler<Ctx, Input, Output>,
    {
        let full_path = self.make_path(&name.into());
        self.procedures.insert(
            full_path,
            Procedure::Subscription {
                handler: into_boxed_subscription(handler),
            },
        );
        self
    }

    /// Merge another router under a namespace
    ///
    /// # Example
    /// ```rust,ignore
    /// let router = Router::new()
    ///     .merge("users", users_router())
    ///     .merge("posts", posts_router());
    /// // Creates: users.get, users.list, posts.get, posts.list, etc.
    /// ```
    pub fn merge<N: Into<String>>(mut self, namespace: N, other: Router<Ctx>) -> Self {
        let namespace = namespace.into();
        for (path, procedure) in other.procedures {
            let full_path = if namespace.is_empty() {
                path
            } else {
                format!("{}.{}", namespace, path)
            };
            self.procedures.insert(full_path, procedure);
        }
        self.middleware.extend(other.middleware);
        self
    }

    fn make_path(&self, name: &str) -> String {
        if self.prefix.is_empty() {
            name.to_string()
        } else {
            format!("{}.{}", self.prefix, name)
        }
    }

    /// Get the context reference
    pub fn get_context(&self) -> Option<&Ctx> {
        self.context.as_ref()
    }

    /// List all registered procedure paths
    pub fn procedures(&self) -> Vec<String> {
        let mut paths: Vec<_> = self.procedures.keys().cloned().collect();
        paths.sort();
        paths
    }

    /// Check if a path is a subscription
    pub fn is_subscription(&self, path: &str) -> bool {
        matches!(
            self.procedures.get(path),
            Some(Procedure::Subscription { .. })
        )
    }

    /// Compile the router for optimized execution.
    ///
    /// This pre-computes middleware chains for all procedures at build time,
    /// eliminating per-request chain construction overhead. The compiled router
    /// provides O(1) lookup and execution for procedure calls.
    ///
    /// # Example
    /// ```rust,ignore
    /// let router = Router::new()
    ///     .context(AppContext::default())
    ///     .middleware(logging)
    ///     .middleware(auth)
    ///     .query("health", health_handler)
    ///     .mutation("create", create_handler)
    ///     .compile();
    ///
    /// // Middleware chains are pre-built, no per-request overhead
    /// let result = router.call("health", json!(null)).await;
    /// ```
    pub fn compile(self) -> CompiledRouter<Ctx> {
        let mut compiled_chains = HashMap::new();
        let mut subscriptions = HashMap::new();

        for (path, procedure) in self.procedures {
            match procedure {
                Procedure::Handler {
                    handler,
                    procedure_type,
                } => {
                    // Build the handler as the final step
                    let final_handler: Next<Ctx> = Arc::new(move |ctx, req| {
                        let handler = handler.clone();
                        Box::pin(async move { (handler)(ctx, req.input).await })
                    });

                    // Apply middleware in reverse order (last added = innermost)
                    // This builds the chain once at compile time
                    let chain = self
                        .middleware
                        .iter()
                        .rev()
                        .fold(final_handler, |next, mw| {
                            let mw = mw.clone();
                            Arc::new(move |ctx, req| {
                                let mw = mw.clone();
                                let next = next.clone();
                                Box::pin(async move { (mw)(ctx, req, next).await })
                            })
                        });

                    compiled_chains.insert(
                        path,
                        CompiledChain {
                            chain,
                            procedure_type,
                        },
                    );
                }
                Procedure::Subscription { handler } => {
                    // Subscriptions don't use middleware chains
                    subscriptions.insert(path, handler);
                }
            }
        }

        CompiledRouter {
            context: self.context,
            compiled_chains,
            subscriptions,
        }
    }

    /// Call a procedure by path
    pub async fn call(&self, path: &str, input: serde_json::Value) -> RpcResult<serde_json::Value> {
        let procedure = self
            .procedures
            .get(path)
            .ok_or_else(|| RpcError::procedure_not_found(path))?;

        match procedure {
            Procedure::Handler {
                handler,
                procedure_type,
            } => {
                let ctx = Context::new(
                    self.context
                        .clone()
                        .ok_or_else(|| RpcError::internal("Router context not initialized"))?,
                );

                let request = Request {
                    path: path.to_string(),
                    procedure_type: procedure_type.clone(),
                    input: input.clone(),
                };

                // Build the handler as the final step
                let handler = handler.clone();
                let final_handler: Next<Ctx> = Arc::new(move |ctx, req| {
                    let handler = handler.clone();
                    Box::pin(async move { (handler)(ctx, req.input).await })
                });

                // Apply middleware in reverse order (last added = innermost)
                let chain = self
                    .middleware
                    .iter()
                    .rev()
                    .fold(final_handler, |next, mw| {
                        let mw = mw.clone();
                        Arc::new(move |ctx, req| {
                            let mw = mw.clone();
                            let next = next.clone();
                            Box::pin(async move { (mw)(ctx, req, next).await })
                        })
                    });

                chain(ctx, request).await
            }
            Procedure::Subscription { .. } => Err(RpcError::bad_request(
                "Cannot call subscription procedure with 'call'. Use 'subscribe' instead.",
            )),
        }
    }

    /// Subscribe to a streaming procedure
    pub async fn subscribe(
        &self,
        path: &str,
        input: serde_json::Value,
        sub_ctx: SubscriptionContext,
    ) -> RpcResult<mpsc::Receiver<Event<serde_json::Value>>> {
        let procedure = self
            .procedures
            .get(path)
            .ok_or_else(|| RpcError::procedure_not_found(path))?;

        match procedure {
            Procedure::Subscription { handler } => {
                let ctx = Context::new(
                    self.context
                        .clone()
                        .ok_or_else(|| RpcError::internal("Router context not initialized"))?,
                );

                (handler)(ctx, sub_ctx, input).await
            }
            Procedure::Handler { .. } => Err(RpcError::bad_request(
                "Cannot subscribe to non-subscription procedure. Use 'call' instead.",
            )),
        }
    }

    /// Execute a batch of RPC calls in parallel.
    ///
    /// This method processes multiple procedure calls in a single operation,
    /// executing them in parallel (if configured) and returning results in
    /// the same order as the input requests.
    ///
    /// # Arguments
    ///
    /// * `batch` - The batch request containing multiple procedure calls
    /// * `config` - Configuration for batch processing (max size, parallel execution)
    ///
    /// # Returns
    ///
    /// A `BatchResponse` containing results for each request in order.
    /// Individual failures do not affect other requests in the batch.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let batch = BatchRequest::new()
    ///     .add("1", "user.get", json!({"id": 1}))
    ///     .add("2", "user.list", json!(null));
    ///
    /// let config = BatchConfig::default();
    /// let response = router.call_batch(batch, &config).await?;
    ///
    /// for result in response.results {
    ///     println!("{}: {:?}", result.id, result.result);
    /// }
    /// ```
    pub async fn call_batch(
        &self,
        batch: BatchRequest,
        config: &BatchConfig,
    ) -> RpcResult<BatchResponse> {
        // Validate batch against configuration
        batch.validate(config)?;

        if config.parallel_execution {
            // Execute all requests in parallel using futures::join_all pattern
            let futures: Vec<_> = batch
                .requests
                .iter()
                .map(|req| {
                    let id = req.id.clone();
                    let path = req.path.clone();
                    let input = req.input.clone();
                    async move {
                        match self.call(&path, input).await {
                            Ok(data) => BatchResult::success(id, data),
                            Err(error) => BatchResult::error(id, error),
                        }
                    }
                })
                .collect();

            let results = futures::future::join_all(futures).await;
            Ok(BatchResponse::new(results))
        } else {
            // Execute requests sequentially
            let mut results = Vec::with_capacity(batch.requests.len());
            for req in batch.requests {
                let result = match self.call(&req.path, req.input).await {
                    Ok(data) => BatchResult::success(req.id, data),
                    Err(error) => BatchResult::error(req.id, error),
                };
                results.push(result);
            }
            Ok(BatchResponse::new(results))
        }
    }
}

// Implement DynRouter for type erasure
impl<Ctx: Clone + Send + Sync + 'static> DynRouter for Router<Ctx> {
    fn call<'a>(
        &'a self,
        path: &'a str,
        input: serde_json::Value,
    ) -> Pin<Box<dyn Future<Output = RpcResult<serde_json::Value>> + Send + 'a>> {
        Box::pin(async move { Router::call(self, path, input).await })
    }

    fn procedures(&self) -> Vec<String> {
        Router::procedures(self)
    }

    fn is_subscription(&self, path: &str) -> bool {
        Router::is_subscription(self, path)
    }

    fn subscribe<'a>(
        &'a self,
        path: &'a str,
        input: serde_json::Value,
        ctx: SubscriptionContext,
    ) -> Pin<
        Box<dyn Future<Output = RpcResult<mpsc::Receiver<Event<serde_json::Value>>>> + Send + 'a>,
    > {
        Box::pin(async move { Router::subscribe(self, path, input, ctx).await })
    }
}
