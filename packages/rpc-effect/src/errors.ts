// =============================================================================
// @tauri-nexus/rpc-effect - Effect Error Utilities
// =============================================================================
// Pure Effect error constructors, type guards, pattern matching, and combinators.
// No parsing logic - that belongs in the transport layer.

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
