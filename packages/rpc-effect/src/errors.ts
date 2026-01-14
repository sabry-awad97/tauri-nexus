// =============================================================================
// @tauri-nexus/rpc-effect - Effect Error Utilities
// =============================================================================
// Pure Effect error constructors and type guards.
// Complex parsing logic should be in rpc-core.

import { Effect, Match } from "effect";
import {
  RpcCallError,
  RpcTimeoutError,
  RpcCancelledError,
  RpcValidationError,
  RpcNetworkError,
  type RpcEffectError,
  type ValidationIssue,
} from "./types";

// =============================================================================
// Error Constructors
// =============================================================================

export const makeCallError = (
  code: string,
  message: string,
  details?: unknown,
  cause?: string
): RpcCallError => new RpcCallError({ code, message, details, cause });

export const makeTimeoutError = (
  path: string,
  timeoutMs: number
): RpcTimeoutError => new RpcTimeoutError({ path, timeoutMs });

export const makeCancelledError = (
  path: string,
  reason?: string
): RpcCancelledError => new RpcCancelledError({ path, reason });

export const makeValidationError = (
  path: string,
  issues: readonly ValidationIssue[]
): RpcValidationError => new RpcValidationError({ path, issues });

export const makeNetworkError = (
  path: string,
  originalError: unknown
): RpcNetworkError => new RpcNetworkError({ path, originalError });

// =============================================================================
// Type Guards
// =============================================================================

export const isEffectRpcError = (error: unknown): error is RpcEffectError => {
  if (typeof error !== "object" || error === null) return false;
  const tag = (error as { _tag?: string })._tag;
  return (
    tag === "RpcCallError" ||
    tag === "RpcTimeoutError" ||
    tag === "RpcCancelledError" ||
    tag === "RpcValidationError" ||
    tag === "RpcNetworkError"
  );
};

// =============================================================================
// Minimal Error Conversion (for internal use)
// =============================================================================

/**
 * Convert an unknown error to an Effect RPC error.
 * Handles basic cases including JSON string parsing for transport errors.
 */
export const toEffectError = (
  error: unknown,
  path: string,
  timeoutMs?: number
): RpcEffectError => {
  // Already an Effect error
  if (isEffectRpcError(error)) {
    return error;
  }

  // AbortError handling
  if (error instanceof Error && error.name === "AbortError") {
    if (timeoutMs !== undefined) {
      return makeTimeoutError(path, timeoutMs);
    }
    return makeCancelledError(path);
  }

  // RPC error shape (code + message)
  if (isRpcErrorShape(error)) {
    return makeCallError(error.code, error.message, error.details, error.cause);
  }

  // JSON string error (common from transports)
  if (typeof error === "string") {
    try {
      const parsed = JSON.parse(error);
      if (isRpcErrorShape(parsed)) {
        return makeCallError(
          parsed.code,
          parsed.message,
          parsed.details,
          parsed.cause
        );
      }
    } catch {
      // Not JSON, treat as plain string
    }
    return makeCallError("UNKNOWN", error);
  }

  // Generic Error with possible JSON message
  if (error instanceof Error) {
    try {
      const parsed = JSON.parse(error.message);
      if (isRpcErrorShape(parsed)) {
        return makeCallError(
          parsed.code,
          parsed.message,
          parsed.details,
          parsed.cause
        );
      }
    } catch {
      // Not JSON
    }
    return makeCallError("UNKNOWN", error.message, undefined, error.stack);
  }

  // Fallback
  return makeCallError("UNKNOWN", String(error));
};

interface RpcErrorShape {
  code: string;
  message: string;
  details?: unknown;
  cause?: string;
}

const isRpcErrorShape = (error: unknown): error is RpcErrorShape =>
  typeof error === "object" &&
  error !== null &&
  "code" in error &&
  "message" in error &&
  typeof (error as RpcErrorShape).code === "string" &&
  typeof (error as RpcErrorShape).message === "string";

// =============================================================================
// Effect Error Utilities
// =============================================================================

export const failWithCallError = (
  code: string,
  message: string,
  details?: unknown
): Effect.Effect<never, RpcCallError> =>
  Effect.fail(makeCallError(code, message, details));

export const failWithTimeout = (
  path: string,
  timeoutMs: number
): Effect.Effect<never, RpcTimeoutError> =>
  Effect.fail(makeTimeoutError(path, timeoutMs));

export const failWithValidation = (
  path: string,
  issues: readonly ValidationIssue[]
): Effect.Effect<never, RpcValidationError> =>
  Effect.fail(makeValidationError(path, issues));

// =============================================================================
// Error Type Guards
// =============================================================================

export const isRpcCallError = (error: unknown): error is RpcCallError =>
  error instanceof RpcCallError;

export const isRpcTimeoutError = (error: unknown): error is RpcTimeoutError =>
  error instanceof RpcTimeoutError;

export const isRpcCancelledError = (
  error: unknown
): error is RpcCancelledError => error instanceof RpcCancelledError;

export const isRpcValidationError = (
  error: unknown
): error is RpcValidationError => error instanceof RpcValidationError;

export const isRpcNetworkError = (error: unknown): error is RpcNetworkError =>
  error instanceof RpcNetworkError;

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

// =============================================================================
// Error Matching Utilities
// =============================================================================

/**
 * Match on RPC error types using Effect's Match.
 */
export const matchError = <A>(
  error: RpcEffectError,
  handlers: {
    onCallError: (e: RpcCallError) => A;
    onTimeoutError: (e: RpcTimeoutError) => A;
    onCancelledError: (e: RpcCancelledError) => A;
    onValidationError: (e: RpcValidationError) => A;
    onNetworkError: (e: RpcNetworkError) => A;
  }
) =>
  Match.value(error).pipe(
    Match.tag("RpcCallError", handlers.onCallError),
    Match.tag("RpcTimeoutError", handlers.onTimeoutError),
    Match.tag("RpcCancelledError", handlers.onCancelledError),
    Match.tag("RpcValidationError", handlers.onValidationError),
    Match.tag("RpcNetworkError", handlers.onNetworkError),
    Match.exhaustive
  );

// =============================================================================
// Backward Compatibility
// =============================================================================

/** Alias for toEffectError - kept for backward compatibility */
export const parseEffectError = toEffectError;
