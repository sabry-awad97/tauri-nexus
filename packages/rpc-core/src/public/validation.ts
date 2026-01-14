// =============================================================================
// @tauri-nexus/rpc-core - Path Validation (Public API)
// =============================================================================
// Simple validation API using pure functions from rpc-effect.

import {
  validatePathPure,
  isValidPathPure,
  validatePathOrThrow,
  createRpcError,
} from "@tauri-nexus/rpc-effect";

/**
 * Validate procedure path format.
 * Throws RpcError if validation fails.
 */
export function validatePath(path: string): void {
  const result = validatePathPure(path);
  if (!result.valid) {
    const message = result.issues.map((i) => i.message).join("; ");
    throw createRpcError("VALIDATION_ERROR", message);
  }
}

/**
 * Check if a path is valid without throwing.
 */
export const isValidPath = isValidPathPure;

/**
 * Validate and return the path, throwing if invalid.
 * Uses the pure function from rpc-effect.
 */
export const validateAndReturnPath = validatePathOrThrow;
