// =============================================================================
// @tauri-nexus/rpc-core - Effect-Based Type Definitions
// =============================================================================
// Internal Effect types for the RPC system. These provide type-safe error
// handling and dependency injection through Effect's service pattern.

import { Context, Data } from "effect";
import type { ProcedureType, RpcError as PublicRpcError } from "../core/types";

// =============================================================================
// Effect Error Types
// =============================================================================

/**
 * Tagged union for RPC errors using Effect's Data.TaggedError.
 * This provides discriminated union support for pattern matching.
 */
export class RpcCallError extends Data.TaggedError("RpcCallError")<{
  readonly code: string;
  readonly message: string;
  readonly details?: unknown;
  readonly cause?: string;
}> {
  /** Convert to public RpcError format */
  toPublic(): PublicRpcError {
    return {
      code: this.code,
      message: this.message,
      details: this.details,
      cause: this.cause,
    };
  }
}

export class RpcTimeoutError extends Data.TaggedError("RpcTimeoutError")<{
  readonly timeoutMs: number;
  readonly path: string;
}> {
  toPublic(): PublicRpcError {
    return {
      code: "TIMEOUT",
      message: `Request to '${this.path}' timed out after ${this.timeoutMs}ms`,
      details: { timeoutMs: this.timeoutMs, path: this.path },
    };
  }
}

export class RpcCancelledError extends Data.TaggedError("RpcCancelledError")<{
  readonly path: string;
  readonly reason?: string;
}> {
  toPublic(): PublicRpcError {
    return {
      code: "CANCELLED",
      message: this.reason ?? `Request to '${this.path}' was cancelled`,
      details: { path: this.path },
    };
  }
}

export class RpcValidationError extends Data.TaggedError("RpcValidationError")<{
  readonly path: string;
  readonly issues: readonly ValidationIssue[];
}> {
  toPublic(): PublicRpcError {
    return {
      code: "VALIDATION_ERROR",
      message: `Validation failed for '${this.path}'`,
      details: { issues: this.issues },
    };
  }
}

export class RpcNetworkError extends Data.TaggedError("RpcNetworkError")<{
  readonly path: string;
  readonly originalError: unknown;
}> {
  toPublic(): PublicRpcError {
    return {
      code: "INTERNAL_ERROR",
      message: `Network error calling '${this.path}'`,
      details: { originalError: String(this.originalError) },
    };
  }
}

/** Validation issue structure */
export interface ValidationIssue {
  readonly path: readonly (string | number)[];
  readonly message: string;
  readonly code: string;
}

/** Union of all RPC error types */
export type RpcEffectError =
  | RpcCallError
  | RpcTimeoutError
  | RpcCancelledError
  | RpcValidationError
  | RpcNetworkError;

// =============================================================================
// Effect Service Tags (Dependency Injection)
// =============================================================================

/**
 * Configuration service for RPC calls.
 */
export interface RpcConfig {
  readonly defaultTimeout?: number;
  readonly subscriptionPaths: ReadonlySet<string>;
  readonly validateInput?: boolean;
  readonly validateOutput?: boolean;
}

export class RpcConfigService extends Context.Tag("RpcConfigService")<
  RpcConfigService,
  RpcConfig
>() {}

/**
 * Transport layer abstraction for making actual RPC calls.
 * This allows swapping implementations (Tauri, HTTP, mock, etc.)
 */
export interface RpcTransport {
  readonly call: <T>(path: string, input: unknown) => Promise<T>;
  readonly subscribe: <T>(
    path: string,
    input: unknown,
    options?: SubscribeTransportOptions,
  ) => Promise<AsyncIterable<T>>;
}

export interface SubscribeTransportOptions {
  readonly lastEventId?: string;
  readonly signal?: AbortSignal;
}

export class RpcTransportService extends Context.Tag("RpcTransportService")<
  RpcTransportService,
  RpcTransport
>() {}

/**
 * Interceptor chain service for middleware-like functionality.
 */
export interface RpcInterceptorChain {
  readonly interceptors: readonly RpcInterceptor[];
}

export interface RpcInterceptor {
  readonly name: string;
  readonly intercept: <T>(
    ctx: InterceptorContext,
    next: () => Promise<T>,
  ) => Promise<T>;
}

export interface InterceptorContext {
  readonly path: string;
  readonly input: unknown;
  readonly type: ProcedureType;
  readonly meta: Record<string, unknown>;
  readonly signal?: AbortSignal;
}

export class RpcInterceptorService extends Context.Tag("RpcInterceptorService")<
  RpcInterceptorService,
  RpcInterceptorChain
>() {}

/**
 * Logger service for debugging and monitoring.
 */
export interface RpcLogger {
  readonly debug: (message: string, data?: unknown) => void;
  readonly info: (message: string, data?: unknown) => void;
  readonly warn: (message: string, data?: unknown) => void;
  readonly error: (message: string, data?: unknown) => void;
}

export class RpcLoggerService extends Context.Tag("RpcLoggerService")<
  RpcLoggerService,
  RpcLogger
>() {}

// =============================================================================
// Request/Response Context Types
// =============================================================================

/**
 * Internal request context with full type information.
 */
export interface EffectRequestContext<TInput = unknown> {
  readonly path: string;
  readonly input: TInput;
  readonly type: ProcedureType;
  readonly meta: Record<string, unknown>;
  readonly signal?: AbortSignal;
  readonly timeout?: number;
}

/**
 * Internal response context.
 */
export interface EffectResponseContext<TOutput = unknown> {
  readonly data: TOutput;
  readonly meta: Record<string, unknown>;
  readonly durationMs: number;
}
