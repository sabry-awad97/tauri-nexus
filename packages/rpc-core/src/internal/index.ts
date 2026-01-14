// =============================================================================
// @tauri-nexus/rpc-core - Internal Bridge to rpc-effect
// =============================================================================
// Re-exports from rpc-effect with Tauri-specific transport layer.
// NO duplication - everything comes from rpc-effect.

import { Layer } from "effect";
import { invoke } from "@tauri-apps/api/core";
import {
  makeConfigLayer,
  makeTransportLayer,
  makeInterceptorLayer,
  makeLoggerLayer,
  fromTransportError,
  type RpcConfig,
  type RpcTransport,
} from "@tauri-nexus/rpc-effect";
import { createEventIterator } from "../subscription";

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
  parseError: fromTransportError,
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
// Re-exports from rpc-effect (single source of truth)
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
  type ErrorHandlers,
  type VirtualErrorCode,
  type InterceptorHandler,
  type InterceptorOptions,
  type RetryInterceptorOptions,
  type AuthInterceptorOptions,
  // Public Error Types
  type PublicRpcError,
  type RpcErrorCode,
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
  // Error constructors
  makeCallError,
  makeTimeoutError,
  makeCancelledError,
  makeValidationError,
  makeNetworkError,
  // Type guards
  isEffectRpcError,
  isRpcCallError,
  isRpcTimeoutError,
  isRpcCancelledError,
  isRpcValidationError,
  isRpcNetworkError,
  isPublicRpcError,
  // Code utilities
  getErrorCode,
  hasCode,
  hasAnyCode,
  isRetryableError,
  // Pattern matching
  matchError,
  // Effect combinators
  failWithCallError,
  failWithTimeout,
  failWithValidation,
  failWithNetwork,
  failWithCancelled,
  // Error conversion (single source of truth)
  toPublicError,
  fromPublicError,
  // Error parsing
  type RpcErrorShape,
  type ErrorParserOptions,
  isRpcErrorShape,
  parseJsonError,
  makeCallErrorFromShape,
  parseToEffectError,
  fromTransportError,
  parseEffectError,
  parseToPublicError,
  // Rate limit helpers
  isRateLimitError,
  getRateLimitRetryAfter,
  // Validation
  validatePath,
  validatePaths,
  isValidPath,
  validatePathPure,
  isValidPathPure,
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
  defaultParseError,
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
  // Interceptors
  createInterceptorFactory,
  createSimpleInterceptor,
  composeInterceptors,
  loggingInterceptor,
  retryInterceptor,
  errorHandlerInterceptor,
  authInterceptor,
  timingInterceptor,
  dedupeInterceptor,
  // Link
  EffectLink,
  type EffectLinkConfig,
} from "@tauri-nexus/rpc-effect";
