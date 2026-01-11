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
    procedure::RegisteredProcedure,
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
            procedure_type: compiled.procedure_type,
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
    pub fn middleware_fn(mut self, middleware: MiddlewareFn<Ctx>) -> Self {
        self.middleware.push(middleware);
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
    pub fn procedure<N: Into<String>>(self, path: N) -> ProcedureChain<Ctx> {
        ProcedureChain {
            router: self,
            path: path.into(),
            middleware: Vec::new(),
            output_transformer: None,
        }
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
                    procedure_type: *procedure_type,
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

// =============================================================================
// Procedure Chain (Fluent API)
// =============================================================================

/// A fluent builder for configuring and registering a procedure on a router.
///
/// This struct is returned by `Router::procedure()` and allows you to configure
/// per-procedure middleware, input validation, and output transformation before
/// registering the procedure as a query or mutation.
///
/// # Example
/// ```rust,ignore
/// let router = Router::new()
///     .context(AppContext::default())
///     .procedure("users.get")
///         .use_middleware(auth_middleware)
///         .input::<GetUserInput>()
///         .query(get_user);
/// ```
pub struct ProcedureChain<Ctx: Clone + Send + Sync + 'static> {
    router: Router<Ctx>,
    path: String,
    middleware: Vec<MiddlewareFn<Ctx>>,
    output_transformer:
        Option<Arc<dyn Fn(serde_json::Value) -> serde_json::Value + Send + Sync + 'static>>,
}

impl<Ctx: Clone + Send + Sync + 'static> ProcedureChain<Ctx> {
    /// Add middleware to this procedure.
    ///
    /// Middleware is executed in registration order (first registered = outermost).
    pub fn use_middleware<F, Fut>(mut self, middleware: F) -> Self
    where
        F: Fn(Context<Ctx>, Request, Next<Ctx>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = RpcResult<Response>> + Send + 'static,
    {
        self.middleware.push(Arc::new(move |ctx, req, next| {
            Box::pin(middleware(ctx, req, next))
        }));
        self
    }

    /// Add a middleware function (already wrapped as MiddlewareFn).
    pub fn use_middleware_fn(mut self, middleware: MiddlewareFn<Ctx>) -> Self {
        self.middleware.push(middleware);
        self
    }

    /// Set the input type for this procedure.
    ///
    /// Returns a `TypedProcedureChain` that allows you to register the procedure
    /// as a query or mutation with the specified input type.
    pub fn input<Input>(self) -> TypedProcedureChain<Ctx, Input>
    where
        Input: DeserializeOwned + Send + 'static,
    {
        TypedProcedureChain {
            router: self.router,
            path: self.path,
            middleware: self.middleware,
            output_transformer: self.output_transformer,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Set the input type with validation for this procedure.
    ///
    /// Returns a `ValidatedProcedureChain` that automatically validates input
    /// before calling the handler.
    pub fn input_validated<Input>(self) -> ValidatedProcedureChain<Ctx, Input>
    where
        Input: DeserializeOwned + Validate + Send + 'static,
    {
        ValidatedProcedureChain {
            router: self.router,
            path: self.path,
            middleware: self.middleware,
            output_transformer: self.output_transformer,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Set an output transformer for this procedure.
    pub fn output<F>(mut self, transformer: F) -> Self
    where
        F: Fn(serde_json::Value) -> serde_json::Value + Send + Sync + 'static,
    {
        self.output_transformer = Some(Arc::new(transformer));
        self
    }

    /// Transform the context type before the handler executes.
    ///
    /// This method allows you to enrich or transform the context with additional
    /// data before the handler is called. The transformer function receives the
    /// original context and returns a new context type.
    ///
    /// # Example
    /// ```rust,ignore
    /// let router = Router::new()
    ///     .context(AppContext::default())
    ///     .procedure("users.profile")
    ///         .context(|ctx: Context<AppContext>| async move {
    ///             let user = authenticate(&ctx).await?;
    ///             Ok(AuthContext { app: ctx.inner().clone(), user })
    ///         })
    ///         .input::<GetProfileInput>()
    ///         .query(get_profile); // Handler receives Context<AuthContext>
    /// ```
    pub fn context<NewCtx, F, Fut>(self, transformer: F) -> ContextTransformedChain<Ctx, NewCtx>
    where
        NewCtx: Clone + Send + Sync + 'static,
        F: Fn(Context<Ctx>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = RpcResult<NewCtx>> + Send + 'static,
    {
        ContextTransformedChain {
            router: self.router,
            path: self.path,
            middleware: self.middleware,
            output_transformer: self.output_transformer,
            context_transformer: Arc::new(move |ctx| Box::pin(transformer(ctx))),
            _phantom: std::marker::PhantomData,
        }
    }

    /// Register this procedure as a query with no input (unit type).
    pub fn query<H, Fut, Output>(self, handler: H) -> Router<Ctx>
    where
        H: Fn(Context<Ctx>, ()) -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = RpcResult<Output>> + Send + 'static,
        Output: Serialize + Send + 'static,
    {
        self.input::<()>().query(handler)
    }

    /// Register this procedure as a mutation with no input (unit type).
    pub fn mutation<H, Fut, Output>(self, handler: H) -> Router<Ctx>
    where
        H: Fn(Context<Ctx>, ()) -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = RpcResult<Output>> + Send + 'static,
        Output: Serialize + Send + 'static,
    {
        self.input::<()>().mutation(handler)
    }
}

/// A typed procedure chain with a specific input type.
pub struct TypedProcedureChain<Ctx: Clone + Send + Sync + 'static, Input> {
    router: Router<Ctx>,
    path: String,
    middleware: Vec<MiddlewareFn<Ctx>>,
    output_transformer:
        Option<Arc<dyn Fn(serde_json::Value) -> serde_json::Value + Send + Sync + 'static>>,
    _phantom: std::marker::PhantomData<Input>,
}

impl<Ctx: Clone + Send + Sync + 'static, Input: DeserializeOwned + Send + 'static>
    TypedProcedureChain<Ctx, Input>
{
    /// Add middleware to this procedure.
    pub fn use_middleware<F, Fut>(mut self, middleware: F) -> Self
    where
        F: Fn(Context<Ctx>, Request, Next<Ctx>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = RpcResult<Response>> + Send + 'static,
    {
        self.middleware.push(Arc::new(move |ctx, req, next| {
            Box::pin(middleware(ctx, req, next))
        }));
        self
    }

    /// Set an output transformer for this procedure.
    pub fn output<F>(mut self, transformer: F) -> Self
    where
        F: Fn(serde_json::Value) -> serde_json::Value + Send + Sync + 'static,
    {
        self.output_transformer = Some(Arc::new(transformer));
        self
    }

    /// Register this procedure as a query.
    pub fn query<H, Fut, Output>(self, handler: H) -> Router<Ctx>
    where
        H: Fn(Context<Ctx>, Input) -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = RpcResult<Output>> + Send + 'static,
        Output: Serialize + Send + 'static,
    {
        self.build_procedure(ProcedureType::Query, handler)
    }

    /// Register this procedure as a mutation.
    pub fn mutation<H, Fut, Output>(self, handler: H) -> Router<Ctx>
    where
        H: Fn(Context<Ctx>, Input) -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = RpcResult<Output>> + Send + 'static,
        Output: Serialize + Send + 'static,
    {
        self.build_procedure(ProcedureType::Mutation, handler)
    }

    fn build_procedure<H, Fut, Output>(
        mut self,
        procedure_type: ProcedureType,
        handler: H,
    ) -> Router<Ctx>
    where
        H: Fn(Context<Ctx>, Input) -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = RpcResult<Output>> + Send + 'static,
        Output: Serialize + Send + 'static,
    {
        let output_transformer = self.output_transformer;
        let middleware = self.middleware;

        // Create the core handler
        let core_handler: BoxedHandler<Ctx> = Arc::new(move |ctx, input_value| {
            let handler = handler.clone();
            let output_transformer = output_transformer.clone();

            Box::pin(async move {
                // Deserialize input
                let input: Input = serde_json::from_value(input_value)
                    .map_err(|e| RpcError::bad_request(format!("Invalid input: {}", e)))?;

                // Call handler
                let output = handler(ctx, input).await?;

                // Serialize output
                let mut output_value = serde_json::to_value(output).map_err(|e| {
                    RpcError::internal(format!("Failed to serialize output: {}", e))
                })?;

                // Apply output transformer if present
                if let Some(transformer) = output_transformer {
                    output_value = transformer(output_value);
                }

                Ok(output_value)
            })
        });

        // Wrap with per-procedure middleware if any
        let final_handler: BoxedHandler<Ctx> = if middleware.is_empty() {
            core_handler
        } else {
            let handler_as_next: Next<Ctx> = Arc::new(move |ctx, req| {
                let handler = core_handler.clone();
                Box::pin(async move { (handler)(ctx, req.input).await })
            });

            // Chain middleware in reverse order (last added = innermost)
            let mut chain = handler_as_next;
            for mw in middleware.into_iter().rev() {
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
                        procedure_type,
                    };
                    (chain)(ctx, req).await
                })
            })
        };

        let full_path = self.router.make_path(&self.path);
        self.router.procedures.insert(
            full_path,
            Procedure::Handler {
                handler: final_handler,
                procedure_type,
            },
        );
        self.router
    }
}

/// A validated procedure chain with automatic input validation.
pub struct ValidatedProcedureChain<Ctx: Clone + Send + Sync + 'static, Input> {
    router: Router<Ctx>,
    path: String,
    middleware: Vec<MiddlewareFn<Ctx>>,
    output_transformer:
        Option<Arc<dyn Fn(serde_json::Value) -> serde_json::Value + Send + Sync + 'static>>,
    _phantom: std::marker::PhantomData<Input>,
}

impl<Ctx: Clone + Send + Sync + 'static, Input: DeserializeOwned + Validate + Send + 'static>
    ValidatedProcedureChain<Ctx, Input>
{
    /// Add middleware to this procedure.
    pub fn use_middleware<F, Fut>(mut self, middleware: F) -> Self
    where
        F: Fn(Context<Ctx>, Request, Next<Ctx>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = RpcResult<Response>> + Send + 'static,
    {
        self.middleware.push(Arc::new(move |ctx, req, next| {
            Box::pin(middleware(ctx, req, next))
        }));
        self
    }

    /// Set an output transformer for this procedure.
    pub fn output<F>(mut self, transformer: F) -> Self
    where
        F: Fn(serde_json::Value) -> serde_json::Value + Send + Sync + 'static,
    {
        self.output_transformer = Some(Arc::new(transformer));
        self
    }

    /// Register this procedure as a query with validation.
    pub fn query<H, Fut, Output>(self, handler: H) -> Router<Ctx>
    where
        H: Fn(Context<Ctx>, Input) -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = RpcResult<Output>> + Send + 'static,
        Output: Serialize + Send + 'static,
    {
        self.build_validated_procedure(ProcedureType::Query, handler)
    }

    /// Register this procedure as a mutation with validation.
    pub fn mutation<H, Fut, Output>(self, handler: H) -> Router<Ctx>
    where
        H: Fn(Context<Ctx>, Input) -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = RpcResult<Output>> + Send + 'static,
        Output: Serialize + Send + 'static,
    {
        self.build_validated_procedure(ProcedureType::Mutation, handler)
    }

    fn build_validated_procedure<H, Fut, Output>(
        mut self,
        procedure_type: ProcedureType,
        handler: H,
    ) -> Router<Ctx>
    where
        H: Fn(Context<Ctx>, Input) -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = RpcResult<Output>> + Send + 'static,
        Output: Serialize + Send + 'static,
    {
        let output_transformer = self.output_transformer;
        let middleware = self.middleware;

        // Create the core handler with validation
        let core_handler: BoxedHandler<Ctx> = Arc::new(move |ctx, input_value| {
            let handler = handler.clone();
            let output_transformer = output_transformer.clone();

            Box::pin(async move {
                // Deserialize input
                let input: Input = serde_json::from_value(input_value)
                    .map_err(|e| RpcError::bad_request(format!("Invalid input: {}", e)))?;

                // Validate input
                let validation_result = input.validate();
                if !validation_result.is_valid() {
                    return Err(RpcError::validation("Validation failed").with_details(
                        serde_json::to_value(&validation_result.errors).unwrap_or_default(),
                    ));
                }

                // Call handler
                let output = handler(ctx, input).await?;

                // Serialize output
                let mut output_value = serde_json::to_value(output).map_err(|e| {
                    RpcError::internal(format!("Failed to serialize output: {}", e))
                })?;

                // Apply output transformer if present
                if let Some(transformer) = output_transformer {
                    output_value = transformer(output_value);
                }

                Ok(output_value)
            })
        });

        // Wrap with per-procedure middleware if any
        let final_handler: BoxedHandler<Ctx> = if middleware.is_empty() {
            core_handler
        } else {
            let handler_as_next: Next<Ctx> = Arc::new(move |ctx, req| {
                let handler = core_handler.clone();
                Box::pin(async move { (handler)(ctx, req.input).await })
            });

            // Chain middleware in reverse order (last added = innermost)
            let mut chain = handler_as_next;
            for mw in middleware.into_iter().rev() {
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
                        procedure_type,
                    };
                    (chain)(ctx, req).await
                })
            })
        };

        let full_path = self.router.make_path(&self.path);
        self.router.procedures.insert(
            full_path,
            Procedure::Handler {
                handler: final_handler,
                procedure_type,
            },
        );
        self.router
    }
}

