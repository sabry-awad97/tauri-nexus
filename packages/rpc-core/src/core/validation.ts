// =============================================================================
// @tauri-nexus/rpc-core - Path Validation
// =============================================================================
// Re-exports validation from rpc-effect.

export {
  validatePath as validatePathEffect,
  validatePaths as validatePathsEffect,
  validateAndNormalizePath as validateAndNormalizePathEffect,
  isValidPath as isValidPathEffect,
  validatePathWithRules as validatePathWithRulesEffect,
  type PathValidationRules,
} from "@tauri-nexus/rpc-effect";
