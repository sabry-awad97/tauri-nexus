// =============================================================================
// @tauri-nexus/rpc-core - Core Call Functions
// =============================================================================
// Low-level RPC call and subscribe functions.
// Uses Effect internally for type-safe error handling and composition,
// but exposes a simple Promise-based API.

import { Effect, pipe, Layer } from "effect";
import type {
  CallOptions,
  SubscriptionOptions,
  RequestContext,
  BatchResponse,
  SingleRequest,
  BatchCallOptions,
} from "../core/types";
import { getConfig } from "./config";
import {
  callEffect,
  validatePath as validatePathEffect,
} from "../internal/effect-call";
import { toPublicError, parseEffectError } from "../internal/effect-errors";
import {
  makeConfigLayer,
  makeInterceptorLayer,
  makeLoggerLayer,
  TauriTransportLayer,
  type RpcServices,
} from "../internal/effect-runtime";
import type { RpcEffectError, RpcInterceptor } from "../internal/effect-types";
import { createEventIterator } from "../subscription/event-iterator";

// =============================================================================
// Layer Construction from Config
// =============================================================================

/**
 * Build Effect layer from current global config.
 */
function buildLayerFromConfig(): Layer.Layer<RpcServices> {
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
}

/**
 * Run an Effect with the current config layer.
 */
async function runWithConfig<T>(
  effect: Effect.Effect<T, RpcEffectError, RpcServices>,
): Promise<T> {
  const layer = buildLayerFromConfig();
  const provided = pipe(effect, Effect.provide(layer));
  return Effect.runPromise(provided);
}

// =============================================================================
// Path Validation (Effect-based)
// =============================================================================

/**
 * Validate path using Effect internally.
 */
export function validatePath(path: string): void {
  // Run synchronously for backwards compatibility
  Effect.runSync(
    pipe(
      validatePathEffect(path),
      Effect.mapError((error) => {
        throw toPublicError(error);
      }),
      Effect.catchAll(() => Effect.void),
    ),
  );
}

// =============================================================================
// Core Call Functions
// =============================================================================

/**
 * Make an RPC call (query or mutation).
 * Uses Effect internally for robust error handling.
 */
export async function call<T>(
  path: string,
  input: unknown = null,
  options?: CallOptions,
): Promise<T> {
  const config = getConfig();
  const ctx: RequestContext = {
    path,
    input,
    type: "query",
    meta: options?.meta,
    signal: options?.signal,
  };

  // Lifecycle hook: before request
  config.onRequest?.(ctx);

  try {
    const result = await runWithConfig(
      callEffect<T>(path, input, {
        signal: options?.signal,
        timeout: options?.timeout ?? config.timeout,
        meta: options?.meta,
      }),
    );

    // Lifecycle hook: after response
    config.onResponse?.(ctx, result);
    return result;
  } catch (error) {
    // Convert errors to public format
    let rpcError;
    
    if (isEffectError(error)) {
      // Effect-based RPC error
      rpcError = toPublicError(error);
    } else if (error instanceof Error && "cause" in error && isEffectError(error.cause)) {
      // Effect wraps errors in Error with cause
      rpcError = toPublicError(error.cause as RpcEffectError);
    } else {
      // Use parseEffectError to handle FiberFailure and other Effect error wrappers
      rpcError = toPublicError(parseEffectError(error, path, options?.timeout));
    }

    // Lifecycle hook: on error
    config.onError?.(ctx, rpcError);
    throw rpcError;
  }
}

/**
 * Subscribe to a streaming procedure.
 * Uses Effect internally for setup, returns async iterator.
 */
export async function subscribe<T>(
  path: string,
  input: unknown = null,
  options?: SubscriptionOptions,
): Promise<ReturnType<typeof createEventIterator<T>>> {
  const config = getConfig();
  const ctx: RequestContext = {
    path,
    input,
    type: "subscription",
    meta: options?.meta,
    signal: options?.signal,
  };

  // Lifecycle hook: before request
  config.onRequest?.(ctx);

  try {
    // Use Effect for validation and setup, but call createEventIterator directly
    // since it returns the proper EventIterator type with return() method
    validatePath(path);

    const iterator = await createEventIterator<T>(path, input, options);
    return iterator;
  } catch (error) {
    const rpcError = isEffectError(error)
      ? toPublicError(error)
      : toPublicError(parseEffectError(error, path));

    config.onError?.(ctx, rpcError);
    throw rpcError;
  }
}

// =============================================================================
// Batch Call Functions
// =============================================================================

import { invoke } from "@tauri-apps/api/core";
import type { BatchRequest } from "../core/types";

/**
 * Execute batch requests by calling the batch endpoint.
 * This calls the actual batch endpoint, not individual calls.
 */
export async function executeBatch<T = unknown>(
  requests: SingleRequest[],
  options?: BatchCallOptions,
): Promise<BatchResponse<T>> {
  // Validate all paths first
  for (const req of requests) {
    validatePath(req.path);
  }

  const normalizedRequests = requests.map((req) => ({
    ...req,
    input: req.input === undefined ? null : req.input,
  }));

  const batchRequest: BatchRequest = { requests: normalizedRequests };
  const timeoutMs = options?.timeout;

  try {
    if (timeoutMs) {
      const controller = new AbortController();
      const timeoutId = setTimeout(() => controller.abort(), timeoutMs);

      try {
        const result = await invoke<BatchResponse<T>>(
          "plugin:rpc|rpc_call_batch",
          { batch: batchRequest },
        );
        clearTimeout(timeoutId);
        return result;
      } catch (error) {
        clearTimeout(timeoutId);
        throw error;
      }
    }

    return await invoke<BatchResponse<T>>("plugin:rpc|rpc_call_batch", {
      batch: batchRequest,
    });
  } catch (error) {
    const rpcError = isEffectError(error)
      ? toPublicError(error)
      : toPublicError(parseEffectError(error, "batch", timeoutMs));

    console.warn(
      `[RPC] Batch request failed: ${rpcError.code} - ${rpcError.message}`,
      rpcError.details,
    );
    throw rpcError;
  }
}

// =============================================================================
// Helpers
// =============================================================================

/**
 * Check if error is an Effect-based RPC error.
 */
function isEffectError(error: unknown): error is RpcEffectError {
  return (
    typeof error === "object" &&
    error !== null &&
    "_tag" in error &&
    typeof (error as { _tag: string })._tag === "string"
  );
}
