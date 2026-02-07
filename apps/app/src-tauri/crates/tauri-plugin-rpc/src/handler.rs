//! Handler traits and utilities
//!
//! This module provides the core handler abstraction for RPC procedures,
//! including automatic input validation and type-safe handler execution.

use crate::{Context, RpcResult};
use serde::{Serialize, de::DeserializeOwned};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tracing::{trace, warn};

/// Boxed handler for type erasure
pub(crate) type BoxedHandler<Ctx> = Arc<
    dyn Fn(
            Context<Ctx>,
            serde_json::Value,
        ) -> Pin<Box<dyn Future<Output = RpcResult<serde_json::Value>> + Send>>
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

/// Convert a handler into a boxed handler for storage (without validation)
pub(crate) fn into_boxed<Ctx, Input, Output, H>(handler: H) -> BoxedHandler<Ctx>
where
    Ctx: Clone + Send + Sync + 'static,
    Input: DeserializeOwned + Send + 'static,
    Output: Serialize + Send + 'static,
    H: Handler<Ctx, Input, Output>,
{
    let handler = Arc::new(handler);
    Arc::new(move |ctx, input_value| {
        let handler = Arc::clone(&handler);
        Box::pin(async move {
            // Deserialize input
            if tracing::enabled!(tracing::Level::TRACE) {
                let input_size = estimate_json_size(&input_value);
                trace!(input_size, "Deserializing handler input");
            }

            let input: Input = serde_json::from_value(input_value).map_err(|e| {
                warn!(
                    error = %e,
                    type_name = std::any::type_name::<Input>(),
                    "Handler input deserialization failed"
                );
                e
            })?;

            // Execute handler
            trace!("Executing handler");
            let output = handler.call(ctx, input).await.inspect_err(|e| {
                warn!(
                    error_code = %e.code,
                    error_message = %e.message,
                    "Handler execution failed"
                );
            })?;

            // Serialize output
            let output_value = serde_json::to_value(output).map_err(|e| {
                warn!(
                    error = %e,
                    type_name = std::any::type_name::<Output>(),
                    "Handler output serialization failed"
                );
                e
            })?;

            if tracing::enabled!(tracing::Level::TRACE) {
                let output_size = estimate_json_size(&output_value);
                trace!(output_size, "Handler completed successfully");
            }

            Ok(output_value)
        })
    })
}

/// Estimate JSON value size without allocating a string
#[inline]
fn estimate_json_size(value: &serde_json::Value) -> usize {
    match value {
        serde_json::Value::Null => 4,
        serde_json::Value::Bool(_) => 5,
        serde_json::Value::Number(n) => n.to_string().len(),
        serde_json::Value::String(s) => s.len() + 2,
        serde_json::Value::Array(arr) => {
            arr.iter().map(estimate_json_size).sum::<usize>() + arr.len() + 1
        }
        serde_json::Value::Object(obj) => {
            obj.iter()
                .map(|(k, v)| k.len() + estimate_json_size(v) + 3)
                .sum::<usize>()
                + obj.len()
                + 1
        }
    }
}
