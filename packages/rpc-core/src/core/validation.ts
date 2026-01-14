// =============================================================================
// @tauri-nexus/rpc-core - Path Validation
// =============================================================================
// Pure re-exports from rpc-effect (single source of truth).

export {
  // Pure validation functions
  validatePathPure,
  isValidPathPure,
  validatePathOrThrow,
  type PathValidationResult,
  // Effect-based validation
  validatePath as validatePathEffect,
  validatePaths as validatePathsEffect,
  validateAndNormalizePath as validateAndNormalizePathEffect,
  isValidPath as isValidPathEffect,
  validatePathWithRules as validatePathWithRulesEffect,
  type PathValidationRules,
} from "@tauri-nexus/rpc-effect";
