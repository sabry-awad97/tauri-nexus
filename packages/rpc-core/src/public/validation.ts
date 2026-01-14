// =============================================================================
// @tauri-nexus/rpc-core - Path Validation (Public API)
// =============================================================================
// Simple path validation without Effect dependencies.

import { validatePathPure, isValidPathPure } from "@tauri-nexus/rpc-effect";
import { createError } from "../core/errors";

// =============================================================================
// Public Validation Functions
// =============================================================================

/**
 * Validate procedure path format.
 * Throws RpcError if validation fails.
 *
 * Valid paths: "health", "user.get", "api.v1.users.list"
 * Invalid: "", ".path", "path.", "path..name", "path/name"
 */
export function validatePath(path: string): void {
  const result = validatePathPure(path);

  if (!result.valid) {
    const message = result.issues.map((i) => i.message).join("; ");
    throw createError("VALIDATION_ERROR", message);
  }
}

/**
 * Check if a path is valid without throwing.
 */
export function isValidPath(path: string): boolean {
  return isValidPathPure(path);
}

/**
 * Validate and return the path, throwing if invalid.
 */
export function validateAndReturnPath(path: string): string {
  validatePath(path);
  return path;
}
