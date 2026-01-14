// =============================================================================
// @tauri-nexus/rpc-core - Call Functions (Public Promise API)
// =============================================================================
// Promise-based wrappers for RPC call and subscribe functions.

import { Effect, pipe, Layer } from "effect";
import { invoke } from "@tauri-apps/api/core";
import type {
  CallOptions,
  SubscriptionOptions,
  RequestContext,
  BatchResponse,
  SingleRequest,
  BatchCallOptions,
  BatchRequest,
  EventIterator,
} from "../core/types";
import { getConfig } from "../client/config";
import {
  callEffect,
  subscribeEffect,
  validatePath as validatePathEffect,
  toPublicError,
  parseEffectError,
  makeConfigLayer,
  makeInterceptorLayer,
  makeLoggerLayer,
  TauriTransportLayer,
  type RpcServices,
  type RpcEffectError,
  type RpcInterceptor,
} from "../internal";

// =============================================================================
// Layer Construction
// =============================================================================

const buildLayerFromConfig = (): Layer.Layer<RpcServices> => {
  const config = getConfig();

  const interceptors: RpcInterceptor[] = (config.middleware ?? []).map(
    (mw, index) => ({
      name: `middleware-${index}`,
      intercept: async <T>(
        ctx: {
          path: string;
          input: unknown;
          type: string;
          meta: Record<string, unknown>;
        },
        next: () => Promise<T>
      ) => {
        const requestCtx: RequestContext = {
          path: ctx.path,
          input: ctx.input,
          type: ctx.type as "query" | "mutation" | "subscription",
          meta: ctx.meta,
        };
        return mw(requestCtx, next);
      },
    })
  );

  return Layer.mergeAll(
    makeConfigLayer({
      defaultTimeout: config.timeout,
      subscriptionPaths: new Set(config.subscriptionPaths ?? []),
    }),
    TauriTransportLayer,
    makeInterceptorLayer({ interceptors }),
    makeLoggerLayer()
  );
};

// =============================================================================
// Effect Runners
// =============================================================================

const runWithConfig = <T>(
  effect: Effect.Effect<T, RpcEffectError, RpcServices>
): Effect.Effect<T, RpcEffectError> =>
  pipe(effect, Effect.provide(buildLayerFromConfig()));

const runToPromise = async <T>(
  effect: Effect.Effect<T, RpcEffectError>,
  path: string,
  timeoutMs?: number
): Promise<T> => {
  try {
    return await Effect.runPromise(effect);
  } catch (error) {
    throw toPublicError(parseEffectError(error, path, timeoutMs));
  }
};

// =============================================================================
// Call Implementation
// =============================================================================

const callWithLifecycle = <T>(
  path: string,
  input: unknown,
  options?: CallOptions
) =>
  Effect.gen(function* () {
    const config = getConfig();
    const ctx: RequestContext = {
      path,
      input,
      type: "query",
      meta: options?.meta,
      signal: options?.signal,
    };

    if (config.onRequest) {
      yield* Effect.sync(() => config.onRequest!(ctx));
    }

    const result = yield* pipe(
      callEffect<T>(path, input, {
        signal: options?.signal,
        timeout: options?.timeout ?? config.timeout,
        meta: options?.meta,
      }),
      Effect.tapError((error) =>
        Effect.sync(() => {
          config.onError?.(ctx, toPublicError(error));
        })
      )
    );

    if (config.onResponse) {
      yield* Effect.sync(() => config.onResponse!(ctx, result));
    }

    return result;
  });

/**
 * Make an RPC call (query or mutation).
 */
export async function call<T>(
  path: string,
  input: unknown = null,
  options?: CallOptions
): Promise<T> {
  const effect = pipe(
    callWithLifecycle<T>(path, input, options),
    runWithConfig
  );
  return runToPromise(effect, path, options?.timeout);
}

// =============================================================================
// Subscribe Implementation
// =============================================================================

const subscribeWithLifecycle = <T>(
  path: string,
  input: unknown,
  options?: SubscriptionOptions
) =>
  Effect.gen(function* () {
    const config = getConfig();
    const ctx: RequestContext = {
      path,
      input,
      type: "subscription",
      meta: options?.meta,
      signal: options?.signal,
    };

    if (config.onRequest) {
      yield* Effect.sync(() => config.onRequest!(ctx));
    }

    const iterator = yield* pipe(
      subscribeEffect<T>(path, input, {
        signal: options?.signal,
        lastEventId: options?.lastEventId,
        meta: options?.meta,
      }),
      Effect.tapError((error) =>
        Effect.sync(() => {
          config.onError?.(ctx, toPublicError(error));
        })
      )
    );

    return iterator;
  });

/**
 * Subscribe to a streaming procedure.
 */
export async function subscribe<T>(
  path: string,
  input: unknown = null,
  options?: SubscriptionOptions
): Promise<EventIterator<T>> {
  const effect = pipe(
    subscribeWithLifecycle<T>(path, input, options),
    runWithConfig
  );
  return runToPromise(effect, path) as Promise<EventIterator<T>>;
}

// =============================================================================
// Batch Implementation
// =============================================================================

const executeBatchEffect = <T = unknown>(
  requests: SingleRequest[],
  options?: BatchCallOptions
) =>
  Effect.gen(function* () {
    for (const req of requests) {
      yield* validatePathEffect(req.path);
    }

    const normalizedRequests = requests.map((req) => ({
      ...req,
      input: req.input === undefined ? null : req.input,
    }));

    const batchRequest: BatchRequest = { requests: normalizedRequests };
    const timeoutMs = options?.timeout;

    const executeInvoke = Effect.tryPromise({
      try: () =>
        invoke<BatchResponse<T>>("plugin:rpc|rpc_call_batch", {
          batch: batchRequest,
        }),
      catch: (error) => parseEffectError(error, "batch", timeoutMs),
    });

    if (timeoutMs) {
      return yield* pipe(
        Effect.acquireUseRelease(
          Effect.sync(() => {
            const controller = new AbortController();
            const timeoutId = setTimeout(() => controller.abort(), timeoutMs);
            return timeoutId;
          }),
          () => executeInvoke,
          (timeoutId) => Effect.sync(() => clearTimeout(timeoutId))
        )
      );
    }

    return yield* executeInvoke;
  });

/**
 * Execute batch requests.
 */
export async function executeBatch<T = unknown>(
  requests: SingleRequest[],
  options?: BatchCallOptions
): Promise<BatchResponse<T>> {
  try {
    return await Effect.runPromise(executeBatchEffect<T>(requests, options));
  } catch (error) {
    const rpcError = toPublicError(
      parseEffectError(error, "batch", options?.timeout)
    );
    console.warn(
      `[RPC] Batch request failed: ${rpcError.code} - ${rpcError.message}`,
      rpcError.details
    );
    throw rpcError;
  }
}
