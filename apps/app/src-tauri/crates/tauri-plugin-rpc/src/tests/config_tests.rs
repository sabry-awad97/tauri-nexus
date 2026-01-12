//! Configuration tests - Property-based tests for RpcConfig
//!
//! Tests that configuration defaults are sensible and validation works correctly.

use proptest::prelude::*;

use crate::batch::BatchConfig;
use crate::config::{BackpressureStrategy, ConfigValidationError, RpcConfig};

// =============================================================================
// Property-Based Tests
// =============================================================================

proptest! {
    /// **Property 12: Configuration Defaults**
    /// *For any* RpcConfig created with `Default::default()`, all fields SHALL have
    /// sensible non-zero values that allow the plugin to function correctly.
    /// **Feature: tauri-rpc-plugin-optimization, Property 12: Configuration Defaults**
    #[test]
    fn prop_configuration_defaults_are_valid(_dummy in 0..1i32) {
        let config = RpcConfig::default();

        // All defaults should pass validation
        prop_assert!(config.validate().is_ok(), "Default config should be valid");

        // max_input_size should be non-zero and reasonable (at least 1KB)
        prop_assert!(config.max_input_size > 0, "max_input_size should be > 0");
        prop_assert!(config.max_input_size >= 1024, "max_input_size should be at least 1KB");

        // default_channel_buffer should be non-zero and reasonable
        prop_assert!(config.default_channel_buffer > 0, "default_channel_buffer should be > 0");
        prop_assert!(config.default_channel_buffer >= 1, "default_channel_buffer should be at least 1");

        // cleanup_interval_secs should be non-zero
        prop_assert!(config.cleanup_interval_secs > 0, "cleanup_interval_secs should be > 0");

        // debug_logging default should be false (production-safe)
        prop_assert!(!config.debug_logging, "debug_logging should default to false");
    }

    /// Property: Invalid configurations are rejected
    /// Configurations with zero values for critical fields should fail validation
    #[test]
    fn prop_invalid_configs_rejected(
        max_input_size in 0usize..2,
        channel_buffer in 0usize..2,
        cleanup_interval in 0u64..2,
    ) {
        let config = RpcConfig {
            max_input_size,
            default_channel_buffer: channel_buffer,
            cleanup_interval_secs: cleanup_interval,
            ..RpcConfig::default()
        };

        let result = config.validate();

        // If any critical field is 0, validation should fail
        if max_input_size == 0 || channel_buffer == 0 || cleanup_interval == 0 {
            prop_assert!(result.is_err(), "Config with zero values should be invalid");
        } else {
            prop_assert!(result.is_ok(), "Config with non-zero values should be valid");
        }
    }

    /// Property: Builder pattern preserves validity
    /// Using builder methods with valid values should produce valid configs
    #[test]
    fn prop_builder_pattern_preserves_validity(
        max_input_size in 1usize..10_000_000,
        channel_buffer in 1usize..1000,
        cleanup_interval in 1u64..3600,
        debug_logging in any::<bool>(),
    ) {
        let config = RpcConfig::new()
            .with_max_input_size(max_input_size)
            .with_channel_buffer(channel_buffer)
            .with_cleanup_interval(cleanup_interval)
            .with_debug_logging(debug_logging);

        prop_assert!(config.validate().is_ok(), "Builder-created config should be valid");
        prop_assert_eq!(config.max_input_size, max_input_size);
        prop_assert_eq!(config.default_channel_buffer, channel_buffer);
        prop_assert_eq!(config.cleanup_interval_secs, cleanup_interval);
        prop_assert_eq!(config.debug_logging, debug_logging);
    }

    /// Property: Backpressure strategy can be set to any variant
    #[test]
    fn prop_backpressure_strategy_variants(strategy_idx in 0usize..3) {
        let strategies = [
            BackpressureStrategy::Block,
            BackpressureStrategy::DropOldest,
            BackpressureStrategy::Error,
        ];
        let strategy = strategies[strategy_idx];

        let config = RpcConfig::new().with_backpressure_strategy(strategy);

        prop_assert!(config.validate().is_ok());
        prop_assert_eq!(config.backpressure_strategy, strategy);
    }

    /// **Property 2: BatchConfig Builder Preserves Values**
    /// *For any* valid BatchConfig value, calling `RpcConfig::new().with_batch_config(config)`
    /// SHALL result in an RpcConfig where `batch_config` equals the provided config.
    /// **Feature: tauri-plugin-rpc-improvements, Property 2: BatchConfig Builder Preserves Values**
    /// **Validates: Requirements 2.3**
    #[test]
    fn prop_batch_config_builder_preserves_values(
        max_size in 1usize..1000,
        parallel in any::<bool>(),
    ) {
        let batch_config = BatchConfig::new()
            .with_max_batch_size(max_size)
            .with_parallel_execution(parallel);
        let rpc_config = RpcConfig::new().with_batch_config(batch_config);

        prop_assert_eq!(rpc_config.batch_config.max_batch_size, max_size);
        prop_assert_eq!(rpc_config.batch_config.parallel_execution, parallel);
    }

    /// **Property 3: RpcConfig Validation Propagates to BatchConfig**
    /// *For any* RpcConfig with an invalid BatchConfig (e.g., max_batch_size = 0),
    /// calling `validate()` on the RpcConfig SHALL return an error.
    /// **Feature: tauri-plugin-rpc-improvements, Property 3: RpcConfig Validation Propagates to BatchConfig**
    /// **Validates: Requirements 2.4**
    #[test]
    fn prop_rpc_config_validation_propagates_to_batch_config(
        valid_max_input in 1usize..1000000,
    ) {
        // Valid RpcConfig with invalid BatchConfig should fail
        let invalid_batch = BatchConfig::new().with_max_batch_size(0);
        let config = RpcConfig::new()
            .with_max_input_size(valid_max_input)
            .with_batch_config(invalid_batch);

        let result = config.validate();
        prop_assert!(result.is_err(), "Config with invalid BatchConfig should fail validation");

        // Verify it's specifically an InvalidBatchConfig error
        if let Err(ConfigValidationError::InvalidBatchConfig(_)) = result {
            // Expected error type
        } else {
            prop_assert!(false, "Expected InvalidBatchConfig error");
        }
    }
}

