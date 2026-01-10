//! # Tauri RPC Plugin
//!
//! A type-safe, ORPC-style RPC framework for Tauri applications.
//!
//! ## Features
//! - Builder pattern for router configuration
//! - Context injection for handlers
//! - Middleware support with async/await
//! - Nested routers with namespacing
//! - Type-safe error handling
//!
//! ## Example
//! ```rust,ignore
//! use tauri_plugin_rpc::*;
//!
//! fn create_router() -> Router<AppContext> {
//!     Router::new()
//!         .context(AppContext::default())
//!         .middleware(logging)
//!         .query("health", health_check)
//!         .merge("users", users_router())
//! }
//!
//! tauri::Builder::default()
//!     .plugin(tauri_plugin_rpc::init(create_router()))
//!     .run(tauri::generate_context!())
//! ```

mod context;
mod error;
mod handler;
pub mod middleware;
mod plugin;
mod router;
pub mod types;

// Public API
pub use context::{Context, EmptyContext};
pub use error::{RpcError, RpcResult};
pub use handler::Handler;
pub use middleware::{Middleware, Next, Request, ProcedureType};
pub use plugin::{init, DynRouter};
pub use router::Router;
pub use types::*;

/// Prelude for convenient imports
pub mod prelude {
    pub use crate::{
        Context, EmptyContext, Handler, Middleware, Next, 
        ProcedureType, Request, RpcError, RpcResult, Router,
        PaginatedResponse, PaginationInput, SuccessResponse,
        init,
    };
}
