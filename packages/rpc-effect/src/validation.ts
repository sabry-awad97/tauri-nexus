// =============================================================================
// @tauri-nexus/rpc-effect - Path Validation
// =============================================================================
// Effect-based path validation utilities.

import { Effect, pipe } from "effect";
import type { RpcEffectError, ValidationIssue } from "./types";
import { makeValidationError } from "./errors";

// =============================================================================
// Constants
// =============================================================================

const PATH_REGEX = /^[a-zA-Z0-9_.]+$/;

// =============================================================================
// Types
// =============================================================================

export interface PathValidationResult {
  readonly valid: boolean;
  readonly issues: readonly ValidationIssue[];
}

export interface PathValidationRules {
  readonly allowEmpty?: boolean;
  readonly maxLength?: number;
  readonly minSegments?: number;
  readonly maxSegments?: number;
  readonly allowedPrefixes?: readonly string[];
  readonly disallowedPrefixes?: readonly string[];
}

// =============================================================================
// Pure Functions
// =============================================================================

export const validatePathPure = (path: string): PathValidationResult => {
  const issues: ValidationIssue[] = [];

  if (!path) {
    issues.push({
      path: [],
      message: "Procedure path cannot be empty",
      code: "empty",
    });
  }

  if (path.startsWith(".") || path.endsWith(".")) {
    issues.push({
      path: [],
      message: "Procedure path cannot start or end with a dot",
      code: "invalid_format",
    });
  }

  if (path.includes("..")) {
    issues.push({
      path: [],
      message: "Procedure path cannot contain consecutive dots",
      code: "invalid_format",
    });
  }

  if (path && !PATH_REGEX.test(path)) {
    const invalidChars = path
      .split("")
      .filter((ch) => !/[a-zA-Z0-9_.]/.test(ch));
    issues.push({
      path: [],
      message: `Procedure path contains invalid characters: '${invalidChars.join(
        "', '"
      )}'`,
      code: "invalid_chars",
    });
  }

  return { valid: issues.length === 0, issues };
};

export const isValidPathPure = (path: string): boolean =>
  validatePathPure(path).valid;

export const validatePathOrThrow = (path: string): string => {
  const result = validatePathPure(path);
  if (!result.valid) {
    const message = result.issues.map((i) => i.message).join("; ");
    throw new Error(`Invalid path '${path}': ${message}`);
  }
  return path;
};

// =============================================================================
// Effect-Based Validation
// =============================================================================

export const validatePath = (
  path: string
): Effect.Effect<string, RpcEffectError> =>
  Effect.gen(function* () {
    const result = validatePathPure(path);
    if (!result.valid) {
      return yield* Effect.fail(makeValidationError(path, result.issues));
    }
    return path;
  });

export const validatePaths = (
  paths: readonly string[]
): Effect.Effect<readonly string[], RpcEffectError> =>
  Effect.gen(function* () {
    const allIssues: Array<{ path: string; issues: ValidationIssue[] }> = [];

    for (const path of paths) {
      const result = yield* pipe(validatePath(path), Effect.either);
      if (result._tag === "Left") {
        const error = result.left;
        if (error._tag === "RpcValidationError") {
          allIssues.push({ path, issues: [...error.issues] });
        }
      }
    }

    if (allIssues.length > 0) {
      const combinedIssues: ValidationIssue[] = allIssues.flatMap(
        ({ path, issues }) =>
          issues.map((issue) => ({
            ...issue,
            message: `[${path}] ${issue.message}`,
          }))
      );
      return yield* Effect.fail(makeValidationError("batch", combinedIssues));
    }

    return paths;
  });

export const validateAndNormalizePath = (
  path: string
): Effect.Effect<string, RpcEffectError> =>
  pipe(
    validatePath(path),
    Effect.map((validPath) => validPath.toLowerCase())
  );

export const isValidPath = (path: string): Effect.Effect<boolean> =>
  pipe(
    validatePath(path),
    Effect.map(() => true),
    Effect.catchAll(() => Effect.succeed(false))
  );

export const validatePathWithRules = (
  path: string,
  rules: PathValidationRules = {}
): Effect.Effect<string, RpcEffectError> =>
  Effect.gen(function* () {
    if (!rules.allowEmpty || path) {
      yield* validatePath(path);
    }

    const issues: ValidationIssue[] = [];

    if (rules.maxLength !== undefined && path.length > rules.maxLength) {
      issues.push({
        path: [],
        message: `Path exceeds maximum length of ${rules.maxLength}`,
        code: "max_length",
      });
    }

    const segments = path.split(".");
    if (
      rules.minSegments !== undefined &&
      segments.length < rules.minSegments
    ) {
      issues.push({
        path: [],
        message: `Path must have at least ${rules.minSegments} segments`,
        code: "min_segments",
      });
    }
    if (
      rules.maxSegments !== undefined &&
      segments.length > rules.maxSegments
    ) {
      issues.push({
        path: [],
        message: `Path cannot have more than ${rules.maxSegments} segments`,
        code: "max_segments",
      });
    }

    if (rules.allowedPrefixes && rules.allowedPrefixes.length > 0) {
      const hasAllowedPrefix = rules.allowedPrefixes.some((prefix) =>
        path.startsWith(prefix)
      );
      if (!hasAllowedPrefix) {
        issues.push({
          path: [],
          message: `Path must start with one of: ${rules.allowedPrefixes.join(
            ", "
          )}`,
          code: "invalid_prefix",
        });
      }
    }

    if (rules.disallowedPrefixes) {
      const hasDisallowedPrefix = rules.disallowedPrefixes.find((prefix) =>
        path.startsWith(prefix)
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
