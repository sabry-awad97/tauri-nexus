// =============================================================================
// TauriLink - oRPC-style Link Abstraction for Tauri RPC
// =============================================================================
// Provides a flexible link pattern for configuring RPC client behavior,
// similar to oRPC's RPCLink but adapted for Tauri's invoke system.

import { invoke } from "@tauri-apps/api/core";
import type { RpcError, RpcErrorCode, ProcedureType } from "./types";
import { createEventIterator } from "./event-iterator";

// =============================================================================
// Types
// =============================================================================

/** Request context passed through interceptors */
export interface LinkRequestContext<TClientContext = unknown> {
  /** Procedure path (e.g., "user.get") */
  path: string;
  /** Input data */
  input: unknown;
  /** Procedure type */
  type: ProcedureType;
  /** Client context provided at call time */
  context: TClientContext;
  /** Abort signal for cancellation */
  signal?: AbortSignal;
  /** Custom metadata */
  meta: Record<string, unknown>;
}

/** Response from a link call */
export interface LinkResponse<TOutput = unknown> {
  /** Response data */
  data: TOutput;
  /** Response metadata */
  meta?: Record<string, unknown>;
}

/** Interceptor function type */
export type LinkInterceptor<TClientContext = unknown> = <T>(
  ctx: LinkRequestContext<TClientContext>,
  next: () => Promise<T>
) => Promise<T>;

/** Error handler function */
export type ErrorHandler<TClientContext = unknown> = (
  error: RpcError,
  ctx: LinkRequestContext<TClientContext>
) => void | Promise<void>;

/** Request handler function */
export type RequestHandler<TClientContext = unknown> = (
  ctx: LinkRequestContext<TClientContext>
) => void | Promise<void>;

/** Response handler function */
export type ResponseHandler<TClientContext = unknown> = <T>(
  data: T,
  ctx: LinkRequestContext<TClientContext>
) => void | Promise<void>;

// =============================================================================
// Link Configuration
// =============================================================================

export interface TauriLinkConfig<TClientContext = unknown> {
  /** Interceptors - executed in order, wrapping the request */
  interceptors?: LinkInterceptor<TClientContext>[];
  /** Called before each request */
  onRequest?: RequestHandler<TClientContext>;
  /** Called after successful response */
  onResponse?: ResponseHandler<TClientContext>;
  /** Called on error */
  onError?: ErrorHandler<TClientContext>;
  /** Global request timeout in milliseconds */
  timeout?: number;
  /** Paths that are subscriptions */
  subscriptionPaths?: string[];
}

// =============================================================================
// Helper Functions
// =============================================================================

/** Create an RPC error */
function createRpcError(
  code: RpcErrorCode | string,
  message: string,
  details?: unknown
): RpcError {
  return { code, message, details };
}

/** Check if error is an RPC error */
function isRpcError(error: unknown): error is RpcError {
  return (
    typeof error === "object" &&
    error !== null &&
    "code" in error &&
    "message" in error &&
    typeof (error as RpcError).code === "string" &&
    typeof (error as RpcError).message === "string"
  );
}

/** Parse error from backend response */
function parseError(error: unknown, timeoutMs?: number): RpcError {
  if (error instanceof Error) {
    if (error.name === "AbortError") {
      if (timeoutMs !== undefined) {
        return createRpcError("TIMEOUT", `Request timed out after ${timeoutMs}ms`, { timeoutMs });
      }
      return createRpcError("CANCELLED", "Request was cancelled");
    }
    return createRpcError("UNKNOWN", error.message);
  }

  if (typeof error === "string") {
    try {
      const parsed = JSON.parse(error);
      if (isRpcError(parsed)) return parsed;
      return createRpcError("UNKNOWN", error);
    } catch {
      return createRpcError("UNKNOWN", error);
    }
  }

  if (isRpcError(error)) return error;
  return createRpcError("UNKNOWN", String(error));
}

/** Validate procedure path */
function validatePath(path: string): void {
  if (!path) {
    throw createRpcError("VALIDATION_ERROR", "Procedure path cannot be empty");
  }
  if (path.startsWith(".") || path.endsWith(".")) {
    throw createRpcError("VALIDATION_ERROR", "Procedure path cannot start or end with a dot");
  }
  if (path.includes("..")) {
    throw createRpcError("VALIDATION_ERROR", "Procedure path cannot contain consecutive dots");
  }
  for (const ch of path) {
    if (!/[a-zA-Z0-9_.]/.test(ch)) {
      throw createRpcError("VALIDATION_ERROR", `Procedure path contains invalid character: '${ch}'`);
    }
  }
}

