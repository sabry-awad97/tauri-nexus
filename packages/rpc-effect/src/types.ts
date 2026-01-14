// =============================================================================
// @tauri-nexus/rpc-effect - Effect-Based Type Definitions
// =============================================================================
// Core Effect types for the RPC system providing type-safe error handling
// and dependency injection through Effect's service pattern.
//
// Uses Effect.Service pattern (Effect 3.9+) for combined Tag + Layer definitions.

import { Context, Data, Layer } from "effect";

// =============================================================================
// Core Types (Effect-only, no external dependencies)
// =============================================================================

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

// =============================================================================
// Effect Error Types
// =============================================================================

/**
 * Tagged union for RPC errors using Effect's Data.TaggedError.
 * Provides discriminated union support for pattern matching.
 */
export class RpcCallError extends Data.TaggedError("RpcCallError")<{
  readonly code: string;
  readonly message: string;
  readonly details?: unknown;
  readonly cause?: string;
}> {}

export class RpcTimeoutError extends Data.TaggedError("RpcTimeoutError")<{
  readonly timeoutMs: number;
  readonly path: string;
}> {}

export class RpcCancelledError extends Data.TaggedError("RpcCancelledError")<{
  readonly path: string;
  readonly reason?: string;
}> {}

export class RpcValidationError extends Data.TaggedError("RpcValidationError")<{
  readonly path: string;
  readonly issues: readonly ValidationIssue[];
}> {}

export class RpcNetworkError extends Data.TaggedError("RpcNetworkError")<{
  readonly path: string;
  readonly originalError: unknown;
}> {}

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
// Service Interfaces
// =============================================================================

/**
 * Configuration for RPC calls.
 */
export interface RpcConfig {
  readonly defaultTimeout?: number;
  readonly subscriptionPaths: ReadonlySet<string>;
  readonly validateInput?: boolean;
  readonly validateOutput?: boolean;
}

/**
 * Transport layer abstraction for making actual RPC calls.
 */
export interface RpcTransport {
  readonly call: <T>(path: string, input: unknown) => Promise<T>;
  readonly callBatch: <T>(
    requests: readonly { id: string; path: string; input: unknown }[]
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
    options?: SubscribeTransportOptions
  ) => Promise<EventIterator<T>>;
  /**
   * Convert transport errors to Effect RPC errors.
   * If not provided, a default converter will be used.
   */
  readonly parseError?: (
    error: unknown,
    path: string,
    timeoutMs?: number
  ) => RpcEffectError;
}

export interface SubscribeTransportOptions {
  readonly lastEventId?: string;
  readonly signal?: AbortSignal;
}

/**
 * Interceptor chain for middleware-like functionality.
 */
export interface RpcInterceptorChain {
  readonly interceptors: readonly RpcInterceptor[];
}

export interface RpcInterceptor {
  readonly name: string;
  readonly intercept: <T>(
    ctx: InterceptorContext,
    next: () => Promise<T>
  ) => Promise<T>;
}

export interface InterceptorContext {
  readonly path: string;
  readonly input: unknown;
  readonly type: ProcedureType;
  readonly meta: Record<string, unknown>;
  readonly signal?: AbortSignal;
}

/**
 * Logger interface for debugging and monitoring.
 */
export interface RpcLogger {
  readonly debug: (message: string, data?: unknown) => void;
  readonly info: (message: string, data?: unknown) => void;
  readonly warn: (message: string, data?: unknown) => void;
  readonly error: (message: string, data?: unknown) => void;
}

// =============================================================================
// Default Implementations
// =============================================================================

const defaultRpcConfig: RpcConfig = {
  defaultTimeout: undefined,
  subscriptionPaths: new Set(),
  validateInput: false,
  validateOutput: false,
};

const defaultInterceptorChain: RpcInterceptorChain = {
  interceptors: [],
};

const noopLogger: RpcLogger = {
  debug: () => {},
  info: () => {},
  warn: () => {},
  error: () => {},
};

/** Console logger implementation */
export const consoleLogger: RpcLogger = {
  debug: (msg, data) => console.debug(`[RPC] ${msg}`, data ?? ""),
  info: (msg, data) => console.info(`[RPC] ${msg}`, data ?? ""),
  warn: (msg, data) => console.warn(`[RPC] ${msg}`, data ?? ""),
  error: (msg, data) => console.error(`[RPC] ${msg}`, data ?? ""),
};

// =============================================================================
// Effect Services (Context.Tag pattern with static layer helpers)
// =============================================================================

/**
 * Configuration service for RPC calls.
 *
 * @example
 * ```ts
 * // Use default config
 * Effect.provide(program, RpcConfigService.Default)
 *
 * // Custom config
 * Effect.provide(program, RpcConfigService.layer({ defaultTimeout: 5000 }))
 * ```
 */
export class RpcConfigService extends Context.Tag("RpcConfigService")<
  RpcConfigService,
  RpcConfig
>() {
  /** Default layer with default config */
  static Default = Layer.succeed(RpcConfigService, defaultRpcConfig);

  /** Create a custom config by merging with defaults */
  static config(config: Partial<RpcConfig> = {}): RpcConfig {
    return {
      ...defaultRpcConfig,
      ...config,
      subscriptionPaths: new Set([
        ...defaultRpcConfig.subscriptionPaths,
        ...(config.subscriptionPaths ?? []),
      ]),
    };
  }

  /** Create a layer with custom config */
  static layer(config: Partial<RpcConfig> = {}) {
    return Layer.succeed(RpcConfigService, RpcConfigService.config(config));
  }
}

/**
 * Transport service - no default implementation.
 * Must be provided by the user (e.g., TauriTransport, FetchTransport).
 *
 * @example
 * ```ts
 * Effect.provide(program, RpcTransportService.layer(myTransport))
 * ```
 */
export class RpcTransportService extends Context.Tag("RpcTransportService")<
  RpcTransportService,
  RpcTransport
>() {
  /** Create a layer with the given transport */
  static layer(transport: RpcTransport) {
    return Layer.succeed(RpcTransportService, transport);
  }
}

/**
 * Interceptor chain service for middleware-like functionality.
 */
export class RpcInterceptorService extends Context.Tag("RpcInterceptorService")<
  RpcInterceptorService,
  RpcInterceptorChain
>() {
  /** Default layer with empty interceptor chain */
  static Default = Layer.succeed(
    RpcInterceptorService,
    defaultInterceptorChain
  );

  /** Create a layer with custom interceptors */
  static layer(chain: RpcInterceptorChain) {
    return Layer.succeed(RpcInterceptorService, chain);
  }

  /** Create a layer with interceptor array */
  static withInterceptors(interceptors: readonly RpcInterceptor[]) {
    return Layer.succeed(RpcInterceptorService, { interceptors });
  }
}

/**
 * Logger service for debugging and monitoring.
 */
export class RpcLoggerService extends Context.Tag("RpcLoggerService")<
  RpcLoggerService,
  RpcLogger
>() {
  /** Default layer with noop logger */
  static Default = Layer.succeed(RpcLoggerService, noopLogger);

  /** Create a layer with custom logger */
  static layer(logger: RpcLogger) {
    return Layer.succeed(RpcLoggerService, logger);
  }

  /** Layer with console logging enabled */
  static Console = Layer.succeed(RpcLoggerService, consoleLogger);
}

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
