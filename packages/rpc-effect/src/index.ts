// =============================================================================
// @tauri-nexus/rpc-effect - Effect-Based RPC Library
// =============================================================================
// Pure Effect-based RPC implementation with clean architecture.
// Includes idiomatic Effect patterns: Schema validation, Context services,
// Metrics, Circuit Breaker, Rate Limiting, Caching, and Request Batching.

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
  // Types
  type ResilienceServices,
  type ResilienceErrors,
  type SchemaConfig,
  type ResilienceConfig,
  type CallOptions,
  type SubscribeOptions,
  // Error handling
  defaultParseError,
  // Call
  call,
  createCall,
  createResilientCall,
  // Subscribe
  subscribe,
  subscribeStream,
  subscribeCollect,
  subscribeForEach,
  createSubscribe,
  // Batch
  type BatchRequestItem,
  type BatchResultItem,
  type BatchRequest,
  type BatchResponse,
  validateBatchRequests,
  batchCall,
  batchCallParallel,
  batchCallParallelCollect,
  batchCallParallelFailFast,
  batchCallSequential,
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
  // Atomic operations using Ref.modify
  incrementAndGetConsumers,
  decrementAndGetConsumers,
  incrementAndGetReconnectAttempts,
  markCompletedOnce,
  updateAndGetLastEventId,
  getState,
  offerEvent,
  sendShutdownSentinels,
  takeFromQueue,
  // Schedule-based reconnection
  createReconnectSchedule,
  withReconnection,
  calculateReconnectDelay,
  shouldReconnect,
  prepareReconnect,
  waitForReconnect,
  maxReconnectsExceededError,
  processDataEvent,
  processErrorEvent,
  generateSubscriptionId,
  extractSubscriptionError,
  // Stream-based API
  type SubscriptionStreamConfig,
  type AsyncIteratorConfig,
  createSubscriptionStream,
  createManagedSubscriptionStream,
  scopedConnection,
  collectStream,
  runStreamWithCallbacks,
  runStreamInterruptible,
  createAsyncIterator,
  // Resource management
  withSubscription,
  // PubSub for multi-consumer
  type BroadcastSubscription,
  createBroadcastSubscription,
  createScopedBroadcastSubscription,
  // Stream from async iterable
  createEventSourceStream,
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

// =============================================================================
// Schema Validation (Effect Schema)
// =============================================================================
// Note: Schema-validated calls are available via callWithSchema and
// resilientCallWithSchema in the operations module. These functions
// accept Effect Schema definitions for input/output validation.

export {
  // Error schema utilities (used internally)
  createSchemaValidationError,
  mapSchemaError,
  schemaIssueToValidationIssue,
} from "./schema";

// =============================================================================
// Request Context (Effect Context)
// =============================================================================
// Core context types used internally by call(). Additional context utilities
// (ResponseContext, TimingContext, etc.) are available in the context module.

export {
  // Request context (used by call.ts)
  RequestContext,
  type RequestContextData,
  createRequestContext,
  // Trace context (used by call.ts)
  TraceContext,
  type TraceContextData,
  createTraceContext,
  generateTraceId,
  generateSpanId,
  // Span management (used by resilience)
  SpanContext,
  withSpan,
} from "./context";

// =============================================================================
// Metrics (Effect Metric)
// =============================================================================

export {
  // Core metrics
  rpcCallCounter,
  rpcErrorCounter,
  rpcLatencyHistogram,
  rpcActiveCallsGauge,
  rpcRetryCounter,
  rpcCacheHitCounter,
  rpcCacheMissCounter,
  // Metric combinators
  withMetrics,
  withLatencyTracking,
  withErrorCounting,
  withActiveCallTracking,
  // Metric tags
  type MetricTags,
  createMetricTags,
  // Metric service
  MetricsService,
  type MetricsConfig,
  createMetricsLayer,
  // Metric snapshots
  getMetricSnapshot,
  type MetricSnapshot,
  // Configuration
  defaultLatencyBoundaries,
  createLatencyBoundaries,
  createMetricName,
  type MetricNamespace,
} from "./metrics";

// =============================================================================
// Resilience (Circuit Breaker, Bulkhead, Rate Limiting)
// =============================================================================
// Core resilience services and combinators used by call(). Additional utilities
// (getCircuitState, resetCircuit, createTokenBucket, hedging) are available
// in the resilience module directly.

export {
  // Circuit breaker
  CircuitBreakerService,
  type CircuitBreakerConfig,
  createCircuitBreaker,
  withCircuitBreaker,
  CircuitOpenError,
  // Bulkhead
  BulkheadService,
  type BulkheadConfig,
  createBulkhead,
  withBulkhead,
  BulkheadFullError,
  // Rate limiting
  RateLimiterService,
  type RateLimiterConfig,
  createRateLimiter,
  withRateLimit,
  RateLimitExceededError,
} from "./resilience";

// =============================================================================
// Cache (Effect Cache)
// =============================================================================
// Core cache types used internally by call(). Additional cache utilities
// (invalidation, SWR, warming) are available in the cache module.

export {
  // Cache service (used by call.ts)
  RpcCacheService,
  type RpcCacheConfig,
  createRpcCacheLayer,
  // Cache combinator (used by call.ts)
  withCache,
} from "./cache";
