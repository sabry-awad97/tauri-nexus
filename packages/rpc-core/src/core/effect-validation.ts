// =============================================================================
// @tauri-nexus/rpc-core - Effect-Based Path Validation
// =============================================================================
// Input validation utilities using Effect for type-safe error handling.

import { Effect, pipe } from "effect";
import type { RpcEffectError, ValidationIssue } from "../internal/effect-types";
import { makeValidationError } from "../internal/effect-errors";

// =============================================================================
// Constants
// =============================================================================

const PATH_REGEX = /^[a-zA-Z0-9_.]+$/;

// =============================================================================
// Effect-Based Validation
// =============================================================================

/**
 * Validate procedure path format using Effect.
 * Returns the validated path on success, or fails with RpcValidationError.
 *
 * Valid paths: "health", "user.get", "api.v1.users.list"
 * Invalid: "", ".path", "path.", "path..name", "path/name"
 */
export const validatePathEffect = (
  path: string,
): Effect.Effect<string, RpcEffectError> =>
  Effect.gen(function* () {
    const issues: ValidationIssue[] = [];

    // Check empty
    if (!path) {
      issues.push({
        path: [],
        message: "Procedure path cannot be empty",
        code: "empty",
      });
    }

    // Check leading/trailing dots
    if (path.startsWith(".") || path.endsWith(".")) {
      issues.push({
        path: [],
        message: "Procedure path cannot start or end with a dot",
        code: "invalid_format",
      });
    }

    // Check consecutive dots
    if (path.includes("..")) {
      issues.push({
        path: [],
        message: "Procedure path cannot contain consecutive dots",
        code: "invalid_format",
      });
    }

    // Check invalid characters
    if (path && !PATH_REGEX.test(path)) {
      const invalidChars = path
        .split("")
        .filter((ch) => !/[a-zA-Z0-9_.]/.test(ch));
      issues.push({
        path: [],
        message: `Procedure path contains invalid characters: '${invalidChars.join("', '")}'`,
        code: "invalid_chars",
      });
    }

    if (issues.length > 0) {
      return yield* Effect.fail(makeValidationError(path, issues));
    }

    return path;
  });

/**
 * Validate multiple paths, collecting all errors.
 */
export const validatePathsEffect = (
  paths: readonly string[],
): Effect.Effect<readonly string[], RpcEffectError> =>
  Effect.gen(function* () {
    const allIssues: Array<{ path: string; issues: ValidationIssue[] }> = [];

    for (const path of paths) {
      const result = yield* pipe(
        validatePathEffect(path),
        Effect.either,
      );

      if (result._tag === "Left") {
        const error = result.left;
        if (error._tag === "RpcValidationError") {
          allIssues.push({ path, issues: [...error.issues] });
        }
      }
    }

    if (allIssues.length > 0) {
      const combinedIssues: ValidationIssue[] = allIssues.flatMap(({ path, issues }) =>
        issues.map((issue) => ({
          ...issue,
          message: `[${path}] ${issue.message}`,
        })),
      );
      return yield* Effect.fail(makeValidationError("batch", combinedIssues));
    }

    return paths;
  });

/**
 * Validate path and transform it (e.g., normalize).
 */
export const validateAndNormalizePathEffect = (
  path: string,
): Effect.Effect<string, RpcEffectError> =>
  pipe(
    validatePathEffect(path),
    Effect.map((validPath) => validPath.toLowerCase()),
  );

/**
 * Check if a path is valid without throwing.
 * Returns true if valid, false otherwise.
 */
export const isValidPathEffect = (path: string): Effect.Effect<boolean> =>
  pipe(
    validatePathEffect(path),
    Effect.map(() => true),
    Effect.catchAll(() => Effect.succeed(false)),
  );

/**
 * Validate path with custom rules.
 */
export interface PathValidationRules {
  readonly allowEmpty?: boolean;
  readonly maxLength?: number;
  readonly minSegments?: number;
  readonly maxSegments?: number;
  readonly allowedPrefixes?: readonly string[];
  readonly disallowedPrefixes?: readonly string[];
}

export const validatePathWithRulesEffect = (
  path: string,
  rules: PathValidationRules = {},
): Effect.Effect<string, RpcEffectError> =>
  Effect.gen(function* () {
    // First run standard validation (unless empty is allowed)
    if (!rules.allowEmpty || path) {
      yield* validatePathEffect(path);
    }

    const issues: ValidationIssue[] = [];

    // Check max length
    if (rules.maxLength !== undefined && path.length > rules.maxLength) {
      issues.push({
        path: [],
        message: `Path exceeds maximum length of ${rules.maxLength}`,
        code: "max_length",
      });
    }

    // Check segment count
    const segments = path.split(".");
    if (rules.minSegments !== undefined && segments.length < rules.minSegments) {
      issues.push({
        path: [],
        message: `Path must have at least ${rules.minSegments} segments`,
        code: "min_segments",
      });
    }
    if (rules.maxSegments !== undefined && segments.length > rules.maxSegments) {
      issues.push({
        path: [],
        message: `Path cannot have more than ${rules.maxSegments} segments`,
        code: "max_segments",
      });
    }

    // Check allowed prefixes
    if (rules.allowedPrefixes && rules.allowedPrefixes.length > 0) {
      const hasAllowedPrefix = rules.allowedPrefixes.some((prefix) =>
        path.startsWith(prefix),
      );
      if (!hasAllowedPrefix) {
        issues.push({
          path: [],
          message: `Path must start with one of: ${rules.allowedPrefixes.join(", ")}`,
          code: "invalid_prefix",
        });
      }
    }

    // Check disallowed prefixes
    if (rules.disallowedPrefixes) {
      const hasDisallowedPrefix = rules.disallowedPrefixes.find((prefix) =>
        path.startsWith(prefix),
      );
      if (hasDisallowedPrefix) {
        issues.push({
          path: [],
          message: `Path cannot start with: ${hasDisallowedPrefix}`,
          code: "disallowed_prefix",
        });
      }
    }

    if (issues.length > 0) {
      return yield* Effect.fail(makeValidationError(path, issues));
    }

    return path;
  });

// =============================================================================
// Synchronous Wrappers (for backwards compatibility)
// =============================================================================

/**
 * Validate path synchronously, throwing on error.
 */
export const validatePathSync = (path: string): string => {
  const result = Effect.runSync(
    pipe(
      validatePathEffect(path),
      Effect.either,
    ),
  );
  
  if (result._tag === "Left") {
    const error = result.left;
    if (error._tag === "RpcValidationError") {
      throw new Error(error.issues.map((i) => i.message).join("; "));
    }
    throw new Error(String(error));
  }
  
  return result.right;
};

/**
 * Check if path is valid synchronously.
 */
export const isValidPathSync = (path: string): boolean =>
  Effect.runSync(isValidPathEffect(path));
