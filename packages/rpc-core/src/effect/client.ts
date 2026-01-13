// =============================================================================
// @tauri-nexus/rpc-core - Effect Client Factory
// =============================================================================
// Creates a type-safe RPC client that uses Effect internally but exposes
// a simple Promise-based API.

import type { CallOptions, SubscriptionOptions } from "../core/types";
import type { RouterClient } from "../core/inference";
import { EffectLink } from "../internal/effect-link";
import { toPublicError } from "../internal/effect-errors";
import type { RpcInterceptor } from "../internal/effect-types";
import { TypedBatchBuilder } from "../client/batch";

// =============================================================================
// Configuration
// =============================================================================

/**
 * Configuration for creating an Effect-based client.
 */
export interface EffectClientConfig {
  /** Paths that should be treated as subscriptions */
  readonly subscriptionPaths?: readonly string[];
  /** Default timeout in milliseconds */
  readonly timeout?: number;
  /** Interceptors to apply to all requests */
  readonly interceptors?: readonly RpcInterceptor[];
  /** Enable debug logging */
  readonly debug?: boolean;
}

// =============================================================================
// Client Type
// =============================================================================

/**
 * Effect-based RPC client type.
 */
export type EffectClient<T> = RouterClient<T> & {
  /** Brand to carry contract type */
  readonly __contract?: T;
  /** Create a batch builder */
  batch(): TypedBatchBuilder<T, Record<string, never>>;
};

// =============================================================================
// Client Proxy Implementation
// =============================================================================

const CLIENT_PROXY = Symbol("effect-client-proxy");

function createClientProxy<T>(
  link: EffectLink,
  pathParts: string[],
): EffectClient<T> {
  const handler = async function (
    inputOrOptions?: unknown,
    maybeOptions?: CallOptions | SubscriptionOptions,
  ) {
    const fullPath = pathParts.join(".");

    try {
      if (link.isSubscription(fullPath)) {
        return await link.runSubscribe(
          fullPath,
          inputOrOptions,
          maybeOptions as SubscriptionOptions,
        );
      }

      return await link.runCall(
        fullPath,
        inputOrOptions,
        maybeOptions as CallOptions,
      );
    } catch (error) {
      // Convert Effect errors to public format
      if (typeof error === "object" && error !== null && "_tag" in error) {
        throw toPublicError(error as Parameters<typeof toPublicError>[0]);
      }
      throw error;
    }
  };

  return new Proxy(handler as unknown as EffectClient<T>, {
    get(_target, prop: string | symbol) {
      if (prop === CLIENT_PROXY) return true;
      if (typeof prop === "symbol") return undefined;

      if (prop === "batch" && pathParts.length === 0) {
        return () => new TypedBatchBuilder<T, Record<string, never>>();
      }

      return createClientProxy(link, [...pathParts, prop]);
    },
    apply(_, __, args: unknown[]) {
      const fullPath = pathParts.join(".");

      const execute = async () => {
        try {
          if (link.isSubscription(fullPath)) {
            return await link.runSubscribe(
              fullPath,
              args[0],
              args[1] as SubscriptionOptions,
            );
          }

          return await link.runCall(fullPath, args[0], args[1] as CallOptions);
        } catch (error) {
          if (typeof error === "object" && error !== null && "_tag" in error) {
            throw toPublicError(error as Parameters<typeof toPublicError>[0]);
          }
          throw error;
        }
      };

      return execute();
    },
  });
}

// =============================================================================
// Public Factory
// =============================================================================

/**
 * Create a type-safe RPC client using Effect internally.
 *
 * This provides the same simple Promise-based API as `createClient`,
 * but uses Effect for internal error handling and composition.
 *
 * @example
 * ```typescript
 * import { createEffectClient, loggingInterceptor } from '@tauri-nexus/rpc-core/effect';
 *
 * const rpc = createEffectClient<AppContract>({
 *   subscriptionPaths: ['stream.events'],
 *   interceptors: [loggingInterceptor()],
 *   debug: true,
 * });
 *
 * // Use like normal - Promise-based API
 * const user = await rpc.user.get({ id: 1 });
 * ```
 */
export function createEffectClient<T>(
  config: EffectClientConfig = {},
): EffectClient<T> {
  const link = new EffectLink({
    subscriptionPaths: config.subscriptionPaths,
    timeout: config.timeout,
    interceptors: config.interceptors,
    debug: config.debug,
  });

  return createClientProxy<T>(link, []);
}
