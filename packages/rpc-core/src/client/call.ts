// =============================================================================
// @tauri-nexus/rpc-core - Core Call Functions
// =============================================================================
// Low-level RPC call and subscribe functions.
// Uses Effect throughout for type-safe error handling and composition,
// exposing a simple Promise-based API.

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
} from "../core/types";
import { getConfig } from "./config";
import {
  callEffect,
  subscribeEffect,
  validatePath as validatePathEffect,
} from "../internal/effect-call";
import {
  toPublicError,
  parseEffectError,
} from "../internal/effect-errors";
import {
  makeConfigLayer,
  makeInterceptorLayer,
  makeLoggerLayer,
  TauriTransportLayer,
  type RpcServices,
} from "../internal/effect-runtime";
import type { RpcEffectError, RpcInterceptor } from "../internal/effect-types";

// =============================================================================
// Layer Construction from Config
// =============================================================================

/**
 * Build Effect layer from current global config.
 */
const buildLayerFromConfig = (): Layer.Layer<RpcServices> => {
  const config = getConfig();

  // Convert middleware to interceptors
  const interceptors: RpcInterceptor[] = (config.middleware ?? []).map(
    (mw, index) => ({
      name: `middleware-${index}`,
      intercept: async <T>(
        ctx: { path: string; input: unknown; type: string; meta: Record<string, unknown> },
        next: () => Promise<T>,
      ) => {
        const requestCtx: RequestContext = {
          path: ctx.path,
          input: ctx.input,
          type: ctx.type as "query" | "mutation" | "subscription",
          meta: ctx.meta,
        };
        return mw(requestCtx, next);
      },
    }),
  );

  return Layer.mergeAll(
    makeConfigLayer({
      defaultTimeout: config.timeout,
      subscriptionPaths: new Set(config.subscriptionPaths ?? []),
    }),
    TauriTransportLayer,
    makeInterceptorLayer({ interceptors }),
    makeLoggerLayer(),
  );
};

// =============================================================================
// Effect Runners
// =============================================================================

/**
 * Run an Effect with the current config layer and convert errors.
 */
const runWithConfig = <T>(
  effect: Effect.Effect<T, RpcEffectError, RpcServices>,
): Effect.Effect<T, RpcEffectError> =>
  pipe(effect, Effect.provide(buildLayerFromConfig()));

/**
 * Run Effect and convert to Promise with proper error handling.
 */
const runToPromise = async <T>(
  effect: Effect.Effect<T, RpcEffectError>,
  path: string,
  timeoutMs?: number,
): Promise<T> => {
  try {
    return await Effect.runPromise(effect);
  } catch (error) {
    // Convert any error to public format
    throw toPublicError(parseEffectError(error, path, timeoutMs));
  }
};

// =============================================================================
// Path Validation Effect
// =============================================================================

/**
 * Validate path using Effect.
 */
export const validatePathSync = (path: string): void => {
  const result = Effect.runSync(
    pipe(
      validatePathEffect(path),
      Effect.either,
    ),
  );
  
  if (result._tag === "Left") {
    throw toPublicError(result.left);
  }
};

// Backwards compatibility alias
export const validatePath = validatePathSync;

// =============================================================================
// Core Call Effect
// =============================================================================

/**
 * Effect for making an RPC call with lifecycle hooks.
 */
const callWithLifecycle = <T>(
  path: string,
  input: unknown,
  options?: CallOptions,
): Effect.Effect<T, RpcEffectError, RpcServices> =>
  Effect.gen(function* () {
    const config = getConfig();
    const ctx: RequestContext = {
      path,
      input,
      type: "query",
      meta: options?.meta,
      signal: options?.signal,
    };

    // Lifecycle hook: before request
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
        }),
      ),
    );

    // Lifecycle hook: after response
    if (config.onResponse) {
      yield* Effect.sync(() => config.onResponse!(ctx, result));
    }

    return result;
  });

/**
 * Make an RPC call (query or mutation).
 * Uses Effect throughout for robust error handling.
 */
export async function call<T>(
  path: string,
  input: unknown = null,
  options?: CallOptions,
): Promise<T> {
  const effect = pipe(
    callWithLifecycle<T>(path, input, options),
    runWithConfig,
  );
  return runToPromise(effect, path, options?.timeout);
}

// =============================================================================
// Subscribe Effect
// =============================================================================

/**
 * Effect for subscribing with lifecycle hooks.
 */
const subscribeWithLifecycle = <T>(
  path: string,
  input: unknown,
  options?: SubscriptionOptions,
): Effect.Effect<AsyncIterable<T>, RpcEffectError, RpcServices> =>
  Effect.gen(function* () {
    const config = getConfig();
    const ctx: RequestContext = {
      path,
      input,
      type: "subscription",
      meta: options?.meta,
      signal: options?.signal,
    };

    // Lifecycle hook: before request
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
        }),
      ),
    );

    return iterator;
  });

/**
 * Subscribe to a streaming procedure.
 * Uses Effect throughout for setup, returns async iterator.
 */
export async function subscribe<T>(
  path: string,
  input: unknown = null,
  options?: SubscriptionOptions,
): Promise<AsyncIterable<T>> {
  const effect = pipe(
    subscribeWithLifecycle<T>(path, input, options),
    runWithConfig,
  );
  return runToPromise(effect, path);
}

// =============================================================================
// Batch Call Effect
// =============================================================================

/**
 * Effect for executing batch requests.
 */
const executeBatchEffect = <T = unknown>(
  requests: SingleRequest[],
  options?: BatchCallOptions,
): Effect.Effect<BatchResponse<T>, RpcEffectError> =>
  Effect.gen(function* () {
    // Validate all paths
    for (const req of requests) {
      yield* validatePathEffect(req.path);
    }

    const normalizedRequests = requests.map((req) => ({
      ...req,
      input: req.input === undefined ? null : req.input,
    }));

    const batchRequest: BatchRequest = { requests: normalizedRequests };
    const timeoutMs = options?.timeout;

    // Execute with optional timeout
    const executeInvoke = Effect.tryPromise({
      try: () =>
        invoke<BatchResponse<T>>("plugin:rpc|rpc_call_batch", {
          batch: batchRequest,
        }),
      catch: (error) => parseEffectError(error, "batch", timeoutMs),
    });

    if (timeoutMs) {
      const controller = new AbortController();

      return yield* pipe(
        Effect.acquireUseRelease(
          // Acquire: set up timeout
          Effect.sync(() => {
            const timeoutId = setTimeout(() => controller.abort(), timeoutMs);
            return timeoutId;
          }),
          // Use: execute the invoke
          () => executeInvoke,
          // Release: clear timeout
          (timeoutId) => Effect.sync(() => clearTimeout(timeoutId)),
        ),
      );
    }

    return yield* executeInvoke;
  });

/**
 * Execute batch requests by calling the batch endpoint.
 * Uses Effect throughout for consistent error handling.
 */
export async function executeBatch<T = unknown>(
  requests: SingleRequest[],
  options?: BatchCallOptions,
): Promise<BatchResponse<T>> {
  try {
    return await Effect.runPromise(executeBatchEffect<T>(requests, options));
  } catch (error) {
    const rpcError = toPublicError(parseEffectError(error, "batch", options?.timeout));
    console.warn(
      `[RPC] Batch request failed: ${rpcError.code} - ${rpcError.message}`,
      rpcError.details,
    );
    throw rpcError;
  }
}
