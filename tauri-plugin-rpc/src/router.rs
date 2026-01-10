//! Router implementation with builder pattern

use crate::{
    handler::{into_boxed, BoxedHandler, Handler},
    middleware::{MiddlewareFn, Next, ProcedureType, Request, Response},
    plugin::DynRouter,
    subscription::{
        into_boxed_subscription, BoxedSubscriptionHandler, Event, SubscriptionContext,
        SubscriptionHandler,
    },
    Context, EmptyContext, RpcError, RpcResult,
};
use serde::{de::DeserializeOwned, Serialize};
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
        self.middleware.push(Arc::new(move |ctx, req, next| {
            Box::pin(f(ctx, req, next))
        }));
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
        matches!(self.procedures.get(path), Some(Procedure::Subscription { .. }))
    }

    /// Call a procedure by path
    pub async fn call(&self, path: &str, input: serde_json::Value) -> RpcResult<serde_json::Value> {
        let procedure = self
            .procedures
            .get(path)
            .ok_or_else(|| RpcError::procedure_not_found(path))?;

        match procedure {
            Procedure::Handler { handler, procedure_type } => {
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
                let chain = self.middleware.iter().rev().fold(final_handler, |next, mw| {
                    let mw = mw.clone();
                    Arc::new(move |ctx, req| {
                        let mw = mw.clone();
                        let next = next.clone();
                        Box::pin(async move { (mw)(ctx, req, next).await })
                    })
                });

                chain(ctx, request).await
            }
            Procedure::Subscription { .. } => {
                Err(RpcError::bad_request(
                    "Cannot call subscription procedure with 'call'. Use 'subscribe' instead.",
                ))
            }
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
            Procedure::Handler { .. } => {
                Err(RpcError::bad_request(
                    "Cannot subscribe to non-subscription procedure. Use 'call' instead.",
                ))
            }
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
    ) -> Pin<Box<dyn Future<Output = RpcResult<mpsc::Receiver<Event<serde_json::Value>>>> + Send + 'a>> {
        Box::pin(async move { Router::subscribe(self, path, input, ctx).await })
    }
}
