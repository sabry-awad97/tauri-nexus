// =============================================================================
// Error Schemas - Schema definitions for RPC errors
// =============================================================================

import { Schema, ParseResult } from "effect";
import { Effect } from "effect";
import type { ValidationIssue } from "../core/types";
import { RpcValidationError } from "../core/errors";

// =============================================================================
// Error Schemas
// =============================================================================

/**
 * Schema for validation issues.
 */
export const ValidationIssueSchema = Schema.Struct({
  path: Schema.Array(Schema.Union(Schema.String, Schema.Number)),
  message: Schema.String,
  code: Schema.String,
}).pipe(
  Schema.annotations({
    identifier: "ValidationIssue",
    description: "Validation issue details",
  }),
);

/**
 * Schema for RPC error structure.
 */
export const RpcErrorSchema = Schema.Struct({
  code: Schema.String,
  message: Schema.String,
  details: Schema.optional(Schema.Unknown),
  issues: Schema.optional(Schema.Array(ValidationIssueSchema)),
}).pipe(
  Schema.annotations({
    identifier: "RpcError",
    description: "RPC error payload",
  }),
);

// =============================================================================
// Type Inference
// =============================================================================

export type RpcErrorShape = typeof RpcErrorSchema.Type;

// =============================================================================
// Encoding/Decoding
// =============================================================================

/**
 * Decode an RPC error from unknown input.
 */
export const decodeRpcError = Schema.decodeUnknown(RpcErrorSchema);

/**
 * Encode an RPC error to unknown output.
 */
export const encodeRpcError = Schema.encodeUnknown(RpcErrorSchema);

// =============================================================================
// Conversion Utilities
// =============================================================================

/**
 * Convert a Schema ParseError issue to a ValidationIssue.
 */
export const schemaIssueToValidationIssue = (
  error: ParseResult.ParseError,
): ValidationIssue[] => {
  const issues: ValidationIssue[] = [];

  const extractIssues = (
    issue: ParseResult.ParseIssue,
    currentPath: (string | number)[] = [],
  ): void => {
    switch (issue._tag) {
      case "Type":
        issues.push({
          path: currentPath,
          message: `Type error: expected ${issue.ast._tag}`,
          code: "TYPE_ERROR",
        });
        break;

      case "Missing":
        issues.push({
          path: currentPath,
          message: "Missing required field",
          code: "MISSING_FIELD",
        });
        break;

      case "Unexpected":
        issues.push({
          path: currentPath,
          message: "Unexpected field",
          code: "UNEXPECTED_FIELD",
        });
        break;

      case "Forbidden":
        issues.push({
          path: currentPath,
          message: "Forbidden value",
          code: "FORBIDDEN",
        });
        break;

      case "Pointer": {
        const pathSegments = Array.isArray(issue.path)
          ? issue.path
          : [issue.path];
        extractIssues(issue.issue, [...currentPath, ...pathSegments]);
        break;
      }

      case "Refinement":
        issues.push({
          path: currentPath,
          message: "Refinement failed",
          code: "REFINEMENT_ERROR",
        });
        break;

      case "Transformation":
        issues.push({
          path: currentPath,
          message: "Transformation failed",
          code: "TRANSFORMATION_ERROR",
        });
        break;

      case "Composite": {
        const subIssues = Array.isArray(issue.issues)
          ? issue.issues
          : [issue.issues];
        for (const subIssue of subIssues) {
          extractIssues(subIssue, currentPath);
        }
        break;
      }
    }
  };

  extractIssues(error.issue);
  return issues;
};

/**
 * Create an RpcValidationError from a Schema ParseError.
 */
export const createSchemaValidationError = (
  path: string,
  error: ParseResult.ParseError,
): RpcValidationError => {
  const issues = schemaIssueToValidationIssue(error);
  return new RpcValidationError({ path, issues });
};

/**
 * Map Schema ParseError to RpcValidationError in an Effect.
 */
export const mapSchemaError =
  (path: string) =>
  <A>(
    effect: Effect.Effect<A, ParseResult.ParseError>,
  ): Effect.Effect<A, RpcValidationError> =>
    effect.pipe(
      Effect.mapError((error) => createSchemaValidationError(path, error)),
    );
