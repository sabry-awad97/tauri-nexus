//! Configuration module for the RPC plugin.
//!
//! This module provides the [`RpcConfig`] struct for customizing plugin behavior.
//!
//! # Example
//! ```rust,ignore
//! use tauri_plugin_rpc::RpcConfig;
//!
//! let config = RpcConfig {
//!     max_input_size: 512 * 1024,  // 512KB
//!     default_channel_buffer: 64,
//!     debug_logging: true,
//!     cleanup_interval_secs: 30,
//! };
//! ```

use serde::{Deserialize, Serialize};

/// Plugin configuration for customizing RPC behavior.
///
/// All fields have sensible defaults that allow the plugin to function correctly
/// out of the box. Use [`RpcConfig::default()`] to get the default configuration.
///
/// # Fields
///
/// * `max_input_size` - Maximum input JSON size in bytes. Requests exceeding this
///   limit will be rejected with a `PayloadTooLarge` error. Default: 1MB (1,048,576 bytes).
///
/// * `default_channel_buffer` - Default buffer size for subscription channels.
///   Larger values allow more events to be queued before backpressure is applied.
///   Default: 32 events.
///
/// * `debug_logging` - Enable verbose debug logging for troubleshooting.
///   Default: false.
///
/// * `cleanup_interval_secs` - Interval in seconds for subscription cleanup tasks.
///   Default: 60 seconds.
///
/// # Example
/// ```rust,ignore
/// use tauri_plugin_rpc::RpcConfig;
///
/// // Use defaults
/// let config = RpcConfig::default();
///
/// // Or customize
/// let config = RpcConfig {
///     max_input_size: 2 * 1024 * 1024,  // 2MB
///     default_channel_buffer: 128,
///     debug_logging: cfg!(debug_assertions),
///     cleanup_interval_secs: 120,
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcConfig {
    /// Maximum input JSON size in bytes (default: 1MB)
    pub max_input_size: usize,
    /// Default subscription channel buffer size (default: 32)
    pub default_channel_buffer: usize,
    /// Enable debug logging (default: false)
    pub debug_logging: bool,
    /// Subscription cleanup interval in seconds (default: 60)
    pub cleanup_interval_secs: u64,
}

impl Default for RpcConfig {
    fn default() -> Self {
        Self {
            max_input_size: 1024 * 1024, // 1MB
            default_channel_buffer: 32,
            debug_logging: false,
            cleanup_interval_secs: 60,
        }
    }
}

impl RpcConfig {
    /// Create a new configuration with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the maximum input size in bytes.
    ///
    /// # Example
    /// ```rust,ignore
    /// let config = RpcConfig::new().with_max_input_size(512 * 1024);
    /// ```
    pub fn with_max_input_size(mut self, size: usize) -> Self {
        self.max_input_size = size;
        self
    }

    /// Set the default channel buffer size.
    ///
    /// # Example
    /// ```rust,ignore
    /// let config = RpcConfig::new().with_channel_buffer(64);
    /// ```
    pub fn with_channel_buffer(mut self, size: usize) -> Self {
        self.default_channel_buffer = size;
        self
    }

    /// Enable or disable debug logging.
    ///
    /// # Example
    /// ```rust,ignore
    /// let config = RpcConfig::new().with_debug_logging(true);
    /// ```
    pub fn with_debug_logging(mut self, enabled: bool) -> Self {
        self.debug_logging = enabled;
        self
    }

    /// Set the cleanup interval in seconds.
    ///
    /// # Example
    /// ```rust,ignore
    /// let config = RpcConfig::new().with_cleanup_interval(30);
    /// ```
    pub fn with_cleanup_interval(mut self, secs: u64) -> Self {
        self.cleanup_interval_secs = secs;
        self
    }
}
