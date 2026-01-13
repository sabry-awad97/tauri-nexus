// =============================================================================
// @tauri-nexus/rpc-core - Core Call Functions
// =============================================================================
// Low-level RPC call and subscribe functions.

import { invoke } from "@tauri-apps/api/core";
import type {
  CallOptions,
  SubscriptionOptions,
  RequestContext,
  BatchRequest,
  BatchResponse,
  SingleRequest,
  BatchCallOptions,
} from "../core/types";
import { validatePath } from "../core/validation";
import { parseError, isRpcError, createError } from "../core/errors";
import { getConfig } from "./config";
import { createEventIterator } from "../subscription/event-iterator";

// =============================================================================
// Middleware Execution
// =============================================================================

/**
 * Execute middleware chain with error wrapping.
 */
async function executeWithMiddleware<T>(
  ctx: RequestContext,
  fn: () => Promise<T>,
): Promise<T> {
  const middleware = getConfig().middleware ?? [];

  let next = fn;
  for (let i = middleware.length - 1; i >= 0; i--) {
    const mw = middleware[i];
    const currentNext = next;
    const middlewareIndex = i;
    next = async () => {
      try {
        return await mw(ctx, currentNext);
      } catch (error) {
        if (!isRpcError(error)) {
          throw createError(
            "MIDDLEWARE_ERROR",
            error instanceof Error ? error.message : String(error),
            {
              middlewareIndex,
              originalError: error instanceof Error ? error.message : error,
            },
          );
        }
        throw error;
      }
    };
  }

  return next();
}

// =============================================================================
// Core Call Functions
// =============================================================================

/**
 * Make an RPC call (query or mutation).
 */
export async function call<T>(
  path: string,
  input: unknown = null,
  options?: CallOptions,
): Promise<T> {
  validatePath(path);

  const config = getConfig();
  const ctx: RequestContext = {
    path,
    input,
    type: "query",
    meta: options?.meta,
    signal: options?.signal,
  };

  config.onRequest?.(ctx);

  const timeoutMs = options?.timeout;

  try {
    const result = await executeWithMiddleware(ctx, async () => {
      if (timeoutMs) {
        const controller = new AbortController();
        const timeoutId = setTimeout(() => controller.abort(), timeoutMs);

        try {
          const result = await invoke<T>("plugin:rpc|rpc_call", {
            path,
            input,
          });
          clearTimeout(timeoutId);
          return result;
        } catch (error) {
          clearTimeout(timeoutId);
          throw error;
        }
      }

      return invoke<T>("plugin:rpc|rpc_call", { path, input });
    });

    config.onResponse?.(ctx, result);
    return result;
  } catch (error) {
    const rpcError = parseError(error, timeoutMs);
    config.onError?.(ctx, rpcError);
    throw rpcError;
  }
}

/**
 * Subscribe to a streaming procedure.
 */
export async function subscribe<T>(
  path: string,
  input: unknown = null,
  options?: SubscriptionOptions,
): Promise<ReturnType<typeof createEventIterator<T>>> {
  validatePath(path);

  const config = getConfig();
  const ctx: RequestContext = {
    path,
    input,
    type: "subscription",
    meta: options?.meta,
    signal: options?.signal,
  };

  config.onRequest?.(ctx);

  try {
    return await createEventIterator<T>(path, input, options);
  } catch (error) {
    const rpcError = parseError(error);
    config.onError?.(ctx, rpcError);
    throw rpcError;
  }
}

// =============================================================================
// Batch Call Functions
// =============================================================================

/**
 * Execute batch requests.
 */
export async function executeBatch<T = unknown>(
  requests: SingleRequest[],
  options?: BatchCallOptions,
): Promise<BatchResponse<T>> {
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
    const rpcError = parseError(error, timeoutMs);
    console.warn(
      `[RPC] Batch request failed: ${rpcError.code} - ${rpcError.message}`,
      rpcError.details,
    );
    throw rpcError;
  }
}
