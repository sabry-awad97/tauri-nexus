// =============================================================================
// @tauri-nexus/rpc-core - Core Domain Types
// =============================================================================
// Pure TypeScript types. Error types imported from rpc-effect.

// =============================================================================
// Error Types - from rpc-effect (single source of truth)
// =============================================================================

export type { RpcError, RpcErrorCode } from "@tauri-nexus/rpc-effect";

// =============================================================================
// Procedure Types
// =============================================================================

/** Procedure types */
export type ProcedureType = "query" | "mutation" | "subscription";

/** Base procedure definition */
export interface ProcedureDef<
  TType extends ProcedureType = ProcedureType,
  TInput = unknown,
  TOutput = unknown,
> {
  readonly _type: TType;
  readonly _input: TInput;
  readonly _output: TOutput;
}

/** Query procedure - for reading data */
export interface QueryDef<
  TInput = void,
  TOutput = unknown,
> extends ProcedureDef<"query", TInput, TOutput> {}

/** Mutation procedure - for writing data */
export interface MutationDef<
  TInput = void,
  TOutput = unknown,
> extends ProcedureDef<"mutation", TInput, TOutput> {}

/** Subscription procedure - for streaming data */
export interface SubscriptionDef<
  TInput = void,
  TOutput = unknown,
> extends ProcedureDef<"subscription", TInput, TOutput> {}

// =============================================================================
// Contract Types
// =============================================================================

/** Base procedure definition for contract */
export type ProcedureDefinition = {
  type: ProcedureType;
  input: unknown;
  output: unknown;
};

/** Check if a type is a procedure definition */
export type IsProcedure<T> = T extends ProcedureDef ? true : false;

/** Check if a type is a router (nested object with procedures) */
export type IsRouter<T> = T extends object
  ? T extends ProcedureDef
    ? false
    : true
  : false;

// =============================================================================
// Event Types
// =============================================================================

/** Event with optional metadata for streaming */
export interface Event<T> {
  readonly data: T;
  readonly id?: string;
  readonly retry?: number;
}

/** Event metadata for SSE-style streaming */
export interface EventMeta {
  readonly id?: string;
  readonly retry?: number;
}

// =============================================================================
// Batch Request Types
// =============================================================================

import type { RpcError } from "@tauri-nexus/rpc-effect";

/**
 * A single request within a batch.
 */
export interface SingleRequest {
  readonly id: string;
  readonly path: string;
  readonly input: unknown;
}

/**
 * A batch of RPC requests.
 */
export interface BatchRequest {
  readonly requests: readonly SingleRequest[];
}

/**
 * Result of a single request within a batch.
 */
export interface BatchResult<T = unknown> {
  readonly id: string;
  readonly data?: T;
  readonly error?: RpcError;
}

/**
 * Response containing results for all requests in a batch.
 */
export interface BatchResponse<T = unknown> {
  readonly results: readonly BatchResult<T>[];
}

// =============================================================================
// Call Options
// =============================================================================

/** Options for query/mutation calls */
export interface CallOptions {
  readonly signal?: AbortSignal;
  readonly timeout?: number;
  readonly meta?: Record<string, unknown>;
}

/** Options for subscription calls */
export interface SubscriptionOptions extends CallOptions {
  readonly lastEventId?: string;
  readonly autoReconnect?: boolean;
  readonly reconnectDelay?: number;
  readonly maxReconnects?: number;
}

/** Options for batch calls */
export interface BatchCallOptions {
  readonly signal?: AbortSignal;
  readonly timeout?: number;
  readonly meta?: Record<string, unknown>;
}

// =============================================================================
// Subscribe Request
// =============================================================================

/** Subscribe request payload sent to backend */
export interface SubscribeRequest {
  readonly id?: string;
  readonly path: string;
  readonly input: unknown;
  readonly lastEventId?: string;
}

// =============================================================================
// Middleware Types
// =============================================================================

/** Request context passed through middleware */
export interface RequestContext {
  readonly path: string;
  input: unknown;
  readonly type: ProcedureType;
  meta?: Record<string, unknown>;
  readonly signal?: AbortSignal;
}

/** Response context from middleware */
export interface ResponseContext<T = unknown> {
  readonly data: T;
  readonly meta?: Record<string, unknown>;
}

/** Middleware function type */
export type Middleware = <T>(
  ctx: RequestContext,
  next: () => Promise<T>,
) => Promise<T>;

// =============================================================================
// Event Iterator Types
// =============================================================================

/** Async event iterator for subscriptions */
export interface EventIterator<T> extends AsyncIterable<T> {
  return(): Promise<void>;
  [Symbol.asyncIterator](): AsyncIterator<T>;
}

// =============================================================================
// Utility Types
// =============================================================================

/** Make all properties optional recursively */
export type DeepPartial<T> = T extends object
  ? { [P in keyof T]?: DeepPartial<T[P]> }
  : T;

/** Prettify type for better IntelliSense display */
export type Prettify<T> = {
  [K in keyof T]: T[K];
} & {};
