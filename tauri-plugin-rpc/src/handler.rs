//! Handler traits and utilities

use crate::{Context, RpcResult};
use serde::{de::DeserializeOwned, Serialize};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Boxed handler for type erasure
pub(crate) type BoxedHandler<Ctx> = Arc<
    dyn Fn(Context<Ctx>, serde_json::Value) -> Pin<Box<dyn Future<Output = RpcResult<serde_json::Value>> + Send>>
        + Send
        + Sync,
>;

/// Trait for handler functions
/// 
/// Automatically implemented for async functions with the signature:
/// `async fn(Context<Ctx>, Input) -> RpcResult<Output>`
pub trait Handler<Ctx, Input, Output>: Clone + Send + Sync + 'static
where
    Ctx: Clone + Send + Sync + 'static,
    Input: DeserializeOwned + Send + 'static,
    Output: Serialize + Send + 'static,
{
    /// The future type returned by the handler
    type Future: Future<Output = RpcResult<Output>> + Send;

    /// Call the handler with context and input
    fn call(&self, ctx: Context<Ctx>, input: Input) -> Self::Future;
}

// Implement Handler for async functions
impl<Ctx, Input, Output, F, Fut> Handler<Ctx, Input, Output> for F
where
    Ctx: Clone + Send + Sync + 'static,
    Input: DeserializeOwned + Send + 'static,
    Output: Serialize + Send + 'static,
    F: Fn(Context<Ctx>, Input) -> Fut + Clone + Send + Sync + 'static,
    Fut: Future<Output = RpcResult<Output>> + Send + 'static,
{
    type Future = Fut;

    fn call(&self, ctx: Context<Ctx>, input: Input) -> Self::Future {
        (self)(ctx, input)
    }
}

/// Convert a handler into a boxed handler for storage
pub(crate) fn into_boxed<Ctx, Input, Output, H>(handler: H) -> BoxedHandler<Ctx>
where
    Ctx: Clone + Send + Sync + 'static,
    Input: DeserializeOwned + Send + 'static,
    Output: Serialize + Send + 'static,
    H: Handler<Ctx, Input, Output>,
{
    Arc::new(move |ctx, input_value| {
        let handler = handler.clone();
        Box::pin(async move {
            let input: Input = serde_json::from_value(input_value)?;
            let output = handler.call(ctx, input).await?;
            Ok(serde_json::to_value(output)?)
        })
    })
}
