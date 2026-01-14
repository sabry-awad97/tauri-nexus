// =============================================================================
// @tauri-nexus/rpc-core - Path Validation
// =============================================================================
// Pure validation functions from rpc-effect.
// Effect-based validation is also available for internal use.

// Pure functions (no Effect dependency at runtime)
export {
  validatePathPure,
  isValidPathPure,
  validatePathOrThrow,
  type PathValidationResult,
  type PathValidationRules,
} from "@tauri-nexus/rpc-effect";

// Effect-based validation (for internal use)
export {
  validatePath as validatePathEffect,
  validatePaths as validatePathsEffect,
  validateAndNormalizePath as validateAndNormalizePathEffect,
  isValidPath as isValidPathEffect,
  validatePathWithRules as validatePathWithRulesEffect,
} from "@tauri-nexus/rpc-effect";
