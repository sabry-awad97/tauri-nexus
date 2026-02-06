//! Test module for tauri-plugin-rpc
//!
//! This module contains property-based tests using proptest
//! to validate correctness properties of the RPC framework.

#[cfg(test)]
pub mod subscription_tests;

#[cfg(test)]
pub mod handler_tests;

#[cfg(test)]
pub mod error_tests;

#[cfg(test)]
pub mod validation_tests;

#[cfg(test)]
pub mod middleware_tests;

#[cfg(test)]
pub mod config_tests;

#[cfg(test)]
pub mod batch_tests;

#[cfg(test)]
pub mod plugin_helpers_tests;
