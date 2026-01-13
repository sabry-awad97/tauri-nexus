// =============================================================================
// @tauri-nexus/rpc-core - Effect-Based Utilities
// =============================================================================
// Retry, deduplication, and timing utilities using Effect.

import { Effect, Schedule, Duration, Ref, HashMap, Option, pipe } from "effect";
import { invoke } from "@tauri-apps/api/core";
import { makeCallError, makeNetworkError } from "../internal/effect-errors";
import type { RpcEffectError } from "../internal/effect-types";

// =============================================================================
// Backend Utilities (Effect-based)
// =============================================================================

/**
 * Get list of available procedures from backend.
 */
export const getProceduresEffect = (): Effect.Effect<
  string[],
  RpcEffectError
> =>
  Effect.tryPromise({
    try: () => invoke<string[]>("plugin:rpc|rpc_procedures"),
    catch: (error) => makeNetworkError("rpc_procedures", error),
  });

/**
 * Get current subscription count from backend.
 */
export const getSubscriptionCountEffect = (): Effect.Effect<
  number,
  RpcEffectError
> =>
  Effect.tryPromise({
    try: () => invoke<number>("plugin:rpc|rpc_subscription_count"),
    catch: (error) => makeNetworkError("rpc_subscription_count", error),
  });

// =============================================================================
// Timing Utilities (Effect-based)
// =============================================================================

/**
 * Sleep effect with milliseconds.
 */
export const sleepEffect = (ms: number): Effect.Effect<void> =>
  Effect.sleep(Duration.millis(ms));

/**
 * Calculate exponential backoff with optional jitter.
 * Returns the delay in milliseconds.
 */
export const calculateBackoffEffect = (
  attempt: number,
  baseDelay: number = 1000,
  maxDelay: number = 30000,
  jitter: boolean = true,
): Effect.Effect<number> =>
  Effect.sync(() => {
    const exponentialDelay = baseDelay * Math.pow(2, attempt);
    const cappedDelay = Math.min(exponentialDelay, maxDelay);

    if (jitter) {
      return cappedDelay * (0.5 + Math.random() * 0.5);
    }

    return cappedDelay;
  });

// =============================================================================
// Retry Logic (Effect-based)
// =============================================================================

/** Retry configuration for Effect */
export interface EffectRetryConfig {
  readonly maxRetries: number;
  readonly baseDelay: number;
  readonly maxDelay: number;
  readonly retryableCodes: readonly string[];
  readonly jitter: boolean;
  readonly backoff: "linear" | "exponential";
}

export const defaultEffectRetryConfig: EffectRetryConfig = {
  maxRetries: 3,
  baseDelay: 1000,
  maxDelay: 30000,
  retryableCodes: ["INTERNAL_ERROR", "TIMEOUT", "UNAVAILABLE"],
  jitter: true,
  backoff: "exponential",
};

/**
 * Check if an error is retryable based on its code.
 */
const isRetryableError = (
  error: RpcEffectError,
  retryableCodes: readonly string[],
): boolean => {
  if (error._tag === "RpcCallError") {
    return retryableCodes.includes(error.code);
  }
  if (error._tag === "RpcTimeoutError") {
    return retryableCodes.includes("TIMEOUT");
  }
  if (error._tag === "RpcNetworkError") {
    return retryableCodes.includes("INTERNAL_ERROR");
  }
  // Don't retry validation or cancellation errors
  return false;
};

/**
 * Create a retry schedule based on configuration.
 */
export const createRetrySchedule = (
  config: Partial<EffectRetryConfig> = {},
): Schedule.Schedule<number, RpcEffectError> => {
  const {
    maxRetries,
    baseDelay,
    maxDelay: _maxDelay,
    retryableCodes,
    jitter,
    backoff,
  } = {
    ...defaultEffectRetryConfig,
    ...config,
  };

  // Base schedule with backoff
  const baseSchedule =
    backoff === "exponential"
      ? Schedule.exponential(Duration.millis(baseDelay), 2)
      : Schedule.linear(Duration.millis(baseDelay));

  // Apply jitter if enabled
  const jitteredSchedule = jitter
    ? Schedule.jittered(baseSchedule)
    : baseSchedule;

  // Limit retries and filter by retryable errors
  return pipe(
    jitteredSchedule,
    Schedule.whileInput((error: RpcEffectError) =>
      isRetryableError(error, retryableCodes),
    ),
    Schedule.intersect(Schedule.recurs(maxRetries)),
    Schedule.map(() => maxRetries),
  );
};

/**
 * Execute an effect with retry logic using Effect's Schedule.
 */
export const withRetryEffect = <A, R>(
  effect: Effect.Effect<A, RpcEffectError, R>,
  config: Partial<EffectRetryConfig> = {},
): Effect.Effect<A, RpcEffectError, R> => {
  const schedule = createRetrySchedule(config);
  return Effect.retry(effect, schedule);
};

/**
 * Execute an effect with retry, returning the result and attempt count.
 */
export const withRetryEffectDetailed = <A, R>(
  effect: Effect.Effect<A, RpcEffectError, R>,
  config: Partial<EffectRetryConfig> = {},
): Effect.Effect<{ result: A; attempts: number }, RpcEffectError, R> =>
  Effect.gen(function* () {
    const attemptRef = yield* Ref.make(0);
    const schedule = createRetrySchedule(config);

    const result = yield* pipe(
      effect,
      Effect.tap(() => Ref.update(attemptRef, (n) => n + 1)),
      Effect.tapError(() => Ref.update(attemptRef, (n) => n + 1)),
      Effect.retry(schedule),
    );

    const attempts = yield* Ref.get(attemptRef);
    return { result, attempts };
  });

