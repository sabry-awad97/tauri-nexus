//! Tauri Type-Safe RPC Plugin
//!
//! ORPC-style router with builder pattern, context, and middleware support.

mod router;
mod context;
pub mod middleware;
mod handler;
mod error;
mod plugin;
pub mod types;

pub use router::*;
pub use context::*;
pub use middleware::*;
pub use handler::*;
pub use error::*;
pub use plugin::*;
pub use types::*;

// Re-export for convenience
pub use serde;
pub use serde_json;