// =============================================================================
// Context Transformed Chain (Fluent API with context transformation)
// =============================================================================

/// Type alias for a context transformer function in the router.
type RouterContextTransformer<FromCtx, ToCtx> = Arc<
    dyn Fn(Context<FromCtx>) -> Pin<Box<dyn Future<Output = RpcResult<ToCtx>> + Send>>
        + Send
        + Sync,
>;

/// A procedure chain with context transformation.
///
/// This struct is returned by `ProcedureChain::context()` and allows you to
/// transform the context type before the handler executes.
pub struct ContextTransformedChain<
    OrigCtx: Clone + Send + Sync + 'static,
    NewCtx: Clone + Send + Sync + 'static,
> {
    router: Router<OrigCtx>,
    path: String,
    middleware: Vec<MiddlewareFn<OrigCtx>>,
    output_transformer:
        Option<Arc<dyn Fn(serde_json::Value) -> serde_json::Value + Send + Sync + 'static>>,
    context_transformer: RouterContextTransformer<OrigCtx, NewCtx>,
    _phantom: std::marker::PhantomData<NewCtx>,
}

impl<OrigCtx: Clone + Send + Sync + 'static, NewCtx: Clone + Send + Sync + 'static>
    ContextTransformedChain<OrigCtx, NewCtx>
{
    /// Add middleware to this procedure.
    ///
    /// Note: Middleware operates on the original context type, before transformation.
    pub fn use_middleware<F, Fut>(mut self, middleware: F) -> Self
    where
        F: Fn(Context<OrigCtx>, Request, Next<OrigCtx>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = RpcResult<Response>> + Send + 'static,
    {
        self.middleware.push(Arc::new(move |ctx, req, next| {
            Box::pin(middleware(ctx, req, next))
        }));
        self
    }

    /// Set an output transformer for this procedure.
    pub fn output<F>(mut self, transformer: F) -> Self
    where
        F: Fn(serde_json::Value) -> serde_json::Value + Send + Sync + 'static,
    {
        self.output_transformer = Some(Arc::new(transformer));
        self
    }

    /// Set the input type for this procedure.
    pub fn input<Input>(self) -> ContextTransformedTypedChain<OrigCtx, NewCtx, Input>
    where
        Input: DeserializeOwned + Send + 'static,
    {
        ContextTransformedTypedChain {
            router: self.router,
            path: self.path,
            middleware: self.middleware,
            output_transformer: self.output_transformer,
            context_transformer: self.context_transformer,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Set the input type with validation for this procedure.
    pub fn input_validated<Input>(self) -> ContextTransformedValidatedChain<OrigCtx, NewCtx, Input>
    where
        Input: DeserializeOwned + Validate + Send + 'static,
    {
        ContextTransformedValidatedChain {
            router: self.router,
            path: self.path,
            middleware: self.middleware,
            output_transformer: self.output_transformer,
            context_transformer: self.context_transformer,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Register this procedure as a query with no input (unit type).
    pub fn query<H, Fut, Output>(self, handler: H) -> Router<OrigCtx>
    where
        H: Fn(Context<NewCtx>, ()) -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = RpcResult<Output>> + Send + 'static,
        Output: Serialize + Send + 'static,
    {
        self.input::<()>().query(handler)
    }

    /// Register this procedure as a mutation with no input (unit type).
    pub fn mutation<H, Fut, Output>(self, handler: H) -> Router<OrigCtx>
    where
        H: Fn(Context<NewCtx>, ()) -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = RpcResult<Output>> + Send + 'static,
        Output: Serialize + Send + 'static,
    {
        self.input::<()>().mutation(handler)
    }
}

/// A context-transformed typed procedure chain.
pub struct ContextTransformedTypedChain<
    OrigCtx: Clone + Send + Sync + 'static,
    NewCtx: Clone + Send + Sync + 'static,
    Input,
> {
    router: Router<OrigCtx>,
    path: String,
    middleware: Vec<MiddlewareFn<OrigCtx>>,
    output_transformer:
        Option<Arc<dyn Fn(serde_json::Value) -> serde_json::Value + Send + Sync + 'static>>,
    context_transformer: RouterContextTransformer<OrigCtx, NewCtx>,
    _phantom: std::marker::PhantomData<(NewCtx, Input)>,
}

impl<
    OrigCtx: Clone + Send + Sync + 'static,
    NewCtx: Clone + Send + Sync + 'static,
    Input: DeserializeOwned + Send + 'static,
> ContextTransformedTypedChain<OrigCtx, NewCtx, Input>
{
    /// Add middleware to this procedure.
    pub fn use_middleware<F, Fut>(mut self, middleware: F) -> Self
    where
        F: Fn(Context<OrigCtx>, Request, Next<OrigCtx>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = RpcResult<Response>> + Send + 'static,
    {
        self.middleware.push(Arc::new(move |ctx, req, next| {
            Box::pin(middleware(ctx, req, next))
        }));
        self
    }

    /// Set an output transformer for this procedure.
    pub fn output<F>(mut self, transformer: F) -> Self
    where
        F: Fn(serde_json::Value) -> serde_json::Value + Send + Sync + 'static,
    {
        self.output_transformer = Some(Arc::new(transformer));
        self
    }

    /// Register this procedure as a query.
    pub fn query<H, Fut, Output>(self, handler: H) -> Router<OrigCtx>
    where
        H: Fn(Context<NewCtx>, Input) -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = RpcResult<Output>> + Send + 'static,
        Output: Serialize + Send + 'static,
    {
        self.build_procedure(ProcedureType::Query, handler)
    }

    /// Register this procedure as a mutation.
    pub fn mutation<H, Fut, Output>(self, handler: H) -> Router<OrigCtx>
    where
        H: Fn(Context<NewCtx>, Input) -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = RpcResult<Output>> + Send + 'static,
        Output: Serialize + Send + 'static,
    {
        self.build_procedure(ProcedureType::Mutation, handler)
    }

    fn build_procedure<H, Fut, Output>(
        mut self,
        procedure_type: ProcedureType,
        handler: H,
    ) -> Router<OrigCtx>
    where
        H: Fn(Context<NewCtx>, Input) -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = RpcResult<Output>> + Send + 'static,
        Output: Serialize + Send + 'static,
    {
        let output_transformer = self.output_transformer;
        let context_transformer = self.context_transformer;
        let middleware = self.middleware;

        // Create the core handler with context transformation
        let core_handler: BoxedHandler<OrigCtx> = Arc::new(move |ctx, input_value| {
            let handler = handler.clone();
            let output_transformer = output_transformer.clone();
            let context_transformer = context_transformer.clone();

            Box::pin(async move {
                // Transform context
                let new_ctx_state = (context_transformer)(ctx).await?;
                let new_ctx = Context::new(new_ctx_state);

                // Deserialize input
                let input: Input = serde_json::from_value(input_value)
                    .map_err(|e| RpcError::bad_request(format!("Invalid input: {}", e)))?;

                // Call handler with transformed context
                let output = handler(new_ctx, input).await?;

                // Serialize output
                let mut output_value = serde_json::to_value(output).map_err(|e| {
                    RpcError::internal(format!("Failed to serialize output: {}", e))
                })?;

                // Apply output transformer if present
                if let Some(transformer) = output_transformer {
                    output_value = transformer(output_value);
                }

                Ok(output_value)
            })
        });

        // Wrap with per-procedure middleware if any
        let final_handler: BoxedHandler<OrigCtx> = if middleware.is_empty() {
            core_handler
        } else {
            let handler_as_next: Next<OrigCtx> = Arc::new(move |ctx, req| {
                let handler = core_handler.clone();
                Box::pin(async move { (handler)(ctx, req.input).await })
            });

            // Chain middleware in reverse order (last added = innermost)
            let mut chain = handler_as_next;
            for mw in middleware.into_iter().rev() {
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
                        procedure_type,
                    };
                    (chain)(ctx, req).await
                })
            })
        };

        let full_path = self.router.make_path(&self.path);
        self.router.procedures.insert(
            full_path,
            Procedure::Handler {
                handler: final_handler,
                procedure_type,
            },
        );
        self.router
    }
}

/// A context-transformed validated procedure chain.
pub struct ContextTransformedValidatedChain<
    OrigCtx: Clone + Send + Sync + 'static,
    NewCtx: Clone + Send + Sync + 'static,
    Input,
> {
    router: Router<OrigCtx>,
    path: String,
    middleware: Vec<MiddlewareFn<OrigCtx>>,
    output_transformer:
        Option<Arc<dyn Fn(serde_json::Value) -> serde_json::Value + Send + Sync + 'static>>,
    context_transformer: RouterContextTransformer<OrigCtx, NewCtx>,
    _phantom: std::marker::PhantomData<(NewCtx, Input)>,
}

impl<
    OrigCtx: Clone + Send + Sync + 'static,
    NewCtx: Clone + Send + Sync + 'static,
    Input: DeserializeOwned + Validate + Send + 'static,
> ContextTransformedValidatedChain<OrigCtx, NewCtx, Input>
{
    /// Add middleware to this procedure.
    pub fn use_middleware<F, Fut>(mut self, middleware: F) -> Self
    where
        F: Fn(Context<OrigCtx>, Request, Next<OrigCtx>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = RpcResult<Response>> + Send + 'static,
    {
        self.middleware.push(Arc::new(move |ctx, req, next| {
            Box::pin(middleware(ctx, req, next))
        }));
        self
    }

    /// Set an output transformer for this procedure.
    pub fn output<F>(mut self, transformer: F) -> Self
    where
        F: Fn(serde_json::Value) -> serde_json::Value + Send + Sync + 'static,
    {
        self.output_transformer = Some(Arc::new(transformer));
        self
    }

    /// Register this procedure as a query with validation.
    pub fn query<H, Fut, Output>(self, handler: H) -> Router<OrigCtx>
    where
        H: Fn(Context<NewCtx>, Input) -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = RpcResult<Output>> + Send + 'static,
        Output: Serialize + Send + 'static,
    {
        self.build_validated_procedure(ProcedureType::Query, handler)
    }

    /// Register this procedure as a mutation with validation.
    pub fn mutation<H, Fut, Output>(self, handler: H) -> Router<OrigCtx>
    where
        H: Fn(Context<NewCtx>, Input) -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = RpcResult<Output>> + Send + 'static,
        Output: Serialize + Send + 'static,
    {
        self.build_validated_procedure(ProcedureType::Mutation, handler)
    }

    fn build_validated_procedure<H, Fut, Output>(
        mut self,
        procedure_type: ProcedureType,
        handler: H,
    ) -> Router<OrigCtx>
    where
        H: Fn(Context<NewCtx>, Input) -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = RpcResult<Output>> + Send + 'static,
        Output: Serialize + Send + 'static,
    {
        let output_transformer = self.output_transformer;
        let context_transformer = self.context_transformer;
        let middleware = self.middleware;

        // Create the core handler with context transformation and validation
        let core_handler: BoxedHandler<OrigCtx> = Arc::new(move |ctx, input_value| {
            let handler = handler.clone();
            let output_transformer = output_transformer.clone();
            let context_transformer = context_transformer.clone();

            Box::pin(async move {
                // Transform context
                let new_ctx_state = (context_transformer)(ctx).await?;
                let new_ctx = Context::new(new_ctx_state);

                // Deserialize input
                let input: Input = serde_json::from_value(input_value)
                    .map_err(|e| RpcError::bad_request(format!("Invalid input: {}", e)))?;

                // Validate input
                let validation_result = input.validate();
                if !validation_result.is_valid() {
                    return Err(RpcError::validation("Validation failed").with_details(
                        serde_json::to_value(&validation_result.errors).unwrap_or_default(),
                    ));
                }

                // Call handler with transformed context
                let output = handler(new_ctx, input).await?;

                // Serialize output
                let mut output_value = serde_json::to_value(output).map_err(|e| {
                    RpcError::internal(format!("Failed to serialize output: {}", e))
                })?;

                // Apply output transformer if present
                if let Some(transformer) = output_transformer {
                    output_value = transformer(output_value);
                }

                Ok(output_value)
            })
        });

        // Wrap with per-procedure middleware if any
        let final_handler: BoxedHandler<OrigCtx> = if middleware.is_empty() {
            core_handler
        } else {
            let handler_as_next: Next<OrigCtx> = Arc::new(move |ctx, req| {
                let handler = core_handler.clone();
                Box::pin(async move { (handler)(ctx, req.input).await })
            });

            // Chain middleware in reverse order (last added = innermost)
            let mut chain = handler_as_next;
            for mw in middleware.into_iter().rev() {
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
                        procedure_type,
                    };
                    (chain)(ctx, req).await
                })
            })
        };

        let full_path = self.router.make_path(&self.path);
        self.router.procedures.insert(
            full_path,
            Procedure::Handler {
                handler: final_handler,
                procedure_type,
            },
        );
        self.router
    }
}

