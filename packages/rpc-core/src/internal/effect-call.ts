// =============================================================================
// @tauri-nexus/rpc-core - Effect-Based RPC Call Implementation
// =============================================================================
// Core RPC call logic using Effect for type-safe error handling and composition.

import { Effect, pipe, Duration } from "effect";
import {
  RpcConfigService,
  RpcTransportService,
  RpcInterceptorService,
  RpcLoggerService,
  type RpcEffectError,
  type InterceptorContext,
} from "./effect-types";
import {
  parseEffectError,
  makeValidationError,
} from "./effect-errors";
import type { RpcServices } from "./effect-runtime";

// =============================================================================
// Path Validation
// =============================================================================

const PATH_REGEX = /^[a-zA-Z0-9_.]+$/;

/**
 * Validate a procedure path.
 */
export const validatePath = (
  path: string,
): Effect.Effect<string, RpcEffectError> => {
  if (!path) {
    return Effect.fail(
      makeValidationError(path, [
        { path: [], message: "Procedure path cannot be empty", code: "empty" },
      ]),
    );
  }

  if (path.startsWith(".") || path.endsWith(".")) {
    return Effect.fail(
      makeValidationError(path, [
        {
          path: [],
          message: "Procedure path cannot start or end with a dot",
          code: "invalid_format",
        },
      ]),
    );
  }

  if (path.includes("..")) {
    return Effect.fail(
      makeValidationError(path, [
        {
          path: [],
          message: "Procedure path cannot contain consecutive dots",
          code: "invalid_format",
        },
      ]),
    );
  }

  if (!PATH_REGEX.test(path)) {
    return Effect.fail(
      makeValidationError(path, [
        {
          path: [],
          message: "Procedure path contains invalid characters",
          code: "invalid_chars",
        },
      ]),
    );
  }

  return Effect.succeed(path);
};

// =============================================================================
// Interceptor Execution
// =============================================================================

/**
 * Execute the interceptor chain around a core operation.
 */
const executeWithInterceptors = <T>(
  ctx: InterceptorContext,
  operation: () => Promise<T>,
): Effect.Effect<T, RpcEffectError, RpcInterceptorService> =>
  Effect.gen(function* () {
    const { interceptors } = yield* RpcInterceptorService;

    // Build the interceptor chain from inside out
    let next = operation;
    for (let i = interceptors.length - 1; i >= 0; i--) {
      const interceptor = interceptors[i];
      const currentNext = next;
      next = () => interceptor.intercept(ctx, currentNext);
    }

    return yield* Effect.tryPromise({
      try: () => next(),
      catch: (error) => parseEffectError(error, ctx.path),
    });
  });

// =============================================================================
// Core Call Effect
// =============================================================================

/**
 * Options for the call effect.
 */
export interface CallEffectOptions {
  readonly signal?: AbortSignal;
  readonly timeout?: number;
  readonly meta?: Record<string, unknown>;
}

/**
 * Make an RPC call using Effect.
 * This is the core implementation that handles all the complexity.
 */
export const callEffect = <T>(
  path: string,
  input: unknown,
  options: CallEffectOptions = {},
): Effect.Effect<T, RpcEffectError, RpcServices> =>
  Effect.gen(function* () {
    // Validate path
    yield* validatePath(path);

    // Get services
    const config = yield* RpcConfigService;
    const transport = yield* RpcTransportService;
    const logger = yield* RpcLoggerService;

    const timeoutMs = options.timeout ?? config.defaultTimeout;

    // Build interceptor context
    const ctx: InterceptorContext = {
      path,
      input,
      type: "query",
      meta: options.meta ?? {},
      signal: options.signal,
    };

    logger.debug(`Calling ${path}`, { input, timeout: timeoutMs });
    const startTime = Date.now();

    // Execute with interceptors
    const result = yield* executeWithInterceptors<T>(ctx, async () => {
      if (timeoutMs) {
        // Create timeout race
        const controller = new AbortController();
        const timeoutId = setTimeout(() => controller.abort(), timeoutMs);

        try {
          const result = await transport.call<T>(path, input);
          clearTimeout(timeoutId);
          return result;
        } catch (error) {
          clearTimeout(timeoutId);
          throw error;
        }
      }

      return transport.call<T>(path, input);
    });

    const durationMs = Date.now() - startTime;
    logger.debug(`Completed ${path} in ${durationMs}ms`, { result });

    return result;
  });

/**
 * Make an RPC call with timeout using Effect's built-in timeout.
 */
