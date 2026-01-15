// =============================================================================
// Rate Limiter - Token bucket rate limiting
// =============================================================================

import { Effect, Context, Layer, Ref, Data } from "effect";

// =============================================================================
// Types
// =============================================================================

/**
 * Rate limiter configuration.
 */
export interface RateLimiterConfig {
  /** Maximum requests per window */
  readonly maxRequests: number;
  /** Time window in milliseconds */
  readonly windowMs: number;
  /** Whether to use sliding window */
  readonly slidingWindow: boolean;
}

/**
 * Rate limiter state.
 */
export interface RateLimiterState {
  readonly tokens: number;
  readonly lastRefill: number;
  readonly requestTimestamps: readonly number[];
}

/**
 * Token bucket configuration.
 */
export interface TokenBucketConfig {
  /** Maximum tokens in bucket */
  readonly capacity: number;
  /** Tokens added per refill */
  readonly refillAmount: number;
  /** Refill interval in milliseconds */
  readonly refillIntervalMs: number;
}

// =============================================================================
// Errors
// =============================================================================

/**
 * Error thrown when rate limit is exceeded.
 */
export class RateLimitExceededError extends Data.TaggedError(
  "RateLimitExceededError",
)<{
  readonly path: string;
  readonly retryAfterMs: number;
  readonly limit: number;
  readonly remaining: number;
}> {
  get message(): string {
    return `Rate limit exceeded for ${this.path}. Retry after ${this.retryAfterMs}ms`;
  }
}

// =============================================================================
// Default Configuration
// =============================================================================

const defaultConfig: RateLimiterConfig = {
  maxRequests: 100,
  windowMs: 60000,
  slidingWindow: true,
};

const defaultTokenBucketConfig: TokenBucketConfig = {
  capacity: 100,
  refillAmount: 10,
  refillIntervalMs: 1000,
};

// =============================================================================
// Rate Limiter Service
// =============================================================================

/**
 * Rate limiter service interface.
 */
export interface RateLimiterServiceInterface {
  readonly stateRef: Ref.Ref<RateLimiterState>;
  readonly config: RateLimiterConfig;
}

/**
 * Rate limiter service.
 */
export class RateLimiterService extends Context.Tag("RateLimiterService")<
  RateLimiterService,
  RateLimiterServiceInterface
>() {}

// =============================================================================
// Rate Limiter Creation
// =============================================================================

const initialState: RateLimiterState = {
  tokens: 0,
  lastRefill: Date.now(),
  requestTimestamps: [],
};

/**
 * Create a rate limiter instance.
 */
export const createRateLimiter = (
  config: Partial<RateLimiterConfig> = {},
): Effect.Effect<RateLimiterServiceInterface> =>
  Effect.gen(function* () {
    const fullConfig = { ...defaultConfig, ...config };
    const stateRef = yield* Ref.make<RateLimiterState>({
      ...initialState,
      tokens: fullConfig.maxRequests,
    });
    return { stateRef, config: fullConfig };
  });

/**
 * Create a rate limiter layer.
 */
export const createRateLimiterLayer = (
  config: Partial<RateLimiterConfig> = {},
): Layer.Layer<RateLimiterService> =>
  Layer.effect(RateLimiterService, createRateLimiter(config));

// =============================================================================
// Token Bucket
// =============================================================================

/**
 * Token bucket state.
 */
interface TokenBucketState {
  readonly tokens: number;
  readonly lastRefill: number;
}

/**
 * Create a token bucket rate limiter.
 */
export const createTokenBucket = (
  config: Partial<TokenBucketConfig> = {},
): Effect.Effect<{
  acquire: Effect.Effect<boolean>;
  getTokens: Effect.Effect<number>;
}> =>
  Effect.gen(function* () {
    const fullConfig = { ...defaultTokenBucketConfig, ...config };
    const stateRef = yield* Ref.make<TokenBucketState>({
      tokens: fullConfig.capacity,
      lastRefill: Date.now(),
    });

    const refill = Ref.modify(stateRef, (state) => {
      const now = Date.now();
      const elapsed = now - state.lastRefill;
      const refills = Math.floor(elapsed / fullConfig.refillIntervalMs);

      if (refills > 0) {
        const newTokens = Math.min(
          fullConfig.capacity,
          state.tokens + refills * fullConfig.refillAmount,
        );
        return [
          newTokens,
          {
            tokens: newTokens,
            lastRefill:
              state.lastRefill + refills * fullConfig.refillIntervalMs,
          },
        ];
      }

      return [state.tokens, state];
    });

    const acquire = Effect.gen(function* () {
      yield* refill;
      return yield* Ref.modify(stateRef, (state) => {
        if (state.tokens > 0) {
          return [true, { ...state, tokens: state.tokens - 1 }];
        }
        return [false, state];
      });
    });

    const getTokens = Effect.gen(function* () {
      yield* refill;
      const state = yield* Ref.get(stateRef);
      return state.tokens;
    });

    return { acquire, getTokens };
  });

// =============================================================================
// Rate Limiter Combinator
// =============================================================================

/**
 * Wrap an effect with rate limiting.
 */
export const withRateLimit = <A, E, R>(
  path: string,
  effect: Effect.Effect<A, E, R>,
): Effect.Effect<A, E | RateLimitExceededError, R | RateLimiterService> =>
  Effect.gen(function* () {
    const { stateRef, config } = yield* RateLimiterService;
    const now = Date.now();

    // Check and update rate limit
    const result = yield* Ref.modify(stateRef, (state) => {
      if (config.slidingWindow) {
        // Sliding window: filter out old timestamps
        const windowStart = now - config.windowMs;
        const validTimestamps = state.requestTimestamps.filter(
          (ts) => ts > windowStart,
        );

        if (validTimestamps.length >= config.maxRequests) {
          // Calculate retry after
          const oldestInWindow = validTimestamps[0];
          const retryAfter = oldestInWindow + config.windowMs - now;
          return [
            {
              allowed: false,
              retryAfter,
              remaining: 0,
            },
            state,
          ];
        }

        return [
          {
            allowed: true,
            retryAfter: 0,
            remaining: config.maxRequests - validTimestamps.length - 1,
          },
          {
            ...state,
            requestTimestamps: [...validTimestamps, now],
          },
        ];
      } else {
        // Fixed window
        const windowStart = Math.floor(now / config.windowMs) * config.windowMs;
        const isNewWindow = state.lastRefill < windowStart;

        const currentCount = isNewWindow ? 0 : state.requestTimestamps.length;

        if (currentCount >= config.maxRequests) {
          const retryAfter = windowStart + config.windowMs - now;
          return [
            {
              allowed: false,
              retryAfter,
              remaining: 0,
            },
            state,
          ];
        }

        return [
          {
            allowed: true,
            retryAfter: 0,
            remaining: config.maxRequests - currentCount - 1,
          },
          {
            ...state,
            lastRefill: isNewWindow ? windowStart : state.lastRefill,
            requestTimestamps: isNewWindow
              ? [now]
              : [...state.requestTimestamps, now],
          },
        ];
      }
    });

    if (!result.allowed) {
      return yield* Effect.fail(
        new RateLimitExceededError({
          path,
          retryAfterMs: result.retryAfter,
          limit: config.maxRequests,
          remaining: result.remaining,
        }),
      );
    }

    return yield* effect;
  });
