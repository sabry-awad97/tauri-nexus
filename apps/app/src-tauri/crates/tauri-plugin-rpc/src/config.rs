//! Configuration module for the RPC plugin.
//!
//! This module provides the [`RpcConfig`] struct for customizing plugin behavior,
//! as well as [`PluginConfig`] for plugin-level settings like shutdown and event naming.
//!
//! # Example
//! ```rust,ignore
//! use tauri_plugin_rpc::{RpcConfig, BackpressureStrategy, BatchConfig, PluginConfig};
//! use std::time::Duration;
//!
//! let config = RpcConfig {
//!     max_input_size: 512 * 1024,  // 512KB
//!     default_channel_buffer: 64,
//!     backpressure_strategy: BackpressureStrategy::DropOldest,
//!     debug_logging: true,
//!     cleanup_interval_secs: 30,
//!     batch_config: BatchConfig::default(),
//! };
//!
//! let plugin_config = PluginConfig::default()
//!     .with_shutdown_timeout(Duration::from_secs(10))
//!     .with_event_prefix("custom:events:");
//! ```

use crate::batch::BatchConfig;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::Duration;

/// Error type for configuration validation failures.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ConfigValidationError {
    /// max_input_size must be greater than 0
    InvalidMaxInputSize,
    /// default_channel_buffer must be greater than 0
    InvalidChannelBuffer,
    /// cleanup_interval_secs must be greater than 0
    InvalidCleanupInterval,
    /// BatchConfig validation failed
    InvalidBatchConfig(String),
}

impl fmt::Display for ConfigValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMaxInputSize => {
                write!(f, "max_input_size must be greater than 0")
            }
            Self::InvalidChannelBuffer => {
                write!(f, "default_channel_buffer must be greater than 0")
            }
            Self::InvalidCleanupInterval => {
                write!(f, "cleanup_interval_secs must be greater than 0")
            }
            Self::InvalidBatchConfig(msg) => {
                write!(f, "invalid batch config: {}", msg)
            }
        }
    }
}

impl std::error::Error for ConfigValidationError {}

/// Strategy for handling backpressure when subscription channels are full.
///
/// When a subscription channel reaches its buffer capacity, this strategy
/// determines how new events are handled.
///
/// # Variants
///
/// * `Block` - Block the producer until space is available. This ensures no
///   events are lost but may slow down the producer.
///
/// * `DropOldest` - Drop the oldest events in the buffer to make room for new
///   ones. This ensures the producer is never blocked but may lose events.
///
/// * `Error` - Return an error when the channel is full. This allows the
///   producer to handle backpressure explicitly.
///
/// # Example
/// ```rust,ignore
/// use tauri_plugin_rpc::{RpcConfig, BackpressureStrategy};
///
/// let config = RpcConfig::new()
///     .with_backpressure_strategy(BackpressureStrategy::DropOldest);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum BackpressureStrategy {
    /// Block the producer until space is available in the channel.
    /// This ensures no events are lost but may slow down the producer.
    #[default]
    Block,
    /// Drop the oldest events in the buffer to make room for new ones.
    /// This ensures the producer is never blocked but may lose events.
    DropOldest,
    /// Return an error when the channel is full.
    /// This allows the producer to handle backpressure explicitly.
    Error,
}

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
/// * `backpressure_strategy` - Strategy for handling backpressure when subscription
///   channels are full. Default: `Block`.
///
/// * `debug_logging` - Enable verbose debug logging for troubleshooting.
///   Default: false.
///
/// * `cleanup_interval_secs` - Interval in seconds for subscription cleanup tasks.
///   Default: 60 seconds.
///
/// * `batch_config` - Configuration for batch request processing.
///   Default: `BatchConfig::default()`.
///
/// # Example
/// ```rust,ignore
/// use tauri_plugin_rpc::{RpcConfig, BackpressureStrategy, BatchConfig};
///
/// // Use defaults
/// let config = RpcConfig::default();
///
/// // Or customize
/// let config = RpcConfig {
///     max_input_size: 2 * 1024 * 1024,  // 2MB
///     default_channel_buffer: 128,
///     backpressure_strategy: BackpressureStrategy::DropOldest,
///     debug_logging: cfg!(debug_assertions),
///     cleanup_interval_secs: 120,
///     batch_config: BatchConfig::default(),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcConfig {
    /// Maximum input JSON size in bytes (default: 1MB)
    pub max_input_size: usize,
    /// Default subscription channel buffer size (default: 32)
    pub default_channel_buffer: usize,
    /// Strategy for handling backpressure when channels are full (default: Block)
    pub backpressure_strategy: BackpressureStrategy,
    /// Enable debug logging (default: false)
    pub debug_logging: bool,
    /// Subscription cleanup interval in seconds (default: 60)
    pub cleanup_interval_secs: u64,
    /// Batch request configuration
    pub batch_config: BatchConfig,
}

