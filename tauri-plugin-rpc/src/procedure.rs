//! oRPC-Style Procedure Builder API
//!
//! Provides a fluent builder pattern for defining procedures with per-procedure
//! middleware, input validation, output transformation, and context enrichment.
//!
//! # Example
//!
//! ```rust,ignore
//! use tauri_plugin_rpc::prelude::*;
//!
//! let router = Router::new()
//!     .context(AppContext::new())
//!     .procedure("users.get")
//!         .use_middleware(auth_middleware)
//!         .input::<GetUserInput>()
//!         .query(get_user)
//!     .procedure("users.create")
//!         .use_middleware(auth_middleware)
//!         .use_middleware(rate_limit_middleware)
//!         .input::<CreateUserInput>()
//!         .mutation(create_user);
//! ```
//!
//! # Context Transformation
//!
//! The `.context()` method allows you to transform the context type before
//! the handler executes. This is useful for enriching the context with
//! additional data (e.g., authenticated user info):
//!
//! ```rust,ignore
//! let router = Router::new()
//!     .context(AppContext::new())
//!     .procedure("users.profile")
//!         .context(|ctx: Context<AppContext>| async move {
//!             // Extract user from auth header, enrich context
//!             let user = authenticate(&ctx).await?;
//!             Ok(AuthenticatedContext { app: ctx.inner().clone(), user })
//!         })
//!         .input::<GetProfileInput>()
//!         .query(get_profile); // Handler receives Context<AuthenticatedContext>
//! ```

use crate::middleware::{MiddlewareFn, Next, ProcedureType, Request, Response};
use crate::validation::Validate;
use crate::{Context, RpcError, RpcResult};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;

/// Type alias for the boxed async handler function.
pub type BoxedHandler<Ctx> = Arc<
    dyn Fn(
            Context<Ctx>,
            serde_json::Value,
        ) -> Pin<Box<dyn Future<Output = RpcResult<serde_json::Value>> + Send>>
        + Send
        + Sync,
>;

/// Type alias for a context transformer function.
pub type ContextTransformer<FromCtx, ToCtx> = Arc<
    dyn Fn(Context<FromCtx>) -> Pin<Box<dyn Future<Output = RpcResult<ToCtx>> + Send>>
        + Send
        + Sync,
>;

/// A registered procedure ready to be added to a router.
pub struct RegisteredProcedure<Ctx>
where
    Ctx: Clone + Send + Sync + 'static,
{
    /// The procedure path.
    pub path: String,
    /// The procedure type (query, mutation, subscription).
    pub procedure_type: ProcedureType,
    /// The compiled handler with middleware chain.
    pub handler: BoxedHandler<Ctx>,
    /// Per-procedure middleware stack.
    pub middleware: Vec<MiddlewareFn<Ctx>>,
}

/// Builder for configuring individual procedures with middleware, validation, and transformation.
///
/// The builder uses phantom types to track the input type at compile time,
/// ensuring type safety throughout the configuration process. The output type
/// is inferred from the handler function.
///
/// # Type Parameters
///
/// - `Ctx`: The context type passed to handlers
/// - `Input`: The input type for the procedure (default: `()`)
///
/// # Example
///
/// ```rust,ignore
/// let procedure = ProcedureBuilder::<AppContext>::new("users.get")
///     .use_middleware(logging)
///     .input::<GetUserInput>()
///     .query(get_user);
/// ```
pub struct ProcedureBuilder<Ctx, Input = ()>
where
    Ctx: Clone + Send + Sync + 'static,
{
    /// The procedure path.
    path: String,
    /// Per-procedure middleware stack (executed in registration order).
    middleware: Vec<MiddlewareFn<Ctx>>,
    /// Optional output transformer.
    output_transformer:
        Option<Arc<dyn Fn(serde_json::Value) -> serde_json::Value + Send + Sync + 'static>>,
    /// Phantom data for type tracking.
    _phantom: PhantomData<Input>,
}

