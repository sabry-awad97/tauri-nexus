// =============================================================================
// Error Conversion
// =============================================================================
// Convert between Effect errors and serializable RpcError.

import {
  RpcTimeoutError,
  RpcCancelledError,
  RpcValidationError,
  RpcCallError,
  type RpcEffectError,
} from "../core/errors";
import type { ValidationIssue } from "../core/types";
import { matchError } from "../core/error-utils";
import type { RpcError } from "./types";

/**
 * Convert Effect error to serializable RpcError.
 */
export const toRpcError = (error: RpcEffectError): RpcError =>
  matchError<RpcError>(error, {
    onCallError: (e) => ({
      code: e.code,
      message: e.message,
      details: e.details,
      cause: e.cause,
    }),
    onTimeoutError: (e) => ({
      code: "TIMEOUT",
      message: `Request to '${e.path}' timed out after ${e.timeoutMs}ms`,
      details: { timeoutMs: e.timeoutMs, path: e.path },
    }),
    onCancelledError: (e) => ({
      code: "CANCELLED",
      message: e.reason ?? `Request to '${e.path}' was cancelled`,
      details: { path: e.path },
    }),
    onValidationError: (e) => ({
      code: "VALIDATION_ERROR",
      message:
        e.issues.length > 0
          ? e.issues[0].message
          : `Validation failed for '${e.path}'`,
      details: { issues: e.issues },
    }),
    onNetworkError: (e) => ({
      code: "INTERNAL_ERROR",
      message: `Network error calling '${e.path}'`,
      details: { originalError: String(e.originalError) },
    }),
  });

/**
 * Convert serializable RpcError to Effect error.
 */
export const fromRpcError = (error: RpcError, path: string): RpcEffectError => {
  switch (error.code) {
    case "TIMEOUT":
      return new RpcTimeoutError({
        path,
        timeoutMs: (error.details as { timeoutMs?: number })?.timeoutMs ?? 0,
      });
    case "CANCELLED":
      return new RpcCancelledError({ path, reason: error.message });
    case "VALIDATION_ERROR":
      return new RpcValidationError({
        path,
        issues: (error.details as { issues?: ValidationIssue[] })?.issues ?? [],
      });
    default:
      return new RpcCallError({
        code: error.code,
        message: error.message,
        details: error.details,
        cause: error.cause,
      });
  }
};

/**
 * Type guard for serializable RpcError.
 */
export const isRpcError = (error: unknown): error is RpcError =>
  typeof error === "object" &&
  error !== null &&
  "code" in error &&
  "message" in error &&
  typeof (error as RpcError).code === "string" &&
  typeof (error as RpcError).message === "string";

/**
 * Check if error has a specific error code.
 */
export const hasErrorCode = (error: unknown, code: string): boolean =>
  isRpcError(error) && error.code === code;

/**
 * Create a serializable RpcError.
 */
export const createRpcError = (
  code: string,
  message: string,
  details?: unknown,
): RpcError => ({ code, message, details });

// =============================================================================
// Rate Limit Utilities
// =============================================================================

/**
 * Check if error is a rate limit error.
 */
export const isRateLimitError = (error: unknown): error is RpcError =>
  isRpcError(error) && error.code === "RATE_LIMITED";

/**
 * Extract retry-after duration from rate limit error.
 */
export const getRateLimitRetryAfter = (error: RpcError): number | undefined => {
  if (error.code !== "RATE_LIMITED") return undefined;
  const details = error.details as { retry_after_ms?: number } | undefined;
  return typeof details?.retry_after_ms === "number"
    ? details.retry_after_ms
    : undefined;
};