// =============================================================================
// TauriLink Class
// =============================================================================

/**
 * TauriLink - A configurable link for Tauri RPC calls.
 * 
 * Similar to oRPC's RPCLink, but adapted for Tauri's invoke system.
 * Supports client context, interceptors, and lifecycle hooks.
 * 
 * @example
 * ```typescript
 * interface ClientContext {
 *   userId?: string;
 *   token?: string;
 * }
 * 
 * const link = new TauriLink<ClientContext>({
 *   interceptors: [
 *     // Logging interceptor
 *     async (ctx, next) => {
 *       console.log(`[RPC] ${ctx.path}`, ctx.input);
 *       const result = await next();
 *       console.log(`[RPC] ${ctx.path} done`);
 *       return result;
 *     },
 *     // Auth interceptor
 *     async (ctx, next) => {
 *       if (ctx.context.token) {
 *         ctx.meta.authorization = `Bearer ${ctx.context.token}`;
 *       }
 *       return next();
 *     },
 *   ],
 *   onError: (error, ctx) => {
 *     console.error(`Error in ${ctx.path}:`, error);
 *   },
 * });
 * 
 * const client = createClientFromLink<AppContract, ClientContext>(link);
 * 
 * // Call with context
 * const user = await client.user.get({ id: 1 }, { context: { token: 'abc' } });
 * ```
 */
export class TauriLink<TClientContext = unknown> {
  private config: TauriLinkConfig<TClientContext>;

  constructor(config: TauriLinkConfig<TClientContext> = {}) {
    this.config = config;
  }

  /** Execute interceptor chain */
  private async executeInterceptors<T>(
    ctx: LinkRequestContext<TClientContext>,
    fn: () => Promise<T>
  ): Promise<T> {
    const interceptors = this.config.interceptors ?? [];
    
    let next = fn;
    for (let i = interceptors.length - 1; i >= 0; i--) {
      const interceptor = interceptors[i];
      const currentNext = next;
      next = () => interceptor(ctx, currentNext);
    }

    return next();
  }

  /** Make an RPC call */
  async call<T>(
    path: string,
    input: unknown,
    options: LinkCallOptions<TClientContext> = {}
  ): Promise<T> {
    validatePath(path);

    const ctx: LinkRequestContext<TClientContext> = {
      path,
      input,
      type: "query",
      context: options.context ?? ({} as TClientContext),
      signal: options.signal,
      meta: options.meta ?? {},
    };

    const timeoutMs = options.timeout ?? this.config.timeout;

    try {
      await this.config.onRequest?.(ctx);

      const result = await this.executeInterceptors(ctx, async () => {
        if (timeoutMs) {
          const controller = new AbortController();
          const timeoutId = setTimeout(() => controller.abort(), timeoutMs);

          try {
            const result = await invoke<T>("plugin:rpc|rpc_call", { path, input });
            clearTimeout(timeoutId);
            return result;
          } catch (error) {
            clearTimeout(timeoutId);
            throw error;
          }
        }

        return invoke<T>("plugin:rpc|rpc_call", { path, input });
      });

      await this.config.onResponse?.(result, ctx);
      return result;
    } catch (error) {
      const rpcError = parseError(error, timeoutMs);
      await this.config.onError?.(rpcError, ctx);
      throw rpcError;
    }
  }

  /** Subscribe to a streaming procedure */
  async subscribe<T>(
    path: string,
    input: unknown,
    options: LinkSubscribeOptions<TClientContext> = {}
  ): Promise<AsyncIterable<T>> {
    validatePath(path);

    const ctx: LinkRequestContext<TClientContext> = {
      path,
      input,
      type: "subscription",
      context: options.context ?? ({} as TClientContext),
      signal: options.signal,
      meta: options.meta ?? {},
    };

    try {
      await this.config.onRequest?.(ctx);
      return await createEventIterator<T>(path, input, options);
    } catch (error) {
      const rpcError = parseError(error);
      await this.config.onError?.(rpcError, ctx);
      throw rpcError;
    }
  }

  /** Check if path is a subscription */
  isSubscription(path: string): boolean {
    return this.config.subscriptionPaths?.includes(path) ?? false;
  }

