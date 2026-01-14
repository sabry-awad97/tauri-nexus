// =============================================================================
// @tauri-nexus/rpc-effect - Effect Error Utilities
// =============================================================================
// Pure Effect error constructors, type guards, pattern matching, and combinators.

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
// Type Guards
// =============================================================================

const RPC_ERROR_TAGS = [
  "RpcCallError",
  "RpcTimeoutError",
  "RpcCancelledError",
  "RpcValidationError",
  "RpcNetworkError",
] as const;

type RpcErrorTag = (typeof RPC_ERROR_TAGS)[number];

export const isEffectRpcError = (error: unknown): error is RpcEffectError => {
  if (typeof error !== "object" || error === null) return false;
  const tag = (error as { _tag?: string })._tag;
  return RPC_ERROR_TAGS.includes(tag as RpcErrorTag);
};

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

// =============================================================================
// Error Code Utilities
// =============================================================================

export type VirtualErrorCode =
  | "TIMEOUT"
  | "CANCELLED"
  | "VALIDATION_ERROR"
  | "NETWORK_ERROR";

export const getErrorCode = (error: RpcEffectError): string =>
  matchError(error, {
    onCallError: (e) => e.code,
    onTimeoutError: () => "TIMEOUT",
    onCancelledError: () => "CANCELLED",
    onValidationError: () => "VALIDATION_ERROR",
    onNetworkError: () => "NETWORK_ERROR",
  });

export const hasCode = <C extends string>(
  error: RpcEffectError,
  code: C,
): boolean => getErrorCode(error) === code;

export const hasAnyCode = (
  error: RpcEffectError,
  codes: readonly string[],
): boolean => codes.includes(getErrorCode(error));

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

export interface ErrorHandlers<A> {
  readonly onCallError: (e: RpcCallError) => A;
  readonly onTimeoutError: (e: RpcTimeoutError) => A;
  readonly onCancelledError: (e: RpcCancelledError) => A;
  readonly onValidationError: (e: RpcValidationError) => A;
  readonly onNetworkError: (e: RpcNetworkError) => A;
}

export const matchError = <A>(
  error: RpcEffectError,
  handlers: ErrorHandlers<A>,
): A =>
  Match.value(error).pipe(
    Match.tag("RpcCallError", handlers.onCallError),
    Match.tag("RpcTimeoutError", handlers.onTimeoutError),
    Match.tag("RpcCancelledError", handlers.onCancelledError),
    Match.tag("RpcValidationError", handlers.onValidationError),
    Match.tag("RpcNetworkError", handlers.onNetworkError),
    Match.exhaustive,
  ) as A;

// =============================================================================
// Effect Combinators
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

export const failWithNetwork = (
  path: string,
  originalError: unknown,
): Effect.Effect<never, RpcNetworkError> =>
  Effect.fail(makeNetworkError(path, originalError));

export const failWithCancelled = (
  path: string,
  reason?: string,
): Effect.Effect<never, RpcCancelledError> =>
  Effect.fail(makeCancelledError(path, reason));

// =============================================================================
// Serializable RPC Error (for Promise API and transport)
// =============================================================================

/**
 * Plain serializable RPC error object.
 * Used by Promise-based APIs and for transport/serialization.
 * Effect users should use RpcEffectError types for pattern matching.
 */
export interface RpcError {
  readonly code: string;
  readonly message: string;
  readonly details?: unknown;
  readonly cause?: string;
}

export type RpcErrorCode =
  | "BAD_REQUEST"
  | "UNAUTHORIZED"
  | "FORBIDDEN"
  | "NOT_FOUND"
  | "VALIDATION_ERROR"
  | "CONFLICT"
  | "PAYLOAD_TOO_LARGE"
  | "RATE_LIMITED"
  | "INTERNAL_ERROR"
  | "NOT_IMPLEMENTED"
  | "SERVICE_UNAVAILABLE"
  | "PROCEDURE_NOT_FOUND"
  | "SUBSCRIPTION_ERROR"
  | "MIDDLEWARE_ERROR"
  | "SERIALIZATION_ERROR"
  | "TIMEOUT"
  | "CANCELLED"
  | "UNKNOWN";

// =============================================================================
// Conversion (Effect Error <-> Serializable Error)
// =============================================================================

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

// =============================================================================
// RpcError Utilities
// =============================================================================

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
export const hasErrorCode = (
  error: unknown,
  code: RpcErrorCode | string,
): boolean => isRpcError(error) && error.code === code;

/**
 * Create a serializable RpcError.
 */
export const createRpcError = (
  code: RpcErrorCode | string,
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

// =============================================================================
// Error Parsing
// =============================================================================

export interface RpcErrorShape {
  readonly code: string;
  readonly message: string;
  readonly details?: unknown;
  readonly cause?: string;
}

export const isRpcErrorShape = (value: unknown): value is RpcErrorShape =>
  typeof value === "object" &&
  value !== null &&
  "code" in value &&
  "message" in value &&
  typeof (value as RpcErrorShape).code === "string" &&
  typeof (value as RpcErrorShape).message === "string";

export const parseJsonError = (str: string): RpcErrorShape | null => {
  try {
    const parsed = JSON.parse(str);
    return isRpcErrorShape(parsed) ? parsed : null;
  } catch {
    return null;
  }
};

export const makeCallErrorFromShape = (shape: RpcErrorShape): RpcCallError =>
  makeCallError(shape.code, shape.message, shape.details, shape.cause);

const FiberFailureCauseId = Symbol.for("effect/Runtime/FiberFailure/Cause");

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

export interface ErrorParserOptions {
  readonly parseJson?: boolean;
  readonly extractFiberFailure?: boolean;
  readonly unwrapNested?: boolean;
}

export const parseToEffectError = (
  error: unknown,
  path: string,
  timeoutMs?: number,
  options: ErrorParserOptions = { parseJson: true },
): RpcEffectError => {
  if (isEffectRpcError(error)) return error;

  if (error instanceof Error && error.name === "AbortError") {
    return timeoutMs !== undefined
      ? makeTimeoutError(path, timeoutMs)
      : makeCancelledError(path);
  }

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
        options,
      );
    }
  }

  if (isRpcErrorShape(error)) {
    return makeCallErrorFromShape(error);
  }

  if (options.parseJson && typeof error === "string") {
    const parsed = parseJsonError(error);
    return parsed
      ? makeCallErrorFromShape(parsed)
      : makeCallError("UNKNOWN", error);
  }

  if (typeof error === "string") {
    return makeCallError("UNKNOWN", error);
  }

  if (error instanceof Error) {
    if (options.parseJson) {
      const parsed = parseJsonError(error.message);
      if (parsed) return makeCallErrorFromShape(parsed);
    }
    return makeCallError("UNKNOWN", error.message, undefined, error.stack);
  }

  return makeCallError("UNKNOWN", String(error));
};

export const fromTransportError = (
  error: unknown,
  path: string,
  timeoutMs?: number,
): RpcEffectError =>
  parseToEffectError(error, path, timeoutMs, { parseJson: true });

export const parseEffectError = (
  error: unknown,
  path: string,
  timeoutMs?: number,
): RpcEffectError =>
  parseToEffectError(error, path, timeoutMs, {
    parseJson: true,
    extractFiberFailure: true,
    unwrapNested: true,
  });

/**
 * Parse any error to serializable RpcError.
 */
export const parseError = (
  error: unknown,
  path: string,
  timeoutMs?: number,
): RpcError => toRpcError(parseEffectError(error, path, timeoutMs));
