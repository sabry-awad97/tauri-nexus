// =============================================================================
// @tauri-nexus/rpc-effect - Effect-Based RPC Library
// =============================================================================
// Pure Effect-based RPC implementation with no Promise wrappers.
// Use this package for Effect-first development.

// =============================================================================
// Core Types
// =============================================================================

export type {
  ProcedureType,
  Event,
  EventIterator,
  ValidationIssue,
  RpcEffectError,
  RpcConfig,
  RpcTransport,
  SubscribeTransportOptions,
  RpcInterceptorChain,
  RpcInterceptor,
  InterceptorContext,
  RpcLogger,
  EffectRequestContext,
  EffectResponseContext,
} from "./types";

export {
  RpcCallError,
  RpcTimeoutError,
  RpcCancelledError,
  RpcValidationError,
  RpcNetworkError,
  RpcConfigService,
  RpcTransportService,
  RpcInterceptorService,
  RpcLoggerService,
} from "./types";

// =============================================================================
// Error Utilities
// =============================================================================

export {
  // Error constructors
  makeCallError,
  makeTimeoutError,
  makeCancelledError,
  makeValidationError,
  makeNetworkError,
  // Error conversion
  toEffectError,
  isEffectRpcError,
  // Effect utilities
  failWithCallError,
  failWithTimeout,
  failWithValidation,
  // Type guards
  isRpcCallError,
  isRpcTimeoutError,
  isRpcCancelledError,
  isRpcValidationError,
  isRpcNetworkError,
  hasCode,
  matchError,
  // Backward compatibility
  parseEffectError,
} from "./errors";

// =============================================================================
// Validation
// =============================================================================

export {
  // Pure functions (no Effect)
  validatePathPure,
  isValidPathPure,
  validatePathOrThrow,
  type PathValidationResult,
  // Effect-based
  validatePath,
  validatePaths,
  validateAndNormalizePath,
  isValidPath,
  validatePathWithRules,
  type PathValidationRules,
} from "./validation";

// =============================================================================
// Runtime
// =============================================================================

export {
  makeConfigLayer,
  makeTransportLayer,
  makeInterceptorLayer,
  makeLoggerLayer,
  makeRpcLayer,
  makeDebugLayer,
  consoleLogger,
  getRuntime,
  initializeRuntime,
  disposeRuntime,
  runEffect,
  getConfig,
  getTransport,
  getInterceptors,
  getLogger,
  type RpcServices,
} from "./runtime";

// =============================================================================
// Call Effects
// =============================================================================

export {
  call,
  callWithTimeout,
  subscribe,
  validateBatchRequests,
  batchCall,
  type CallOptions,
  type SubscribeOptions,
  type BatchRequestItem,
  type BatchResultItem,
  type BatchRequest,
  type BatchResponse,
} from "./call";

// =============================================================================
// Utilities
// =============================================================================

export {
  sleep,
  calculateBackoff,
  withRetry,
  withRetryDetailed,
  createRetrySchedule,
  defaultRetryConfig,
  createDedupCache,
  deduplicationKey,
  withDedup,
  stableStringify,
  type RetryConfig,
} from "./utils";

// =============================================================================
// Interceptors
// =============================================================================

export {
  loggingInterceptor,
  retryInterceptor,
  errorHandlerInterceptor,
  authInterceptor,
  timingInterceptor,
  dedupeInterceptor,
  // Deprecated aliases
  createLoggingInterceptor,
  createRetryInterceptor,
  createErrorInterceptor,
  createAuthInterceptor,
  type InterceptorOptions,
  type RetryInterceptorOptions,
  type AuthInterceptorOptions,
} from "./interceptors";

// =============================================================================
// Link
// =============================================================================

export { EffectLink, type EffectLinkConfig } from "./link";

// =============================================================================
// Client
// =============================================================================

export {
  createEffectClient,
  createEffectClientWithTransport,
  type EffectClientConfig,
  type EffectClient,
} from "./client";

// =============================================================================
// Subscription Primitives
// =============================================================================

export {
  // Types
  type SubscriptionEventType,
  type SubscriptionEvent,
  type SubscriptionError,
  type SubscriptionState,
  type ReconnectConfig,
  type QueueItem,
  SHUTDOWN_SENTINEL,
  // Configuration
  defaultReconnectConfig,
  // State Management
  createSubscriptionState,
  makeSubscriptionStateRef,
  makeEventQueue,
  markCompleted,
  updateLastEventId,
  incrementConsumers,
  decrementConsumers,
  resetForReconnect,
  incrementReconnectAttempts,
  resetReconnectAttempts,
  // Queue Operations
  offerEvent,
  sendShutdownSentinels,
  takeFromQueue,
  // Reconnection Logic
  calculateReconnectDelay,
  shouldReconnect,
  prepareReconnect,
  waitForReconnect,
  maxReconnectsExceededError,
  // Event Processing
  processDataEvent,
  processErrorEvent,
  // Utilities
  generateSubscriptionId,
  extractSubscriptionError,
} from "./subscription";
