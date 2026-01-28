//! Router implementation with builder pattern
//!
//! This module provides the [`Router`] and [`CompiledRouter`] types for building
//! and executing RPC procedure handlers.
//!
//! # Router
//!
//! The [`Router`] type uses a builder pattern to configure procedures and middleware:
//!
//! ```rust,ignore
//! let router = Router::new()
//!     .context(AppContext::default())
//!     .middleware(logging)
//!     .query("health", health_handler)
//!     .mutation("create", create_handler)
//!     .subscription("events", events_handler)
//!     .merge("users", users_router());
//! ```
//!
//! # Compiled Router
//!
//! For optimized performance, compile the router to pre-build middleware chains:
//!
//! ```rust,ignore
//! let compiled = router.compile();
//! // Middleware chains are now pre-computed for O(1) execution
//! ```
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
