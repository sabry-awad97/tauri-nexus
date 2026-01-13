//! Framework integration modules.
//!
//! This module provides optional integrations with popular Rust frameworks.
//! Each integration is feature-gated to avoid unnecessary dependencies.
//!
//! ## Available Integrations
//!
//! - `tauri` - Integration with Tauri's RPC plugin TypeSchema format

#[cfg(feature = "tauri")]
pub mod tauri;

#[cfg(feature = "tauri")]
pub use self::tauri::*;