// =============================================================================
// Unit Tests
// =============================================================================

#[test]
fn test_default_config_values() {
    let config = RpcConfig::default();

    assert_eq!(config.max_input_size, 1024 * 1024); // 1MB
    assert_eq!(config.default_channel_buffer, 32);
    assert_eq!(config.backpressure_strategy, BackpressureStrategy::Block);
    assert!(!config.debug_logging);
    assert_eq!(config.cleanup_interval_secs, 60);
}

#[test]
fn test_config_validation_errors() {
    // Zero max_input_size
    let config = RpcConfig {
        max_input_size: 0,
        ..RpcConfig::default()
    };
    assert_eq!(
        config.validate(),
        Err(ConfigValidationError::InvalidMaxInputSize)
    );

    // Zero channel buffer
    let config = RpcConfig {
        default_channel_buffer: 0,
        ..RpcConfig::default()
    };
    assert_eq!(
        config.validate(),
        Err(ConfigValidationError::InvalidChannelBuffer)
    );

    // Zero cleanup interval
    let config = RpcConfig {
        cleanup_interval_secs: 0,
        ..RpcConfig::default()
    };
    assert_eq!(
        config.validate(),
        Err(ConfigValidationError::InvalidCleanupInterval)
    );
}

#[test]
fn test_config_builder_chain() {
    let config = RpcConfig::new()
        .with_max_input_size(512 * 1024)
        .with_channel_buffer(64)
        .with_backpressure_strategy(BackpressureStrategy::DropOldest)
        .with_debug_logging(true)
        .with_cleanup_interval(30);

    assert_eq!(config.max_input_size, 512 * 1024);
    assert_eq!(config.default_channel_buffer, 64);
    assert_eq!(
        config.backpressure_strategy,
        BackpressureStrategy::DropOldest
    );
    assert!(config.debug_logging);
    assert_eq!(config.cleanup_interval_secs, 30);
    assert!(config.validate().is_ok());
}

#[test]
fn test_validation_error_display() {
    assert_eq!(
        ConfigValidationError::InvalidMaxInputSize.to_string(),
        "max_input_size must be greater than 0"
    );
    assert_eq!(
        ConfigValidationError::InvalidChannelBuffer.to_string(),
        "default_channel_buffer must be greater than 0"
    );
    assert_eq!(
        ConfigValidationError::InvalidCleanupInterval.to_string(),
        "cleanup_interval_secs must be greater than 0"
    );
}
