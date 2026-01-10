//! Middleware support

use crate::{Context, RpcResult};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Request info passed to middleware
#[derive(Clone, Debug)]
pub struct Request {
    pub path: String,
    pub procedure_type: ProcedureType,
    pub input: serde_json::Value,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ProcedureType {
    Query,
    Mutation,
}

/// Response from a procedure
pub type Response = serde_json::Value;

/// Next function to call the next middleware or handler
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

/// Middleware trait for custom middleware implementations
pub trait Middleware<Ctx: Clone + Send + Sync + 'static>: Send + Sync {
    fn call(
        &self,
        ctx: Context<Ctx>,
        req: Request,
        next: Next<Ctx>,
    ) -> Pin<Box<dyn Future<Output = RpcResult<Response>> + Send>>;
}

/// Create a middleware from a function
pub fn middleware_fn<Ctx, F, Fut>(f: F) -> MiddlewareFn<Ctx>
where
    Ctx: Clone + Send + Sync + 'static,
    F: Fn(Context<Ctx>, Request, Next<Ctx>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = RpcResult<Response>> + Send + 'static,
{
    Arc::new(move |ctx, req, next| Box::pin(f(ctx, req, next)))
}
