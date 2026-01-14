// =============================================================================
// Effect Error Classes
// =============================================================================
// Tagged union error types using Effect's Data.TaggedError.

import { Data } from "effect";
import type { ValidationIssue } from "./types";

/** RPC call error with code and details */
export class RpcCallError extends Data.TaggedError("RpcCallError")<{
  readonly code: string;
  readonly message: string;
  readonly details?: unknown;
  readonly cause?: string;
}> {}

/** Timeout error for RPC calls */
export class RpcTimeoutError extends Data.TaggedError("RpcTimeoutError")<{
  readonly timeoutMs: number;
  readonly path: string;
}> {}

/** Cancelled error for aborted RPC calls */
export class RpcCancelledError extends Data.TaggedError("RpcCancelledError")<{
  readonly path: string;
  readonly reason?: string;
}> {}

/** Validation error for invalid input/output */
export class RpcValidationError extends Data.TaggedError("RpcValidationError")<{
  readonly path: string;
  readonly issues: readonly ValidationIssue[];
}> {}

/** Network error for transport failures */
export class RpcNetworkError extends Data.TaggedError("RpcNetworkError")<{
  readonly path: string;
  readonly originalError: unknown;
}> {}

/** Union of all RPC error types */
export type RpcEffectError =
  | RpcCallError
  | RpcTimeoutError
  | RpcCancelledError
  | RpcValidationError
  | RpcNetworkError;
