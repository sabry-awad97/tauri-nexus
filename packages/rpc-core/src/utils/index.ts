// =============================================================================
// @tauri-nexus/rpc-core - Utility Functions (Internal)
// =============================================================================
// Exports Effect-based APIs only. Promise wrappers are in public/.

export {
  // Backend utilities
  getProceduresEffect,
  getSubscriptionCountEffect,
  // Timing
  sleepEffect,
  calculateBackoffEffect,
  // Retry
  withRetryEffect,
  withRetryEffectDetailed,
  createRetrySchedule,
  type EffectRetryConfig,
  defaultEffectRetryConfig,
  // Deduplication
  createDedupCache,
  deduplicationKeyEffect,
  withDedupEffect,
  // Serialization
  stableStringifyEffect,
} from "./effect-utils";
