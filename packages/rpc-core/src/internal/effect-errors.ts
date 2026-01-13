// =============================================================================
// @tauri-nexus/rpc-core - Effect Error Utilities
// =============================================================================
// Error creation, parsing, and conversion utilities for Effect-based errors.

import { Effect, Match } from "effect";
import type { RpcError as PublicRpcError } from "../core/types";
import {
  RpcCallError,
  RpcTimeoutError,
  RpcCancelledError,
  RpcValidationError,
  RpcNetworkError,
  type RpcEffectError,
  type ValidationIssue,
} from "./effect-types";

// =============================================================================
// Error Constructors
// =============================================================================

/**
 * Create a typed RPC call error.
 */
export const makeCallError = (
  code: string,
  message: string,
  details?: unknown,
  cause?: string,
): RpcCallError =>
  new RpcCallError({ code, message, details, cause });

/**
 * Create a timeout error.
 */
export const makeTimeoutError = (
  path: string,
  timeoutMs: number,
): RpcTimeoutError => new RpcTimeoutError({ path, timeoutMs });

/**
 * Create a cancellation error.
 */
export const makeCancelledError = (
  path: string,
  reason?: string,
): RpcCancelledError => new RpcCancelledError({ path, reason });

/**
 * Create a validation error.
 */
export const makeValidationError = (
  path: string,
  issues: readonly ValidationIssue[],
): RpcValidationError => new RpcValidationError({ path, issues });

/**
 * Create a network error.
 */
export const makeNetworkError = (
  path: string,
  originalError: unknown,
): RpcNetworkError => new RpcNetworkError({ path, originalError });

// =============================================================================
// Error Parsing
// =============================================================================

/**
 * Parse an unknown error into an Effect RPC error.
 */
export const parseEffectError = (
  error: unknown,
  path: string,
  timeoutMs?: number,
): RpcEffectError => {
  // Handle AbortError first (before other Error checks)
  if (error instanceof Error && error.name === "AbortError") {
    if (timeoutMs !== undefined) {
      return makeTimeoutError(path, timeoutMs);
    }
    return makeCancelledError(path);
  }

  // Handle Effect's FiberFailure which wraps the cause
  // FiberFailure has a Symbol property containing the Cause
  const FiberFailureCauseId = Symbol.for("effect/Runtime/FiberFailure/Cause");
  if (
    typeof error === "object" &&
    error !== null &&
    FiberFailureCauseId in error
  ) {
    // Extract the cause from FiberFailure
    const cause = (error as Record<symbol, unknown>)[FiberFailureCauseId];
    // The cause has a structure with failures - try to extract the original error
    if (cause && typeof cause === "object") {
      // Try to get the first failure from the cause
      const failures = extractFailuresFromCause(cause);
      if (failures.length > 0) {
        return parseEffectError(failures[0], path, timeoutMs);
      }
    }
  }

  // Handle Effect's UnknownException which wraps the original error
  // IMPORTANT: Check this BEFORE instanceof Error, because UnknownException extends Error
  // but we want to unwrap and parse the original error
  if (
    typeof error === "object" &&
    error !== null &&
    "error" in error &&
    (error as { error: unknown }).error !== undefined
  ) {
    // Recursively parse the wrapped error
    return parseEffectError((error as { error: unknown }).error, path, timeoutMs);
  }

  // Handle JSON string errors from backend
  if (typeof error === "string") {
    try {
      const parsed = JSON.parse(error);
      if (isPublicRpcError(parsed)) {
        return makeCallError(
          parsed.code,
          parsed.message,
          parsed.details,
          parsed.cause,
        );
      }
      return makeCallError("UNKNOWN", error);
    } catch {
      return makeCallError("UNKNOWN", error);
    }
  }

  // Handle RpcError objects directly
  if (isPublicRpcError(error)) {
    return makeCallError(error.code, error.message, error.details, error.cause);
  }

  // Handle Error instances - check if the message is a JSON string
  if (error instanceof Error) {
    // Try to parse the error message as JSON (some backends wrap errors this way)
    try {
      const parsed = JSON.parse(error.message);
      if (isPublicRpcError(parsed)) {
        return makeCallError(
          parsed.code,
          parsed.message,
          parsed.details,
          parsed.cause,
        );
      }
    } catch {
      // Not JSON, use the message as-is
    }
    return makeCallError("UNKNOWN", error.message, undefined, error.stack);
  }

  // Fallback
  return makeCallError("UNKNOWN", String(error));
};

