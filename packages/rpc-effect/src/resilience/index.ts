// =============================================================================
// Resilience Module - Circuit Breaker, Bulkhead, and Rate Limiting
// =============================================================================
// Provides resilience patterns using idiomatic Effect constructs.

export {
  // Circuit breaker
  CircuitBreakerService,
  type CircuitState,
  type CircuitBreakerConfig,
  type CircuitBreakerState,
  createCircuitBreaker,
  withCircuitBreaker,
  getCircuitState,
  resetCircuit,
  CircuitOpenError,
} from "./circuit-breaker";

export {
  // Bulkhead (concurrency limiting)
  BulkheadService,
  type BulkheadConfig,
  createBulkhead,
  withBulkhead,
  BulkheadFullError,
} from "./bulkhead";

export {
  // Rate limiting
  RateLimiterService,
  type RateLimiterConfig,
  type RateLimiterState,
  createRateLimiter,
  withRateLimit,
  RateLimitExceededError,
  // Token bucket
  createTokenBucket,
  type TokenBucketConfig,
} from "./rate-limiter";

export {
  // Timeout with cleanup
  withTimeoutAndCleanup,
  type TimeoutConfig,
  // Hedging (speculative execution)
  withHedging,
  type HedgingConfig,
} from "./timeout";
