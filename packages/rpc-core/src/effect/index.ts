// =============================================================================
// @tauri-nexus/rpc-core - Public Effect API
// =============================================================================
// Simplified public API for users who want Effect integration without
// dealing with the internal complexity.
//
// Most users should use the standard Promise-based API from the root.
// This module is for advanced users who want Effect's benefits.

export {
  // Effect-based client factory
  createEffectClient,
  type EffectClientConfig,
  type EffectClient,
} from "./client";

export {
  // Effect-based link for advanced composition
  EffectRpcLink,
  type EffectRpcLinkConfig,
} from "./link";

export {
  // Pre-built interceptors
  loggingInterceptor,
  retryInterceptor,
  errorHandlerInterceptor,
  authInterceptor,
  type InterceptorOptions,
  type RetryInterceptorOptions,
  type AuthInterceptorOptions,
} from "./interceptors";

// Re-export error types for pattern matching
export type {
  RpcCallError,
  RpcTimeoutError,
  RpcCancelledError,
  RpcValidationError,
  RpcNetworkError,
  RpcEffectError,
} from "../internal/effect-types";

// Re-export error utilities
export {
  isRpcCallError,
  isRpcTimeoutError,
  isRpcCancelledError,
  isRpcValidationError,
  isRpcNetworkError,
  toPublicError,
} from "../internal/effect-errors";
