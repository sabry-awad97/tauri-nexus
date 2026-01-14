// =============================================================================
// @tauri-nexus/rpc-core - Internal Bridge to rpc-effect
// =============================================================================
// Re-exports from rpc-effect with Tauri-specific transport layer.

import { Layer } from "effect";
import { invoke } from "@tauri-apps/api/core";
import {
  makeConfigLayer,
  makeTransportLayer,
  makeInterceptorLayer,
  makeLoggerLayer,
  matchError,
  type RpcConfig,
  type RpcTransport,
  type RpcEffectError,
  RpcCallError,
  RpcTimeoutError,
  RpcCancelledError,
  RpcValidationError,
} from "@tauri-nexus/rpc-effect";
import { createEventIterator } from "../subscription";
import type { RpcError } from "../core/types";

// =============================================================================
// Tauri Transport
// =============================================================================

const tauriTransport: RpcTransport = {
  call: async <T>(path: string, input: unknown): Promise<T> => {
    return invoke<T>("plugin:rpc|rpc_call", { path, input });
  },
  callBatch: async <T>(
    requests: readonly { id: string; path: string; input: unknown }[]
  ) => {
    const normalizedRequests = requests.map((req) => ({
      ...req,
      input: req.input === undefined ? null : req.input,
    }));
    return invoke<{
      results: readonly {
        id: string;
        data?: T;
        error?: { code: string; message: string; details?: unknown };
      }[];
    }>("plugin:rpc|rpc_call_batch", {
      batch: { requests: normalizedRequests },
    });
  },
  subscribe: async <T>(
    path: string,
    input: unknown,
    options?: { lastEventId?: string; signal?: AbortSignal }
  ) => {
    return createEventIterator<T>(path, input, options);
  },
};

export const TauriTransportLayer = makeTransportLayer(tauriTransport);

// =============================================================================
// Layer Builders
// =============================================================================

export const makeDefaultLayer = (config?: Partial<RpcConfig>) =>
  Layer.mergeAll(
    makeConfigLayer(config),
    TauriTransportLayer,
    makeInterceptorLayer({ interceptors: [] }),
    makeLoggerLayer()
  );

export const makeDebugLayer = (config?: Partial<RpcConfig>) =>
  Layer.mergeAll(
    makeConfigLayer(config),
    TauriTransportLayer,
    makeInterceptorLayer({ interceptors: [] }),
    makeLoggerLayer({
      debug: (msg, data) => console.debug(`[RPC] ${msg}`, data ?? ""),
      info: (msg, data) => console.info(`[RPC] ${msg}`, data ?? ""),
      warn: (msg, data) => console.warn(`[RPC] ${msg}`, data ?? ""),
      error: (msg, data) => console.error(`[RPC] ${msg}`, data ?? ""),
    })
  );

// =============================================================================
// Re-exports from rpc-effect
// =============================================================================

export {
  // Types
  type RpcEffectError,
  type RpcInterceptor,
  type InterceptorContext,
  type RpcConfig,
  type RpcTransport,
  type RpcServices,
  type ValidationIssue,
  // Error classes
  RpcCallError,
  RpcTimeoutError,
  RpcCancelledError,
  RpcValidationError,
  RpcNetworkError,
  // Services
  RpcConfigService,
  RpcTransportService,
  RpcInterceptorService,
  RpcLoggerService,
  // Error utilities
  makeCallError,
  makeTimeoutError,
  makeCancelledError,
  makeValidationError,
  makeNetworkError,
  toEffectError,
  isEffectRpcError,
  isRpcCallError,
  isRpcTimeoutError,
  isRpcCancelledError,
  isRpcValidationError,
  isRpcNetworkError,
  hasCode,
  matchError,
  // Validation
  validatePath,
  validatePaths,
  isValidPath,
  // Runtime
  makeConfigLayer,
  makeTransportLayer,
  makeInterceptorLayer,
  makeLoggerLayer,
  consoleLogger,
  // Call effects
  call as callEffect,
  callWithTimeout,
  subscribe as subscribeEffect,
  batchCall as batchCallEffect,
  type CallOptions as CallEffectOptions,
  type SubscribeOptions as SubscribeEffectOptions,
  // Utils
  withRetry,
  withRetryDetailed,
  createRetrySchedule,
  defaultRetryConfig,
  stableStringify,
  deduplicationKey,
  withDedup,
  sleep,
  calculateBackoff,
  type RetryConfig,
  // Link
  EffectLink,
  type EffectLinkConfig,
} from "@tauri-nexus/rpc-effect";

// Export parseEffectError from local errors module (comprehensive parsing)
export { parseEffectError } from "../core/errors";

// =============================================================================
// Error Conversion to Public Format
// =============================================================================

/**
 * Convert Effect error to public RpcError format.
 */
export const toPublicError = (error: RpcEffectError): RpcError =>
  matchError(error, {
    onCallError: (e) => ({
      code: e.code,
      message: e.message,
      details: e.details,
      cause: e.cause,
    }),
    onTimeoutError: (e) => ({
      code: "TIMEOUT",
      message: `Request to '${e.path}' timed out after ${e.timeoutMs}ms`,
      details: { timeoutMs: e.timeoutMs, path: e.path },
      cause: undefined,
    }),
    onCancelledError: (e) => ({
      code: "CANCELLED",
      message: e.reason ?? `Request to '${e.path}' was cancelled`,
      details: { path: e.path },
      cause: undefined,
    }),
    onValidationError: (e) => ({
      code: "VALIDATION_ERROR",
      message:
        e.issues.length > 0
          ? e.issues[0].message
          : `Validation failed for '${e.path}'`,
      details: { issues: e.issues },
      cause: undefined,
    }),
    onNetworkError: (e) => ({
      code: "INTERNAL_ERROR",
      message: `Network error calling '${e.path}'`,
      details: { originalError: String(e.originalError) },
      cause: undefined,
    }),
  });

/**
 * Convert public RpcError to Effect error.
 */
export const fromPublicError = (
  error: RpcError,
  path: string
): RpcEffectError => {
  switch (error.code) {
    case "TIMEOUT":
      return new RpcTimeoutError({
        path,
        timeoutMs: (error.details as { timeoutMs?: number })?.timeoutMs ?? 0,
      });
    case "CANCELLED":
      return new RpcCancelledError({ path, reason: error.message });
    case "VALIDATION_ERROR":
      return new RpcValidationError({
        path,
        issues:
          (
            error.details as {
              issues?: Array<{
                path: (string | number)[];
                message: string;
                code: string;
              }>;
            }
          )?.issues ?? [],
      });
    default:
      return new RpcCallError({
        code: error.code,
        message: error.message,
        details: error.details,
        cause: error.cause,
      });
  }
};
