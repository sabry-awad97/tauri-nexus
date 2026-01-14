// =============================================================================
// @tauri-nexus/rpc-effect - Effect Client Factory
// =============================================================================
// Creates a type-safe RPC client that uses Effect internally.

import type { RpcInterceptor, EventIterator } from "./types";
import { EffectLink } from "./link";
import type { CallOptions, SubscribeOptions } from "./call";

// =============================================================================
// Configuration
// =============================================================================

export interface EffectClientConfig {
  readonly subscriptionPaths?: readonly string[];
  readonly timeout?: number;
  readonly interceptors?: readonly RpcInterceptor[];
  readonly debug?: boolean;
}

// =============================================================================
// Client Type
// =============================================================================

export type EffectClient<T> = {
  readonly __contract?: T;
  readonly __link: EffectLink;
  call<TResult>(
    path: string,
    input?: unknown,
    options?: CallOptions,
  ): Promise<TResult>;
  subscribe<TResult>(
    path: string,
    input?: unknown,
    options?: SubscribeOptions,
  ): Promise<EventIterator<TResult>>;
  isSubscription(path: string): boolean;
  withInterceptors(interceptors: readonly RpcInterceptor[]): EffectClient<T>;
  withTimeout(timeout: number): EffectClient<T>;
};

// =============================================================================
// Client Implementation
// =============================================================================

class EffectClientImpl<T> implements EffectClient<T> {
  readonly __contract?: T;
  readonly __link: EffectLink;

  constructor(link: EffectLink) {
    this.__link = link;
  }

  async call<TResult>(
    path: string,
    input?: unknown,
    options?: CallOptions,
  ): Promise<TResult> {
    return this.__link.runCall<TResult>(path, input, options);
  }

  async subscribe<TResult>(
    path: string,
    input?: unknown,
    options?: SubscribeOptions,
  ): Promise<EventIterator<TResult>> {
    return this.__link.runSubscribe<TResult>(path, input, options) as Promise<
      EventIterator<TResult>
    >;
  }

  isSubscription(path: string): boolean {
    return this.__link.isSubscription(path);
  }

  withInterceptors(interceptors: readonly RpcInterceptor[]): EffectClient<T> {
    return new EffectClientImpl<T>(this.__link.withInterceptors(interceptors));
  }

  withTimeout(timeout: number): EffectClient<T> {
    return new EffectClientImpl<T>(this.__link.withTimeout(timeout));
  }
}

// =============================================================================
// Public Factory
// =============================================================================

/**
 * Create a type-safe RPC client using Effect internally.
 * Note: You must call setTransport on the returned client's __link before use.
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

  return new EffectClientImpl<T>(link);
}

/**
 * Create an Effect client with a pre-configured transport.
 */
export function createEffectClientWithTransport<T>(
  config: EffectClientConfig & {
    transport: {
      call: <TResult>(path: string, input: unknown) => Promise<TResult>;
      subscribe: <TResult>(
        path: string,
        input: unknown,
        options?: { lastEventId?: string; signal?: AbortSignal },
      ) => Promise<EventIterator<TResult>>;
    };
  },
): EffectClient<T> {
  const link = new EffectLink({
    subscriptionPaths: config.subscriptionPaths,
    timeout: config.timeout,
    interceptors: config.interceptors,
    debug: config.debug,
  });

  link.setTransport(() => config.transport);

  return new EffectClientImpl<T>(link);
}