#[cfg(test)]
mod procedure_chain_tests {
    use super::*;
    use crate::validation::{FieldError, Validate, ValidationResult};
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Default)]
    struct TestContext {
        value: i32,
    }

    #[derive(Debug, Deserialize)]
    struct TestInput {
        name: String,
    }

    #[derive(Debug, Serialize)]
    struct TestOutput {
        message: String,
    }

    #[derive(Debug, Deserialize)]
    struct ValidatedInput {
        name: String,
        age: i32,
    }

    impl Validate for ValidatedInput {
        fn validate(&self) -> ValidationResult {
            let mut errors = Vec::new();
            if self.name.is_empty() {
                errors.push(FieldError::required("name"));
            }
            if self.age < 0 || self.age > 150 {
                errors.push(FieldError::range("age", 0, 150));
            }
            ValidationResult::from_errors(errors)
        }
    }

    async fn test_handler(_ctx: Context<TestContext>, input: TestInput) -> RpcResult<TestOutput> {
        Ok(TestOutput {
            message: format!("Hello, {}!", input.name),
        })
    }

    async fn validated_handler(
        _ctx: Context<TestContext>,
        input: ValidatedInput,
    ) -> RpcResult<TestOutput> {
        Ok(TestOutput {
            message: format!("Hello, {} (age {})!", input.name, input.age),
        })
    }

    #[tokio::test]
    async fn test_procedure_chain_query() {
        let router = Router::new()
            .context(TestContext::default())
            .procedure("users.get")
            .input::<TestInput>()
            .query(test_handler);

        let result = router
            .call("users.get", serde_json::json!({"name": "World"}))
            .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap()["message"], "Hello, World!");
    }

    #[tokio::test]
    async fn test_procedure_chain_mutation() {
        let router = Router::new()
            .context(TestContext::default())
            .procedure("users.create")
            .input::<TestInput>()
            .mutation(test_handler);

        let result = router
            .call("users.create", serde_json::json!({"name": "Alice"}))
            .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap()["message"], "Hello, Alice!");
    }

    #[tokio::test]
    async fn test_procedure_chain_with_validation() {
        let router = Router::new()
            .context(TestContext::default())
            .procedure("users.create")
            .input_validated::<ValidatedInput>()
            .mutation(validated_handler);

        // Valid input
        let result = router
            .call(
                "users.create",
                serde_json::json!({"name": "Bob", "age": 30}),
            )
            .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap()["message"], "Hello, Bob (age 30)!");

        // Invalid input
        let result = router
            .call("users.create", serde_json::json!({"name": "", "age": 200}))
            .await;
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().code,
            crate::RpcErrorCode::ValidationError
        );
    }

    #[tokio::test]
    async fn test_procedure_chain_with_output_transformer() {
        let router = Router::new()
            .context(TestContext::default())
            .procedure("users.get")
            .input::<TestInput>()
            .output(|value| {
                serde_json::json!({
                    "data": value,
                    "wrapped": true
                })
            })
            .query(test_handler);

        let result = router
            .call("users.get", serde_json::json!({"name": "World"}))
            .await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output["wrapped"], true);
        assert_eq!(output["data"]["message"], "Hello, World!");
    }

    #[tokio::test]
    async fn test_procedure_chain_with_middleware() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};

        let call_count = Arc::new(AtomicUsize::new(0));
        let count_clone = call_count.clone();

        let router = Router::new()
            .context(TestContext::default())
            .procedure("users.get")
            .use_middleware(move |ctx, req, next| {
                let count = count_clone.clone();
                async move {
                    count.fetch_add(1, Ordering::SeqCst);
                    next(ctx, req).await
                }
            })
            .input::<TestInput>()
            .query(test_handler);

        let result = router
            .call("users.get", serde_json::json!({"name": "World"}))
            .await;
        assert!(result.is_ok());
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_procedure_chain_multiple_procedures() {
        let router = Router::new()
            .context(TestContext::default())
            .procedure("users.get")
            .input::<TestInput>()
            .query(test_handler)
            .procedure("users.create")
            .input::<TestInput>()
            .mutation(test_handler);

        // Both procedures should be registered
        let procedures = router.procedures();
        assert!(procedures.contains(&"users.get".to_string()));
        assert!(procedures.contains(&"users.create".to_string()));

        // Both should work
        let result1 = router
            .call("users.get", serde_json::json!({"name": "Get"}))
            .await;
        assert!(result1.is_ok());

        let result2 = router
            .call("users.create", serde_json::json!({"name": "Create"}))
            .await;
        assert!(result2.is_ok());
    }

    #[tokio::test]
    async fn test_backward_compatibility_query() {
        // Old API
        let old_router = Router::new()
            .context(TestContext::default())
            .query("users.get", test_handler);

        // New API
        let new_router = Router::new()
            .context(TestContext::default())
            .procedure("users.get")
            .input::<TestInput>()
            .query(test_handler);

        let input = serde_json::json!({"name": "Test"});

        let old_result = old_router.call("users.get", input.clone()).await;
        let new_result = new_router.call("users.get", input).await;

        assert!(old_result.is_ok());
        assert!(new_result.is_ok());
        assert_eq!(old_result.unwrap(), new_result.unwrap());
    }

    #[tokio::test]
    async fn test_backward_compatibility_mutation() {
        // Old API
        let old_router = Router::new()
            .context(TestContext::default())
            .mutation("users.create", test_handler);

        // New API
        let new_router = Router::new()
            .context(TestContext::default())
            .procedure("users.create")
            .input::<TestInput>()
            .mutation(test_handler);

        let input = serde_json::json!({"name": "Test"});

        let old_result = old_router.call("users.create", input.clone()).await;
        let new_result = new_router.call("users.create", input).await;

        assert!(old_result.is_ok());
        assert!(new_result.is_ok());
        assert_eq!(old_result.unwrap(), new_result.unwrap());
    }

    #[tokio::test]
    async fn test_backward_compatibility_validated() {
        // Old API
        let old_router = Router::new()
            .context(TestContext::default())
            .mutation_validated("users.create", validated_handler);

        // New API
        let new_router = Router::new()
            .context(TestContext::default())
            .procedure("users.create")
            .input_validated::<ValidatedInput>()
            .mutation(validated_handler);

        // Valid input
        let valid_input = serde_json::json!({"name": "Test", "age": 25});
        let old_result = old_router.call("users.create", valid_input.clone()).await;
        let new_result = new_router.call("users.create", valid_input).await;

        assert!(old_result.is_ok());
        assert!(new_result.is_ok());
        assert_eq!(old_result.unwrap(), new_result.unwrap());

        // Invalid input - both should fail with validation error
        let invalid_input = serde_json::json!({"name": "", "age": 200});
        let old_result = old_router.call("users.create", invalid_input.clone()).await;
        let new_result = new_router.call("users.create", invalid_input).await;

        assert!(old_result.is_err());
        assert!(new_result.is_err());
        assert_eq!(old_result.unwrap_err().code, new_result.unwrap_err().code);
    }

    // Context transformation tests

    #[derive(Clone)]
    struct AuthContext {
        user_id: String,
        original_value: i32,
    }

    async fn auth_handler(ctx: Context<AuthContext>, input: TestInput) -> RpcResult<TestOutput> {
        Ok(TestOutput {
            message: format!(
                "Hello, {}! User: {}, Value: {}",
                input.name,
                ctx.inner().user_id,
                ctx.inner().original_value
            ),
        })
    }

    #[tokio::test]
    async fn test_context_transformation_basic() {
        let router = Router::new()
            .context(TestContext { value: 42 })
            .procedure("users.profile")
            .context(|ctx: Context<TestContext>| async move {
                Ok(AuthContext {
                    user_id: "user123".to_string(),
                    original_value: ctx.inner().value,
                })
            })
            .input::<TestInput>()
            .query(auth_handler);

        let result = router
            .call("users.profile", serde_json::json!({"name": "World"}))
            .await;
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap()["message"],
            "Hello, World! User: user123, Value: 42"
        );
    }

    #[tokio::test]
    async fn test_context_transformation_with_error() {
        let router = Router::new()
            .context(TestContext { value: 42 })
            .procedure("users.profile")
            .context(|_ctx: Context<TestContext>| async move {
                Err::<AuthContext, _>(RpcError::unauthorized("Not authenticated"))
            })
            .input::<TestInput>()
            .query(auth_handler);

        let result = router
            .call("users.profile", serde_json::json!({"name": "World"}))
            .await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, crate::RpcErrorCode::Unauthorized);
    }

    #[tokio::test]
    async fn test_context_transformation_with_validation() {
        async fn validated_auth_handler(
            ctx: Context<AuthContext>,
            input: ValidatedInput,
        ) -> RpcResult<TestOutput> {
            Ok(TestOutput {
                message: format!(
                    "Hello, {} (age {})! User: {}",
                    input.name,
                    input.age,
                    ctx.inner().user_id
                ),
            })
        }

        let router = Router::new()
            .context(TestContext { value: 42 })
            .procedure("users.profile")
            .context(|ctx: Context<TestContext>| async move {
                Ok(AuthContext {
                    user_id: "user456".to_string(),
                    original_value: ctx.inner().value,
                })
            })
            .input_validated::<ValidatedInput>()
            .query(validated_auth_handler);

        // Valid input
        let result = router
            .call(
                "users.profile",
                serde_json::json!({"name": "Alice", "age": 30}),
            )
            .await;
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap()["message"],
            "Hello, Alice (age 30)! User: user456"
        );

        // Invalid input - validation should still work
        let result = router
            .call("users.profile", serde_json::json!({"name": "", "age": 200}))
            .await;
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().code,
            crate::RpcErrorCode::ValidationError
        );
    }

    #[tokio::test]
    async fn test_context_transformation_mutation() {
        let router = Router::new()
            .context(TestContext { value: 100 })
            .procedure("users.update")
            .context(|ctx: Context<TestContext>| async move {
                Ok(AuthContext {
                    user_id: "mutator".to_string(),
                    original_value: ctx.inner().value,
                })
            })
            .input::<TestInput>()
            .mutation(auth_handler);

        let result = router
            .call("users.update", serde_json::json!({"name": "Test"}))
            .await;
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap()["message"],
            "Hello, Test! User: mutator, Value: 100"
        );
    }

    #[tokio::test]
    async fn test_context_transformation_with_output_transformer() {
        let router = Router::new()
            .context(TestContext { value: 42 })
            .procedure("users.profile")
            .context(|ctx: Context<TestContext>| async move {
                Ok(AuthContext {
                    user_id: "user789".to_string(),
                    original_value: ctx.inner().value,
                })
            })
            .input::<TestInput>()
            .output(|value| {
                serde_json::json!({
                    "data": value,
                    "transformed": true
                })
            })
            .query(auth_handler);

        let result = router
            .call("users.profile", serde_json::json!({"name": "World"}))
            .await;
        assert!(result.is_ok());

        let output = result.unwrap();
        assert_eq!(output["transformed"], true);
        assert_eq!(
            output["data"]["message"],
            "Hello, World! User: user789, Value: 42"
        );
    }

    #[tokio::test]
    async fn test_context_transformation_with_middleware() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};

        let middleware_count = Arc::new(AtomicUsize::new(0));
        let count_clone = middleware_count.clone();

        let router = Router::new()
            .context(TestContext { value: 42 })
            .procedure("users.profile")
            .use_middleware(move |ctx, req, next| {
                let count = count_clone.clone();
                async move {
                    count.fetch_add(1, Ordering::SeqCst);
                    next(ctx, req).await
                }
            })
            .context(|ctx: Context<TestContext>| async move {
                Ok(AuthContext {
                    user_id: "user_with_mw".to_string(),
                    original_value: ctx.inner().value,
                })
            })
            .input::<TestInput>()
            .query(auth_handler);

        let result = router
            .call("users.profile", serde_json::json!({"name": "World"}))
            .await;
        assert!(result.is_ok());
        assert_eq!(middleware_count.load(Ordering::SeqCst), 1);
        assert_eq!(
            result.unwrap()["message"],
            "Hello, World! User: user_with_mw, Value: 42"
        );
    }
}

