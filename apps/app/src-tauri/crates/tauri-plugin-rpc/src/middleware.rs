//! Middleware support for request/response processing

use crate::{Context, RpcResult};
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Type of procedure being called
#[derive(Clone, Debug, PartialEq, Eq, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum ProcedureType {
    /// Read-only operation
    Query,
    /// Write operation
    Mutation,
    /// Streaming subscription
    Subscription,
}

impl std::fmt::Display for ProcedureType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Query => write!(f, "query"),
            Self::Mutation => write!(f, "mutation"),
            Self::Subscription => write!(f, "subscription"),
        }
    }
}

/// Request information passed to middleware
#[derive(Clone, Debug)]
pub struct Request {
    /// Full path of the procedure (e.g., "users.get")
    pub path: String,
    /// Type of procedure
    pub procedure_type: ProcedureType,
    /// Input data as JSON
    pub input: serde_json::Value,
}

impl Request {
    /// Get the namespace (first part of path)
    pub fn namespace(&self) -> Option<&str> {
        self.path.split('.').next()
    }

    /// Get the procedure name (last part of path)
    pub fn procedure(&self) -> &str {
        self.path.split('.').next_back().unwrap_or(&self.path)
    }
}

/// Response type (JSON value)
pub type Response = serde_json::Value;

/// Next function in the middleware chain
pub type Next<Ctx> = Arc<
    dyn Fn(Context<Ctx>, Request) -> Pin<Box<dyn Future<Output = RpcResult<Response>> + Send>>
        + Send
        + Sync,
>;

/// Middleware function type
pub type MiddlewareFn<Ctx> = Arc<
    dyn Fn(
            Context<Ctx>,
            Request,
            Next<Ctx>,
        ) -> Pin<Box<dyn Future<Output = RpcResult<Response>> + Send>>
        + Send
        + Sync,
>;

/// Trait for implementing custom middleware
pub trait Middleware<Ctx: Clone + Send + Sync + 'static>: Send + Sync {
    /// Process the request, optionally calling next
    fn handle(
        &self,
        ctx: Context<Ctx>,
        req: Request,
        next: Next<Ctx>,
    ) -> Pin<Box<dyn Future<Output = RpcResult<Response>> + Send>>;
}

/// Implement Middleware for function types to enable conversion
///
/// This implementation allows both regular functions and closures to be used as middleware.
///
/// # Example with function
/// ```rust,ignore
/// async fn logging<Ctx>(ctx: Context<Ctx>, req: Request, next: Next<Ctx>) -> RpcResult<Response> {
///     println!("[{}] {}", req.procedure_type, req.path);
///     next(ctx, req).await
/// }
///
/// // Use function directly as middleware
/// let router = Router::new().middleware(logging);
/// ```
///
/// # Example with closure
/// ```rust,ignore
/// let log_prefix = "API";
///
/// // Async closure can be used directly as middleware
/// let router = Router::new().middleware(
///     move |ctx: Context<MyCtx>, req: Request, next: Next<MyCtx>| async move {
///         println!("[{}] [{}] {}", log_prefix, req.procedure_type, req.path);
///         next(ctx, req).await
///     }
/// );
/// ```
///
/// # Example with multiple closures
/// ```rust,ignore
/// let router = Router::new()
///     .middleware(move |ctx, req, next| async move {
///         // First middleware
///         println!("Before: {}", req.path);
///         let result = next(ctx, req).await;
///         println!("After: {}", req.path);
///         result
///     })
///     .middleware(move |ctx, req, next| async move {
///         // Second middleware
///         let start = std::time::Instant::now();
///         let result = next(ctx, req).await;
///         println!("Duration: {:?}", start.elapsed());
///         result
///     });
/// ```
impl<Ctx, F, Fut> Middleware<Ctx> for F
where
    Ctx: Clone + Send + Sync + 'static,
    F: Fn(Context<Ctx>, Request, Next<Ctx>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = RpcResult<Response>> + Send + 'static,
{
    fn handle(
        &self,
        ctx: Context<Ctx>,
        req: Request,
        next: Next<Ctx>,
    ) -> Pin<Box<dyn Future<Output = RpcResult<Response>> + Send>> {
        Box::pin(self(ctx, req, next))
    }
}

/// Create middleware from an async function
///
/// # Example
/// ```rust,ignore
/// async fn logging<Ctx>(ctx: Context<Ctx>, req: Request, next: Next<Ctx>) -> RpcResult<Response> {
///     println!("[{}] {}", req.procedure_type, req.path);
///     next(ctx, req).await
/// }
///
/// let middleware = from_fn(logging);
/// ```
pub fn from_fn<Ctx, F, Fut>(f: F) -> MiddlewareFn<Ctx>
where
    Ctx: Clone + Send + Sync + 'static,
    F: Fn(Context<Ctx>, Request, Next<Ctx>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = RpcResult<Response>> + Send + 'static,
{
    Arc::new(move |ctx, req, next| Box::pin(f(ctx, req, next)))
}
