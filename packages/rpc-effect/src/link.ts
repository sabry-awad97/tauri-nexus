// =============================================================================
// @tauri-nexus/rpc-effect - Effect-Based Link Implementation
// =============================================================================
// A composable link system using Effect for advanced use cases.

import { Effect, Layer, pipe } from "effect";
import {
  type RpcConfig,
  type RpcInterceptor,
  type RpcEffectError,
  type EventIterator,
} from "./types";
import {
  call,
  subscribe,
  type CallOptions,
  type SubscribeOptions,
} from "./call";
import {
  makeConfigLayer,
  makeInterceptorLayer,
  makeLoggerLayer,
  makeTransportLayer,
  type RpcServices,
} from "./runtime";

// Re-export interceptor factories for convenience
export {
  createLoggingInterceptor,
  createRetryInterceptor,
  createErrorInterceptor,
  createAuthInterceptor,
} from "./interceptors";

// =============================================================================
// Effect Link Configuration
// =============================================================================

export interface EffectLinkConfig<TContext = unknown> {
  readonly subscriptionPaths?: readonly string[];
  readonly timeout?: number;
  readonly interceptors?: readonly RpcInterceptor[];
  readonly debug?: boolean;
  readonly _context?: TContext;
}

// =============================================================================
// Effect Link Class
// =============================================================================

/**
 * EffectLink - A composable link using Effect for advanced type-safe operations.
 */
export class EffectLink<TContext = unknown> {
  private readonly config: EffectLinkConfig<TContext>;
  private layer: Layer.Layer<RpcServices> | null = null;
  private transportProvider:
    | (() => {
        call: <T>(path: string, input: unknown) => Promise<T>;
        callBatch: <T>(
          requests: readonly { id: string; path: string; input: unknown }[]
        ) => Promise<{
          results: readonly {
            id: string;
            data?: T;
            error?: { code: string; message: string; details?: unknown };
          }[];
        }>;
        subscribe: <T>(
          path: string,
          input: unknown,
          options?: { lastEventId?: string; signal?: AbortSignal }
        ) => Promise<EventIterator<T>>;
      })
    | null = null;

  constructor(config: EffectLinkConfig<TContext> = {}) {
    this.config = config;
  }

  /**
   * Set the transport provider for this link.
   */
  setTransport(
    provider: () => {
      call: <T>(path: string, input: unknown) => Promise<T>;
      callBatch: <T>(
        requests: readonly { id: string; path: string; input: unknown }[]
      ) => Promise<{
        results: readonly {
          id: string;
          data?: T;
          error?: { code: string; message: string; details?: unknown };
        }[];
      }>;
      subscribe: <T>(
        path: string,
        input: unknown,
        options?: { lastEventId?: string; signal?: AbortSignal }
      ) => Promise<EventIterator<T>>;
    }
  ): this {
    this.transportProvider = provider;
    this.layer = null;
    return this;
  }

  private buildLayer(): Layer.Layer<RpcServices> {
    if (!this.transportProvider) {
      throw new Error(
        "Transport not configured. Call setTransport() before using the link."
      );
    }

    const rpcConfig: Partial<RpcConfig> = {
      defaultTimeout: this.config.timeout,
      subscriptionPaths: new Set(this.config.subscriptionPaths ?? []),
    };

    const transport = this.transportProvider();

    return Layer.mergeAll(
      makeConfigLayer(rpcConfig),
      makeTransportLayer(transport),
      makeInterceptorLayer({
        interceptors: [...(this.config.interceptors ?? [])],
      }),
      makeLoggerLayer(
        this.config.debug
          ? {
              debug: (msg, data) => console.debug(`[RPC] ${msg}`, data ?? ""),
              info: (msg, data) => console.info(`[RPC] ${msg}`, data ?? ""),
              warn: (msg, data) => console.warn(`[RPC] ${msg}`, data ?? ""),
              error: (msg, data) => console.error(`[RPC] ${msg}`, data ?? ""),
            }
          : {
              debug: () => {},
              info: () => {},
              warn: () => {},
              error: () => {},
            }
      )
    );
  }

  /**
   * Get the Effect for making an RPC call.
   */
  call<T>(
    path: string,
    input: unknown,
    options?: CallOptions
  ): Effect.Effect<T, RpcEffectError, RpcServices> {
    return call<T>(path, input, options);
  }

  /**
   * Get the Effect for subscribing to a stream.
   */
  subscribe<T>(
    path: string,
    input: unknown,
    options?: SubscribeOptions
  ): Effect.Effect<AsyncIterable<T>, RpcEffectError, RpcServices> {
    return subscribe<T>(path, input, options);
  }

  /**
   * Run a call Effect and return a Promise.
   */
  async runCall<T>(
    path: string,
    input: unknown,
    options?: CallOptions
  ): Promise<T> {
    if (!this.layer) {
      this.layer = this.buildLayer();
    }
    const effect = pipe(
      this.call<T>(path, input, options),
      Effect.provide(this.layer)
    );
    return Effect.runPromise(effect);
  }

  /**
   * Run a subscribe Effect and return a Promise.
   */
  async runSubscribe<T>(
    path: string,
    input: unknown,
    options?: SubscribeOptions
  ): Promise<AsyncIterable<T>> {
    if (!this.layer) {
      this.layer = this.buildLayer();
    }
    const effect = pipe(
      this.subscribe<T>(path, input, options),
      Effect.provide(this.layer)
    );
    return Effect.runPromise(effect);
  }

  /**
   * Check if a path is a subscription.
   */
  isSubscription(path: string): boolean {
    return this.config.subscriptionPaths?.includes(path) ?? false;
  }

  /**
   * Get the service layer for custom Effect composition.
   */
  getLayer(): Layer.Layer<RpcServices> {
    if (!this.layer) {
      this.layer = this.buildLayer();
    }
    return this.layer;
  }

  /**
   * Create a new link with additional interceptors.
   */
  withInterceptors(
    interceptors: readonly RpcInterceptor[]
  ): EffectLink<TContext> {
    const newLink = new EffectLink({
      ...this.config,
      interceptors: [...(this.config.interceptors ?? []), ...interceptors],
    });
    if (this.transportProvider) {
      newLink.setTransport(this.transportProvider);
    }
    return newLink;
  }

  /**
   * Create a new link with a different timeout.
   */
  withTimeout(timeout: number): EffectLink<TContext> {
    const newLink = new EffectLink({
      ...this.config,
      timeout,
    });
    if (this.transportProvider) {
      newLink.setTransport(this.transportProvider);
    }
    return newLink;
  }
}
