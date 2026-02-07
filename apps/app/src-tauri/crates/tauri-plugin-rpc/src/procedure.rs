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
use crate::schema::ProcedureMeta;
use crate::validation::Validate;
use crate::{Context, RpcError, RpcResult};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;
use tracing::{debug, trace, warn};

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

/// Helper macro to generate the handler logic with optional validation and context transformation.
///
/// This macro eliminates code duplication across the four builder types while maintaining
/// type safety and avoiding unsafe code.
macro_rules! build_handler_impl {
    // Simple handler: no validation, no context transformation
    (
        simple,
        $handler:expr,
        $output_transformer:expr,
        $ctx:ident,
        $input_value:ident,
        $Input:ty,
        $Ctx:ty
    ) => {{
        let handler = $handler.clone();
        let output_transformer = $output_transformer.clone();

        Box::pin(async move {
            // Deserialize input
            trace!("Deserializing procedure input");
            let input: $Input = serde_json::from_value($input_value).map_err(|e| {
                warn!(error = %e, "Failed to deserialize input");
                RpcError::bad_request(format!("Invalid input: {}", e))
            })?;

            // Call handler
            trace!("Executing procedure handler");
            let output = handler($ctx, input).await.inspect_err(|e| {
                debug!(error_code = %e.code, "Procedure handler returned error");
            })?;

            // Serialize output
            trace!("Serializing procedure output");
            let mut output_value = serde_json::to_value(output).map_err(|e| {
                warn!(error = %e, "Failed to serialize output");
                RpcError::internal(format!("Failed to serialize output: {}", e))
            })?;

            // Apply output transformer if present
            if let Some(transformer) = output_transformer {
                trace!("Applying output transformer");
                output_value = transformer(output_value);
            }

            trace!("Procedure completed successfully");
            Ok(output_value)
        })
    }};

    // Validated handler: with validation, no context transformation
    (
        validated,
        $handler:expr,
        $output_transformer:expr,
        $ctx:ident,
        $input_value:ident,
        $Input:ty,
        $Ctx:ty
    ) => {{
        let handler = $handler.clone();
        let output_transformer = $output_transformer.clone();

        Box::pin(async move {
            // Deserialize input
            trace!("Deserializing procedure input");
            let input: $Input = serde_json::from_value($input_value).map_err(|e| {
                warn!(error = %e, "Failed to deserialize input");
                RpcError::bad_request(format!("Invalid input: {}", e))
            })?;

            // Validate input
            trace!("Validating procedure input");
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
                    "Input validation failed"
                );

                let details = match serde_json::to_value(&validation_result.errors) {
                    Ok(details) => details,
                    Err(e) => {
                        warn!(error = %e, "Failed to serialize validation errors");
                        serde_json::json!({ "error": "Failed to serialize validation details" })
                    }
                };

                return Err(RpcError::validation("Validation failed").with_details(details));
            }
            trace!("Input validation passed");

            // Call handler
            trace!("Executing procedure handler");
            let output = handler($ctx, input).await.inspect_err(|e| {
                debug!(error_code = %e.code, "Procedure handler returned error");
            })?;

            // Serialize output
            trace!("Serializing procedure output");
            let mut output_value = serde_json::to_value(output).map_err(|e| {
                warn!(error = %e, "Failed to serialize output");
                RpcError::internal(format!("Failed to serialize output: {}", e))
            })?;

            // Apply output transformer if present
            if let Some(transformer) = output_transformer {
                trace!("Applying output transformer");
                output_value = transformer(output_value);
            }

            trace!("Procedure completed successfully");
            Ok(output_value)
        })
    }};

    // Context-transformed handler: with context transformation, no validation
    (
        context_transformed,
        $handler:expr,
        $output_transformer:expr,
        $context_transformer:expr,
        $ctx:ident,
        $input_value:ident,
        $Input:ty,
        $OrigCtx:ty,
        $NewCtx:ty
    ) => {{
        let handler = $handler.clone();
        let output_transformer = $output_transformer.clone();
        let context_transformer = $context_transformer.clone();

        Box::pin(async move {
            // Transform context
            trace!("Transforming procedure context");
            let new_ctx_state = (context_transformer)($ctx).await.inspect_err(|e| {
                debug!(error_code = %e.code, "Context transformation failed");
            })?;
            let new_ctx = Context::new(new_ctx_state);

            // Deserialize input
            trace!("Deserializing procedure input");
            let input: $Input = serde_json::from_value($input_value).map_err(|e| {
                warn!(error = %e, "Failed to deserialize input");
                RpcError::bad_request(format!("Invalid input: {}", e))
            })?;

            // Call handler
            trace!("Executing procedure handler");
            let output = handler(new_ctx, input).await.inspect_err(|e| {
                debug!(error_code = %e.code, "Procedure handler returned error");
            })?;

            // Serialize output
            trace!("Serializing procedure output");
            let mut output_value = serde_json::to_value(output).map_err(|e| {
                warn!(error = %e, "Failed to serialize output");
                RpcError::internal(format!("Failed to serialize output: {}", e))
            })?;

            // Apply output transformer if present
            if let Some(transformer) = output_transformer {
                trace!("Applying output transformer");
                output_value = transformer(output_value);
            }

            trace!("Procedure completed successfully");
            Ok(output_value)
        })
    }};

    // Context-transformed validated handler: with both context transformation and validation
    (
        context_transformed_validated,
        $handler:expr,
        $output_transformer:expr,
        $context_transformer:expr,
        $ctx:ident,
        $input_value:ident,
        $Input:ty,
        $OrigCtx:ty,
        $NewCtx:ty
    ) => {{
        let handler = $handler.clone();
        let output_transformer = $output_transformer.clone();
        let context_transformer = $context_transformer.clone();

        Box::pin(async move {
            // Transform context
            trace!("Transforming procedure context");
            let new_ctx_state = (context_transformer)($ctx).await.inspect_err(|e| {
                debug!(error_code = %e.code, "Context transformation failed");
            })?;
            let new_ctx = Context::new(new_ctx_state);

            // Deserialize input
            trace!("Deserializing procedure input");
            let input: $Input = serde_json::from_value($input_value).map_err(|e| {
                warn!(error = %e, "Failed to deserialize input");
                RpcError::bad_request(format!("Invalid input: {}", e))
            })?;

            // Validate input
            trace!("Validating procedure input");
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
                    "Input validation failed"
                );

                let details = match serde_json::to_value(&validation_result.errors) {
                    Ok(details) => details,
                    Err(e) => {
                        warn!(error = %e, "Failed to serialize validation errors");
                        serde_json::json!({ "error": "Failed to serialize validation details" })
                    }
                };

                return Err(RpcError::validation("Validation failed").with_details(details));
            }
            trace!("Input validation passed");

            // Call handler
            trace!("Executing procedure handler");
            let output = handler(new_ctx, input).await.inspect_err(|e| {
                debug!(error_code = %e.code, "Procedure handler returned error");
            })?;

            // Serialize output
            trace!("Serializing procedure output");
            let mut output_value = serde_json::to_value(output).map_err(|e| {
                warn!(error = %e, "Failed to serialize output");
                RpcError::internal(format!("Failed to serialize output: {}", e))
            })?;

            // Apply output transformer if present
            if let Some(transformer) = output_transformer {
                trace!("Applying output transformer");
                output_value = transformer(output_value);
            }

            trace!("Procedure completed successfully");
            Ok(output_value)
        })
    }};
}

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
    /// OpenAPI metadata for this procedure.
    pub meta: Option<ProcedureMeta>,
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
    /// OpenAPI metadata for this procedure.
    meta: Option<ProcedureMeta>,
    /// Phantom data for type tracking.
    _phantom: PhantomData<Input>,
}

