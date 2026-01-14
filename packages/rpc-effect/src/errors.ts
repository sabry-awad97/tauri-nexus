// =============================================================================
// @tauri-nexus/rpc-effect - Effect Error Utilities
// =============================================================================
// Pure Effect error constructors, type guards, pattern matching, and combinators.
// This is the SINGLE SOURCE OF TRUTH for RPC error handling.
// rpc-core re-exports these utilities - do not duplicate!

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

/** RPC error tags for type checking */
const RPC_ERROR_TAGS = [
  "RpcCallError",
  "RpcTimeoutError",
  "RpcCancelledError",
  "RpcValidationError",
  "RpcNetworkError",
] as const;

type RpcErrorTag = (typeof RPC_ERROR_TAGS)[number];

/** Check if a value is an Effect RPC error */
export const isEffectRpcError = (error: unknown): error is RpcEffectError => {
  if (typeof error !== "object" || error === null) return false;
  const tag = (error as { _tag?: string })._tag;
  return RPC_ERROR_TAGS.includes(tag as RpcErrorTag);
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

/** Virtual codes for non-call errors */
export type VirtualErrorCode =
  | "TIMEOUT"
  | "CANCELLED"
  | "VALIDATION_ERROR"
  | "NETWORK_ERROR";

/** Get the effective code from any RPC error */
export const getErrorCode = (error: RpcEffectError): string =>
  matchError(error, {
    onCallError: (e) => e.code,
    onTimeoutError: () => "TIMEOUT",
    onCancelledError: () => "CANCELLED",
    onValidationError: () => "VALIDATION_ERROR",
    onNetworkError: () => "NETWORK_ERROR",
  });

/** Check if an Effect error has a specific code (type-safe) */
export const hasCode = <C extends string>(
  error: RpcEffectError,
  code: C
): boolean => getErrorCode(error) === code;

/** Check if error matches any of the given codes */
export const hasAnyCode = (
  error: RpcEffectError,
  codes: readonly string[]
): boolean => codes.includes(getErrorCode(error));

/** Check if error is retryable (not a client error) */
export const isRetryableError = (error: RpcEffectError): boolean => {
  const nonRetryable = [
    "VALIDATION_ERROR",
    "UNAUTHORIZED",
    "FORBIDDEN",
    "CANCELLED",
    "BAD_REQUEST",
    "NOT_FOUND",
  ];
  return !hasAnyCode(error, nonRetryable);
};

// =============================================================================
// Pattern Matching
// =============================================================================

/** Error handler functions for pattern matching */
export interface ErrorHandlers<A> {
  readonly onCallError: (e: RpcCallError) => A;
  readonly onTimeoutError: (e: RpcTimeoutError) => A;
  readonly onCancelledError: (e: RpcCancelledError) => A;
  readonly onValidationError: (e: RpcValidationError) => A;
  readonly onNetworkError: (e: RpcNetworkError) => A;
}

/** Match on RPC error types using Effect's Match */
export const matchError = <A>(
  error: RpcEffectError,
  handlers: ErrorHandlers<A>
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
// Public Error Types (for rpc-core consumption)
// =============================================================================

/**
 * Public RPC error structure - the simple, serializable format.
 * This is what gets thrown to consumers and matches the Rust backend format.
 */
export interface PublicRpcError {
  readonly code: string;
  readonly message: string;
  readonly details?: unknown;
  readonly cause?: string;
}

/**
 * Standard RPC error codes matching the Rust backend.
 */
export type RpcErrorCode =
  // Client errors (4xx equivalent)
  | "BAD_REQUEST"
  | "UNAUTHORIZED"
  | "FORBIDDEN"
  | "NOT_FOUND"
  | "VALIDATION_ERROR"
  | "CONFLICT"
  | "PAYLOAD_TOO_LARGE"
  | "RATE_LIMITED"
  // Server errors (5xx equivalent)
  | "INTERNAL_ERROR"
  | "NOT_IMPLEMENTED"
  | "SERVICE_UNAVAILABLE"
  // RPC-specific errors
  | "PROCEDURE_NOT_FOUND"
  | "SUBSCRIPTION_ERROR"
  | "MIDDLEWARE_ERROR"
  | "SERIALIZATION_ERROR"
  // Client-only codes
  | "TIMEOUT"
  | "CANCELLED"
  | "UNKNOWN";

// =============================================================================
// Effect to Public Error Conversion
// =============================================================================

/**
 * Convert Effect error to public RpcError format.
 * Single source of truth for error conversion.
 */
export const toPublicError = (error: RpcEffectError): PublicRpcError =>
  matchError(error, {
    onCallError: (e) => ({
      code: e.code,
      message: e.message,
      details: e.details,
      cause: e.cause,
    }),
    onTimeoutError: (e) => ({
      code: "TIMEOUT" as const,
      message: `Request to '${e.path}' timed out after ${e.timeoutMs}ms`,
      details: { timeoutMs: e.timeoutMs, path: e.path },
      cause: undefined,
    }),
    onCancelledError: (e) => ({
      code: "CANCELLED" as const,
      message: e.reason ?? `Request to '${e.path}' was cancelled`,
      details: { path: e.path },
      cause: undefined,
    }),
    onValidationError: (e) => ({
      code: "VALIDATION_ERROR" as const,
      message:
        e.issues.length > 0
          ? e.issues[0].message
          : `Validation failed for '${e.path}'`,
      details: { issues: e.issues },
      cause: undefined,
    }),
    onNetworkError: (e) => ({
      code: "INTERNAL_ERROR" as const,
      message: `Network error calling '${e.path}'`,
      details: { originalError: String(e.originalError) },
      cause: undefined,
    }),
  });

/**
 * Convert public RpcError to Effect error.
 */
export const fromPublicError = (
  error: PublicRpcError,
  path: string
): RpcEffectError => {
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
        issues:
          (
            error.details as {
              issues?: ValidationIssue[];
            }
          )?.issues ?? [],
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

// =============================================================================
// Public Error Type Guards
// =============================================================================

/**
 * Check if error is a public RPC error.
 */
export const isPublicRpcError = (error: unknown): error is PublicRpcError =>
  typeof error === "object" &&
  error !== null &&
  "code" in error &&
  "message" in error &&
  typeof (error as PublicRpcError).code === "string" &&
  typeof (error as PublicRpcError).message === "string";

/**
 * Check if error has a specific code.
 */
export const hasPublicErrorCode = (
  error: unknown,
  code: RpcErrorCode | string
): boolean => isPublicRpcError(error) && error.code === code;

/**
 * Create a typed public RPC error.
 */
export const createPublicError = (
  code: RpcErrorCode | string,
  message: string,
  details?: unknown
): PublicRpcError => ({ code, message, details });

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
const isRateLimitDetails = (details: unknown): details is RateLimitDetails =>
  typeof details === "object" &&
  details !== null &&
  "retry_after_ms" in details &&
  typeof (details as RateLimitDetails).retry_after_ms === "number";

/**
 * Check if an RPC error is a rate limit error.
 * Works with both Effect errors and public errors.
 *
 * @example
 * ```typescript
 * try {
 *   await client.api.call();
 * } catch (error) {
 *   if (isRateLimitError(error)) {
 *     const retryAfter = getRateLimitRetryAfter(error);
 *     if (retryAfter) {
 *       await sleep(retryAfter);
 *       // retry...
 *     }
 *   }
 * }
 * ```
 */
export const isRateLimitError = (error: unknown): error is PublicRpcError =>
  isPublicRpcError(error) && error.code === "RATE_LIMITED";

/**
 * Extract the retry-after time in milliseconds from a rate limit error.
 * Returns undefined if the error is not a rate limit error or doesn't have retry info.
 *
 * @example
 * ```typescript
 * const retryAfter = getRateLimitRetryAfter(error);
 * if (retryAfter !== undefined) {
 *   console.log(`Rate limited. Retry after ${retryAfter}ms`);
 *   await new Promise(r => setTimeout(r, retryAfter));
 * }
 * ```
 */
export const getRateLimitRetryAfter = (
  error: PublicRpcError
): number | undefined => {
  if (error.code !== "RATE_LIMITED") {
    return undefined;
  }
  if (!isRateLimitDetails(error.details)) {
    return undefined;
  }
  return error.details.retry_after_ms;
};

// =============================================================================
// Error Parsing Utilities
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
export const makeCallErrorFromShape = (shape: RpcErrorShape): RpcCallError =>
  makeCallError(shape.code, shape.message, shape.details, shape.cause);

/** Symbol for Effect's FiberFailure cause */
const FiberFailureCauseId = Symbol.for("effect/Runtime/FiberFailure/Cause");

/**
 * Extract failures from Effect's Cause structure.
 */
const extractFailuresFromCause = (cause: unknown): unknown[] => {
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
};

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

/**
 * Parse error to public format directly.
 * Convenience function that combines parsing and conversion.
 */
export const parseToPublicError = (
  error: unknown,
  path: string,
  timeoutMs?: number
): PublicRpcError => toPublicError(parseEffectError(error, path, timeoutMs));
