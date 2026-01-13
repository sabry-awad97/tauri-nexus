// =============================================================================
// @tauri-nexus/rpc-core - Validation (Public Promise API)
// =============================================================================
// Promise-based wrappers for path validation.

import { Effect, pipe } from "effect";
import { validatePathEffect } from "../core/validation";
import { createError } from "../core/errors";

/**
 * Validate procedure path format.
 * Matches the Rust `validate_path` function in plugin.rs.
 *
 * Valid paths: "health", "user.get", "api.v1.users.list"
 * Invalid: "", ".path", "path.", "path..name", "path/name"
 */
export function validatePath(path: string): void {
  const result = Effect.runSync(pipe(validatePathEffect(path), Effect.either));

  if (result._tag === "Left") {
    const error = result.left;
    if (error._tag === "RpcValidationError") {
      const message = error.issues[0]?.message ?? "Validation error";
      throw createError("VALIDATION_ERROR", message);
    }
    throw createError("VALIDATION_ERROR", String(error));
  }
}

/**
 * Check if a path is valid without throwing.
 */
export function isValidPath(path: string): boolean {
  const result = Effect.runSync(pipe(validatePathEffect(path), Effect.either));
  return result._tag === "Right";
}