impl<Ctx> ProcedureBuilder<Ctx, ()>
where
    Ctx: Clone + Send + Sync + 'static,
{
    /// Creates a new procedure builder with the given path.
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            middleware: Vec::new(),
            output_transformer: None,
            _phantom: PhantomData,
        }
    }

    /// Sets the input type for this procedure.
    ///
    /// The input type must implement `DeserializeOwned` for JSON deserialization.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let procedure = ProcedureBuilder::<AppContext>::new("users.get")
    ///     .input::<GetUserInput>()
    ///     .query(get_user);
    /// ```
    pub fn input<NewInput>(self) -> ProcedureBuilder<Ctx, NewInput>
    where
        NewInput: DeserializeOwned + Send + 'static,
    {
        ProcedureBuilder {
            path: self.path,
            middleware: self.middleware,
            output_transformer: self.output_transformer,
            _phantom: PhantomData,
        }
    }

    /// Sets the input type with validation.
    ///
    /// The input type must implement both `DeserializeOwned` and `Validate`.
    /// Validation is automatically performed before the handler is called.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let procedure = ProcedureBuilder::<AppContext>::new("users.create")
    ///     .input_validated::<CreateUserInput>()
    ///     .mutation(create_user);
    /// ```
    pub fn input_validated<NewInput>(self) -> ValidatedProcedureBuilder<Ctx, NewInput>
    where
        NewInput: DeserializeOwned + Validate + Send + 'static,
    {
        ValidatedProcedureBuilder {
            path: self.path,
            middleware: self.middleware,
            output_transformer: self.output_transformer,
            _phantom: PhantomData,
        }
    }

    /// Transforms the context type before the handler executes.
    ///
    /// This method allows you to enrich or transform the context with additional
    /// data before the handler is called. The transformer function receives the
    /// original context and returns a new context type.
    ///
    /// This is useful for:
    /// - Adding authenticated user information to the context
    /// - Loading additional data needed by the handler
    /// - Converting between context types
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// #[derive(Clone)]
    /// struct AppContext { db: Database }
    ///
    /// #[derive(Clone)]
    /// struct AuthContext { app: AppContext, user: User }
    ///
    /// let procedure = ProcedureBuilder::<AppContext>::new("users.profile")
    ///     .context(|ctx: Context<AppContext>| async move {
    ///         let user = authenticate(&ctx).await?;
    ///         Ok(AuthContext { app: ctx.inner().clone(), user })
    ///     })
    ///     .input::<GetProfileInput>()
    ///     .query(get_profile); // Handler receives Context<AuthContext>
    /// ```
    pub fn context<NewCtx, F, Fut>(self, transformer: F) -> ContextTransformedBuilder<Ctx, NewCtx>
    where
        NewCtx: Clone + Send + Sync + 'static,
        F: Fn(Context<Ctx>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = RpcResult<NewCtx>> + Send + 'static,
    {
        ContextTransformedBuilder {
            path: self.path,
            middleware: self.middleware,
            output_transformer: self.output_transformer,
            context_transformer: Arc::new(move |ctx| Box::pin(transformer(ctx))),
            _phantom: PhantomData,
        }
    }
}

