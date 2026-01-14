// =============================================================================
// @tauri-nexus/rpc-core - Error Handling
// =============================================================================
// Direct imports from rpc-effect. No re-exports - use rpc-effect types directly.
// This file provides only rpc-core specific error utilities.

import {
  type RpcEffectError,
  toRpcError,
  parseEffectError,
} from "@tauri-nexus/rpc-effect";

// =============================================================================
// rpc-core Specific Utilities
// =============================================================================

/**
 * Convert Effect error to serializable RpcError.
 * Use when bridging Effect-based code to Promise-based API.
 */
export const fromEffectError = toRpcError;

/**
 * Parse any error to serializable RpcError.
 * Handles Effect errors, JSON strings, Error objects, and unknown values.
 */
export function parseError(
  error: unknown,
  path: string = "",
): import("@tauri-nexus/rpc-effect").RpcError {
  return toRpcError(parseEffectError(error, path));
}

/**
 * Convert Effect error and throw as serializable RpcError.
 * Useful for Promise-based error boundaries.
 */
export function throwAsRpcError(error: RpcEffectError): never {
  throw toRpcError(error);
}