  /** Get configuration */
  getConfig(): TauriLinkConfig<TClientContext> {
    return this.config;
  }
}

// =============================================================================
// Call Options
// =============================================================================

export interface LinkCallOptions<TClientContext = unknown> {
  /** Client context for this call */
  context?: TClientContext;
  /** Abort signal */
  signal?: AbortSignal;
  /** Request timeout in milliseconds */
  timeout?: number;
  /** Custom metadata */
  meta?: Record<string, unknown>;
}

export interface LinkSubscribeOptions<TClientContext = unknown> extends LinkCallOptions<TClientContext> {
  /** Last event ID for resumption */
  lastEventId?: string;
  /** Auto-reconnect on disconnect */
  autoReconnect?: boolean;
  /** Reconnect delay in milliseconds */
  reconnectDelay?: number;
  /** Maximum reconnect attempts */
  maxReconnects?: number;
}

// =============================================================================
// Client Factory from Link
// =============================================================================

/** Symbol to identify the client proxy */
const CLIENT_PROXY = Symbol("rpc-link-client-proxy");

/** Client with context support */
export type LinkRouterClient<T, TClientContext = unknown> = {
  [K in keyof T]: T[K] extends { type: "subscription"; input: infer I; output: infer O }
    ? I extends void
      ? (options?: LinkSubscribeOptions<TClientContext>) => Promise<AsyncIterable<O>>
      : (input: I, options?: LinkSubscribeOptions<TClientContext>) => Promise<AsyncIterable<O>>
    : T[K] extends { type: "query" | "mutation"; input: infer I; output: infer O }
      ? I extends void
        ? (options?: LinkCallOptions<TClientContext>) => Promise<O>
        : (input: I, options?: LinkCallOptions<TClientContext>) => Promise<O>
      : T[K] extends object
        ? LinkRouterClient<T[K], TClientContext>
        : never;
};

/**
 * Create a type-safe RPC client from a TauriLink.
 * 
 * @example
 * ```typescript
 * const link = new TauriLink<{ token?: string }>({
 *   interceptors: [authInterceptor],
 * });
 * 
 * const client = createClientFromLink<AppContract, { token?: string }>(link);
 * 
 * // Call with context
 * const user = await client.user.get({ id: 1 }, { context: { token: 'abc' } });
 * ```
 */
export function createClientFromLink<T, TClientContext = unknown>(
  link: TauriLink<TClientContext>
): LinkRouterClient<T, TClientContext> {
  function createProxy(pathParts: string[]): unknown {
    const handler = function (
      inputOrOptions?: unknown,
      maybeOptions?: LinkCallOptions<TClientContext> | LinkSubscribeOptions<TClientContext>
    ) {
      const fullPath = pathParts.join(".");

      // Detect if first argument is options (for void-input procedures)
      // Options objects have specific keys: context, signal, timeout, meta
      const isOptionsObject = (obj: unknown): obj is LinkCallOptions<TClientContext> => {
        if (typeof obj !== "object" || obj === null) return false;
        const keys = Object.keys(obj);
        const optionKeys = ["context", "signal", "timeout", "meta"];
        return keys.length > 0 && keys.every(k => optionKeys.includes(k));
      };

      let input: unknown = null;
      let options: LinkCallOptions<TClientContext> | LinkSubscribeOptions<TClientContext> | undefined;

      if (maybeOptions !== undefined) {
        // Two arguments: input and options
        input = inputOrOptions;
        options = maybeOptions;
      } else if (isOptionsObject(inputOrOptions)) {
        // Single argument that looks like options: void input
        input = null;
        options = inputOrOptions;
      } else {
        // Single argument that's input data
        input = inputOrOptions ?? null;
        options = undefined;
      }

      if (link.isSubscription(fullPath)) {
        return link.subscribe(fullPath, input, options as LinkSubscribeOptions<TClientContext>);
      }

      return link.call(fullPath, input, options as LinkCallOptions<TClientContext>);
    };

    return new Proxy(handler, {
      get(_target, prop: string | symbol) {
        if (prop === CLIENT_PROXY) return true;
        if (typeof prop === "symbol") return undefined;
        return createProxy([...pathParts, prop]);
      },
      apply(_, __, args: unknown[]) {
        const fullPath = pathParts.join(".");

        // Same logic as handler
        const isOptionsObject = (obj: unknown): obj is LinkCallOptions<TClientContext> => {
          if (typeof obj !== "object" || obj === null) return false;
          const keys = Object.keys(obj);
          const optionKeys = ["context", "signal", "timeout", "meta"];
          return keys.length > 0 && keys.every(k => optionKeys.includes(k));
        };

        let input: unknown = null;
        let options: LinkCallOptions<TClientContext> | LinkSubscribeOptions<TClientContext> | undefined;

        if (args.length >= 2 && args[1] !== undefined) {
          input = args[0];
          options = args[1] as LinkCallOptions<TClientContext>;
        } else if (isOptionsObject(args[0])) {
          input = null;
          options = args[0];
        } else {
          input = args[0] ?? null;
          options = undefined;
        }

        if (link.isSubscription(fullPath)) {
          return link.subscribe(fullPath, input, options as LinkSubscribeOptions<TClientContext>);
        }

        return link.call(fullPath, input, options as LinkCallOptions<TClientContext>);
      },
    });
  }

  return createProxy([]) as LinkRouterClient<T, TClientContext>;
}

