//! Handler traits and utilities
//!
//! This module provides the core handler abstraction for RPC procedures,
//! including automatic input validation and type-safe handler execution.

use crate::validation::Validate;
use crate::{Context, RpcError, RpcErrorCode, RpcResult};
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
    Arc::new(move |ctx, input_value| {
        let handler = handler.clone();
        Box::pin(async move {
            // Deserialize input
            trace!(
                input_size = input_value.to_string().len(),
                "Deserializing handler input"
            );
            let input: Input = serde_json::from_value(input_value).map_err(|e| {
                warn!(error = %e, "Handler input deserialization failed");
                e
            })?;

            // Execute handler
            trace!("Executing handler");
            let output = handler.call(ctx, input).await.inspect_err(|e| {
                warn!(error_code = %e.code, error_message = %e.message, "Handler execution failed");
            })?;

            // Serialize output
            let output_value = serde_json::to_value(output).map_err(|e| {
                warn!(error = %e, "Handler output serialization failed");
                e
            })?;
            trace!(
                output_size = output_value.to_string().len(),
                "Handler completed successfully"
            );

            Ok(output_value)
        })
    })
}

/// Convert a handler into a boxed handler with automatic input validation.
///
/// This function wraps a handler and automatically validates the input before
/// calling the handler. If validation fails, it returns a VALIDATION_ERROR
/// with field-level details.
///
/// # Example
///
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
/// // Handler will automatically validate input before execution
/// async fn create_user(ctx: Context<AppContext>, input: CreateUserInput) -> RpcResult<User> {
///     // Input is guaranteed to be valid here
///     // ...
/// }
/// ```
pub(crate) fn into_boxed_validated<Ctx, Input, Output, H>(handler: H) -> BoxedHandler<Ctx>
where
    Ctx: Clone + Send + Sync + 'static,
    Input: DeserializeOwned + Validate + Send + 'static,
    Output: Serialize + Send + 'static,
    H: Handler<Ctx, Input, Output>,
{
    Arc::new(move |ctx, input_value| {
        let handler = handler.clone();
        Box::pin(async move {
            // Deserialize input
            trace!(
                input_size = input_value.to_string().len(),
                "Deserializing validated handler input"
            );
            let input: Input = serde_json::from_value(input_value).map_err(|e| {
                warn!(error = %e, "Validated handler input deserialization failed");
                e
            })?;

            // Validate input before calling handler
            trace!("Validating handler input");
            let validation_result = input.validate();
            if !validation_result.is_valid() {
                let error_count = validation_result.errors.len();
                let field_names: Vec<_> = validation_result
                    .errors
                    .iter()
                    .map(|e| e.field.as_str())
                    .collect();
                warn!(
                    error_count = error_count,
                    fields = ?field_names,
                    "Handler input validation failed"
                );
                return Err(RpcError::new(
                    RpcErrorCode::ValidationError,
                    "Input validation failed",
                )
                .with_details(serde_json::json!({
                    "errors": validation_result.errors
                })));
            }
            trace!("Input validation passed");

            // Execute handler
            trace!("Executing validated handler");
            let output = handler.call(ctx, input).await.inspect_err(|e| {
                warn!(error_code = %e.code, error_message = %e.message, "Validated handler execution failed");
            })?;

            // Serialize output
            let output_value = serde_json::to_value(output).map_err(|e| {
                warn!(error = %e, "Validated handler output serialization failed");
                e
            })?;
            trace!(
                output_size = output_value.to_string().len(),
                "Validated handler completed successfully"
            );

            Ok(output_value)
        })
    })
}
