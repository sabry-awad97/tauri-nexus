//! Procedure builder chains
//!
//! This module contains the fluent builder API for defining procedures.

use super::{
    context_transform::ContextTransformedChain, core::Router,
    middleware_chain::build_middleware_chain, types::Procedure,
};
use crate::{
    Context, RpcError, RpcResult,
    handler::BoxedHandler,
    middleware::{MiddlewareFn, Next, ProcedureType, Request, Response},
    validation::Validate,
};
use serde::{Serialize, de::DeserializeOwned};
use std::future::Future;
use std::sync::Arc;

// =============================================================================
// Procedure Chain
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
    pub(crate) router: Router<Ctx>,
    pub(crate) path: String,
    pub(crate) middleware: Vec<MiddlewareFn<Ctx>>,
    pub(crate) output_transformer:
        Option<Arc<dyn Fn(serde_json::Value) -> serde_json::Value + Send + Sync + 'static>>,
}

impl<Ctx: Clone + Send + Sync + 'static> ProcedureChain<Ctx> {
    /// Add middleware to this procedure.
    ///
    /// Middleware is executed in registration order (first registered = outermost).
    #[must_use = "This method returns a new ProcedureChain and does not modify self"]
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
    #[must_use = "This method returns a new ProcedureChain and does not modify self"]
    pub fn use_middleware_fn(mut self, middleware: MiddlewareFn<Ctx>) -> Self {
        self.middleware.push(middleware);
        self
    }

    /// Set the input type for this procedure.
    ///
    /// Returns a `TypedProcedureChain` that allows you to register the procedure
    /// as a query or mutation with the specified input type.
    #[must_use = "This method returns a TypedProcedureChain that must be used to register a procedure"]
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
    #[must_use = "This method returns a ValidatedProcedureChain that must be used to register a procedure"]
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
    #[must_use = "This method returns a new ProcedureChain and does not modify self"]
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
    #[must_use = "This method returns a ContextTransformedChain that must be used to register a procedure"]
    pub fn context<NewCtx, F, Fut>(self, transformer: F) -> ContextTransformedChain<Ctx, NewCtx>
    where
        NewCtx: Clone + Send + Sync + 'static,
        F: Fn(Context<Ctx>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = RpcResult<NewCtx>> + Send + 'static,
    {
        ContextTransformedChain::new(
            self.router,
            self.path,
            self.middleware,
            self.output_transformer,
            Arc::new(move |ctx| Box::pin(transformer(ctx))),
        )
    }

    /// Register this procedure as a query with no input (unit type).
    #[must_use = "This method returns a Router and does not modify self"]
    pub fn query<H, Fut, Output>(self, handler: H) -> Router<Ctx>
    where
        H: Fn(Context<Ctx>, ()) -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = RpcResult<Output>> + Send + 'static,
        Output: Serialize + Send + 'static,
    {
        self.input::<()>().query(handler)
    }

    /// Register this procedure as a mutation with no input (unit type).
    #[must_use = "This method returns a Router and does not modify self"]
    pub fn mutation<H, Fut, Output>(self, handler: H) -> Router<Ctx>
    where
        H: Fn(Context<Ctx>, ()) -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = RpcResult<Output>> + Send + 'static,
        Output: Serialize + Send + 'static,
    {
        self.input::<()>().mutation(handler)
    }
}

// =============================================================================
// Typed Procedure Chain
// =============================================================================

/// A typed procedure chain with a specific input type.
pub struct TypedProcedureChain<Ctx: Clone + Send + Sync + 'static, Input> {
    pub(crate) router: Router<Ctx>,
    pub(crate) path: String,
    pub(crate) middleware: Vec<MiddlewareFn<Ctx>>,
    pub(crate) output_transformer:
        Option<Arc<dyn Fn(serde_json::Value) -> serde_json::Value + Send + Sync + 'static>>,
    pub(crate) _phantom: std::marker::PhantomData<Input>,
}

impl<Ctx: Clone + Send + Sync + 'static, Input: DeserializeOwned + Send + 'static>
    TypedProcedureChain<Ctx, Input>
{
    /// Add middleware to this procedure.
    #[must_use = "This method returns a new TypedProcedureChain and does not modify self"]
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
    #[must_use = "This method returns a new TypedProcedureChain and does not modify self"]
    pub fn output<F>(mut self, transformer: F) -> Self
    where
        F: Fn(serde_json::Value) -> serde_json::Value + Send + Sync + 'static,
    {
        self.output_transformer = Some(Arc::new(transformer));
        self
    }

    /// Register this procedure as a query.
    #[must_use = "This method returns a Router and does not modify self"]
    pub fn query<H, Fut, Output>(self, handler: H) -> Router<Ctx>
    where
        H: Fn(Context<Ctx>, Input) -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = RpcResult<Output>> + Send + 'static,
        Output: Serialize + Send + 'static,
    {
        self.build_procedure(ProcedureType::Query, handler)
    }

    /// Register this procedure as a mutation.
    #[must_use = "This method returns a Router and does not modify self"]
    pub fn mutation<H, Fut, Output>(self, handler: H) -> Router<Ctx>
    where
        H: Fn(Context<Ctx>, Input) -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = RpcResult<Output>> + Send + 'static,
        Output: Serialize + Send + 'static,
    {
        self.build_procedure(ProcedureType::Mutation, handler)
    }

    fn build_procedure<H, Fut, Output>(
        self,
        procedure_type: ProcedureType,
        handler: H,
    ) -> Router<Ctx>
    where
        H: Fn(Context<Ctx>, Input) -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = RpcResult<Output>> + Send + 'static,
        Output: Serialize + Send + 'static,
    {
        let output_transformer = self.output_transformer;

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

        // Wrap with middleware and register
        let final_handler = wrap_with_middleware(core_handler, self.middleware, procedure_type);
        register_procedure(self.router, &self.path, final_handler, procedure_type)
    }
}