/**
 * Extract failures from an Effect Cause object.
 * The Cause structure can be complex (Sequential, Parallel, etc.)
 */
function extractFailuresFromCause(cause: unknown): unknown[] {
  if (!cause || typeof cause !== "object") return [];
  
  const c = cause as Record<string, unknown>;
  
  // Handle Fail cause - contains the actual error
  if (c._tag === "Fail") {
    return [c.error];
  }
  
  // Handle Die cause - contains defect
  if (c._tag === "Die") {
    return [c.defect];
  }
  
  // Handle Sequential cause - has left and right
  if (c._tag === "Sequential" || c._tag === "Parallel") {
    return [
      ...extractFailuresFromCause(c.left),
      ...extractFailuresFromCause(c.right),
    ];
  }
  
  return [];
}

/**
 * Type guard for public RpcError.
 */
const isPublicRpcError = (error: unknown): error is PublicRpcError =>
  typeof error === "object" &&
  error !== null &&
  "code" in error &&
  "message" in error &&
  typeof (error as PublicRpcError).code === "string" &&
  typeof (error as PublicRpcError).message === "string";

// =============================================================================
// Error Conversion
// =============================================================================

/**
 * Convert Effect error to public RpcError format.
 * Uses Effect's Match for exhaustive pattern matching.
 */
export const toPublicError = (error: RpcEffectError): PublicRpcError =>
  Match.value(error).pipe(
    Match.tag("RpcCallError", (e) => e.toPublic()),
    Match.tag("RpcTimeoutError", (e) => e.toPublic()),
    Match.tag("RpcCancelledError", (e) => e.toPublic()),
    Match.tag("RpcValidationError", (e) => e.toPublic()),
    Match.tag("RpcNetworkError", (e) => e.toPublic()),
    Match.exhaustive,
  );

/**
 * Convert public RpcError to Effect error.
 */
export const fromPublicError = (
  error: PublicRpcError,
  path: string,
): RpcEffectError => {
  switch (error.code) {
    case "TIMEOUT":
      return makeTimeoutError(
        path,
        (error.details as { timeoutMs?: number })?.timeoutMs ?? 0,
      );
    case "CANCELLED":
      return makeCancelledError(path, error.message);
    case "VALIDATION_ERROR":
      return makeValidationError(
        path,
        (error.details as { issues?: ValidationIssue[] })?.issues ?? [],
      );
    default:
      return makeCallError(error.code, error.message, error.details, error.cause);
  }
};

// =============================================================================
// Effect Error Utilities
// =============================================================================

/**
 * Fail with a call error.
 */
export const failWithCallError = (
  code: string,
  message: string,
  details?: unknown,
): Effect.Effect<never, RpcCallError> =>
  Effect.fail(makeCallError(code, message, details));

/**
 * Fail with a timeout error.
 */
export const failWithTimeout = (
  path: string,
  timeoutMs: number,
): Effect.Effect<never, RpcTimeoutError> =>
  Effect.fail(makeTimeoutError(path, timeoutMs));

/**
 * Fail with a validation error.
 */
export const failWithValidation = (
  path: string,
  issues: readonly ValidationIssue[],
): Effect.Effect<never, RpcValidationError> =>
  Effect.fail(makeValidationError(path, issues));

// =============================================================================
// Error Type Guards
// =============================================================================

/**
 * Check if error is a specific RPC error type.
 */
export const isRpcCallError = (error: unknown): error is RpcCallError =>
  error instanceof RpcCallError;

export const isRpcTimeoutError = (error: unknown): error is RpcTimeoutError =>
  error instanceof RpcTimeoutError;

export const isRpcCancelledError = (
  error: unknown,
): error is RpcCancelledError => error instanceof RpcCancelledError;

export const isRpcValidationError = (
  error: unknown,
): error is RpcValidationError => error instanceof RpcValidationError;

export const isRpcNetworkError = (error: unknown): error is RpcNetworkError =>
  error instanceof RpcNetworkError;

/**
 * Check if error has a specific code.
 */
export const hasCode = (error: RpcEffectError, code: string): boolean => {
  if (error instanceof RpcCallError) {
    return error.code === code;
  }
  if (error instanceof RpcTimeoutError) {
    return code === "TIMEOUT";
  }
  if (error instanceof RpcCancelledError) {
    return code === "CANCELLED";
  }
  if (error instanceof RpcValidationError) {
    return code === "VALIDATION_ERROR";
  }
  if (error instanceof RpcNetworkError) {
    return code === "INTERNAL_ERROR";
  }
  return false;
};
