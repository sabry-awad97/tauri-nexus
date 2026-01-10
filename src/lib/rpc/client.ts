// =============================================================================
// RPC Client Implementation
// =============================================================================

import { invoke } from '@tauri-apps/api/core';
import type { 
  ContractRouter, 
  RouterClient, 
  RpcError, 
  CallOptions,
  SubscriptionOptions,
} from './types';
import { createEventIterator } from './event-iterator';

// =============================================================================
// Client Configuration
// =============================================================================

export interface RpcClientConfig {
  /** Called before each request */
  onRequest?: (path: string, input: unknown) => void;
  /** Called after successful response */
  onResponse?: (path: string, data: unknown) => void;
  /** Called on error */
  onError?: (path: string, error: RpcError) => void;
  /** Paths that should be treated as subscriptions (for runtime detection) */
  subscriptionPaths?: string[];
}

let globalConfig: RpcClientConfig = {};

/** Configure the RPC client globally */
export function configureRpc(config: RpcClientConfig): void {
  globalConfig = { ...globalConfig, ...config };
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
    return { code: 'UNKNOWN', message: error.message };
  }
  return { code: 'UNKNOWN', message: String(error) };
}

/** Check if error is an RPC error */
export function isRpcError(error: unknown): error is RpcError {
  return (
    typeof error === 'object' &&
    error !== null &&
    'code' in error &&
    'message' in error
  );
}

/** Check if error has a specific code */
export function hasErrorCode(error: unknown, code: string): boolean {
  return isRpcError(error) && error.code === code;
}

// =============================================================================
// Core Call Functions
// =============================================================================

/** Make an RPC call (query or mutation) */
export async function call<T>(
  path: string, 
  input: unknown = {}, 
  _options?: CallOptions
): Promise<T> {
  globalConfig.onRequest?.(path, input);

  try {
    const result = await invoke<T>('plugin:rpc|rpc_call', { path, input });
    globalConfig.onResponse?.(path, result);
    return result;
  } catch (error) {
    const rpcError = parseError(error);
    globalConfig.onError?.(path, rpcError);
    throw rpcError;
  }
}

/** Subscribe to a streaming procedure */
export async function subscribe<T>(
  path: string,
  input: unknown = {},
  options?: SubscriptionOptions
): Promise<ReturnType<typeof createEventIterator<T>>> {
  globalConfig.onRequest?.(path, input);
  return createEventIterator<T>(path, input, options);
}

// =============================================================================
// Client Factory
// =============================================================================

/** Create a type-safe RPC client from a contract */
export function createRpcClient<T extends ContractRouter>(
  config?: RpcClientConfig
): RouterClient<T> {
  if (config) {
    configureRpc(config);
  }
  
  return createClientProxy<T>([]);
}

/** 
 * Create a fully type-safe client with explicit procedure types
 * This is the recommended way to create a client
 */
export function createTypedClient<T extends ContractRouter>(
  config?: RpcClientConfig
): RouterClient<T> {
  if (config) {
    configureRpc(config);
  }
  return createClientProxy<T>([]);
}

// =============================================================================
// Proxy Implementation
// =============================================================================

/** Symbol to mark subscription procedures at runtime */
const SUBSCRIPTION_MARKER = Symbol('subscription');

/** Create a proxy that builds paths and calls the appropriate function */
function createClientProxy<T extends ContractRouter>(
  pathParts: string[]
): RouterClient<T> {
  const handler = function(inputOrOptions?: unknown, options?: CallOptions | SubscriptionOptions) {
    const fullPath = pathParts.join('.');
    
    // Check if this path is registered as a subscription
    const isSubscription = globalConfig.subscriptionPaths?.includes(fullPath);
    
    if (isSubscription) {
      return subscribe(fullPath, inputOrOptions, options as SubscriptionOptions);
    }
    return call(fullPath, inputOrOptions, options as CallOptions);
  };

  return new Proxy(handler as unknown as RouterClient<T>, {
    get(_target, prop: string | symbol) {
      if (prop === SUBSCRIPTION_MARKER) {
        return true;
      }
      if (typeof prop === 'symbol') {
        return undefined;
      }
      return createClientProxy([...pathParts, prop]);
    },
    apply(_, __, args: unknown[]) {
      const fullPath = pathParts.join('.');
      const isSubscription = globalConfig.subscriptionPaths?.includes(fullPath);
      
      if (isSubscription) {
        return subscribe(fullPath, args[0], args[1] as SubscriptionOptions);
      }
      return call(fullPath, args[0], args[1] as CallOptions);
    },
  });
}

// =============================================================================
// Subscription-Aware Client Factory
// =============================================================================

/**
 * Create a client with explicit subscription paths
 * This enables runtime detection of subscriptions
 * 
 * @example
 * const rpc = createClientWithSubscriptions<AppContract>({
 *   subscriptionPaths: ['chat.messages', 'stocks.live']
 * });
 */
export function createClientWithSubscriptions<T extends ContractRouter>(
  config: RpcClientConfig & { subscriptionPaths: string[] }
): RouterClient<T> {
  configureRpc(config);
  return createClientProxy<T>([]);
}