// =============================================================================
// Deduplication (Effect-based)
// =============================================================================

/**
 * Create a deduplication cache for Effect-based operations.
 * Returns functions to execute with deduplication and clear the cache.
 */
export const createDedupCache = <A>(): Effect.Effect<{
  withDedup: (
    key: string,
    effect: Effect.Effect<A, RpcEffectError>,
  ) => Effect.Effect<A, RpcEffectError>;
  clear: () => Effect.Effect<void>;
  clearKey: (key: string) => Effect.Effect<void>;
  size: () => Effect.Effect<number>;
}> =>
  Effect.gen(function* () {
    const cacheRef = yield* Ref.make(HashMap.empty<string, Promise<A>>());

    const withDedup = (
      key: string,
      effect: Effect.Effect<A, RpcEffectError>,
    ): Effect.Effect<A, RpcEffectError> =>
      Effect.gen(function* () {
        const cache = yield* Ref.get(cacheRef);
        const existing = HashMap.get(cache, key);

        if (Option.isSome(existing)) {
          // Return existing promise result
          return yield* Effect.tryPromise({
            try: () => existing.value,
            catch: (error) =>
              makeCallError(
                "DEDUP_ERROR",
                "Deduplicated request failed",
                error,
              ),
          });
        }

        // Create new promise and cache it
        const promise = Effect.runPromise(effect);
        yield* Ref.update(cacheRef, HashMap.set(key, promise));

        // Execute and clean up on completion
        try {
          const result = yield* Effect.tryPromise({
            try: () => promise,
            catch: (error) =>
              makeCallError("DEDUP_ERROR", "Request failed", error),
          });
          return result;
        } finally {
          yield* Ref.update(cacheRef, HashMap.remove(key));
        }
      });

    const clear = (): Effect.Effect<void> =>
      Ref.set(cacheRef, HashMap.empty<string, Promise<A>>());

    const clearKey = (key: string): Effect.Effect<void> =>
      Ref.update(cacheRef, HashMap.remove(key));

    const size = (): Effect.Effect<number> =>
      Effect.map(Ref.get(cacheRef), HashMap.size);

    return { withDedup, clear, clearKey, size };
  });

/**
 * Simple deduplication key generator with stable object serialization.
 */
export const deduplicationKeyEffect = (
  path: string,
  input: unknown,
): Effect.Effect<string> =>
  Effect.sync(() => `${path}:${stableStringifySync(input)}`);

/**
 * Stable JSON stringify (synchronous helper).
 */
const stableStringifySync = (value: unknown): string => {
  if (value === null || value === undefined) {
    return JSON.stringify(value);
  }

  if (typeof value !== "object") {
    return JSON.stringify(value);
  }

  if (Array.isArray(value)) {
    return "[" + value.map(stableStringifySync).join(",") + "]";
  }

  const obj = value as Record<string, unknown>;
  const keys = Object.keys(obj).sort();

  if (keys.length === 0) {
    return "{}";
  }

  const pairs = keys.map(
    (key) => `${JSON.stringify(key)}:${stableStringifySync(obj[key])}`,
  );
  return "{" + pairs.join(",") + "}";
};

// =============================================================================
// Serialization Utilities (Effect-based)
// =============================================================================

/**
 * JSON.stringify with sorted keys for consistent output (Effect version).
 */
export const stableStringifyEffect = (value: unknown): Effect.Effect<string> =>
  Effect.sync(() => stableStringifySync(value));

// =============================================================================
// Global Deduplication (for backwards compatibility)
// =============================================================================

/** Global pending requests map */
const globalPendingRequests = new Map<string, Promise<unknown>>();

/**
 * Execute an effect with global deduplication.
 * Uses a global cache for simple use cases.
 */
export const withDedupEffect = <A>(
  key: string,
  effect: Effect.Effect<A, RpcEffectError>,
): Effect.Effect<A, RpcEffectError> =>
  Effect.gen(function* () {
    const existing = globalPendingRequests.get(key);
    if (existing) {
      return yield* Effect.tryPromise({
        try: () => existing as Promise<A>,
        catch: (error) =>
          makeCallError("DEDUP_ERROR", "Deduplicated request failed", error),
      });
    }

    const promise = Effect.runPromise(effect);
    globalPendingRequests.set(key, promise);

    try {
      const result = yield* Effect.tryPromise({
        try: () => promise,
        catch: (error) => makeCallError("DEDUP_ERROR", "Request failed", error),
      });
      return result;
    } finally {
      globalPendingRequests.delete(key);
    }
  });

// =============================================================================
// Promise-Based Wrappers (for backwards compatibility)
// =============================================================================

/**
 * Get list of available procedures (Promise wrapper).
 */
export const getProcedures = (): Promise<string[]> =>
  Effect.runPromise(getProceduresEffect());

/**
 * Get current subscription count (Promise wrapper).
 */
export const getSubscriptionCount = (): Promise<number> =>
  Effect.runPromise(getSubscriptionCountEffect());
