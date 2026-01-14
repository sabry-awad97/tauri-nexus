// =============================================================================
// RPC Call Operations
// =============================================================================

import { Effect, pipe, Duration } from "effect";
import type { RpcEffectError } from "../core/errors";
import type { InterceptorContext, EventIterator } from "../core/types";
import {
  createCallError,
  createTimeoutError,
  createCancelledError,
  isEffectRpcError,
} from "../core/error-utils";
import {
  RpcConfigService,
  RpcTransportService,
  RpcInterceptorService,
  RpcLoggerService,
  type RpcServices,
} from "../services";
import { validatePath } from "../validation";

// =============================================================================
// Default Error Converter
// =============================================================================

/**
 * Minimal error converter for when transport doesn't provide one.
 */
export const defaultParseError = (
  error: unknown,
  path: string,
  timeoutMs?: number,
): RpcEffectError => {
  if (isEffectRpcError(error)) return error;

  if (error instanceof Error && error.name === "AbortError") {
    return timeoutMs !== undefined
      ? createTimeoutError(path, timeoutMs)
      : createCancelledError(path);
  }

  if (error instanceof Error) {
    return createCallError("UNKNOWN", error.message, undefined, error.stack);
  }

  if (typeof error === "string") {
    return createCallError("UNKNOWN", error);
  }

  return createCallError("UNKNOWN", String(error));
};

const getParseError = (transport: { parseError?: typeof defaultParseError }) =>
  transport.parseError ?? defaultParseError;

// =============================================================================
// Interceptor Execution
// =============================================================================

const executeWithInterceptors = <T>(
  ctx: InterceptorContext,
  operation: () => Promise<T>,
  parseError: (
    error: unknown,
    path: string,
    timeoutMs?: number,
  ) => RpcEffectError,
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
      catch: (error) => parseError(error, ctx.path),
    });
  });

// =============================================================================
// Call Options
// =============================================================================

export interface CallOptions {
  readonly signal?: AbortSignal;
  readonly timeout?: number;
  readonly meta?: Record<string, unknown>;
}

// =============================================================================
// Call Effect
// =============================================================================

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

    const result = yield* executeWithInterceptors<T>(
      ctx,
      async () => {
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
      },
      getParseError(transport),
    );

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
      onTimeout: () => createTimeoutError(path, timeoutMs),
    }),
  );

// =============================================================================
// Subscribe Options
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
      catch: (error) => getParseError(transport)(error, path),
    });

    return iterator;
  });
