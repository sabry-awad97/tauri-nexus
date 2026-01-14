// =============================================================================
// Error Utilities
// =============================================================================
// Constructors, type guards, and pattern matching for Effect errors.

import { Effect, Match } from "effect";
import {
  RpcCallError,
  RpcTimeoutError,
  RpcCancelledError,
  RpcValidationError,
  RpcNetworkError,
  type RpcEffectError,
} from "./errors";
import type { ValidationIssue } from "./types";

// =============================================================================
// Error Constructors
// =============================================================================

export const createCallError = (
  code: string,
  message: string,
  details?: unknown,
  cause?: string,
): RpcCallError => new RpcCallError({ code, message, details, cause });

export const createTimeoutError = (
  path: string,
  timeoutMs: number,
): RpcTimeoutError => new RpcTimeoutError({ path, timeoutMs });

export const createCancelledError = (
  path: string,
  reason?: string,
): RpcCancelledError => new RpcCancelledError({ path, reason });

export const createValidationError = (
  path: string,
  issues: readonly ValidationIssue[],
): RpcValidationError => new RpcValidationError({ path, issues });

export const createNetworkError = (
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
  Effect.fail(createCallError(code, message, details));

export const failWithTimeout = (
  path: string,
  timeoutMs: number,
): Effect.Effect<never, RpcTimeoutError> =>
  Effect.fail(createTimeoutError(path, timeoutMs));

export const failWithValidation = (
  path: string,
  issues: readonly ValidationIssue[],
): Effect.Effect<never, RpcValidationError> =>
  Effect.fail(createValidationError(path, issues));

export const failWithNetwork = (
  path: string,
  originalError: unknown,
): Effect.Effect<never, RpcNetworkError> =>
  Effect.fail(createNetworkError(path, originalError));

export const failWithCancelled = (
  path: string,
  reason?: string,
): Effect.Effect<never, RpcCancelledError> =>
  Effect.fail(createCancelledError(path, reason));