impl Default for RpcConfig {
    fn default() -> Self {
        Self {
            max_input_size: 1024 * 1024, // 1MB
            default_channel_buffer: 32,
            backpressure_strategy: BackpressureStrategy::default(),
            debug_logging: false,
            cleanup_interval_secs: 60,
            batch_config: BatchConfig::default(),
        }
    }
}

impl RpcConfig {
    /// Create a new configuration with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Validate the configuration and return an error if invalid.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - `max_input_size` is 0
    /// - `default_channel_buffer` is 0
    /// - `cleanup_interval_secs` is 0
    /// - `batch_config` is invalid (e.g., max_batch_size is 0)
    ///
    /// # Example
    /// ```rust,ignore
    /// let config = RpcConfig::new();
    /// config.validate().expect("Config should be valid");
    /// ```
    pub fn validate(&self) -> Result<(), ConfigValidationError> {
        if self.max_input_size == 0 {
            return Err(ConfigValidationError::InvalidMaxInputSize);
        }
        if self.default_channel_buffer == 0 {
            return Err(ConfigValidationError::InvalidChannelBuffer);
        }
        if self.cleanup_interval_secs == 0 {
            return Err(ConfigValidationError::InvalidCleanupInterval);
        }
        // Validate embedded BatchConfig
        if let Err(e) = self.batch_config.validate() {
            return Err(ConfigValidationError::InvalidBatchConfig(e));
        }
        Ok(())
    }

    /// Set the maximum input size in bytes.
    ///
    /// # Example
    /// ```rust,ignore
    /// let config = RpcConfig::new().with_max_input_size(512 * 1024);
    /// ```
    #[must_use = "This method returns a new RpcConfig and does not modify self"]
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
    #[must_use = "This method returns a new RpcConfig and does not modify self"]
    pub fn with_channel_buffer(mut self, size: usize) -> Self {
        self.default_channel_buffer = size;
        self
    }

    /// Set the backpressure strategy for subscription channels.
    ///
    /// # Example
    /// ```rust,ignore
    /// use tauri_plugin_rpc::BackpressureStrategy;
    ///
    /// let config = RpcConfig::new()
    ///     .with_backpressure_strategy(BackpressureStrategy::DropOldest);
    /// ```
    #[must_use = "This method returns a new RpcConfig and does not modify self"]
    pub fn with_backpressure_strategy(mut self, strategy: BackpressureStrategy) -> Self {
        self.backpressure_strategy = strategy;
        self
    }

    /// Enable or disable debug logging.
    ///
    /// # Example
    /// ```rust,ignore
    /// let config = RpcConfig::new().with_debug_logging(true);
    /// ```
    #[must_use = "This method returns a new RpcConfig and does not modify self"]
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
    #[must_use = "This method returns a new RpcConfig and does not modify self"]
    pub fn with_cleanup_interval(mut self, secs: u64) -> Self {
        self.cleanup_interval_secs = secs;
        self
    }

    /// Set the batch configuration.
    ///
    /// # Example
    /// ```rust,ignore
    /// use tauri_plugin_rpc::{RpcConfig, BatchConfig};
    ///
    /// let config = RpcConfig::new()
    ///     .with_batch_config(BatchConfig::new().with_max_batch_size(50));
    /// ```
    #[must_use = "This method returns a new RpcConfig and does not modify self"]
    pub fn with_batch_config(mut self, config: BatchConfig) -> Self {
        self.batch_config = config;
        self
    }
}

// =============================================================================
// Plugin Configuration
// =============================================================================

/// Extended configuration for the RPC plugin.
///
/// This configuration extends the base `RpcConfig` with additional options
/// for plugin-level behavior like shutdown, event naming, and event buffering.
///
/// # Example
///
/// ```rust,ignore
/// use tauri_plugin_rpc::PluginConfig;
/// use std::time::Duration;
///
/// let config = PluginConfig::default()
///     .with_shutdown_timeout(Duration::from_secs(10))
///     .with_event_prefix("custom:events:")
///     .with_event_buffering(100, Duration::from_millis(50));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    /// Timeout for plugin shutdown operations.
    ///
    /// This timeout prevents the application from hanging during shutdown
    /// if subscriptions don't clean up promptly.
    ///
    /// Default: 5 seconds
    pub shutdown_timeout: Duration,

    /// Prefix for subscription event names in Tauri event system.
    ///
    /// Format: "{prefix}{subscription_id}"
    /// Example: "rpc:subscription:sub_01234567..."
    ///
    /// Default: "rpc:subscription:"
    pub subscription_event_prefix: String,

    /// Event buffer size for subscriptions.
    ///
    /// When set to a value > 1, events are buffered and flushed in batches
    /// to reduce the number of IPC calls to the frontend.
    ///
    /// Set to 1 to disable buffering (immediate emission).
    ///
    /// Default: 1 (disabled)
    pub event_buffer_size: usize,

    /// Event buffer flush interval.
    ///
    /// When buffering is enabled, events are flushed either when the buffer
    /// is full or after this interval, whichever comes first.
    ///
    /// Default: 50ms
    pub event_buffer_flush_interval: Duration,
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            shutdown_timeout: Duration::from_secs(5),
            subscription_event_prefix: "rpc:subscription:".to_string(),
            event_buffer_size: 1, // Disabled by default
            event_buffer_flush_interval: Duration::from_millis(50),
        }
    }
}

