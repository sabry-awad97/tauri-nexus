// =============================================================================
// @tauri-nexus/rpc-effect - Effect-Based Utilities
// =============================================================================
// Retry, deduplication, and timing utilities using Effect.

import { Effect, Schedule, Duration, Ref, HashMap, Option, pipe } from "effect";
import { makeCallError } from "./errors";
import type { RpcEffectError } from "./types";

// =============================================================================
// Timing Utilities
// =============================================================================

export const sleep = (ms: number): Effect.Effect<void> =>
  Effect.sleep(Duration.millis(ms));

export const calculateBackoff = (
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
// Retry Logic
// =============================================================================

export interface RetryConfig {
  readonly maxRetries: number;
  readonly baseDelay: number;
  readonly maxDelay: number;
  readonly retryableCodes: readonly string[];
  readonly jitter: boolean;
  readonly backoff: "linear" | "exponential";
}

export const defaultRetryConfig: RetryConfig = {
  maxRetries: 3,
  baseDelay: 1000,
  maxDelay: 30000,
  retryableCodes: ["INTERNAL_ERROR", "TIMEOUT", "UNAVAILABLE"],
  jitter: true,
  backoff: "exponential",
};

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
  return false;
};

export const createRetrySchedule = (
  config: Partial<RetryConfig> = {},
): Schedule.Schedule<number, RpcEffectError> => {
  const {
    maxRetries,
    baseDelay,
    maxDelay: _maxDelay,
    retryableCodes,
    jitter,
    backoff,
  } = {
    ...defaultRetryConfig,
    ...config,
  };

  const baseSchedule =
    backoff === "exponential"
      ? Schedule.exponential(Duration.millis(baseDelay), 2)
      : Schedule.linear(Duration.millis(baseDelay));

  const jitteredSchedule = jitter
    ? Schedule.jittered(baseSchedule)
    : baseSchedule;

  return pipe(
    jitteredSchedule,
    Schedule.whileInput((error: RpcEffectError) =>
      isRetryableError(error, retryableCodes),
    ),
    Schedule.intersect(Schedule.recurs(maxRetries)),
    Schedule.map(() => maxRetries),
  );
};

export const withRetry = <A, R>(
  effect: Effect.Effect<A, RpcEffectError, R>,
  config: Partial<RetryConfig> = {},
): Effect.Effect<A, RpcEffectError, R> => {
  const schedule = createRetrySchedule(config);
  return Effect.retry(effect, schedule);
};

export const withRetryDetailed = <A, R>(
  effect: Effect.Effect<A, RpcEffectError, R>,
  config: Partial<RetryConfig> = {},
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
// Deduplication
// =============================================================================

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

        const promise = Effect.runPromise(effect);
        yield* Ref.update(cacheRef, HashMap.set(key, promise));

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

export const deduplicationKey = (
  path: string,
  input: unknown,
): Effect.Effect<string> =>
  Effect.sync(() => `${path}:${stableStringify(input)}`);

// =============================================================================
// Serialization Utilities
// =============================================================================

export const stableStringify = (value: unknown): string => {
  if (value === null || value === undefined) {
    return JSON.stringify(value);
  }

  if (typeof value !== "object") {
    return JSON.stringify(value);
  }

  if (Array.isArray(value)) {
    return "[" + value.map(stableStringify).join(",") + "]";
  }

  const obj = value as Record<string, unknown>;
  const keys = Object.keys(obj).sort();

  if (keys.length === 0) {
    return "{}";
  }

  const pairs = keys.map(
    (key) => `${JSON.stringify(key)}:${stableStringify(obj[key])}`,
  );
  return "{" + pairs.join(",") + "}";
};

// =============================================================================
// Global Deduplication
// =============================================================================

const globalPendingRequests = new Map<string, Promise<unknown>>();

export const withDedup = <A>(
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
