//! Application RPC Module
//!
//! This module defines the RPC router, handlers, and types for the application.

mod context;
mod handlers;
mod types;

pub use context::AppContext;
pub use handlers::create_router;
pub use types::*;

// Re-export plugin types for convenience
pub use tauri_plugin_rpc::prelude::*;
