// =============================================================================
// Retry Utilities
// =============================================================================

import { Effect, Schedule, Duration, Ref, pipe } from "effect";
import type { RpcEffectError } from "../core/errors";

// =============================================================================
// Configuration
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

// =============================================================================
// Helpers
// =============================================================================

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

// =============================================================================
// Schedule Creation
// =============================================================================

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

// =============================================================================
// Retry Functions
// =============================================================================

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