// =============================================================================
// Validated Procedure Chain
// =============================================================================

/// A validated procedure chain with automatic input validation.
pub struct ValidatedProcedureChain<Ctx: Clone + Send + Sync + 'static, Input> {
    pub(crate) router: Router<Ctx>,
    pub(crate) path: String,
    pub(crate) middleware: Vec<MiddlewareFn<Ctx>>,
    pub(crate) output_transformer:
        Option<Arc<dyn Fn(serde_json::Value) -> serde_json::Value + Send + Sync + 'static>>,
    pub(crate) _phantom: std::marker::PhantomData<Input>,
}

impl<Ctx: Clone + Send + Sync + 'static, Input: DeserializeOwned + Validate + Send + 'static>
    ValidatedProcedureChain<Ctx, Input>
{
    /// Add middleware to this procedure.
    #[must_use = "This method returns a new ValidatedProcedureChain and does not modify self"]
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
    #[must_use = "This method returns a new ValidatedProcedureChain and does not modify self"]
    pub fn output<F>(mut self, transformer: F) -> Self
    where
        F: Fn(serde_json::Value) -> serde_json::Value + Send + Sync + 'static,
    {
        self.output_transformer = Some(Arc::new(transformer));
        self
    }

    /// Register this procedure as a query with validation.
    #[must_use = "This method returns a Router and does not modify self"]
    pub fn query<H, Fut, Output>(self, handler: H) -> Router<Ctx>
    where
        H: Fn(Context<Ctx>, Input) -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = RpcResult<Output>> + Send + 'static,
        Output: Serialize + Send + 'static,
    {
        self.build_validated_procedure(ProcedureType::Query, handler)
    }

    /// Register this procedure as a mutation with validation.
    #[must_use = "This method returns a Router and does not modify self"]
    pub fn mutation<H, Fut, Output>(self, handler: H) -> Router<Ctx>
    where
        H: Fn(Context<Ctx>, Input) -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = RpcResult<Output>> + Send + 'static,
        Output: Serialize + Send + 'static,
    {
        self.build_validated_procedure(ProcedureType::Mutation, handler)
    }

    fn build_validated_procedure<H, Fut, Output>(
        self,
        procedure_type: ProcedureType,
        handler: H,
    ) -> Router<Ctx>
    where
        H: Fn(Context<Ctx>, Input) -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = RpcResult<Output>> + Send + 'static,
        Output: Serialize + Send + 'static,
    {
        let output_transformer = self.output_transformer;

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

        // Wrap with middleware and register
        let final_handler = wrap_with_middleware(core_handler, self.middleware, procedure_type);
        register_procedure(self.router, &self.path, final_handler, procedure_type)
    }
}

// =============================================================================
// Helper Functions for Reducing Duplication
// =============================================================================

/// Wraps a core handler with middleware if any are present.
///
/// This helper reduces duplication across all builder chain types by centralizing
/// the middleware wrapping logic.
pub(crate) fn wrap_with_middleware<Ctx: Clone + Send + Sync + 'static>(
    core_handler: BoxedHandler<Ctx>,
    middleware: Vec<MiddlewareFn<Ctx>>,
    procedure_type: ProcedureType,
) -> BoxedHandler<Ctx> {
    if middleware.is_empty() {
        core_handler
    } else {
        let handler_as_next: Next<Ctx> = Arc::new(move |ctx, req| {
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
    }
}

/// Registers a procedure handler in the router.
///
/// This helper reduces duplication by centralizing the procedure registration logic.
pub(crate) fn register_procedure<Ctx: Clone + Send + Sync + 'static>(
    mut router: Router<Ctx>,
    path: &str,
    handler: BoxedHandler<Ctx>,
    procedure_type: ProcedureType,
) -> Router<Ctx> {
    let full_path = router.make_path(path);
    router.procedures.insert(
        full_path,
        Procedure::Handler {
            handler,
            procedure_type,
        },
    );
    router
}