impl<Ctx> ProcedureBuilder<Ctx, ()>
where
    Ctx: Clone + Send + Sync + 'static,
{
    /// Creates a new procedure builder with the given path.
    pub fn new(path: impl Into<String>) -> Self {
        let path = path.into();
        trace!(path = %path, "Creating new ProcedureBuilder");
        Self {
            path,
            middleware: Vec::new(),
            output_transformer: None,
            meta: None,
            _phantom: PhantomData,
        }
    }

    /// Sets OpenAPI metadata for this procedure.
    ///
    /// This provides an oRPC-style way to attach documentation directly
    /// to procedure definitions.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use tauri_plugin_rpc::prelude::*;
    ///
    /// let procedure = ProcedureBuilder::<AppContext>::new("users.get")
    ///     .meta(ProcedureMeta::new()
    ///         .description("Get a user by ID")
    ///         .tag("users")
    ///         .input(TypeSchema::object()
    ///             .with_property("id", TypeSchema::integer())
    ///             .with_required("id"))
    ///         .output(TypeSchema::object()
    ///             .with_property("id", TypeSchema::integer())
    ///             .with_property("name", TypeSchema::string())))
    ///     .input::<GetUserInput>()
    ///     .query(get_user);
    /// ```
    pub fn meta(mut self, meta: ProcedureMeta) -> Self {
        self.meta = Some(meta);
        self
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
            meta: self.meta,
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
            meta: self.meta,
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
            meta: self.meta,
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
        let path = self.path.clone();
        let has_transformer = output_transformer.is_some();
        let middleware_count = self.middleware.len();

        debug!(
            path = %path,
            procedure_type = %procedure_type,
            middleware_count = middleware_count,
            has_output_transformer = has_transformer,
            "Building procedure"
        );

        let boxed_handler: BoxedHandler<Ctx> = Arc::new(move |ctx, input_value| {
            build_handler_impl!(
                simple,
                handler,
                output_transformer,
                ctx,
                input_value,
                Input,
                Ctx
            )
        });

        RegisteredProcedure {
            path: self.path,
            procedure_type,
            handler: boxed_handler,
            middleware: self.middleware,
            meta: self.meta,
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
    meta: Option<ProcedureMeta>,
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
        let path = self.path.clone();
        let has_transformer = output_transformer.is_some();
        let middleware_count = self.middleware.len();

        debug!(
            path = %path,
            procedure_type = %procedure_type,
            middleware_count = middleware_count,
            has_output_transformer = has_transformer,
            validated = true,
            "Building validated procedure"
        );

        let boxed_handler: BoxedHandler<Ctx> = Arc::new(move |ctx, input_value| {
            build_handler_impl!(
                validated,
                handler,
                output_transformer,
                ctx,
                input_value,
                Input,
                Ctx
            )
        });

        RegisteredProcedure {
            path: self.path,
            procedure_type,
            handler: boxed_handler,
            middleware: self.middleware,
            meta: self.meta,
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
    meta: Option<ProcedureMeta>,
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
    ///
    /// Note: Middleware operates on the original context type, before transformation.
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
            meta: self.meta,
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
            meta: self.meta,
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
    meta: Option<ProcedureMeta>,
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
        let path = self.path.clone();
        let has_transformer = output_transformer.is_some();
        let middleware_count = self.middleware.len();

        debug!(
            path = %path,
            procedure_type = %procedure_type,
            middleware_count = middleware_count,
            has_output_transformer = has_transformer,
            context_transformed = true,
            "Building context-transformed procedure"
        );

        let boxed_handler: BoxedHandler<OrigCtx> = Arc::new(move |ctx, input_value| {
            build_handler_impl!(
                context_transformed,
                handler,
                output_transformer,
                context_transformer,
                ctx,
                input_value,
                Input,
                OrigCtx,
                NewCtx
            )
        });

        RegisteredProcedure {
            path: self.path,
            procedure_type,
            handler: boxed_handler,
            middleware: self.middleware,
            meta: self.meta,
        }
    }
}

/// A context-transformed procedure builder with validated input.
///
/// This builder is created when using `.context()` followed by `.input_validated()`
/// and combines both context transformation and input validation capabilities.
///
/// # Type Parameters
///
/// - `OrigCtx`: The original context type from the router
/// - `NewCtx`: The transformed context type that the handler will receive
/// - `Input`: The input type that will be validated before the handler executes
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
/// #[derive(Deserialize, Validate)]
/// struct CreatePostInput {
///     #[validate(length(min = 1, max = 200))]
///     title: String,
///     #[validate(length(min = 1))]
///     content: String,
/// }
///
/// let procedure = ProcedureBuilder::<AppContext>::new("posts.create")
///     .context(|ctx: Context<AppContext>| async move {
///         let user = authenticate(&ctx).await?;
///         Ok(AuthContext { app: ctx.inner().clone(), user })
///     })
///     .input_validated::<CreatePostInput>()
///     .mutation(create_post); // Handler receives Context<AuthContext> and validated input
/// ```
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
    meta: Option<ProcedureMeta>,
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
    ///
    /// Note: Middleware operates on the original context type, before transformation.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let procedure = ProcedureBuilder::<AppContext>::new("posts.create")
    ///     .context(auth_transform)
    ///     .use_middleware(logging)  // Operates on AppContext
    ///     .input_validated::<CreatePostInput>()
    ///     .mutation(create_post);   // Handler receives AuthContext
    /// ```
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
    ///
    /// Note: Middleware operates on the original context type, before transformation.
    pub fn use_middleware_fn(mut self, middleware: MiddlewareFn<OrigCtx>) -> Self {
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
    /// let procedure = ProcedureBuilder::<AppContext>::new("posts.create")
    ///     .context(auth_transform)
    ///     .input_validated::<CreatePostInput>()
    ///     .output(|value| {
    ///         json!({ "data": value, "timestamp": Utc::now().to_rfc3339() })
    ///     })
    ///     .mutation(create_post);
    /// ```
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
        let path = self.path.clone();
        let has_transformer = output_transformer.is_some();
        let middleware_count = self.middleware.len();

        debug!(
            path = %path,
            procedure_type = %procedure_type,
            middleware_count = middleware_count,
            has_output_transformer = has_transformer,
            context_transformed = true,
            validated = true,
            "Building context-transformed validated procedure"
        );

        let boxed_handler: BoxedHandler<OrigCtx> = Arc::new(move |ctx, input_value| {
            build_handler_impl!(
                context_transformed_validated,
                handler,
                output_transformer,
                context_transformer,
                ctx,
                input_value,
                Input,
                OrigCtx,
                NewCtx
            )
        });

        RegisteredProcedure {
            path: self.path,
            procedure_type,
            handler: boxed_handler,
            middleware: self.middleware,
            meta: self.meta,
        }
    }
}
