// =============================================================================
// @tauri-nexus/rpc-core - Path Validation
// =============================================================================
// Input validation utilities matching Rust backend validation.

import { createError } from "./errors";

/**
 * Validate procedure path format.
 * Matches the Rust `validate_path` function in plugin.rs.
 *
 * Valid paths: "health", "user.get", "api.v1.users.list"
 * Invalid: "", ".path", "path.", "path..name", "path/name"
 */
export function validatePath(path: string): void {
  if (!path) {
    throw createError("VALIDATION_ERROR", "Procedure path cannot be empty");
  }
  if (path.startsWith(".") || path.endsWith(".")) {
    throw createError(
      "VALIDATION_ERROR",
      "Procedure path cannot start or end with a dot",
    );
  }
  if (path.includes("..")) {
    throw createError(
      "VALIDATION_ERROR",
      "Procedure path cannot contain consecutive dots",
    );
  }
  for (const ch of path) {
    if (!/[a-zA-Z0-9_.]/.test(ch)) {
      throw createError(
        "VALIDATION_ERROR",
        `Procedure path contains invalid character: '${ch}'`,
      );
    }
  }
}
