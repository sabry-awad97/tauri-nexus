// =============================================================================
// @tauri-nexus/rpc-core - Client Factory
// =============================================================================
// Factory functions for creating type-safe RPC clients.

import type { CallOptions, SubscriptionOptions } from "../core/types";
import type { RouterClient } from "../core/inference";
import {
  configureRpc,
  isSubscriptionPath,
  type RpcClientConfig,
} from "./config";
import { call, subscribe } from "./call";
import { TypedBatchBuilder } from "./batch";

// =============================================================================
// Client Types
// =============================================================================

/**
 * Extended client type that includes the batch() method.
 */
export type RpcClient<T> = RouterClient<T> & {
  /** Brand to carry contract type for inference */
  readonly __contract?: T;
  /**
   * Create a type-safe batch builder for executing multiple requests.
   */
  batch(): TypedBatchBuilder<T, Record<string, never>>;
};

// =============================================================================
// Client Proxy
// =============================================================================

/** Symbol to identify the client proxy */
const CLIENT_PROXY = Symbol("rpc-client-proxy");

/** Create a proxy that builds paths and calls the appropriate function */
function createClientProxy<T>(pathParts: string[]): RpcClient<T> {
  const handler = function (
    inputOrOptions?: unknown,
    maybeOptions?: CallOptions | SubscriptionOptions,
  ) {
    const fullPath = pathParts.join(".");

    if (isSubscriptionPath(fullPath)) {
      return subscribe(
        fullPath,
        inputOrOptions,
        maybeOptions as SubscriptionOptions,
      );
    }

    return call(fullPath, inputOrOptions, maybeOptions as CallOptions);
  };

  return new Proxy(handler as unknown as RpcClient<T>, {
    get(_target, prop: string | symbol) {
      if (prop === CLIENT_PROXY) return true;
      if (typeof prop === "symbol") return undefined;

      if (prop === "batch" && pathParts.length === 0) {
        return () => new TypedBatchBuilder<T, Record<string, never>>();
      }

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
 * const rpc = createClient<MyContract>({
 *   subscriptionPaths: ['stream.events'],
 * });
 *
 * const health = await rpc.health();
 * const user = await rpc.user.get({ id: 1 });
 * ```
 */
export function createClient<T>(config?: RpcClientConfig): RpcClient<T> {
  if (config) {
    configureRpc(config);
  }
  return createClientProxy<T>([]);
}

/**
 * Create a client with explicit subscription paths.
 *
 * @example
 * ```typescript
 * const rpc = createClientWithSubscriptions<MyContract>({
 *   subscriptionPaths: ['stream.counter', 'stream.chat'],
 * });
 * ```
 */
export function createClientWithSubscriptions<T>(
  config: RpcClientConfig & { subscriptionPaths: string[] },
): RpcClient<T> {
  configureRpc(config);
  return createClientProxy<T>([]);
}
