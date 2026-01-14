// =============================================================================
// @tauri-nexus/rpc-core - Error Handling
// =============================================================================
// Error creation, parsing, and type guards.

import type { RpcError, RpcErrorCode } from "./types";
import {
  type RpcEffectError,
  makeCallError,
  makeTimeoutError,
  makeCancelledError,
  isEffectRpcError,
} from "@tauri-nexus/rpc-effect";

// =============================================================================
// Error Type Guards
// =============================================================================

/**
 * Check if error is an RPC error.
 */
export function isRpcError(error: unknown): error is RpcError {
  return (
    typeof error === "object" &&
    error !== null &&
    "code" in error &&
    "message" in error &&
    typeof (error as RpcError).code === "string" &&
    typeof (error as RpcError).message === "string"
  );
}

/**
 * Check if error has a specific code.
 */
export function hasErrorCode(
  error: unknown,
  code: RpcErrorCode | string
): boolean {
  return isRpcError(error) && error.code === code;
}

// =============================================================================
// Error Creation
// =============================================================================

/**
 * Create a typed RPC error.
 */
export function createError(
  code: RpcErrorCode | string,
  message: string,
  details?: unknown
): RpcError {
  return { code, message, details };
}

// =============================================================================
// Error Parsing
// =============================================================================

/**
 * Parse RPC error from backend response.
 */
export function parseError(error: unknown, timeoutMs?: number): RpcError {
  // Handle AbortError (from timeout or manual cancellation)
  if (error instanceof Error) {
    if (error.name === "AbortError") {
      if (timeoutMs !== undefined) {
        return {
          code: "TIMEOUT",
          message: `Request timed out after ${timeoutMs}ms`,
          details: { timeoutMs },
        };
      }
      return { code: "CANCELLED", message: "Request was cancelled" };
    }
    return { code: "UNKNOWN", message: error.message };
  }

  // Handle JSON string errors from backend
  if (typeof error === "string") {
    try {
      const parsed = JSON.parse(error);
      if (isRpcError(parsed)) {
        return parsed;
      }
      return { code: "UNKNOWN", message: error };
    } catch {
      return { code: "UNKNOWN", message: error };
    }
  }

  // Handle RpcError objects directly
  if (isRpcError(error)) {
    return error;
  }

  // Fallback for unknown error types
  return { code: "UNKNOWN", message: String(error) };
}

// =============================================================================
// Effect Error Parsing (comprehensive)
// =============================================================================

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

/**
 * Extract failures from Effect's Cause structure.
 */
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

/**
 * Parse an unknown error into an Effect RPC error.
 * Handles complex cases like JSON parsing, FiberFailure extraction, and nested errors.
 */
export const parseEffectError = (
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

  // Handle Effect FiberFailure
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

  // Handle nested error objects
  if (
    typeof error === "object" &&
    error !== null &&
    "error" in error &&
    (error as { error: unknown }).error !== undefined
  ) {
    return parseEffectError(
      (error as { error: unknown }).error,
      path,
      timeoutMs
    );
  }

  // Handle JSON string errors
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
      return makeCallError("UNKNOWN", error);
    } catch {
      return makeCallError("UNKNOWN", error);
    }
  }

  // Handle RPC error shape objects
  if (isRpcErrorShape(error)) {
    return makeCallError(error.code, error.message, error.details, error.cause);
  }

  // Handle Error objects with JSON message
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

// =============================================================================
// Rate Limit Helpers
// =============================================================================

/**
 * Details structure for rate limit errors from the backend.
 */
interface RateLimitDetails {
  retry_after_ms: number;
  retry_after_secs: number;
}

/**
 * Type guard to check if error details are rate limit details.
 */
function isRateLimitDetails(details: unknown): details is RateLimitDetails {
  return (
    typeof details === "object" &&
    details !== null &&
    "retry_after_ms" in details &&
    typeof (details as RateLimitDetails).retry_after_ms === "number"
  );
}

/**
 * Check if an RPC error is a rate limit error.
 */
export function isRateLimitError(error: unknown): error is RpcError {
  return isRpcError(error) && error.code === "RATE_LIMITED";
}

/**
 * Extract the retry-after time in milliseconds from a rate limit error.
 */
export function getRateLimitRetryAfter(error: RpcError): number | undefined {
  if (error.code !== "RATE_LIMITED") {
    return undefined;
  }
  if (!isRateLimitDetails(error.details)) {
    return undefined;
  }
  return error.details.retry_after_ms;
}
