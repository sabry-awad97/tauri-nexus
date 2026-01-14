// =============================================================================
// @tauri-nexus/rpc-core - Path Validation
// =============================================================================
// Re-exports validation from rpc-effect.
// Provides both pure functions and Effect-based validation.

// Pure validation functions (no Effect dependency)
export {
  validatePathPure,
  isValidPathPure,
  validatePathOrThrow,
  type PathValidationResult,
} from "@tauri-nexus/rpc-effect";

// Effect-based validation (for internal use)
export {
  validatePath as validatePathEffect,
  validatePaths as validatePathsEffect,
  validateAndNormalizePath as validateAndNormalizePathEffect,
  isValidPath as isValidPathEffect,
  validatePathWithRules as validatePathWithRulesEffect,
  type PathValidationRules,
} from "@tauri-nexus/rpc-effect";
