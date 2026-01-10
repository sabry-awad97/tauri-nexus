//! Middleware support for request/response processing

use crate::{Context, RpcResult};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Type of procedure being called
#[derive(Clone, Debug, PartialEq, Eq)]
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
        self.path.split('.').last().unwrap_or(&self.path)
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
    dyn Fn(Context<Ctx>, Request, Next<Ctx>) -> Pin<Box<dyn Future<Output = RpcResult<Response>> + Send>>
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

/// Create middleware from an async function
/// 
/// # Example
/// ```rust,ignore
/// async fn logging<Ctx>(ctx: Context<Ctx>, req: Request, next: Next<Ctx>) -> RpcResult<Response> {
///     println!("[{}] {}", req.procedure_type, req.path);
///     next(ctx, req).await
/// }
/// ```
pub fn from_fn<Ctx, F, Fut>(f: F) -> MiddlewareFn<Ctx>
where
    Ctx: Clone + Send + Sync + 'static,
    F: Fn(Context<Ctx>, Request, Next<Ctx>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = RpcResult<Response>> + Send + 'static,
{
    Arc::new(move |ctx, req, next| Box::pin(f(ctx, req, next)))
}
