//! Context transformation chains
//!
//! This module contains builder chains for procedures with context transformation.
//! These chains allow you to transform the context type before the handler executes,
//! enabling type-safe context transformations in your RPC procedures.

use super::{core::Router, middleware_chain::build_middleware_chain, types::Procedure};
use crate::{
    Context, RpcError, RpcResult,
    handler::BoxedHandler,
    middleware::{MiddlewareFn, Next, ProcedureType, Request, Response},
    validation::Validate,
};
use serde::{Serialize, de::DeserializeOwned};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Type alias for a context transformer function in the router.
type RouterContextTransformer<FromCtx, ToCtx> = Arc<
    dyn Fn(Context<FromCtx>) -> Pin<Box<dyn Future<Output = RpcResult<ToCtx>> + Send>>
        + Send
        + Sync,
>;

// =============================================================================
// Context Transformed Chain
// =============================================================================

/// A procedure chain with context transformation.
///
/// This struct is returned by `ProcedureChain::context()` and allows you to
/// transform the context type before the handler executes.
pub struct ContextTransformedChain<
    OrigCtx: Clone + Send + Sync + 'static,
    NewCtx: Clone + Send + Sync + 'static,
> {
    pub(super) router: Router<OrigCtx>,
    pub(super) path: String,
    pub(super) middleware: Vec<MiddlewareFn<OrigCtx>>,
    pub(super) output_transformer:
        Option<Arc<dyn Fn(serde_json::Value) -> serde_json::Value + Send + Sync + 'static>>,
    pub(super) context_transformer: RouterContextTransformer<OrigCtx, NewCtx>,
    pub(super) _phantom: std::marker::PhantomData<NewCtx>,
}

impl<OrigCtx: Clone + Send + Sync + 'static, NewCtx: Clone + Send + Sync + 'static>
    ContextTransformedChain<OrigCtx, NewCtx>
{
    /// Create a new context transformed chain.
    pub(super) fn new(
        router: Router<OrigCtx>,
        path: String,
        middleware: Vec<MiddlewareFn<OrigCtx>>,
        output_transformer: Option<
            Arc<dyn Fn(serde_json::Value) -> serde_json::Value + Send + Sync + 'static>,
        >,
        context_transformer: RouterContextTransformer<OrigCtx, NewCtx>,
    ) -> Self {
        Self {
            router,
            path,
            middleware,
            output_transformer,
            context_transformer,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Add middleware to this procedure.
    ///
    /// Note: Middleware operates on the original context type, before transformation.
    #[must_use = "This method returns a new ContextTransformedChain and does not modify self"]
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
    #[must_use = "This method returns a new ContextTransformedChain and does not modify self"]
    pub fn output<F>(mut self, transformer: F) -> Self
    where
        F: Fn(serde_json::Value) -> serde_json::Value + Send + Sync + 'static,
    {
        self.output_transformer = Some(Arc::new(transformer));
        self
    }

    /// Set the input type for this procedure.
    #[must_use = "This method returns a ContextTransformedTypedChain that must be used to register a procedure"]
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
    #[must_use = "This method returns a ContextTransformedValidatedChain that must be used to register a procedure"]
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
    #[must_use = "This method returns a Router and does not modify self"]
    pub fn query<H, Fut, Output>(self, handler: H) -> Router<OrigCtx>
    where
        H: Fn(Context<NewCtx>, ()) -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = RpcResult<Output>> + Send + 'static,
        Output: Serialize + Send + 'static,
    {
        self.input::<()>().query(handler)
    }

    /// Register this procedure as a mutation with no input (unit type).
    #[must_use = "This method returns a Router and does not modify self"]
    pub fn mutation<H, Fut, Output>(self, handler: H) -> Router<OrigCtx>
    where
        H: Fn(Context<NewCtx>, ()) -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = RpcResult<Output>> + Send + 'static,
        Output: Serialize + Send + 'static,
    {
        self.input::<()>().mutation(handler)
    }
}

// =============================================================================
// Context Transformed Typed Chain
// =============================================================================

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
    #[must_use = "This method returns a new ContextTransformedTypedChain and does not modify self"]
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
    #[must_use = "This method returns a new ContextTransformedTypedChain and does not modify self"]
    pub fn output<F>(mut self, transformer: F) -> Self
    where
        F: Fn(serde_json::Value) -> serde_json::Value + Send + Sync + 'static,
    {
        self.output_transformer = Some(Arc::new(transformer));
        self
    }

    /// Register this procedure as a query.
    #[must_use = "This method returns a Router and does not modify self"]
    pub fn query<H, Fut, Output>(self, handler: H) -> Router<OrigCtx>
    where
        H: Fn(Context<NewCtx>, Input) -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = RpcResult<Output>> + Send + 'static,
        Output: Serialize + Send + 'static,
    {
        self.build_procedure(ProcedureType::Query, handler)
    }

    /// Register this procedure as a mutation.
    #[must_use = "This method returns a Router and does not modify self"]
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

            // Use the shared middleware chain builder
            let final_chain = build_middleware_chain(middleware, handler_as_next);

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
// Context Transformed Validated Chain
// =============================================================================

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
    #[must_use = "This method returns a new ContextTransformedValidatedChain and does not modify self"]
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
    #[must_use = "This method returns a new ContextTransformedValidatedChain and does not modify self"]
    pub fn output<F>(mut self, transformer: F) -> Self
    where
        F: Fn(serde_json::Value) -> serde_json::Value + Send + Sync + 'static,
    {
        self.output_transformer = Some(Arc::new(transformer));
        self
    }

    /// Register this procedure as a query with validation.
    #[must_use = "This method returns a Router and does not modify self"]
    pub fn query<H, Fut, Output>(self, handler: H) -> Router<OrigCtx>
    where
        H: Fn(Context<NewCtx>, Input) -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = RpcResult<Output>> + Send + 'static,
        Output: Serialize + Send + 'static,
    {
        self.build_validated_procedure(ProcedureType::Query, handler)
    }

    /// Register this procedure as a mutation with validation.
    #[must_use = "This method returns a Router and does not modify self"]
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

            // Use the shared middleware chain builder
            let final_chain = build_middleware_chain(middleware, handler_as_next);

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
