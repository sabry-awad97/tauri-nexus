// =============================================================================
// @tauri-nexus/rpc-core - Client Module
// =============================================================================

// Configuration
export { configureRpc, getConfig, type RpcClientConfig } from "./config";

// Core call functions
export { call, subscribe } from "./call";

// Batch operations
export {
  TypedBatchBuilder,
  TypedBatchResponseWrapper,
  TypedBatchResponse,
  // Effect-based batch
  EffectBatchBuilder,
  EffectBatchResponseWrapper,
  executeBatchEffect,
  createEffectBatch,
} from "./batch";

// Client factories
export {
  createClient,
  createClientWithSubscriptions,
  type RpcClient,
} from "./factory";
