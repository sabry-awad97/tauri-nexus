// =============================================================================
// Tauri RPC Client - Client Implementation
// =============================================================================
// Type-safe RPC client with middleware support and automatic path generation.

import { invoke } from '@tauri-apps/api/core';
import type {
  RouterClient,
  RpcError,
  RpcErrorCode,
  CallOptions,
  SubscriptionOptions,
  Middleware,
  RequestContext,
} from './types';
import { createEventIterator } from './event-iterator';

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
// Error Handling
// =============================================================================

/** Parse RPC error from backend response */
function parseError(error: unknown): RpcError {
  if (typeof error === 'string') {
    try {
      return JSON.parse(error) as RpcError;
    } catch {
      return { code: 'UNKNOWN', message: error };
    }
  }
  if (error instanceof Error) {
    if (error.name === 'AbortError') {
      return { code: 'CANCELLED', message: 'Request was cancelled' };
    }
    return { code: 'UNKNOWN', message: error.message };
  }
  if (isRpcError(error)) {
    return error;
  }
  return { code: 'UNKNOWN', message: String(error) };
}

/** Check if error is an RPC error */
export function isRpcError(error: unknown): error is RpcError {
  return (
    typeof error === 'object' &&
    error !== null &&
    'code' in error &&
    'message' in error &&
    typeof (error as RpcError).code === 'string' &&
    typeof (error as RpcError).message === 'string'
  );
}

/** Check if error has a specific code */
export function hasErrorCode(
  error: unknown,
  code: RpcErrorCode | string
): boolean {
  return isRpcError(error) && error.code === code;
}

/** Create a typed RPC error */
export function createError(
  code: RpcErrorCode | string,
  message: string,
  details?: unknown
): RpcError {
  return { code, message, details };
}

// =============================================================================
// Middleware Execution
// =============================================================================

/** Execute middleware chain */
async function executeWithMiddleware<T>(
  ctx: RequestContext,
  fn: () => Promise<T>
): Promise<T> {
  const middleware = globalConfig.middleware ?? [];

  // Build middleware chain from right to left
  let next = fn;
  for (let i = middleware.length - 1; i >= 0; i--) {
    const mw = middleware[i];
    const currentNext = next;
    next = () => mw(ctx, currentNext);
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
  options?: CallOptions
): Promise<T> {
  const ctx: RequestContext = {
    path,
    input,
    type: 'query', // Will be determined by path in practice
    meta: options?.meta,
    signal: options?.signal,
  };

  globalConfig.onRequest?.(ctx);

  try {
    const result = await executeWithMiddleware(ctx, async () => {
      // Handle timeout
      if (options?.timeout) {
        const controller = new AbortController();
        const timeoutId = setTimeout(() => controller.abort(), options.timeout);

        try {
          const result = await invoke<T>('plugin:rpc|rpc_call', { path, input });
          clearTimeout(timeoutId);
          return result;
        } catch (error) {
          clearTimeout(timeoutId);
          throw error;
        }
      }

      return invoke<T>('plugin:rpc|rpc_call', { path, input });
    });

    globalConfig.onResponse?.(ctx, result);
    return result;
  } catch (error) {
    const rpcError = parseError(error);
    globalConfig.onError?.(ctx, rpcError);
    throw rpcError;
  }
}

/** Subscribe to a streaming procedure */
export async function subscribe<T>(
  path: string,
  input: unknown = null,
  options?: SubscriptionOptions
): Promise<ReturnType<typeof createEventIterator<T>>> {
  const ctx: RequestContext = {
    path,
    input,
    type: 'subscription',
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
const CLIENT_PROXY = Symbol('rpc-client-proxy');

/** Check if path is a subscription */
function isSubscriptionPath(path: string): boolean {
  const paths = globalConfig.subscriptionPaths ?? [];
  return paths.includes(path);
}

/** Create a proxy that builds paths and calls the appropriate function */
function createClientProxy<T>(
  pathParts: string[]
): RouterClient<T> {
  const handler = function (
    inputOrOptions?: unknown,
    maybeOptions?: CallOptions | SubscriptionOptions
  ) {
    const fullPath = pathParts.join('.');

    // Determine if this is a subscription
    if (isSubscriptionPath(fullPath)) {
      return subscribe(fullPath, inputOrOptions, maybeOptions as SubscriptionOptions);
    }

    return call(fullPath, inputOrOptions, maybeOptions as CallOptions);
  };

  return new Proxy(handler as unknown as RouterClient<T>, {
    get(_target, prop: string | symbol) {
      if (prop === CLIENT_PROXY) return true;
      if (typeof prop === 'symbol') return undefined;
      return createClientProxy([...pathParts, prop]);
    },
    apply(_, __, args: unknown[]) {
      const fullPath = pathParts.join('.');

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
export function createClient<T>(
  config?: RpcClientConfig
): RouterClient<T> {
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
  config: RpcClientConfig & { subscriptionPaths: string[] }
): RouterClient<T> {
  configureRpc(config);
  return createClientProxy<T>([]);
}

// =============================================================================
// Utility Exports
// =============================================================================

/** Get list of available procedures from backend */
export async function getProcedures(): Promise<string[]> {
  return invoke<string[]>('plugin:rpc|rpc_procedures');
}

/** Get current subscription count from backend */
export async function getSubscriptionCount(): Promise<number> {
  return invoke<number>('plugin:rpc|rpc_subscription_count');
}

// Re-export for convenience
export { createEventIterator } from './event-iterator';
