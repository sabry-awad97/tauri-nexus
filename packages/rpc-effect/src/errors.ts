// =============================================================================
// @tauri-nexus/rpc-effect - Effect Error Utilities
// =============================================================================
// Pure Effect error constructors, type guards, and pattern matching.
// No parsing logic - that belongs in the transport layer (rpc-core).

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

/** Create an RPC call error */
export const makeCallError = (
  code: string,
  message: string,
  details?: unknown,
  cause?: string
): RpcCallError => new RpcCallError({ code, message, details, cause });

/** Create a timeout error */
export const makeTimeoutError = (
  path: string,
  timeoutMs: number
): RpcTimeoutError => new RpcTimeoutError({ path, timeoutMs });

/** Create a cancellation error */
export const makeCancelledError = (
  path: string,
  reason?: string
): RpcCancelledError => new RpcCancelledError({ path, reason });

/** Create a validation error */
export const makeValidationError = (
  path: string,
  issues: readonly ValidationIssue[]
): RpcValidationError => new RpcValidationError({ path, issues });

/** Create a network error */
export const makeNetworkError = (
  path: string,
  originalError: unknown
): RpcNetworkError => new RpcNetworkError({ path, originalError });

// =============================================================================
// Type Guards
// =============================================================================

/** Check if a value is an Effect RPC error */
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

/** Check if error is an RpcCallError */
export const isRpcCallError = (error: unknown): error is RpcCallError =>
  error instanceof RpcCallError;

/** Check if error is an RpcTimeoutError */
export const isRpcTimeoutError = (error: unknown): error is RpcTimeoutError =>
  error instanceof RpcTimeoutError;

/** Check if error is an RpcCancelledError */
export const isRpcCancelledError = (
  error: unknown
): error is RpcCancelledError => error instanceof RpcCancelledError;

/** Check if error is an RpcValidationError */
export const isRpcValidationError = (
  error: unknown
): error is RpcValidationError => error instanceof RpcValidationError;

/** Check if error is an RpcNetworkError */
export const isRpcNetworkError = (error: unknown): error is RpcNetworkError =>
  error instanceof RpcNetworkError;

/** Check if an Effect error has a specific code */
export const hasCode = (error: RpcEffectError, code: string): boolean =>
  matchError(error, {
    onCallError: (e) => e.code === code,
    onTimeoutError: () => code === "TIMEOUT",
    onCancelledError: () => code === "CANCELLED",
    onValidationError: () => code === "VALIDATION_ERROR",
    onNetworkError: () => code === "INTERNAL_ERROR",
  });

// =============================================================================
// Pattern Matching
// =============================================================================

/** Match on RPC error types using Effect's Match */
export const matchError = <A>(
  error: RpcEffectError,
  handlers: {
    onCallError: (e: RpcCallError) => A;
    onTimeoutError: (e: RpcTimeoutError) => A;
    onCancelledError: (e: RpcCancelledError) => A;
    onValidationError: (e: RpcValidationError) => A;
    onNetworkError: (e: RpcNetworkError) => A;
  }
): A =>
  Match.value(error).pipe(
    Match.tag("RpcCallError", handlers.onCallError),
    Match.tag("RpcTimeoutError", handlers.onTimeoutError),
    Match.tag("RpcCancelledError", handlers.onCancelledError),
    Match.tag("RpcValidationError", handlers.onValidationError),
    Match.tag("RpcNetworkError", handlers.onNetworkError),
    Match.exhaustive
  ) as A;

// =============================================================================
// Effect Combinators
// =============================================================================

/** Fail with a call error */
export const failWithCallError = (
  code: string,
  message: string,
  details?: unknown
): Effect.Effect<never, RpcCallError> =>
  Effect.fail(makeCallError(code, message, details));

/** Fail with a timeout error */
export const failWithTimeout = (
  path: string,
  timeoutMs: number
): Effect.Effect<never, RpcTimeoutError> =>
  Effect.fail(makeTimeoutError(path, timeoutMs));

/** Fail with a validation error */
export const failWithValidation = (
  path: string,
  issues: readonly ValidationIssue[]
): Effect.Effect<never, RpcValidationError> =>
  Effect.fail(makeValidationError(path, issues));

/** Fail with a network error */
export const failWithNetwork = (
  path: string,
  originalError: unknown
): Effect.Effect<never, RpcNetworkError> =>
  Effect.fail(makeNetworkError(path, originalError));

/** Fail with a cancellation error */
export const failWithCancelled = (
  path: string,
  reason?: string
): Effect.Effect<never, RpcCancelledError> =>
  Effect.fail(makeCancelledError(path, reason));

// =============================================================================
// Error Conversion (for transport layer)
// =============================================================================

/** RPC error shape from transport */
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

/** Try to parse a JSON string into an RPC error shape */
const tryParseJsonError = (str: string): RpcErrorShape | null => {
  try {
    const parsed = JSON.parse(str);
    return isRpcErrorShape(parsed) ? parsed : null;
  } catch {
    return null;
  }
};

/**
 * Convert a transport error to an Effect RPC error.
 * Handles: Effect errors (passthrough), AbortError, RPC shape, JSON strings, Error, string.
 */
export const fromTransportError = (
  error: unknown,
  path: string,
  timeoutMs?: number
): RpcEffectError => {
  // Passthrough Effect errors
  if (isEffectRpcError(error)) return error;

  // AbortError → Timeout or Cancelled
  if (error instanceof Error && error.name === "AbortError") {
    return timeoutMs !== undefined
      ? makeTimeoutError(path, timeoutMs)
      : makeCancelledError(path);
  }

  // RPC error shape from transport
  if (isRpcErrorShape(error)) {
    return makeCallError(error.code, error.message, error.details, error.cause);
  }

  // JSON string error (common from Tauri)
  if (typeof error === "string") {
    const parsed = tryParseJsonError(error);
    if (parsed) {
      return makeCallError(
        parsed.code,
        parsed.message,
        parsed.details,
        parsed.cause
      );
    }
    return makeCallError("UNKNOWN", error);
  }

  // Standard Error (may have JSON message)
  if (error instanceof Error) {
    const parsed = tryParseJsonError(error.message);
    if (parsed) {
      return makeCallError(
        parsed.code,
        parsed.message,
        parsed.details,
        parsed.cause
      );
    }
    return makeCallError("UNKNOWN", error.message, undefined, error.stack);
  }

  // Fallback
  return makeCallError("UNKNOWN", String(error));
};
