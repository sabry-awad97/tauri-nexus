// =============================================================================
// @tauri-nexus/rpc-core - Error Handling
// =============================================================================
// Error creation, parsing, and type guards.

import type { RpcError, RpcErrorCode } from "./types";

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
  code: RpcErrorCode | string,
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
  details?: unknown,
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
