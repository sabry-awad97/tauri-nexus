//! Router implementation with builder pattern
//!
//! This module provides the [`Router`] and [`CompiledRouter`] types for building
//! and executing RPC procedure handlers with a fluent, type-safe API.
//!
//! # Module Organization
//!
//! The router module is organized into several submodules for maintainability:
//!
//! - [`core`] - Core router implementation ([`Router`], [`CompiledRouter`])
//! - [`builder`] - Procedure builder chains for the fluent API
//! - [`context_transform`] - Context transformation chains
//! - [`types`] - Shared type definitions
//! - [`middleware_chain`] - Middleware chain building utilities
//! - [`dyn_router`] - Dynamic router trait for polymorphism
//!
//! # Quick Start
//!
//! ## Basic Router
//!
//! The [`Router`] type uses a builder pattern to configure procedures and middleware:
//!
//! ```rust,ignore
//! use tauri_plugin_rpc::{Router, Context, RpcResult};
//!
//! #[derive(Clone)]
//! struct AppContext {
//!     db: Database,
//! }
//!
//! async fn get_user(ctx: Context<AppContext>, input: GetUserInput) -> RpcResult<User> {
//!     let user = ctx.inner().db.get_user(input.id).await?;
//!     Ok(user)
//! }
//!
//! let router = Router::new()
//!     .context(AppContext { db: Database::new() })
//!     .middleware(logging_middleware)
//!     .procedure("users.get")
//!         .input::<GetUserInput>()
//!         .query(get_user);
//! ```
//!
//! ## Compiled Router (Recommended for Production)
//!
//! For optimized performance, compile the router to pre-build middleware chains:
//!
//! ```rust,ignore
//! let compiled = router.compile();
//! // Middleware chains are now pre-computed for O(1) execution
//! // Use compiled router for all production calls
//! let result = compiled.call("users.get", input).await?;
//! ```
//!
//! ## Fluent API with Validation
//!
//! ```rust,ignore
//! router
//!     .procedure("users.create")
//!         .use_middleware(auth_middleware)
//!         .input_validated::<CreateUserInput>()  // Automatic validation
//!         .mutation(create_user);
//! ```
//!
//! ## Context Transformation
//!
//! Transform context types for specific procedures:
//!
//! ```rust,ignore
//! router
//!     .procedure("admin.delete")
//!         .context(|ctx: Context<AppContext>| async move {
//!             // Verify admin permissions and transform context
//!             let user = authenticate(&ctx).await?;
//!             if !user.is_admin {
//!                 return Err(RpcError::unauthorized("Admin required"));
//!             }
//!             Ok(AdminContext { user, app: ctx.inner().clone() })
//!         })
//!         .input::<DeleteInput>()
//!         .mutation(admin_delete);
//! ```
//!
//! # Performance
//!
//! - **Compiled Router**: Pre-builds middleware chains at compile time for O(1) lookup
//! - **Uncompiled Router**: Builds chains dynamically, still efficient for development
//! - **Batch Execution**: Parallel processing support for multiple calls
//! - **Zero-cost Abstractions**: Builder pattern compiles away at runtime
//!
//! # Architecture
//!
//! The router uses a layered architecture:
//!
//! 1. **Builder Layer** - Type-safe fluent API for procedure definition
//! 2. **Middleware Layer** - Composable middleware chains (onion model)
//! 3. **Handler Layer** - Async procedure handlers with context
//! 4. **Execution Layer** - Optimized procedure execution and routing
//!
//! Both `Router` and `CompiledRouter` implement [`DynRouter`] and can be passed
//! to [`init`](crate::init) or [`init_with_config`](crate::init_with_config).

// Module declarations
mod builder;
mod context_transform;
mod core;
mod dyn_router;
mod middleware_chain;
mod types;

// Public re-exports
pub use builder::{ProcedureChain, TypedProcedureChain, ValidatedProcedureChain};
pub use context_transform::{
    ContextTransformedChain, ContextTransformedTypedChain, ContextTransformedValidatedChain,
};
pub use core::{CompiledRouter, Router};

#[allow(unused)]
pub(crate) use middleware_chain::build_middleware_chain;

#[cfg(test)]
mod tests;
