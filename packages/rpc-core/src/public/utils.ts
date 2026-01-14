// =============================================================================
// @tauri-nexus/rpc-core - Utilities (Public API)
// =============================================================================
// Re-exports utility functions for public consumption.

export {
  // Timing
  sleep,
  calculateBackoff,
  // Retry
  withRetry,
  defaultRetryConfig,
  type RetryConfig,
  // Serialization
  stableStringify,
  deduplicationKey,
  // Deduplication
  withDedup,
  // Backend utilities
  getProcedures,
  getSubscriptionCount,
} from "../utils";
