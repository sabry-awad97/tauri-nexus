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
// Error Shape Detection
// =============================================================================

/** Standard RPC error shape from transport/backend */
export interface RpcErrorShape {
  readonly code: string;
  readonly message: string;
  readonly details?: unknown;
  readonly cause?: string;
}

/** Type guard for RPC error shape */
export const isRpcErrorShape = (value: unknown): value is RpcErrorShape =>
  typeof value === "object" &&
  value !== null &&
  "code" in value &&
  "message" in value &&
  typeof (value as RpcErrorShape).code === "string" &&
  typeof (value as RpcErrorShape).message === "string";

/** Parse JSON string to RPC error shape (returns null on failure) */
export const parseJsonError = (str: string): RpcErrorShape | null => {
  try {
    const parsed = JSON.parse(str);
    return isRpcErrorShape(parsed) ? parsed : null;
  } catch {
    return null;
  }
};

/** Create an RPC call error from error shape */
export const makeCallErrorFromShape = (
  shape: RpcErrorShape
): ReturnType<typeof makeCallError> =>
  makeCallError(shape.code, shape.message, shape.details, shape.cause);

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
// Error Parsing (Public RpcError format)
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
// Unified Effect Error Parser
// =============================================================================

/** Symbol for Effect's FiberFailure cause */
const FiberFailureCauseId = Symbol.for("effect/Runtime/FiberFailure/Cause");

/**
 * Extract failures from Effect's Cause structure.
 */
function extractFailuresFromCause(cause: unknown): unknown[] {
  if (!cause || typeof cause !== "object") return [];

  const c = cause as Record<string, unknown>;

  if (c._tag === "Fail") return [c.error];
  if (c._tag === "Die") return [c.defect];

  if (c._tag === "Sequential" || c._tag === "Parallel") {
    return [
      ...extractFailuresFromCause(c.left),
      ...extractFailuresFromCause(c.right),
    ];
  }

  return [];
}

/**
 * Error parser options for customizing behavior.
 */
export interface ErrorParserOptions {
  /** Enable JSON string parsing (for Tauri transport) */
  readonly parseJson?: boolean;
  /** Enable FiberFailure extraction (for complex Effect scenarios) */
  readonly extractFiberFailure?: boolean;
  /** Enable nested error unwrapping */
  readonly unwrapNested?: boolean;
}

/** Default options for transport error parsing */
const defaultParserOptions: ErrorParserOptions = {
  parseJson: true,
  extractFiberFailure: false,
  unwrapNested: false,
};

/** Full options for comprehensive error parsing */
const fullParserOptions: ErrorParserOptions = {
  parseJson: true,
  extractFiberFailure: true,
  unwrapNested: true,
};

/**
 * Unified error parser that converts any error to RpcEffectError.
 * Configurable via options for different use cases.
 */
export const parseToEffectError = (
  error: unknown,
  path: string,
  timeoutMs?: number,
  options: ErrorParserOptions = defaultParserOptions
): RpcEffectError => {
  // 1. Passthrough Effect errors
  if (isEffectRpcError(error)) return error;

  // 2. AbortError → Timeout or Cancelled
  if (error instanceof Error && error.name === "AbortError") {
    return timeoutMs !== undefined
      ? makeTimeoutError(path, timeoutMs)
      : makeCancelledError(path);
  }

  // 3. FiberFailure extraction (optional)
  if (options.extractFiberFailure) {
    if (
      typeof error === "object" &&
      error !== null &&
      FiberFailureCauseId in error
    ) {
      const cause = (error as Record<symbol, unknown>)[FiberFailureCauseId];
      if (cause && typeof cause === "object") {
        const failures = extractFailuresFromCause(cause);
        if (failures.length > 0) {
          return parseToEffectError(failures[0], path, timeoutMs, options);
        }
      }
    }
  }

  // 4. Nested error unwrapping (optional)
  if (options.unwrapNested) {
    if (
      typeof error === "object" &&
      error !== null &&
      "error" in error &&
      (error as { error: unknown }).error !== undefined
    ) {
      return parseToEffectError(
        (error as { error: unknown }).error,
        path,
        timeoutMs,
        options
      );
    }
  }

  // 5. RPC error shape from transport
  if (isRpcErrorShape(error)) {
    return makeCallErrorFromShape(error);
  }

  // 6. JSON string parsing (optional, for Tauri)
  if (options.parseJson && typeof error === "string") {
    const parsed = parseJsonError(error);
    return parsed
      ? makeCallErrorFromShape(parsed)
      : makeCallError("UNKNOWN", error);
  }

  // 7. String error (no JSON parsing)
  if (typeof error === "string") {
    return makeCallError("UNKNOWN", error);
  }

  // 8. Standard Error (may have JSON message)
  if (error instanceof Error) {
    if (options.parseJson) {
      const parsed = parseJsonError(error.message);
      if (parsed) return makeCallErrorFromShape(parsed);
    }
    return makeCallError("UNKNOWN", error.message, undefined, error.stack);
  }

  // 9. Fallback
  return makeCallError("UNKNOWN", String(error));
};

// =============================================================================
// Convenience Aliases (backward compatible)
// =============================================================================

/**
 * Convert a transport error to an Effect RPC error.
 * Standard error converter for Tauri transport with JSON parsing.
 */
export const fromTransportError = (
  error: unknown,
  path: string,
  timeoutMs?: number
): RpcEffectError =>
  parseToEffectError(error, path, timeoutMs, defaultParserOptions);

/**
 * Parse an unknown error into an Effect RPC error.
 * Comprehensive parser with FiberFailure extraction and nested unwrapping.
 */
export const parseEffectError = (
  error: unknown,
  path: string,
  timeoutMs?: number
): RpcEffectError =>
  parseToEffectError(error, path, timeoutMs, fullParserOptions);

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