export const callWithTimeout = <T>(
  path: string,
  input: unknown,
  timeoutMs: number,
  options: Omit<CallEffectOptions, "timeout"> = {},
): Effect.Effect<T, RpcEffectError, RpcServices> =>
  pipe(
    callEffect<T>(path, input, { ...options, timeout: timeoutMs }),
    Effect.timeoutFail({
      duration: Duration.millis(timeoutMs),
      onTimeout: () =>
        parseEffectError(
          new DOMException("Timeout", "AbortError"),
          path,
          timeoutMs,
        ),
    }),
  );

// =============================================================================
// Subscribe Effect
// =============================================================================

/**
 * Options for the subscribe effect.
 */
export interface SubscribeEffectOptions {
  readonly signal?: AbortSignal;
  readonly lastEventId?: string;
  readonly meta?: Record<string, unknown>;
}

/**
 * Subscribe to a streaming procedure using Effect.
 */
export const subscribeEffect = <T>(
  path: string,
  input: unknown,
  options: SubscribeEffectOptions = {},
): Effect.Effect<AsyncIterable<T>, RpcEffectError, RpcServices> =>
  Effect.gen(function* () {
    // Validate path
    yield* validatePath(path);

    // Get services
    const transport = yield* RpcTransportService;
    const logger = yield* RpcLoggerService;

    logger.debug(`Subscribing to ${path}`, { input });

    // Create subscription
    const iterator = yield* Effect.tryPromise({
      try: () =>
        transport.subscribe<T>(path, input, {
          lastEventId: options.lastEventId,
          signal: options.signal,
        }),
      catch: (error) => parseEffectError(error, path),
    });

    return iterator;
  });

// =============================================================================
// Batch Call Effect
// =============================================================================

/**
 * Single request in a batch.
 */
export interface BatchRequestItem {
  readonly id: string;
  readonly path: string;
  readonly input: unknown;
}

/**
 * Result of a single batch request.
 */
export interface BatchResultItem<T = unknown> {
  readonly id: string;
  readonly data?: T;
  readonly error?: { code: string; message: string; details?: unknown };
}

/**
 * Execute a batch of RPC calls.
 */
export const batchCallEffect = <T = unknown>(
  requests: readonly BatchRequestItem[],
  options: CallEffectOptions = {},
): Effect.Effect<readonly BatchResultItem<T>[], RpcEffectError, RpcServices> =>
  Effect.gen(function* () {
    const logger = yield* RpcLoggerService;

    // Validate all paths
    for (const req of requests) {
      yield* validatePath(req.path);
    }

    logger.debug(`Executing batch with ${requests.length} requests`);

    // Execute all calls in parallel
    const results = yield* Effect.all(
      requests.map((req) =>
        pipe(
          callEffect<T>(req.path, req.input, options),
          Effect.map(
            (data): BatchResultItem<T> => ({
              id: req.id,
              data,
            }),
          ),
          Effect.catchAll((error) =>
            Effect.succeed<BatchResultItem<T>>({
              id: req.id,
              error: {
                code:
                  "code" in error ? String(error.code) : "UNKNOWN",
                message:
                  "message" in error ? String(error.message) : "Unknown error",
                details: "details" in error ? error.details : undefined,
              },
            }),
          ),
        ),
      ),
      { concurrency: "unbounded" },
    );

    return results;
  });

// =============================================================================
// Retry Logic
// =============================================================================

/**
 * Retry options.
 */
export interface RetryOptions {
  readonly maxRetries: number;
  readonly delay: number;
  readonly backoff?: "linear" | "exponential";
  readonly retryOn?: (error: RpcEffectError) => boolean;
}

/**
 * Wrap a call effect with retry logic.
 */
export const withRetry = <T>(
  effect: Effect.Effect<T, RpcEffectError, RpcServices>,
  options: RetryOptions,
): Effect.Effect<T, RpcEffectError, RpcServices> => {
  const { maxRetries, delay, backoff = "linear", retryOn } = options;

  const shouldRetry = (error: RpcEffectError, attempt: number): boolean => {
    if (attempt >= maxRetries) return false;
    if (retryOn) return retryOn(error);

    // Default: retry on network/timeout errors, not on validation
    if (error._tag === "RpcValidationError") return false;
    if (error._tag === "RpcCancelledError") return false;
    return true;
  };

  const getDelay = (attempt: number): number => {
    if (backoff === "exponential") {
      return delay * Math.pow(2, attempt);
    }
    return delay * (attempt + 1);
  };

  const loop = (attempt: number): Effect.Effect<T, RpcEffectError, RpcServices> =>
    pipe(
      effect,
      Effect.catchAll((error) => {
        if (shouldRetry(error, attempt)) {
          const retryDelay = getDelay(attempt);
          return pipe(
            Effect.sleep(Duration.millis(retryDelay)),
            Effect.flatMap(() => loop(attempt + 1)),
          );
        }
        return Effect.fail(error);
      }),
    );

  return loop(0);
};
