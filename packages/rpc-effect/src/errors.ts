// =============================================================================
// @tauri-nexus/rpc-effect - Effect Error Utilities
// =============================================================================
// Error creation, parsing, and conversion utilities for Effect-based errors.

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
  cause?: string,
): RpcCallError => new RpcCallError({ code, message, details, cause });

export const makeTimeoutError = (
  path: string,
  timeoutMs: number,
): RpcTimeoutError => new RpcTimeoutError({ path, timeoutMs });

export const makeCancelledError = (
  path: string,
  reason?: string,
): RpcCancelledError => new RpcCancelledError({ path, reason });

export const makeValidationError = (
  path: string,
  issues: readonly ValidationIssue[],
): RpcValidationError => new RpcValidationError({ path, issues });

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
  if (isEffectRpcError(error)) {
    return error;
  }

  if (error instanceof Error && error.name === "AbortError") {
    if (timeoutMs !== undefined) {
      return makeTimeoutError(path, timeoutMs);
    }
    return makeCancelledError(path);
  }

  const FiberFailureCauseId = Symbol.for("effect/Runtime/FiberFailure/Cause");
  if (
    typeof error === "object" &&
    error !== null &&
    FiberFailureCauseId in error
  ) {
    const cause = (error as Record<symbol, unknown>)[FiberFailureCauseId];
    if (cause && typeof cause === "object") {
      const failures = extractFailuresFromCause(cause);
      if (failures.length > 0) {
        return parseEffectError(failures[0], path, timeoutMs);
      }
    }
  }

  if (
    typeof error === "object" &&
    error !== null &&
    "error" in error &&
    (error as { error: unknown }).error !== undefined
  ) {
    return parseEffectError(
      (error as { error: unknown }).error,
      path,
      timeoutMs,
    );
  }

  if (typeof error === "string") {
    try {
      const parsed = JSON.parse(error);
      if (isRpcErrorShape(parsed)) {
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

  if (isRpcErrorShape(error)) {
    return makeCallError(error.code, error.message, error.details, error.cause);
  }

  if (error instanceof Error) {
    try {
      const parsed = JSON.parse(error.message);
      if (isRpcErrorShape(parsed)) {
        return makeCallError(
          parsed.code,
          parsed.message,
          parsed.details,
          parsed.cause,
        );
      }
    } catch {
      // Not JSON
    }
    return makeCallError("UNKNOWN", error.message, undefined, error.stack);
  }

  return makeCallError("UNKNOWN", String(error));
};

const isEffectRpcError = (error: unknown): error is RpcEffectError => {
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

function extractFailuresFromCause(cause: unknown): unknown[] {
  if (!cause || typeof cause !== "object") return [];

  const c = cause as Record<string, unknown>;

  if (c._tag === "Fail") {
    return [c.error];
  }

  if (c._tag === "Die") {
    return [c.defect];
  }

  if (c._tag === "Sequential" || c._tag === "Parallel") {
    return [
      ...extractFailuresFromCause(c.left),
      ...extractFailuresFromCause(c.right),
    ];
  }

  return [];
}

// =============================================================================
// Effect Error Utilities
// =============================================================================

export const failWithCallError = (
  code: string,
  message: string,
  details?: unknown,
): Effect.Effect<never, RpcCallError> =>
  Effect.fail(makeCallError(code, message, details));

export const failWithTimeout = (
  path: string,
  timeoutMs: number,
): Effect.Effect<never, RpcTimeoutError> =>
  Effect.fail(makeTimeoutError(path, timeoutMs));

export const failWithValidation = (
  path: string,
  issues: readonly ValidationIssue[],
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
  error: unknown,
): error is RpcCancelledError => error instanceof RpcCancelledError;

export const isRpcValidationError = (
  error: unknown,
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
  },
) =>
  Match.value(error).pipe(
    Match.tag("RpcCallError", handlers.onCallError),
    Match.tag("RpcTimeoutError", handlers.onTimeoutError),
    Match.tag("RpcCancelledError", handlers.onCancelledError),
    Match.tag("RpcValidationError", handlers.onValidationError),
    Match.tag("RpcNetworkError", handlers.onNetworkError),
    Match.exhaustive,
  );
