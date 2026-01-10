// =============================================================================
// Tauri RPC Client - Client Implementation
// =============================================================================
// Type-safe RPC client with middleware support and automatic path generation.

import { invoke } from "@tauri-apps/api/core";
import type {
  RouterClient,
  RpcError,
  RpcErrorCode,
  CallOptions,
  SubscriptionOptions,
  Middleware,
  RequestContext,
} from "./types";
import { createEventIterator } from "./event-iterator";

// =============================================================================
// Client Configuration
// =============================================================================

export interface RpcClientConfig {
  /** Middleware stack - executed in order */
  middleware?: Middleware[];
  /** Paths that are subscriptions (for runtime detection) */
  subscriptionPaths?: string[];
  /** Global request timeout in milliseconds */
  timeout?: number;
  /** Called before each request */
  onRequest?: (ctx: RequestContext) => void;
  /** Called after successful response */
  onResponse?: <R>(ctx: RequestContext, data: R) => void;
  /** Called on error */
  onError?: (ctx: RequestContext, error: RpcError) => void;
}

/** Global configuration store */
let globalConfig: RpcClientConfig = {};

/** Configure the RPC client globally */
export function configureRpc(config: RpcClientConfig): void {
  globalConfig = { ...globalConfig, ...config };
}

/** Get current configuration */
export function getConfig(): RpcClientConfig {
  return globalConfig;
}

// =============================================================================
// Input Validation (matches Rust validation)
// =============================================================================

/**
 * Validate procedure path format.
 * Matches the Rust `validate_path` function in plugin.rs.
 *
 * Valid paths: "health", "user.get", "api.v1.users.list"
 * Invalid: "", ".path", "path.", "path..name", "path/name"
 */
export function validatePath(path: string): void {
  if (!path) {
    throw createError("VALIDATION_ERROR", "Procedure path cannot be empty");
  }
  if (path.startsWith(".") || path.endsWith(".")) {
    throw createError(
      "VALIDATION_ERROR",
      "Procedure path cannot start or end with a dot",
    );
  }
  if (path.includes("..")) {
    throw createError(
      "VALIDATION_ERROR",
      "Procedure path cannot contain consecutive dots",
    );
  }
  for (const ch of path) {
    if (!/[a-zA-Z0-9_.]/.test(ch)) {
      throw createError(
        "VALIDATION_ERROR",
        `Procedure path contains invalid character: '${ch}'`,
      );
    }
  }
}

// =============================================================================
// Error Handling
// =============================================================================

/** Parse RPC error from backend response */
function parseError(error: unknown, timeoutMs?: number): RpcError {
  // Handle AbortError (from timeout or manual cancellation)
  if (error instanceof Error) {
    if (error.name === "AbortError") {
      // If we have a timeout value, this was a timeout
      if (timeoutMs !== undefined) {
        return {
          code: "TIMEOUT",
          message: `Request timed out after ${timeoutMs}ms`,
          details: { timeoutMs },
        };
      }
      return { code: "CANCELLED", message: "Request was cancelled" };
    }
    return { code: "UNKNOWN", message: error.message };
  }

  // Handle JSON string errors from backend
  if (typeof error === "string") {
    try {
      const parsed = JSON.parse(error);
      if (isRpcError(parsed)) {
        return parsed;
      }
      return { code: "UNKNOWN", message: error };
    } catch {
      return { code: "UNKNOWN", message: error };
    }
  }

  // Handle RpcError objects directly
  if (isRpcError(error)) {
    return error;
  }

  // Fallback for unknown error types
  return { code: "UNKNOWN", message: String(error) };
}

/** Check if error is an RPC error */
export function isRpcError(error: unknown): error is RpcError {
  return (
    typeof error === "object" &&
    error !== null &&
    "code" in error &&
    "message" in error &&
    typeof (error as RpcError).code === "string" &&
    typeof (error as RpcError).message === "string"
  );
}

/** Check if error has a specific code */
export function hasErrorCode(
  error: unknown,
  code: RpcErrorCode | string,
): boolean {
  return isRpcError(error) && error.code === code;
}

/** Create a typed RPC error */
export function createError(
  code: RpcErrorCode | string,
  message: string,
  details?: unknown,
): RpcError {
  return { code, message, details };
}

// =============================================================================
// Middleware Execution
// =============================================================================

/** Execute middleware chain with error wrapping */
async function executeWithMiddleware<T>(
  ctx: RequestContext,
  fn: () => Promise<T>,
): Promise<T> {
  const middleware = globalConfig.middleware ?? [];

  // Build middleware chain from right to left
  let next = fn;
  for (let i = middleware.length - 1; i >= 0; i--) {
    const mw = middleware[i];
    const currentNext = next;
    const middlewareIndex = i;
    next = async () => {
      try {
        return await mw(ctx, currentNext);
      } catch (error) {
        // Wrap non-RpcError middleware errors
        if (!isRpcError(error)) {
          throw createError(
            "MIDDLEWARE_ERROR",
            error instanceof Error ? error.message : String(error),
            {
              middlewareIndex,
              originalError: error instanceof Error ? error.message : error,
            },
          );
        }
        throw error;
      }
    };
  }

  return next();
}

// =============================================================================
// Core Call Functions
// =============================================================================

