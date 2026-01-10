//! App RPC - Router, Types, and Context

mod types;
mod context;
mod handlers;

pub use types::*;
pub use context::*;
pub use handlers::*;

// Re-export plugin utilities
pub use tauri_plugin_rpc::{
    Router, Context, RpcError, RpcResult,
    PaginatedResponse, PaginationInput, SuccessResponse,
};
