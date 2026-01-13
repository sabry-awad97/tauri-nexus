// =============================================================================
// @tauri-nexus/rpc-core - Client Module (Internal)
// =============================================================================
// Exports Effect-based APIs only. Promise wrappers are in public/.

// Configuration
export {
  configureRpc,
  getConfig,
  isSubscriptionPath,
  type RpcClientConfig,
} from "./config";

// Batch operations (Effect-based)
export {
  EffectBatchBuilder,
  EffectBatchResponseWrapper,
  executeBatchEffect,
  createEffectBatch,
} from "./batch";
