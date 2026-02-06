//! Core router implementations
//!
//! This module contains the main `Router` and `CompiledRouter` types.

use super::{
    middleware_chain::build_middleware_chain,
    types::{CompiledChain, Procedure},
};
use crate::{
    Context, EmptyContext, RpcError, RpcResult,
    batch::{BatchConfig, BatchRequest, BatchResponse, BatchResult},
    handler::{BoxedHandler, Handler, into_boxed},
    middleware::{MiddlewareFn, Next, ProcedureType, Request},
    procedure::RegisteredProcedure,
    subscription::{
        BoxedSubscriptionHandler, Event, SubscriptionContext, SubscriptionHandler,
        into_boxed_subscription,
    },
};
use serde::{Serialize, de::DeserializeOwned};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;

// =============================================================================
// Compiled Router
// =============================================================================

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
            tracing::debug!(
                path = %path,
                "Attempted to call subscription procedure"
            );
            return Err(RpcError::bad_request(
                "Cannot call subscription procedure with 'call'. Use 'subscribe' instead.",
            ));
        }

        let compiled = self.compiled_chains.get(path).ok_or_else(|| {
            tracing::debug!(path = %path, "Procedure not found");

            // Provide helpful error with available procedures
            let available: Vec<String> = self.compiled_chains.keys().cloned().collect();
            let mut error = RpcError::procedure_not_found(path);

            if !available.is_empty() {
                error = error.with_details(serde_json::json!({
                    "available_procedures": available,
                    "requested": path
                }));
            }

            error
        })?;

        let ctx = Context::new(
            self.context
                .clone()
                .ok_or_else(|| RpcError::internal("Router context not initialized"))?,
        );

        let request = Request {
            path: path.to_string(),
            procedure_type: compiled.procedure_type,
            input,
        };

        tracing::trace!(
            path = %path,
            procedure_type = %compiled.procedure_type,
            "Executing compiled procedure"
        );

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
        let handler = self.subscriptions.get(path).ok_or_else(|| {
            tracing::debug!(path = %path, "Subscription procedure not found");

            // Provide helpful error with available subscriptions
            let available: Vec<String> = self.subscriptions.keys().cloned().collect();
            let mut error = RpcError::procedure_not_found(path);

            if !available.is_empty() {
                error = error.with_details(serde_json::json!({
                    "available_subscriptions": available,
                    "requested": path
                }));
            }

            error
        })?;

        // Check if it's actually a subscription
        if self.compiled_chains.contains_key(path) {
            tracing::debug!(
                path = %path,
                "Attempted to subscribe to non-subscription procedure"
            );
            return Err(RpcError::bad_request(
                "Cannot subscribe to non-subscription procedure. Use 'call' instead.",
            ));
        }

        let ctx = Context::new(
            self.context
                .clone()
                .ok_or_else(|| RpcError::internal("Router context not initialized"))?,
        );

        tracing::trace!(
            path = %path,
            subscription_id = %sub_ctx.subscription_id,
            "Starting subscription"
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

// =============================================================================
// Router
// =============================================================================

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
    pub(crate) context: Option<Ctx>,
    pub(crate) procedures: HashMap<String, Procedure<Ctx>>,
    pub(crate) middleware: Vec<MiddlewareFn<Ctx>>,
    pub(crate) prefix: String,
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
    #[must_use = "This method returns a new Router and does not modify self"]
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
    #[must_use = "This method returns a new Router and does not modify self"]
    pub fn middleware<F, Fut>(mut self, f: F) -> Self
    where
        F: Fn(Context<Ctx>, Request, Next<Ctx>) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = RpcResult<serde_json::Value>> + Send + 'static,
    {
        self.middleware
            .push(Arc::new(move |ctx, req, next| Box::pin(f(ctx, req, next))));
        self
    }

    /// Add a pre-wrapped middleware function to the router.
    ///
    /// Use this method when you have a `MiddlewareFn<Ctx>` (e.g., from
    /// `auth_middleware` or `rate_limit_middleware`).
    ///
    /// # Example
    /// ```rust,ignore
    /// use tauri_plugin_rpc::auth::auth_middleware;
    ///
    /// let router = Router::new()
    ///     .context(AppContext::default())
    ///     .middleware_fn(auth_middleware(provider))
    ///     .query("protected", handler);
    /// ```
    #[must_use = "This method returns a new Router and does not modify self"]
    pub fn middleware_fn(mut self, middleware: MiddlewareFn<Ctx>) -> Self {
        self.middleware.push(middleware);
        self
    }

    /// Add a query procedure (read-only operation)
    #[must_use = "This method returns a new Router and does not modify self"]
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
    #[must_use = "This method returns a new Router and does not modify self"]
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
    #[must_use = "This method returns a new Router and does not modify self"]
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
    #[must_use = "This method returns a new Router and does not modify self"]
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

    /// Register a pre-built procedure from the ProcedureBuilder API.
    ///
    /// This method allows you to add a `RegisteredProcedure` that was created
    /// using the standalone `ProcedureBuilder`.
    ///
    /// # Example
    /// ```rust,ignore
    /// let get_user_proc = ProcedureBuilder::<AppContext>::new("users.get")
    ///     .use_middleware(auth_middleware)
    ///     .input::<GetUserInput>()
    ///     .query(get_user);
    ///
    /// let router = Router::new()
    ///     .context(AppContext::default())
    ///     .register(get_user_proc);
    /// ```
    #[must_use = "This method returns a new Router and does not modify self"]
    pub fn register(mut self, procedure: RegisteredProcedure<Ctx>) -> Self {
        let full_path = self.make_path(&procedure.path);

        // Create a handler that wraps the procedure's handler with its middleware
        let proc_handler = procedure.handler;
        let proc_middleware = procedure.middleware;

        // Build the middleware chain for this procedure
        let final_handler: BoxedHandler<Ctx> = if proc_middleware.is_empty() {
            // No per-procedure middleware, use handler directly
            Arc::new(move |ctx, input| {
                let handler = proc_handler.clone();
                Box::pin(async move { (handler)(ctx, input).await })
            })
        } else {
            // Build middleware chain
            let handler_as_next: Next<Ctx> = Arc::new(move |ctx, req| {
                let handler = proc_handler.clone();
                Box::pin(async move { (handler)(ctx, req.input).await })
            });

            // Chain middleware in reverse order (last added = innermost)
            let mut chain = handler_as_next;
            for mw in proc_middleware.into_iter().rev() {
                let next = chain;
                chain = Arc::new(move |ctx, req| {
                    let mw = mw.clone();
                    let next = next.clone();
                    Box::pin(async move { (mw)(ctx, req, next).await })
                });
            }

            let final_chain = chain;
            Arc::new(move |ctx, input| {
                let chain = final_chain.clone();
                Box::pin(async move {
                    let req = Request {
                        path: String::new(),
                        input,
                        procedure_type: ProcedureType::Query,
                    };
                    (chain)(ctx, req).await
                })
            })
        };

        self.procedures.insert(
            full_path,
            Procedure::Handler {
                handler: final_handler,
                procedure_type: procedure.procedure_type,
            },
        );
        self
    }

    pub(crate) fn make_path(&self, name: &str) -> String {
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

        let middleware_count = self.middleware.len();

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

                    // Use the shared middleware chain builder
                    // This builds the chain once at compile time
                    let chain = build_middleware_chain(self.middleware.clone(), final_handler);

                    tracing::trace!(
                        path = %path,
                        procedure_type = %procedure_type,
                        "Compiled procedure chain"
                    );

                    compiled_chains.insert(
                        path,
                        CompiledChain {
                            chain,
                            procedure_type,
                        },
                    );
                }
                Procedure::Subscription { handler } => {
                    tracing::trace!(
                        path = %path,
                        "Registered subscription handler"
                    );
                    // Subscriptions don't use middleware chains
                    subscriptions.insert(path, handler);
                }
            }
        }

        tracing::debug!(
            procedures = %compiled_chains.len(),
            subscriptions = %subscriptions.len(),
            middleware = %middleware_count,
            "Router compiled"
        );

        CompiledRouter {
            context: self.context,
            compiled_chains,
            subscriptions,
        }
    }

    /// Call a procedure by path
    pub async fn call(&self, path: &str, input: serde_json::Value) -> RpcResult<serde_json::Value> {
        let procedure = self.procedures.get(path).ok_or_else(|| {
            // Provide helpful error with available procedures
            let available: Vec<String> = self
                .procedures
                .keys()
                .filter(|k| {
                    !matches!(
                        self.procedures.get(*k),
                        Some(Procedure::Subscription { .. })
                    )
                })
                .cloned()
                .collect();

            let mut error = RpcError::procedure_not_found(path);

            if !available.is_empty() {
                error = error.with_details(serde_json::json!({
                    "available_procedures": available,
                    "requested": path
                }));
            }

            error
        })?;

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
                    procedure_type: *procedure_type,
                    input: input.clone(),
                };

                // Build the handler as the final step
                let handler = handler.clone();
                let final_handler: Next<Ctx> = Arc::new(move |ctx, req| {
                    let handler = handler.clone();
                    Box::pin(async move { (handler)(ctx, req.input).await })
                });

                // Use the shared middleware chain builder
                let chain = build_middleware_chain(self.middleware.clone(), final_handler);

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

    /// Start building a procedure with the oRPC-style fluent API.
    ///
    /// This method returns a `ProcedureChain` that allows you to configure
    /// per-procedure middleware, input validation, and output transformation
    /// before registering the procedure as a query or mutation.
    ///
    /// # Example
    /// ```rust,ignore
    /// let router = Router::new()
    ///     .context(AppContext::default())
    ///     .procedure("users.get")
    ///         .use_middleware(auth_middleware)
    ///         .input::<GetUserInput>()
    ///         .query(get_user)
    ///     .procedure("users.create")
    ///         .use_middleware(auth_middleware)
    ///         .use_middleware(rate_limit_middleware)
    ///         .input::<CreateUserInput>()
    ///         .mutation(create_user);
    /// ```
    #[must_use = "This method returns a ProcedureChain that must be used to register a procedure"]
    pub fn procedure<N>(self, path: N) -> super::builder::ProcedureChain<Ctx>
    where
        N: Into<String>,
    {
        super::builder::ProcedureChain {
            router: self,
            path: path.into(),
            middleware: Vec::new(),
            output_transformer: None,
        }
    }
}
