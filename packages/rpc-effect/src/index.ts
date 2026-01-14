// =============================================================================
// @tauri-nexus/rpc-effect - Effect-Based RPC Library
// =============================================================================
// Pure Effect-based RPC implementation with clean architecture.

// =============================================================================
// Core Types & Errors
// =============================================================================

export type {
  ProcedureType,
  Event,
  EventIterator,
  ValidationIssue,
  RpcConfig,
  RpcTransport,
  SubscribeTransportOptions,
  RpcInterceptorChain,
  RpcInterceptor,
  InterceptorContext,
  RpcLogger,
  EffectRequestContext,
  EffectResponseContext,
} from "./core";

export {
  // Error classes
  RpcCallError,
  RpcTimeoutError,
  RpcCancelledError,
  RpcValidationError,
  RpcNetworkError,
  type RpcEffectError,
  // Error constructors
  createCallError,
  createTimeoutError,
  createCancelledError,
  createValidationError,
  createNetworkError,
  // Type guards
  isEffectRpcError,
  isRpcCallError,
  isRpcTimeoutError,
  isRpcCancelledError,
  isRpcValidationError,
  isRpcNetworkError,
  // Code utilities
  type VirtualErrorCode,
  getErrorCode,
  hasCode,
  hasAnyCode,
  isRetryableError,
  // Pattern matching
  type ErrorHandlers,
  matchError,
  // Effect combinators
  failWithCallError,
  failWithTimeout,
  failWithValidation,
  failWithNetwork,
  failWithCancelled,
} from "./core";

// =============================================================================
// Services
// =============================================================================

export {
  RpcConfigService,
  RpcTransportService,
  RpcInterceptorService,
  RpcLoggerService,
  consoleLogger,
  type RpcServices,
} from "./services";

// =============================================================================
// Serializable Errors (for Promise API)
// =============================================================================

export {
  type RpcError,
  type RpcErrorCode,
  type RpcErrorShape,
  toRpcError,
  fromRpcError,
  isRpcError,
  hasErrorCode,
  createRpcError,
  isRateLimitError,
  getRateLimitRetryAfter,
  type ErrorParserOptions,
  isRpcErrorShape,
  parseJsonError,
  createCallErrorFromShape,
  parseToEffectError,
  fromTransportError,
  parseEffectError,
  parseError,
} from "./serializable";

// =============================================================================
// Validation
// =============================================================================

export {
  validatePathPure,
  isValidPathPure,
  validatePathOrThrow,
  type PathValidationResult,
  validatePath,
  validatePaths,
  validateAndNormalizePath,
  isValidPath,
  validatePathWithRules,
  type PathValidationRules,
} from "./validation";

// =============================================================================
// Operations
// =============================================================================

export {
  defaultParseError,
  type CallOptions,
  call,
  callWithTimeout,
  type SubscribeOptions,
  subscribe,
  type BatchRequestItem,
  type BatchResultItem,
  type BatchRequest,
  type BatchResponse,
  validateBatchRequests,
  batchCall,
} from "./operations";

// =============================================================================
// Interceptors
// =============================================================================

export {
  type InterceptorOptions,
  type InterceptorHandler,
  createInterceptorFactory,
  createSimpleInterceptor,
  composeInterceptors,
  loggingInterceptor,
  type LoggingInterceptorOptions,
  retryInterceptor,
  type RetryInterceptorOptions,
  authInterceptor,
  type AuthInterceptorOptions,
  timingInterceptor,
  dedupeInterceptor,
  type DedupeInterceptorOptions,
  errorHandlerInterceptor,
} from "./interceptors";

// =============================================================================
// Utilities
// =============================================================================

export {
  stableStringify,
  sleep,
  calculateBackoff,
  type RetryConfig,
  defaultRetryConfig,
  createRetrySchedule,
  withRetry,
  withRetryDetailed,
  createDedupCache,
  deduplicationKey,
  withDedup,
} from "./utils";

// =============================================================================
// Subscription
// =============================================================================

export {
  type SubscriptionEventType,
  type SubscriptionEvent,
  type SubscriptionError,
  type SubscriptionState,
  type ReconnectConfig,
  type QueueItem,
  SHUTDOWN_SENTINEL,
  defaultReconnectConfig,
  createSubscriptionState,
  createSubscriptionStateRef,
  createEventQueue,
  markCompleted,
  updateLastEventId,
  incrementConsumers,
  decrementConsumers,
  resetForReconnect,
  incrementReconnectAttempts,
  resetReconnectAttempts,
  offerEvent,
  sendShutdownSentinels,
  takeFromQueue,
  calculateReconnectDelay,
  shouldReconnect,
  prepareReconnect,
  waitForReconnect,
  maxReconnectsExceededError,
  processDataEvent,
  processErrorEvent,
  generateSubscriptionId,
  extractSubscriptionError,
} from "./subscription";

// =============================================================================
// Client
// =============================================================================

export {
  EffectLink,
  type EffectLinkConfig,
  createEffectClient,
  createEffectClientWithTransport,
  type EffectClientConfig,
  type EffectClient,
  createRpcLayer,
  createDebugLayer,
  getRuntime,
  initializeRuntime,
  disposeRuntime,
  runEffect,
  getConfig,
  getTransport,
  getInterceptors,
  getLogger,
} from "./client";