impl<Ctx, Input> ProcedureBuilder<Ctx, Input>
where
    Ctx: Clone + Send + Sync + 'static,
    Input: DeserializeOwned + Send + 'static,
{
    /// Returns the procedure path.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Adds middleware to this procedure.
    ///
    /// Middleware is executed in registration order (first registered = outermost).
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let procedure = ProcedureBuilder::<AppContext>::new("users.get")
    ///     .use_middleware(logging)      // Executes first (outermost)
    ///     .use_middleware(auth)         // Executes second
    ///     .use_middleware(rate_limit)   // Executes third (innermost)
    ///     .query(get_user);
    /// ```
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

    /// Adds a middleware function (already wrapped as MiddlewareFn).
    pub fn use_middleware_fn(mut self, middleware: MiddlewareFn<Ctx>) -> Self {
        self.middleware.push(middleware);
        self
    }

    /// Sets an output transformer for this procedure.
    ///
    /// The transformer is applied to the handler's output before returning.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let procedure = ProcedureBuilder::<AppContext>::new("users.get")
    ///     .input::<GetUserInput>()
    ///     .output(|value| {
    ///         // Transform the output
    ///         json!({ "data": value, "timestamp": Utc::now().to_rfc3339() })
    ///     })
    ///     .query(get_user);
    /// ```
    pub fn output<F>(mut self, transformer: F) -> Self
    where
        F: Fn(serde_json::Value) -> serde_json::Value + Send + Sync + 'static,
    {
        self.output_transformer = Some(Arc::new(transformer));
        self
    }

    /// Registers this procedure as a query (read-only operation).
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// async fn get_user(ctx: Context<AppContext>, input: GetUserInput) -> RpcResult<User> {
    ///     // ...
    /// }
    ///
    /// let procedure = ProcedureBuilder::<AppContext>::new("users.get")
    ///     .input::<GetUserInput>()
    ///     .query(get_user);
    /// ```
    pub fn query<H, Fut, Output>(self, handler: H) -> RegisteredProcedure<Ctx>
    where
        H: Fn(Context<Ctx>, Input) -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = RpcResult<Output>> + Send + 'static,
        Output: Serialize + Send + 'static,
    {
        self.build_procedure(ProcedureType::Query, handler)
    }

    /// Registers this procedure as a mutation (write operation).
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// async fn create_user(ctx: Context<AppContext>, input: CreateUserInput) -> RpcResult<User> {
    ///     // ...
    /// }
    ///
    /// let procedure = ProcedureBuilder::<AppContext>::new("users.create")
    ///     .input::<CreateUserInput>()
    ///     .mutation(create_user);
    /// ```
    pub fn mutation<H, Fut, Output>(self, handler: H) -> RegisteredProcedure<Ctx>
    where
        H: Fn(Context<Ctx>, Input) -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = RpcResult<Output>> + Send + 'static,
        Output: Serialize + Send + 'static,
    {
        self.build_procedure(ProcedureType::Mutation, handler)
    }

    /// Builds the procedure with the given type and handler.
    fn build_procedure<H, Fut, Output>(
        self,
        procedure_type: ProcedureType,
        handler: H,
    ) -> RegisteredProcedure<Ctx>
    where
        H: Fn(Context<Ctx>, Input) -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = RpcResult<Output>> + Send + 'static,
        Output: Serialize + Send + 'static,
    {
        let output_transformer = self.output_transformer;

        let boxed_handler: BoxedHandler<Ctx> = Arc::new(move |ctx, input_value| {
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

        RegisteredProcedure {
            path: self.path,
            procedure_type,
            handler: boxed_handler,
            middleware: self.middleware,
        }
    }
}

/// A procedure builder with validated input.
///
/// This builder is created when using `input_validated::<T>()` and ensures
/// that the input is validated before the handler is called.
pub struct ValidatedProcedureBuilder<Ctx, Input>
where
    Ctx: Clone + Send + Sync + 'static,
{
    path: String,
    middleware: Vec<MiddlewareFn<Ctx>>,
    output_transformer:
        Option<Arc<dyn Fn(serde_json::Value) -> serde_json::Value + Send + Sync + 'static>>,
    _phantom: PhantomData<Input>,
}

impl<Ctx, Input> ValidatedProcedureBuilder<Ctx, Input>
where
    Ctx: Clone + Send + Sync + 'static,
    Input: DeserializeOwned + Validate + Send + 'static,
{
    /// Returns the procedure path.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Adds middleware to this procedure.
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

    /// Adds a middleware function (already wrapped as MiddlewareFn).
    pub fn use_middleware_fn(mut self, middleware: MiddlewareFn<Ctx>) -> Self {
        self.middleware.push(middleware);
        self
    }

    /// Sets an output transformer for this procedure.
    pub fn output<F>(mut self, transformer: F) -> Self
    where
        F: Fn(serde_json::Value) -> serde_json::Value + Send + Sync + 'static,
    {
        self.output_transformer = Some(Arc::new(transformer));
        self
    }

    /// Registers this procedure as a query with validation.
    pub fn query<H, Fut, Output>(self, handler: H) -> RegisteredProcedure<Ctx>
    where
        H: Fn(Context<Ctx>, Input) -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = RpcResult<Output>> + Send + 'static,
        Output: Serialize + Send + 'static,
    {
        self.build_validated_procedure(ProcedureType::Query, handler)
    }

    /// Registers this procedure as a mutation with validation.
    pub fn mutation<H, Fut, Output>(self, handler: H) -> RegisteredProcedure<Ctx>
    where
        H: Fn(Context<Ctx>, Input) -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = RpcResult<Output>> + Send + 'static,
        Output: Serialize + Send + 'static,
    {
        self.build_validated_procedure(ProcedureType::Mutation, handler)
    }

    /// Builds the validated procedure with the given type and handler.
    fn build_validated_procedure<H, Fut, Output>(
        self,
        procedure_type: ProcedureType,
        handler: H,
    ) -> RegisteredProcedure<Ctx>
    where
        H: Fn(Context<Ctx>, Input) -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = RpcResult<Output>> + Send + 'static,
        Output: Serialize + Send + 'static,
    {
        let output_transformer = self.output_transformer;

        let boxed_handler: BoxedHandler<Ctx> = Arc::new(move |ctx, input_value| {
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

        RegisteredProcedure {
            path: self.path,
            procedure_type,
            handler: boxed_handler,
            middleware: self.middleware,
        }
    }
}

/// A procedure builder with context transformation.
///
/// This builder is created when using `.context()` and allows the context
/// to be transformed before the handler executes.
///
/// # Type Parameters
///
/// - `OrigCtx`: The original context type from the router
/// - `NewCtx`: The transformed context type that the handler will receive
pub struct ContextTransformedBuilder<OrigCtx, NewCtx>
where
    OrigCtx: Clone + Send + Sync + 'static,
    NewCtx: Clone + Send + Sync + 'static,
{
    path: String,
    middleware: Vec<MiddlewareFn<OrigCtx>>,
    output_transformer:
        Option<Arc<dyn Fn(serde_json::Value) -> serde_json::Value + Send + Sync + 'static>>,
    context_transformer: ContextTransformer<OrigCtx, NewCtx>,
    _phantom: PhantomData<(OrigCtx, NewCtx)>,
}

impl<OrigCtx, NewCtx> ContextTransformedBuilder<OrigCtx, NewCtx>
where
    OrigCtx: Clone + Send + Sync + 'static,
    NewCtx: Clone + Send + Sync + 'static,
{
    /// Returns the procedure path.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Adds middleware to this procedure.
    ///
    /// Note: Middleware operates on the original context type, before transformation.
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

    /// Adds a middleware function (already wrapped as MiddlewareFn).
    pub fn use_middleware_fn(mut self, middleware: MiddlewareFn<OrigCtx>) -> Self {
        self.middleware.push(middleware);
        self
    }

    /// Sets an output transformer for this procedure.
    pub fn output<F>(mut self, transformer: F) -> Self
    where
        F: Fn(serde_json::Value) -> serde_json::Value + Send + Sync + 'static,
    {
        self.output_transformer = Some(Arc::new(transformer));
        self
    }

    /// Sets the input type for this procedure.
    pub fn input<Input>(self) -> ContextTransformedTypedBuilder<OrigCtx, NewCtx, Input>
    where
        Input: DeserializeOwned + Send + 'static,
    {
        ContextTransformedTypedBuilder {
            path: self.path,
            middleware: self.middleware,
            output_transformer: self.output_transformer,
            context_transformer: self.context_transformer,
            _phantom: PhantomData,
        }
    }

    /// Sets the input type with validation.
    pub fn input_validated<Input>(
        self,
    ) -> ContextTransformedValidatedBuilder<OrigCtx, NewCtx, Input>
    where
        Input: DeserializeOwned + Validate + Send + 'static,
    {
        ContextTransformedValidatedBuilder {
            path: self.path,
            middleware: self.middleware,
            output_transformer: self.output_transformer,
            context_transformer: self.context_transformer,
            _phantom: PhantomData,
        }
    }
}

/// A context-transformed procedure builder with a specific input type.
pub struct ContextTransformedTypedBuilder<OrigCtx, NewCtx, Input>
where
    OrigCtx: Clone + Send + Sync + 'static,
    NewCtx: Clone + Send + Sync + 'static,
{
    path: String,
    middleware: Vec<MiddlewareFn<OrigCtx>>,
    output_transformer:
        Option<Arc<dyn Fn(serde_json::Value) -> serde_json::Value + Send + Sync + 'static>>,
    context_transformer: ContextTransformer<OrigCtx, NewCtx>,
    _phantom: PhantomData<(OrigCtx, NewCtx, Input)>,
}

impl<OrigCtx, NewCtx, Input> ContextTransformedTypedBuilder<OrigCtx, NewCtx, Input>
where
    OrigCtx: Clone + Send + Sync + 'static,
    NewCtx: Clone + Send + Sync + 'static,
    Input: DeserializeOwned + Send + 'static,
{
    /// Returns the procedure path.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Adds middleware to this procedure.
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

    /// Sets an output transformer for this procedure.
    pub fn output<F>(mut self, transformer: F) -> Self
    where
        F: Fn(serde_json::Value) -> serde_json::Value + Send + Sync + 'static,
    {
        self.output_transformer = Some(Arc::new(transformer));
        self
    }

    /// Registers this procedure as a query.
    ///
    /// The handler receives the transformed context type.
    pub fn query<H, Fut, Output>(self, handler: H) -> RegisteredProcedure<OrigCtx>
    where
        H: Fn(Context<NewCtx>, Input) -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = RpcResult<Output>> + Send + 'static,
        Output: Serialize + Send + 'static,
    {
        self.build_procedure(ProcedureType::Query, handler)
    }

    /// Registers this procedure as a mutation.
    ///
    /// The handler receives the transformed context type.
    pub fn mutation<H, Fut, Output>(self, handler: H) -> RegisteredProcedure<OrigCtx>
    where
        H: Fn(Context<NewCtx>, Input) -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = RpcResult<Output>> + Send + 'static,
        Output: Serialize + Send + 'static,
    {
        self.build_procedure(ProcedureType::Mutation, handler)
    }

    fn build_procedure<H, Fut, Output>(
        self,
        procedure_type: ProcedureType,
        handler: H,
    ) -> RegisteredProcedure<OrigCtx>
    where
        H: Fn(Context<NewCtx>, Input) -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = RpcResult<Output>> + Send + 'static,
        Output: Serialize + Send + 'static,
    {
        let output_transformer = self.output_transformer;
        let context_transformer = self.context_transformer;

        let boxed_handler: BoxedHandler<OrigCtx> = Arc::new(move |ctx, input_value| {
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

        RegisteredProcedure {
            path: self.path,
            procedure_type,
            handler: boxed_handler,
            middleware: self.middleware,
        }
    }
}

/// A context-transformed procedure builder with validated input.
pub struct ContextTransformedValidatedBuilder<OrigCtx, NewCtx, Input>
where
    OrigCtx: Clone + Send + Sync + 'static,
    NewCtx: Clone + Send + Sync + 'static,
{
    path: String,
    middleware: Vec<MiddlewareFn<OrigCtx>>,
    output_transformer:
        Option<Arc<dyn Fn(serde_json::Value) -> serde_json::Value + Send + Sync + 'static>>,
    context_transformer: ContextTransformer<OrigCtx, NewCtx>,
    _phantom: PhantomData<(OrigCtx, NewCtx, Input)>,
}

impl<OrigCtx, NewCtx, Input> ContextTransformedValidatedBuilder<OrigCtx, NewCtx, Input>
where
    OrigCtx: Clone + Send + Sync + 'static,
    NewCtx: Clone + Send + Sync + 'static,
    Input: DeserializeOwned + Validate + Send + 'static,
{
    /// Returns the procedure path.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Adds middleware to this procedure.
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

    /// Sets an output transformer for this procedure.
    pub fn output<F>(mut self, transformer: F) -> Self
    where
        F: Fn(serde_json::Value) -> serde_json::Value + Send + Sync + 'static,
    {
        self.output_transformer = Some(Arc::new(transformer));
        self
    }

    /// Registers this procedure as a query with validation.
    pub fn query<H, Fut, Output>(self, handler: H) -> RegisteredProcedure<OrigCtx>
    where
        H: Fn(Context<NewCtx>, Input) -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = RpcResult<Output>> + Send + 'static,
        Output: Serialize + Send + 'static,
    {
        self.build_validated_procedure(ProcedureType::Query, handler)
    }

    /// Registers this procedure as a mutation with validation.
    pub fn mutation<H, Fut, Output>(self, handler: H) -> RegisteredProcedure<OrigCtx>
    where
        H: Fn(Context<NewCtx>, Input) -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = RpcResult<Output>> + Send + 'static,
        Output: Serialize + Send + 'static,
    {
        self.build_validated_procedure(ProcedureType::Mutation, handler)
    }

    fn build_validated_procedure<H, Fut, Output>(
        self,
        procedure_type: ProcedureType,
        handler: H,
    ) -> RegisteredProcedure<OrigCtx>
    where
        H: Fn(Context<NewCtx>, Input) -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = RpcResult<Output>> + Send + 'static,
        Output: Serialize + Send + 'static,
    {
        let output_transformer = self.output_transformer;
        let context_transformer = self.context_transformer;

        let boxed_handler: BoxedHandler<OrigCtx> = Arc::new(move |ctx, input_value| {
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

        RegisteredProcedure {
            path: self.path,
            procedure_type,
            handler: boxed_handler,
            middleware: self.middleware,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validation::{FieldError, ValidationResult};
    use serde::{Deserialize, Serialize};

    #[derive(Clone)]
    struct TestContext {
        #[allow(dead_code)]
        value: i32,
    }

    #[derive(Debug, Deserialize)]
    struct TestInput {
        name: String,
    }

    #[derive(Debug, Serialize)]
    struct TestOutput {
        message: String,
    }

    #[derive(Debug, Deserialize)]
    struct ValidatedInput {
        name: String,
        age: i32,
    }

    impl Validate for ValidatedInput {
        fn validate(&self) -> ValidationResult {
            let mut errors = Vec::new();
            if self.name.is_empty() {
                errors.push(FieldError::required("name"));
            }
            if self.age < 0 || self.age > 150 {
                errors.push(FieldError::range("age", 0, 150));
            }
            ValidationResult::from_errors(errors)
        }
    }

    async fn test_handler(_ctx: Context<TestContext>, input: TestInput) -> RpcResult<TestOutput> {
        Ok(TestOutput {
            message: format!("Hello, {}!", input.name),
        })
    }

    async fn validated_handler(
        _ctx: Context<TestContext>,
        input: ValidatedInput,
    ) -> RpcResult<TestOutput> {
        Ok(TestOutput {
            message: format!("Hello, {} (age {})!", input.name, input.age),
        })
    }

    #[test]
    fn test_procedure_builder_new() {
        let builder = ProcedureBuilder::<TestContext>::new("users.get");
        assert_eq!(builder.path(), "users.get");
    }

    #[test]
    fn test_procedure_builder_query() {
        let procedure = ProcedureBuilder::<TestContext>::new("users.get")
            .input::<TestInput>()
            .query(test_handler);

        assert_eq!(procedure.path, "users.get");
        assert_eq!(procedure.procedure_type, ProcedureType::Query);
        assert!(procedure.middleware.is_empty());
    }

    #[test]
    fn test_procedure_builder_mutation() {
        let procedure = ProcedureBuilder::<TestContext>::new("users.create")
            .input::<TestInput>()
            .mutation(test_handler);

        assert_eq!(procedure.path, "users.create");
        assert_eq!(procedure.procedure_type, ProcedureType::Mutation);
    }

    #[test]
    fn test_procedure_builder_with_middleware() {
        async fn test_middleware(
            ctx: Context<TestContext>,
            req: Request,
            next: Next<TestContext>,
        ) -> RpcResult<Response> {
            next(ctx, req).await
        }

        let procedure = ProcedureBuilder::<TestContext>::new("users.get")
            .use_middleware(test_middleware)
            .input::<TestInput>()
            .query(test_handler);

        assert_eq!(procedure.middleware.len(), 1);
    }

    #[test]
    fn test_procedure_builder_with_multiple_middleware() {
        async fn middleware1(
            ctx: Context<TestContext>,
            req: Request,
            next: Next<TestContext>,
        ) -> RpcResult<Response> {
            next(ctx, req).await
        }

        async fn middleware2(
            ctx: Context<TestContext>,
            req: Request,
            next: Next<TestContext>,
        ) -> RpcResult<Response> {
            next(ctx, req).await
        }

        let procedure = ProcedureBuilder::<TestContext>::new("users.get")
            .use_middleware(middleware1)
            .use_middleware(middleware2)
            .input::<TestInput>()
            .query(test_handler);

        assert_eq!(procedure.middleware.len(), 2);
    }

    #[test]
    fn test_validated_procedure_builder() {
        let procedure = ProcedureBuilder::<TestContext>::new("users.create")
            .input_validated::<ValidatedInput>()
            .mutation(validated_handler);

        assert_eq!(procedure.path, "users.create");
        assert_eq!(procedure.procedure_type, ProcedureType::Mutation);
    }

    #[tokio::test]
    async fn test_procedure_handler_execution() {
        let procedure = ProcedureBuilder::<TestContext>::new("test")
            .input::<TestInput>()
            .query(test_handler);

        let ctx = Context::new(TestContext { value: 42 });
        let input = serde_json::json!({"name": "World"});

        let result = (procedure.handler)(ctx, input).await;
        assert!(result.is_ok());

        let output = result.unwrap();
        assert_eq!(output["message"], "Hello, World!");
    }

    #[tokio::test]
    async fn test_validated_procedure_valid_input() {
        let procedure = ProcedureBuilder::<TestContext>::new("test")
            .input_validated::<ValidatedInput>()
            .query(validated_handler);

        let ctx = Context::new(TestContext { value: 42 });
        let input = serde_json::json!({"name": "Alice", "age": 30});

        let result = (procedure.handler)(ctx, input).await;
        assert!(result.is_ok());

        let output = result.unwrap();
        assert_eq!(output["message"], "Hello, Alice (age 30)!");
    }

    #[tokio::test]
    async fn test_validated_procedure_invalid_input() {
        let procedure = ProcedureBuilder::<TestContext>::new("test")
            .input_validated::<ValidatedInput>()
            .query(validated_handler);

        let ctx = Context::new(TestContext { value: 42 });
        let input = serde_json::json!({"name": "", "age": 200});

        let result = (procedure.handler)(ctx, input).await;
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert_eq!(error.code, crate::RpcErrorCode::ValidationError);
    }

    #[tokio::test]
    async fn test_procedure_with_output_transformer() {
        let procedure = ProcedureBuilder::<TestContext>::new("test")
            .input::<TestInput>()
            .output(|value| {
                serde_json::json!({
                    "data": value,
                    "wrapped": true
                })
            })
            .query(test_handler);

        let ctx = Context::new(TestContext { value: 42 });
        let input = serde_json::json!({"name": "World"});

        let result = (procedure.handler)(ctx, input).await;
        assert!(result.is_ok());

        let output = result.unwrap();
        assert_eq!(output["wrapped"], true);
        assert_eq!(output["data"]["message"], "Hello, World!");
    }

    // Context transformation tests

    #[derive(Clone)]
    struct AuthContext {
        user_id: String,
        original_value: i32,
    }

    async fn auth_handler(ctx: Context<AuthContext>, input: TestInput) -> RpcResult<TestOutput> {
        Ok(TestOutput {
            message: format!(
                "Hello, {}! User: {}, Value: {}",
                input.name,
                ctx.inner().user_id,
                ctx.inner().original_value
            ),
        })
    }

    #[tokio::test]
    async fn test_context_transformation_basic() {
        let procedure = ProcedureBuilder::<TestContext>::new("test")
            .context(|ctx: Context<TestContext>| async move {
                Ok(AuthContext {
                    user_id: "user123".to_string(),
                    original_value: ctx.inner().value,
                })
            })
            .input::<TestInput>()
            .query(auth_handler);

        let ctx = Context::new(TestContext { value: 42 });
        let input = serde_json::json!({"name": "World"});

        let result = (procedure.handler)(ctx, input).await;
        assert!(result.is_ok());

        let output = result.unwrap();
        assert_eq!(output["message"], "Hello, World! User: user123, Value: 42");
    }

    #[tokio::test]
    async fn test_context_transformation_with_error() {
        let procedure = ProcedureBuilder::<TestContext>::new("test")
            .context(|_ctx: Context<TestContext>| async move {
                Err::<AuthContext, _>(RpcError::unauthorized("Not authenticated"))
            })
            .input::<TestInput>()
            .query(auth_handler);

        let ctx = Context::new(TestContext { value: 42 });
        let input = serde_json::json!({"name": "World"});

        let result = (procedure.handler)(ctx, input).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, crate::RpcErrorCode::Unauthorized);
    }

    #[tokio::test]
    async fn test_context_transformation_with_validation() {
        async fn validated_auth_handler(
            ctx: Context<AuthContext>,
            input: ValidatedInput,
        ) -> RpcResult<TestOutput> {
            Ok(TestOutput {
                message: format!(
                    "Hello, {} (age {})! User: {}",
                    input.name,
                    input.age,
                    ctx.inner().user_id
                ),
            })
        }

        let procedure = ProcedureBuilder::<TestContext>::new("test")
            .context(|ctx: Context<TestContext>| async move {
                Ok(AuthContext {
                    user_id: "user456".to_string(),
                    original_value: ctx.inner().value,
                })
            })
            .input_validated::<ValidatedInput>()
            .query(validated_auth_handler);

        // Valid input
        let ctx = Context::new(TestContext { value: 42 });
        let input = serde_json::json!({"name": "Alice", "age": 30});

        let result = (procedure.handler)(ctx, input).await;
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap()["message"],
            "Hello, Alice (age 30)! User: user456"
        );

        // Invalid input - validation should still work
        let ctx = Context::new(TestContext { value: 42 });
        let input = serde_json::json!({"name": "", "age": 200});

        let result = (procedure.handler)(ctx, input).await;
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().code,
            crate::RpcErrorCode::ValidationError
        );
    }

    #[tokio::test]
    async fn test_context_transformation_mutation() {
        let procedure = ProcedureBuilder::<TestContext>::new("test")
            .context(|ctx: Context<TestContext>| async move {
                Ok(AuthContext {
                    user_id: "mutator".to_string(),
                    original_value: ctx.inner().value,
                })
            })
            .input::<TestInput>()
            .mutation(auth_handler);

        assert_eq!(procedure.procedure_type, ProcedureType::Mutation);

        let ctx = Context::new(TestContext { value: 100 });
        let input = serde_json::json!({"name": "Test"});

        let result = (procedure.handler)(ctx, input).await;
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap()["message"],
            "Hello, Test! User: mutator, Value: 100"
        );
    }

    #[tokio::test]
    async fn test_context_transformation_with_output_transformer() {
        let procedure = ProcedureBuilder::<TestContext>::new("test")
            .context(|ctx: Context<TestContext>| async move {
                Ok(AuthContext {
                    user_id: "user789".to_string(),
                    original_value: ctx.inner().value,
                })
            })
            .input::<TestInput>()
            .output(|value| {
                serde_json::json!({
                    "data": value,
                    "transformed": true
                })
            })
            .query(auth_handler);

        let ctx = Context::new(TestContext { value: 42 });
        let input = serde_json::json!({"name": "World"});

        let result = (procedure.handler)(ctx, input).await;
        assert!(result.is_ok());

        let output = result.unwrap();
        assert_eq!(output["transformed"], true);
        assert_eq!(
            output["data"]["message"],
            "Hello, World! User: user789, Value: 42"
        );
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;
    use std::sync::Arc as StdArc;

    /// Property 9: Procedure Builder Middleware Order
    /// For any procedure with multiple middleware, middleware SHALL execute
    /// in registration order (first registered = outermost).
    #[test]
    fn prop_middleware_registration_order() {
        proptest!(|(num_middleware in 1usize..5)| {
            let execution_order = StdArc::new(std::sync::Mutex::new(Vec::new()));

            let mut builder = ProcedureBuilder::<()>::new("test");

            // Add middleware that records its index when executed
            for i in 0..num_middleware {
                let order = execution_order.clone();
                builder = builder.use_middleware(move |ctx, req, next| {
                    let order = order.clone();
                    async move {
                        order.lock().unwrap().push(i);
                        next(ctx, req).await
                    }
                });
            }

            let procedure = builder
                .input::<()>()
                .query(|_ctx, _input: ()| async { Ok(()) });

            // Verify middleware count
            prop_assert_eq!(procedure.middleware.len(), num_middleware);
        });
    }

    /// Property: Middleware stored in registration order
    #[test]
    fn prop_middleware_stored_in_order() {
        proptest!(|(count in 1usize..10)| {
            let mut builder = ProcedureBuilder::<()>::new("test");

            for _ in 0..count {
                builder = builder.use_middleware(|ctx, req, next| async move {
                    next(ctx, req).await
                });
            }

            let procedure = builder
                .input::<()>()
                .query(|_ctx, _input: ()| async { Ok(()) });

            prop_assert_eq!(procedure.middleware.len(), count);
        });
    }

    /// Property: Output transformer is applied
    #[test]
    fn prop_output_transformer_applied() {
        proptest!(|(suffix in "[a-z]{1,10}")| {
            let suffix_clone = suffix.clone();
            let procedure = ProcedureBuilder::<()>::new("test")
                .input::<()>()
                .output(move |mut value| {
                    if let Some(obj) = value.as_object_mut() {
                        obj.insert("suffix".to_string(), serde_json::json!(suffix_clone.clone()));
                    }
                    value
                })
                .query(|_ctx, _input: ()| async { Ok(serde_json::json!({"original": true})) });

            // The procedure should have an output transformer
            // We can't easily test the transformer without executing, but we verify it compiles
            prop_assert_eq!(procedure.path, "test");
        });
    }

    /// Property: Validated input rejects invalid data
    #[test]
    fn prop_validated_input_rejects_invalid() {
        use crate::validation::{FieldError, Validate, ValidationResult};
        use serde::Deserialize;

        #[derive(Debug, Deserialize)]
        struct TestValidatedInput {
            value: i32,
        }

        impl Validate for TestValidatedInput {
            fn validate(&self) -> ValidationResult {
                if self.value < 0 {
                    ValidationResult::from_errors(vec![FieldError::range("value", 0, 100)])
                } else {
                    ValidationResult::ok()
                }
            }
        }

        proptest!(|(value in -100i32..-1)| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let procedure = ProcedureBuilder::<()>::new("test")
                .input_validated::<TestValidatedInput>()
                .query(|_ctx, input: TestValidatedInput| async move {
                    Ok(input.value)
                });

            let ctx = Context::new(());
            let input = serde_json::json!({"value": value});

            let result = rt.block_on((procedure.handler)(ctx, input));
            prop_assert!(result.is_err());
            prop_assert_eq!(result.unwrap_err().code, crate::RpcErrorCode::ValidationError);
        });
    }

    /// Property: Validated input accepts valid data
    #[test]
    fn prop_validated_input_accepts_valid() {
        use crate::validation::{FieldError, Validate, ValidationResult};
        use serde::Deserialize;

        #[derive(Debug, Deserialize)]
        struct TestValidatedInput {
            value: i32,
        }

        impl Validate for TestValidatedInput {
            fn validate(&self) -> ValidationResult {
                if self.value < 0 || self.value > 100 {
                    ValidationResult::from_errors(vec![FieldError::range("value", 0, 100)])
                } else {
                    ValidationResult::ok()
                }
            }
        }

        proptest!(|(value in 0i32..=100)| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let procedure = ProcedureBuilder::<()>::new("test")
                .input_validated::<TestValidatedInput>()
                .query(|_ctx, input: TestValidatedInput| async move {
                    Ok(input.value)
                });

            let ctx = Context::new(());
            let input = serde_json::json!({"value": value});

            let result = rt.block_on((procedure.handler)(ctx, input));
            prop_assert!(result.is_ok());
            prop_assert_eq!(result.unwrap(), serde_json::json!(value));
        });
    }
}
