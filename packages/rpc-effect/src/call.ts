// =============================================================================
// @tauri-nexus/rpc-effect - Effect-Based RPC Call Implementation
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
  type EventIterator,
} from "./types";
import { parseEffectError } from "./errors";
import { validatePath } from "./validation";
import type { RpcServices } from "./runtime";

// =============================================================================
// Interceptor Execution
// =============================================================================

const executeWithInterceptors = <T>(
  ctx: InterceptorContext,
  operation: () => Promise<T>,
): Effect.Effect<T, RpcEffectError, RpcInterceptorService> =>
  Effect.gen(function* () {
    const { interceptors } = yield* RpcInterceptorService;

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

export interface CallOptions {
  readonly signal?: AbortSignal;
  readonly timeout?: number;
  readonly meta?: Record<string, unknown>;
}

/**
 * Make an RPC call using Effect.
 */
export const call = <T>(
  path: string,
  input: unknown,
  options: CallOptions = {},
): Effect.Effect<T, RpcEffectError, RpcServices> =>
  Effect.gen(function* () {
    yield* validatePath(path);

    const config = yield* RpcConfigService;
    const transport = yield* RpcTransportService;
    const logger = yield* RpcLoggerService;

    const timeoutMs = options.timeout ?? config.defaultTimeout;

    const ctx: InterceptorContext = {
      path,
      input,
      type: "query",
      meta: options.meta ?? {},
      signal: options.signal,
    };

    logger.debug(`Calling ${path}`, { input, timeout: timeoutMs });
    const startTime = Date.now();

    const result = yield* executeWithInterceptors<T>(ctx, async () => {
      if (timeoutMs) {
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
  options: Omit<CallOptions, "timeout"> = {},
): Effect.Effect<T, RpcEffectError, RpcServices> =>
  pipe(
    call<T>(path, input, { ...options, timeout: timeoutMs }),
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

export interface SubscribeOptions {
  readonly signal?: AbortSignal;
  readonly lastEventId?: string;
  readonly meta?: Record<string, unknown>;
}

/**
 * Subscribe to a streaming procedure using Effect.
 */
export const subscribe = <T>(
  path: string,
  input: unknown,
  options: SubscribeOptions = {},
): Effect.Effect<EventIterator<T>, RpcEffectError, RpcServices> =>
  Effect.gen(function* () {
    yield* validatePath(path);

    const transport = yield* RpcTransportService;
    const logger = yield* RpcLoggerService;

    logger.debug(`Subscribing to ${path}`, { input });

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
// Batch Types and Validation
// =============================================================================

export interface BatchRequestItem {
  readonly id: string;
  readonly path: string;
  readonly input: unknown;
}

export interface BatchResultItem<T = unknown> {
  readonly id: string;
  readonly data?: T;
  readonly error?: { code: string; message: string; details?: unknown };
}

export interface BatchRequest {
  readonly requests: readonly BatchRequestItem[];
}

export interface BatchResponse<T = unknown> {
  readonly results: readonly BatchResultItem<T>[];
}

/**
 * Validate batch requests (paths only).
 * Use this before executing a batch via custom transport.
 */
export const validateBatchRequests = (
  requests: readonly BatchRequestItem[],
): Effect.Effect<readonly BatchRequestItem[], RpcEffectError> =>
  Effect.gen(function* () {
    for (const req of requests) {
      yield* validatePath(req.path);
    }
    return requests;
  });

/**
 * Execute a batch of RPC calls using parallel individual calls.
 * Note: For production use with Tauri, use the dedicated batch endpoint
 * via rpc-core's executeBatch which calls plugin:rpc|rpc_call_batch.
 * This function is provided for testing or when no batch endpoint exists.
 */
export const batchCallParallel = <T = unknown>(
  requests: readonly BatchRequestItem[],
  options: CallOptions = {},
): Effect.Effect<readonly BatchResultItem<T>[], RpcEffectError, RpcServices> =>
  Effect.gen(function* () {
    const logger = yield* RpcLoggerService;

    for (const req of requests) {
      yield* validatePath(req.path);
    }

    logger.debug(`Executing batch with ${requests.length} requests (parallel)`);

    const results = yield* Effect.all(
      requests.map((req) =>
        pipe(
          call<T>(req.path, req.input, options),
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
                code: "code" in error ? String(error.code) : "UNKNOWN",
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