/** Make an RPC call (query or mutation) */
export async function call<T>(
  path: string,
  input: unknown = null,
  options?: CallOptions,
): Promise<T> {
  // Validate path format (matches Rust validation)
  validatePath(path);

  const ctx: RequestContext = {
    path,
    input,
    type: "query", // Will be determined by path in practice
    meta: options?.meta,
    signal: options?.signal,
  };

  globalConfig.onRequest?.(ctx);

  const timeoutMs = options?.timeout;

  try {
    const result = await executeWithMiddleware(ctx, async () => {
      // Handle timeout
      if (timeoutMs) {
        const controller = new AbortController();
        const timeoutId = setTimeout(() => controller.abort(), timeoutMs);

        try {
          const result = await invoke<T>("plugin:rpc|rpc_call", {
            path,
            input,
          });
          clearTimeout(timeoutId);
          return result;
        } catch (error) {
          clearTimeout(timeoutId);
          throw error;
        }
      }

      return invoke<T>("plugin:rpc|rpc_call", { path, input });
    });

    globalConfig.onResponse?.(ctx, result);
    return result;
  } catch (error) {
    const rpcError = parseError(error, timeoutMs);
    globalConfig.onError?.(ctx, rpcError);
    throw rpcError;
  }
}

/** Subscribe to a streaming procedure */
export async function subscribe<T>(
  path: string,
  input: unknown = null,
  options?: SubscriptionOptions,
): Promise<ReturnType<typeof createEventIterator<T>>> {
  // Validate path format (matches Rust validation)
  validatePath(path);

  const ctx: RequestContext = {
    path,
    input,
    type: "subscription",
    meta: options?.meta,
    signal: options?.signal,
  };

  globalConfig.onRequest?.(ctx);

  try {
    return await createEventIterator<T>(path, input, options);
  } catch (error) {
    const rpcError = parseError(error);
    globalConfig.onError?.(ctx, rpcError);
    throw rpcError;
  }
}

// =============================================================================
// Client Factory
// =============================================================================

/** Symbol to identify the client proxy */
const CLIENT_PROXY = Symbol("rpc-client-proxy");

/** Check if path is a subscription */
function isSubscriptionPath(path: string): boolean {
  const paths = globalConfig.subscriptionPaths ?? [];
  return paths.includes(path);
}

/** Create a proxy that builds paths and calls the appropriate function */
function createClientProxy<T>(pathParts: string[]): RouterClient<T> {
  const handler = function (
    inputOrOptions?: unknown,
    maybeOptions?: CallOptions | SubscriptionOptions,
  ) {
    const fullPath = pathParts.join(".");

    // Determine if this is a subscription
    if (isSubscriptionPath(fullPath)) {
      return subscribe(
        fullPath,
        inputOrOptions,
        maybeOptions as SubscriptionOptions,
      );
    }

    return call(fullPath, inputOrOptions, maybeOptions as CallOptions);
  };

  return new Proxy(handler as unknown as RouterClient<T>, {
    get(_target, prop: string | symbol) {
      if (prop === CLIENT_PROXY) return true;
      if (typeof prop === "symbol") return undefined;
      return createClientProxy([...pathParts, prop]);
    },
    apply(_, __, args: unknown[]) {
      const fullPath = pathParts.join(".");

      if (isSubscriptionPath(fullPath)) {
        return subscribe(fullPath, args[0], args[1] as SubscriptionOptions);
      }

      return call(fullPath, args[0], args[1] as CallOptions);
    },
  });
}

// =============================================================================
// Public Client Factories
// =============================================================================

/**
 * Create a type-safe RPC client from a contract definition.
 *
 * @example
 * ```typescript
 * interface MyContract {
 *   health: { type: 'query'; input: void; output: { status: string } };
 *   user: {
 *     get: { type: 'query'; input: { id: number }; output: User };
 *     create: { type: 'mutation'; input: CreateUserInput; output: User };
 *   };
 *   stream: {
 *     events: { type: 'subscription'; input: void; output: Event };
 *   };
 * }
 *
 * const rpc = createClient<MyContract>({
 *   subscriptionPaths: ['stream.events'],
 * });
 *
 * // Full type safety!
 * const health = await rpc.health();
 * const user = await rpc.user.get({ id: 1 });
 * const stream = await rpc.stream.events();
 * ```
 */
export function createClient<T>(config?: RpcClientConfig): RouterClient<T> {
  if (config) {
    configureRpc(config);
  }
  return createClientProxy<T>([]);
}

/**
 * Create a client with explicit subscription paths.
 * This is the recommended way to create a client when you have subscriptions.
 *
 * @example
 * ```typescript
 * const rpc = createClientWithSubscriptions<MyContract>({
 *   subscriptionPaths: ['stream.counter', 'stream.chat'],
 *   middleware: [loggingMiddleware],
 * });
 * ```
 */
export function createClientWithSubscriptions<T>(
  config: RpcClientConfig & { subscriptionPaths: string[] },
): RouterClient<T> {
  configureRpc(config);
  return createClientProxy<T>([]);
}

// =============================================================================
// Utility Exports
// =============================================================================

/** Get list of available procedures from backend */
export async function getProcedures(): Promise<string[]> {
  return invoke<string[]>("plugin:rpc|rpc_procedures");
}

/** Get current subscription count from backend */
export async function getSubscriptionCount(): Promise<number> {
  return invoke<number>("plugin:rpc|rpc_subscription_count");
}

// Re-export for convenience
export { createEventIterator } from "./event-iterator";
