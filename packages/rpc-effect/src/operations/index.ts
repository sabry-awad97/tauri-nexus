// =============================================================================
// Operations Module Exports
// =============================================================================

export {
  defaultParseError,
  type CallOptions,
  call,
  callWithTimeout,
  type SubscribeOptions,
  subscribe,
} from "./call";

export {
  type BatchRequestItem,
  type BatchResultItem,
  type BatchRequest,
  type BatchResponse,
  validateBatchRequests,
  batchCall,
  // Parallel batch operations with Effect.all
  batchCallParallel,
  batchCallParallelCollect,
  batchCallParallelFailFast,
  batchCallSequential,
} from "./batch";
