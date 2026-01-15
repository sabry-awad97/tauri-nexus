// =============================================================================
// @tauri-nexus/rpc-core - Internal Bridge to rpc-effect
// =============================================================================
// Re-exports from rpc-effect with Tauri-specific transport layer.
// NO duplication - everything comes from rpc-effect.

import { Layer } from "effect";
import { invoke } from "@tauri-apps/api/core";
import {
  RpcConfigService,
  RpcTransportService,
  RpcInterceptorService,
  RpcLoggerService,
  consoleLogger,
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
    requests: readonly { id: string; path: string; input: unknown }[],
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
    options?: { lastEventId?: string; signal?: AbortSignal },
  ) => {
    return createEventIterator<T>(path, input, options);
  },
  parseError: fromTransportError,
};

export const TauriTransportLayer = RpcTransportService.layer(tauriTransport);

// =============================================================================
// Layer Builders
// =============================================================================

export const makeDefaultLayer = (config?: Partial<RpcConfig>) =>
  Layer.mergeAll(
    RpcConfigService.layer(config),
    TauriTransportLayer,
    RpcInterceptorService.Default,
    RpcLoggerService.Default,
  );

export const makeDebugLayer = (config?: Partial<RpcConfig>) =>
  Layer.mergeAll(
    RpcConfigService.layer(config),
    TauriTransportLayer,
    RpcInterceptorService.Default,
    RpcLoggerService.layer(consoleLogger),
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
  type RpcError,
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
  // Error constructors (using new naming)
  createCallError,
  createTimeoutError,
  createCancelledError,
  createValidationError,
  createNetworkError,
  // Type guards
  isEffectRpcError,
  isRpcCallError,
  isRpcTimeoutError,
  isRpcCancelledError,
  isRpcValidationError,
  isRpcNetworkError,
  isRpcError,
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
  toRpcError,
  fromRpcError,
  // Error parsing
  type RpcErrorShape,
  type ErrorParserOptions,
  isRpcErrorShape,
  parseJsonError,
  createCallErrorFromShape,
  parseToEffectError,
  fromTransportError,
  parseEffectError,
  parseError,
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
  createRpcLayer,
  createDebugLayer,
  consoleLogger,
  // Call effects
  call as callEffect,
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