#[cfg(test)]
mod procedure_chain_proptests {
    use super::*;
    use proptest::prelude::*;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Default)]
    struct TestContext;

    #[derive(Debug, Deserialize, Serialize)]
    struct EchoInput {
        value: String,
    }

    async fn echo_handler(_ctx: Context<TestContext>, input: EchoInput) -> RpcResult<EchoInput> {
        Ok(input)
    }

    /// Property 10: Procedure Builder Backward Compatibility
    /// The new fluent API should produce identical results to the old API
    /// for equivalent procedure definitions.
    #[test]
    fn prop_backward_compatibility_query_results() {
        proptest!(|(value in "[a-zA-Z0-9]{1,20}")| {
            let rt = tokio::runtime::Runtime::new().unwrap();

            // Old API
            let old_router = Router::new()
                .context(TestContext)
                .query("test", echo_handler);

            // New API
            let new_router = Router::new()
                .context(TestContext)
                .procedure("test")
                .input::<EchoInput>()
                .query(echo_handler);

            let input = serde_json::json!({"value": value});

            let old_result = rt.block_on(old_router.call("test", input.clone()));
            let new_result = rt.block_on(new_router.call("test", input));

            prop_assert!(old_result.is_ok());
            prop_assert!(new_result.is_ok());
            prop_assert_eq!(old_result.unwrap(), new_result.unwrap());
        });
    }

    /// Property: Multiple procedures can be chained
    #[test]
    fn prop_multiple_procedures_registered() {
        proptest!(|(count in 1usize..5)| {
            let mut router = Router::new().context(TestContext);

            for i in 0..count {
                let path = format!("proc{}", i);
                router = router
                    .procedure(&path)
                    .input::<EchoInput>()
                    .query(echo_handler);
            }

            let procedures = router.procedures();
            prop_assert_eq!(procedures.len(), count);

            for i in 0..count {
                let path = format!("proc{}", i);
                prop_assert!(procedures.contains(&path));
            }
        });
    }

    /// Property: Procedure chain preserves procedure type
    #[test]
    fn prop_procedure_type_preserved() {
        proptest!(|(is_query in proptest::bool::ANY)| {
            let router = if is_query {
                Router::new()
                    .context(TestContext)
                    .procedure("test")
                    .input::<EchoInput>()
                    .query(echo_handler)
            } else {
                Router::new()
                    .context(TestContext)
                    .procedure("test")
                    .input::<EchoInput>()
                    .mutation(echo_handler)
            };

            // Both should be callable (not subscriptions)
            prop_assert!(!router.is_subscription("test"));
        });
    }
}