impl PluginConfig {
    /// Create a new plugin configuration with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the shutdown timeout.
    ///
    /// # Arguments
    ///
    /// * `timeout` - Maximum time to wait for shutdown
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let config = PluginConfig::new()
    ///     .with_shutdown_timeout(Duration::from_secs(10));
    /// ```
    #[must_use = "This method returns a new PluginConfig and does not modify self"]
    pub fn with_shutdown_timeout(mut self, timeout: Duration) -> Self {
        self.shutdown_timeout = timeout;
        self
    }

    /// Set the subscription event prefix.
    ///
    /// # Arguments
    ///
    /// * `prefix` - The event prefix string
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let config = PluginConfig::new()
    ///     .with_event_prefix("custom:events:");
    /// ```
    #[must_use = "This method returns a new PluginConfig and does not modify self"]
    pub fn with_event_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.subscription_event_prefix = prefix.into();
        self
    }

    /// Enable event buffering with specified buffer size and flush interval.
    ///
    /// When buffering is enabled, events are collected in a buffer and flushed
    /// either when the buffer reaches capacity or after the flush interval,
    /// whichever comes first. This reduces IPC overhead for high-frequency events.
    ///
    /// # Arguments
    ///
    /// * `buffer_size` - Number of events to buffer before flushing (must be > 1)
    /// * `flush_interval` - Maximum time to wait before flushing buffered events
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// // Buffer up to 100 events, flush every 50ms
    /// let config = PluginConfig::new()
    ///     .with_event_buffering(100, Duration::from_millis(50));
    /// ```
    #[must_use = "This method returns a new PluginConfig and does not modify self"]
    pub fn with_event_buffering(mut self, buffer_size: usize, flush_interval: Duration) -> Self {
        self.event_buffer_size = buffer_size;
        self.event_buffer_flush_interval = flush_interval;
        self
    }

    /// Disable event buffering (immediate emission).
    ///
    /// This is the default behavior. Events are emitted immediately as they arrive.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let config = PluginConfig::new()
    ///     .without_event_buffering();
    /// ```
    #[must_use = "This method returns a new PluginConfig and does not modify self"]
    pub fn without_event_buffering(mut self) -> Self {
        self.event_buffer_size = 1;
        self
    }

    /// Check if event buffering is enabled.
    ///
    /// Returns `true` if buffer size is greater than 1.
    pub fn is_buffering_enabled(&self) -> bool {
        self.event_buffer_size > 1
    }

    /// Validate the configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - `shutdown_timeout` is zero
    /// - `subscription_event_prefix` is empty
    /// - `event_buffer_size` is zero
    /// - `event_buffer_flush_interval` is zero (when buffering is enabled)
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let config = PluginConfig::new();
    /// config.validate().expect("Config should be valid");
    /// ```
    pub fn validate(&self) -> Result<(), String> {
        if self.shutdown_timeout.is_zero() {
            return Err("shutdown_timeout must be greater than zero".to_string());
        }
        if self.subscription_event_prefix.is_empty() {
            return Err("subscription_event_prefix cannot be empty".to_string());
        }
        if self.event_buffer_size == 0 {
            return Err("event_buffer_size must be greater than zero".to_string());
        }
        if self.is_buffering_enabled() && self.event_buffer_flush_interval.is_zero() {
            return Err(
                "event_buffer_flush_interval must be greater than zero when buffering is enabled"
                    .to_string(),
            );
        }
        Ok(())
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod plugin_config_tests {
    use super::*;
    use proptest::prelude::*;

    // Property 23: Configuration validation at initialization
    proptest! {
        #[test]
        fn prop_configuration_validation(
            timeout_secs in 1u64..3600u64,
            prefix in "[a-z:]{1,50}",
            buffer_size in 1usize..1000usize,
            flush_ms in 1u64..1000u64
        ) {
            let config = PluginConfig::new()
                .with_shutdown_timeout(Duration::from_secs(timeout_secs))
                .with_event_prefix(prefix)
                .with_event_buffering(buffer_size, Duration::from_millis(flush_ms));

            assert!(config.validate().is_ok());
        }

        #[test]
        fn prop_zero_timeout_invalid(
            prefix in "[a-z:]{1,50}",
            buffer_size in 1usize..100usize,
            flush_ms in 1u64..100u64
        ) {
            let config = PluginConfig {
                shutdown_timeout: Duration::from_secs(0),
                subscription_event_prefix: prefix,
                event_buffer_size: buffer_size,
                event_buffer_flush_interval: Duration::from_millis(flush_ms),
            };

            assert!(config.validate().is_err());
        }

        #[test]
        fn prop_empty_prefix_invalid(
            timeout_secs in 1u64..3600u64,
            buffer_size in 1usize..100usize,
            flush_ms in 1u64..100u64
        ) {
            let config = PluginConfig {
                shutdown_timeout: Duration::from_secs(timeout_secs),
                subscription_event_prefix: String::new(),
                event_buffer_size: buffer_size,
                event_buffer_flush_interval: Duration::from_millis(flush_ms),
            };

            assert!(config.validate().is_err());
        }
    }

    // Unit tests for configuration builder
    #[test]
    fn test_with_shutdown_timeout() {
        let config = PluginConfig::new().with_shutdown_timeout(Duration::from_secs(10));

        assert_eq!(config.shutdown_timeout, Duration::from_secs(10));
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_with_event_prefix() {
        let config = PluginConfig::new().with_event_prefix("custom:prefix:");

        assert_eq!(config.subscription_event_prefix, "custom:prefix:");
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_default_configuration_values() {
        let config = PluginConfig::default();

        assert_eq!(config.shutdown_timeout, Duration::from_secs(5));
        assert_eq!(config.subscription_event_prefix, "rpc:subscription:");
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_builder_chain() {
        let config = PluginConfig::new()
            .with_shutdown_timeout(Duration::from_secs(15))
            .with_event_prefix("test:events:");

        assert_eq!(config.shutdown_timeout, Duration::from_secs(15));
        assert_eq!(config.subscription_event_prefix, "test:events:");
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validation_errors() {
        // Zero timeout
        let config = PluginConfig {
            shutdown_timeout: Duration::from_secs(0),
            subscription_event_prefix: "rpc:subscription:".to_string(),
            event_buffer_size: 1,
            event_buffer_flush_interval: Duration::from_millis(50),
        };
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("shutdown_timeout"));

        // Empty prefix
        let config = PluginConfig {
            shutdown_timeout: Duration::from_secs(5),
            subscription_event_prefix: String::new(),
            event_buffer_size: 1,
            event_buffer_flush_interval: Duration::from_millis(50),
        };
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("subscription_event_prefix"));
    }
}

// Task 10.2: Unit tests for buffer configuration
#[test]
fn test_with_event_buffering() {
    let config = PluginConfig::new().with_event_buffering(100, Duration::from_millis(50));

    assert_eq!(config.event_buffer_size, 100);
    assert_eq!(
        config.event_buffer_flush_interval,
        Duration::from_millis(50)
    );
    assert!(config.is_buffering_enabled());
    assert!(config.validate().is_ok());
}

#[test]
fn test_without_event_buffering() {
    let config = PluginConfig::new()
        .with_event_buffering(100, Duration::from_millis(50))
        .without_event_buffering();

    assert_eq!(config.event_buffer_size, 1);
    assert!(!config.is_buffering_enabled());
    assert!(config.validate().is_ok());
}

#[test]
fn test_buffering_disabled_by_default() {
    let config = PluginConfig::default();

    assert_eq!(config.event_buffer_size, 1);
    assert!(!config.is_buffering_enabled());
}

#[test]
fn test_buffering_validation_zero_buffer_size() {
    let config = PluginConfig {
        shutdown_timeout: Duration::from_secs(5),
        subscription_event_prefix: "rpc:subscription:".to_string(),
        event_buffer_size: 0,
        event_buffer_flush_interval: Duration::from_millis(50),
    };

    let result = config.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("event_buffer_size"));
}

#[test]
fn test_buffering_validation_zero_flush_interval() {
    let config = PluginConfig {
        shutdown_timeout: Duration::from_secs(5),
        subscription_event_prefix: "rpc:subscription:".to_string(),
        event_buffer_size: 100,
        event_buffer_flush_interval: Duration::from_secs(0),
    };

    let result = config.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("event_buffer_flush_interval"));
}

#[test]
fn test_buffering_size_one_is_valid() {
    // Buffer size of 1 means immediate emission (no buffering)
    let config = PluginConfig {
        shutdown_timeout: Duration::from_secs(5),
        subscription_event_prefix: "rpc:subscription:".to_string(),
        event_buffer_size: 1,
        event_buffer_flush_interval: Duration::from_secs(0), // Can be zero when buffering disabled
    };

    assert!(config.validate().is_ok());
    assert!(!config.is_buffering_enabled());
}
