// =============================================================================
// @tauri-nexus/rpc-core - Path Validation
// =============================================================================
// Input validation utilities matching Rust backend validation.
// Consolidated module - all validation uses Effect internally.

import { createError } from "./errors";
import { validatePathEffect } from "./effect-validation";
import { Effect, pipe } from "effect";

// Re-export Effect-based validation utilities
export {
  validatePathEffect,
  validatePathsEffect,
  validateAndNormalizePathEffect,
  isValidPathEffect,
  validatePathWithRulesEffect,
  validatePathSync,
  isValidPathSync,
  type PathValidationRules,
} from "./effect-validation";

/**
 * Validate procedure path format.
 * Matches the Rust `validate_path` function in plugin.rs.
 *
 * Valid paths: "health", "user.get", "api.v1.users.list"
 * Invalid: "", ".path", "path.", "path..name", "path/name"
 * 
 * For Effect-based validation, use validatePathEffect.
 */
export function validatePath(path: string): void {
  const result = Effect.runSync(
    pipe(
      validatePathEffect(path),
      Effect.either,
    ),
  );
  
  if (result._tag === "Left") {
    const error = result.left;
    if (error._tag === "RpcValidationError") {
      // Use the first issue's message for backwards compatibility
      const message = error.issues[0]?.message ?? "Validation error";
      throw createError("VALIDATION_ERROR", message);
    }
    throw createError("VALIDATION_ERROR", String(error));
  }
}