// =============================================================================
// Interceptor Helpers
// =============================================================================

/**
 * Create an error handler interceptor.
 * 
 * @example
 * ```typescript
 * const link = new TauriLink({
 *   interceptors: [
 *     onError((error) => {
 *       console.error('RPC Error:', error);
 *     }),
 *   ],
 * });
 * ```
 */
export function onError<TClientContext = unknown>(
  handler: (error: RpcError, ctx: LinkRequestContext<TClientContext>) => void
): LinkInterceptor<TClientContext> {
  return async (ctx, next) => {
    try {
      return await next();
    } catch (error) {
      if (isRpcError(error)) {
        handler(error, ctx);
      }
      throw error;
    }
  };
}

/**
 * Create a logging interceptor.
 * 
 * @example
 * ```typescript
 * const link = new TauriLink({
 *   interceptors: [logging()],
 * });
 * ```
 */
export function logging<TClientContext = unknown>(
  options: { prefix?: string } = {}
): LinkInterceptor<TClientContext> {
  const prefix = options.prefix ?? "[RPC]";
  return async (ctx, next) => {
    const start = performance.now();
    console.log(`${prefix} ${ctx.path}`, ctx.input);
    try {
      const result = await next();
      const duration = (performance.now() - start).toFixed(2);
      console.log(`${prefix} ${ctx.path} completed in ${duration}ms`);
      return result;
    } catch (error) {
      const duration = (performance.now() - start).toFixed(2);
      console.error(`${prefix} ${ctx.path} failed in ${duration}ms`, error);
      throw error;
    }
  };
}

/**
 * Create a retry interceptor.
 * 
 * @example
 * ```typescript
 * const link = new TauriLink({
 *   interceptors: [
 *     retry({ maxRetries: 3, delay: 1000 }),
 *   ],
 * });
 * ```
 */
export function retry<TClientContext = unknown>(
  options: { maxRetries?: number; delay?: number; shouldRetry?: (error: RpcError) => boolean } = {}
): LinkInterceptor<TClientContext> {
  const maxRetries = options.maxRetries ?? 3;
  const delay = options.delay ?? 1000;
  const shouldRetry = options.shouldRetry ?? ((error) => 
    error.code === "SERVICE_UNAVAILABLE" || error.code === "TIMEOUT"
  );

  return async (_ctx, next) => {
    let lastError: RpcError | undefined;
    
    for (let attempt = 0; attempt <= maxRetries; attempt++) {
      try {
        return await next();
      } catch (error) {
        if (!isRpcError(error) || !shouldRetry(error) || attempt === maxRetries) {
          throw error;
        }
        lastError = error;
        await new Promise(resolve => setTimeout(resolve, delay * (attempt + 1)));
      }
    }

    throw lastError;
  };
}

// =============================================================================
// Type Inference for Client Context
// =============================================================================

/**
 * Infer the client context type from a link.
 * 
 * @example
 * ```typescript
 * const link = new TauriLink<{ token: string }>();
 * type Context = InferLinkContext<typeof link>; // { token: string }
 * ```
 */
export type InferLinkContext<T> = T extends TauriLink<infer C> ? C : never;

/**
 * Infer the client context type from a client.
 * 
 * @example
 * ```typescript
 * const client = createClientFromLink<AppContract, { token: string }>(link);
 * type Context = InferClientContext<typeof client>; // { token: string }
 * ```
 */
export type InferClientContext<T> = T extends LinkRouterClient<infer _, infer C> ? C : never;
