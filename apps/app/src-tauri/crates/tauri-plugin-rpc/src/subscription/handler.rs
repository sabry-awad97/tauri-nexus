//! Subscription handler trait and type conversions.
//!
//! This module provides the trait and utilities for defining subscription handlers
//! that can be registered with the RPC router.

use serde::Serialize;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::{Context, RpcResult};

use super::{Event, EventStream, SubscriptionContext};

// =============================================================================
// Subscription Handler Types
// =============================================================================

/// Return type for subscription handlers
pub type SubscriptionResult<T> = RpcResult<EventStream<T>>;

/// Boxed subscription handler for type erasure
pub type BoxedSubscriptionHandler<Ctx> = Arc<
    dyn Fn(
            Context<Ctx>,
            SubscriptionContext,
            serde_json::Value,
        ) -> Pin<
            Box<dyn Future<Output = RpcResult<mpsc::Receiver<Event<serde_json::Value>>>> + Send>,
        > + Send
        + Sync,
>;

// =============================================================================
// Subscription Handler Trait
// =============================================================================

/// Trait for subscription handler functions
pub trait SubscriptionHandler<Ctx, Input, Output>: Clone + Send + Sync + 'static
where
    Ctx: Clone + Send + Sync + 'static,
    Input: serde::de::DeserializeOwned + Send + 'static,
    Output: Serialize + Send + 'static,
{
    /// The future type returned by the handler
    type Future: Future<Output = SubscriptionResult<Output>> + Send;

    /// Call the handler
    fn call(&self, ctx: Context<Ctx>, sub_ctx: SubscriptionContext, input: Input) -> Self::Future;
}

// Implement for async functions
impl<Ctx, Input, Output, F, Fut> SubscriptionHandler<Ctx, Input, Output> for F
where
    Ctx: Clone + Send + Sync + 'static,
    Input: serde::de::DeserializeOwned + Send + 'static,
    Output: Serialize + Send + 'static,
    F: Fn(Context<Ctx>, SubscriptionContext, Input) -> Fut + Clone + Send + Sync + 'static,
    Fut: Future<Output = SubscriptionResult<Output>> + Send + 'static,
{
    type Future = Fut;

    fn call(&self, ctx: Context<Ctx>, sub_ctx: SubscriptionContext, input: Input) -> Self::Future {
        (self)(ctx, sub_ctx, input)
    }
}

// =============================================================================
// Handler Conversion
// =============================================================================

/// Convert a subscription handler into a boxed handler
pub fn into_boxed_subscription<Ctx, Input, Output, H>(handler: H) -> BoxedSubscriptionHandler<Ctx>
where
    Ctx: Clone + Send + Sync + 'static,
    Input: serde::de::DeserializeOwned + Send + 'static,
    Output: Serialize + Send + 'static,
    H: SubscriptionHandler<Ctx, Input, Output>,
{
    Arc::new(move |ctx, sub_ctx, input_value| {
        let handler = handler.clone();
        Box::pin(async move {
            let input: Input = serde_json::from_value(input_value)?;
            let stream = handler.call(ctx, sub_ctx, input).await?;

            // Convert typed stream to JSON stream
            let (tx, rx) = mpsc::channel(32);
            tokio::spawn(async move {
                let mut stream = stream;
                while let Some(event) = stream.recv().await {
                    // Properly handle serialization errors instead of silently converting to Null
                    let data = match serde_json::to_value(&event.data) {
                        Ok(value) => value,
                        Err(e) => {
                            tracing::error!(
                                error = %e,
                                "Failed to serialize subscription event data"
                            );
                            // Stop the stream on serialization error
                            break;
                        }
                    };

                    let json_event = Event {
                        data,
                        id: event.id,
                        retry: event.retry,
                    };
                    if tx.send(json_event).await.is_err() {
                        break;
                    }
                }
            });

            Ok(rx)
        })
    })
}
