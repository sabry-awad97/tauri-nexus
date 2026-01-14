// =============================================================================
// Core Type Definitions
// =============================================================================
// Pure type definitions with no implementations or dependencies.

/** Procedure types */
export type ProcedureType = "query" | "mutation" | "subscription";

/** Event with optional metadata for streaming */
export interface Event<T> {
  readonly data: T;
  readonly id?: string;
  readonly retry?: number;
}

/** Async event iterator for subscriptions */
export interface EventIterator<T> extends AsyncIterable<T> {
  return(): Promise<void>;
  [Symbol.asyncIterator](): AsyncIterator<T>;
}

/** Validation issue structure */
export interface ValidationIssue {
  readonly path: readonly (string | number)[];
  readonly message: string;
  readonly code: string;
}

// =============================================================================
// Service Interfaces
// =============================================================================

/** Configuration for RPC calls */
export interface RpcConfig {
  readonly defaultTimeout?: number;
  readonly subscriptionPaths: ReadonlySet<string>;
  readonly validateInput?: boolean;
  readonly validateOutput?: boolean;
}

/** Transport layer abstraction for making actual RPC calls */
export interface RpcTransport {
  readonly call: <T>(path: string, input: unknown) => Promise<T>;
  readonly callBatch: <T>(
    requests: readonly { id: string; path: string; input: unknown }[],
  ) => Promise<{
    results: readonly {
      id: string;
      data?: T;
      error?: { code: string; message: string; details?: unknown };
    }[];
  }>;
  readonly subscribe: <T>(
    path: string,
    input: unknown,
    options?: SubscribeTransportOptions,
  ) => Promise<EventIterator<T>>;
  readonly parseError?: (
    error: unknown,
    path: string,
    timeoutMs?: number,
  ) => import("./errors").RpcEffectError;
}

export interface SubscribeTransportOptions {
  readonly lastEventId?: string;
  readonly signal?: AbortSignal;
}

/** Interceptor chain for middleware-like functionality */
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

/** Logger interface for debugging and monitoring */
export interface RpcLogger {
  readonly debug: (message: string, data?: unknown) => void;
  readonly info: (message: string, data?: unknown) => void;
  readonly warn: (message: string, data?: unknown) => void;
  readonly error: (message: string, data?: unknown) => void;
}

// =============================================================================
// Request/Response Context Types
// =============================================================================

/** Internal request context with full type information */
export interface EffectRequestContext<TInput = unknown> {
  readonly path: string;
  readonly input: TInput;
  readonly type: ProcedureType;
  readonly meta: Record<string, unknown>;
  readonly signal?: AbortSignal;
  readonly timeout?: number;
}

/** Internal response context */
export interface EffectResponseContext<TOutput = unknown> {
  readonly data: TOutput;
  readonly meta: Record<string, unknown>;
  readonly durationMs: number;
}
