// =============================================================================
// @tauri-nexus/rpc-core - TauriLink Implementation
// =============================================================================
// Configurable link for Tauri RPC calls with interceptor support.

import { Effect, pipe, Layer } from "effect";
import {
  callEffect,
  subscribeEffect,
  validatePath,
  toRpcError,
  parseEffectError,
  RpcConfigService,
  RpcInterceptorService,
  RpcLoggerService,
  TauriTransportLayer,
  type RpcServices,
  type RpcEffectError,
  type RpcInterceptor,
  type InterceptorContext,
} from "../internal";
import type {
  TauriLinkConfig,
  LinkRequestContext,
  LinkCallOptions,
  LinkSubscribeOptions,
} from "./types";

// =============================================================================
// TauriLink Class
// =============================================================================

/**
 * TauriLink - A configurable link for Tauri RPC calls.
 * Provides interceptor support and lifecycle hooks.
 */
export class TauriLink<TClientContext = unknown> {
  private readonly config: TauriLinkConfig<TClientContext>;
  private readonly layer: Layer.Layer<RpcServices>;

  constructor(config: TauriLinkConfig<TClientContext> = {}) {
    this.config = config;
    this.layer = this.buildLayer();
  }

  // ===========================================================================
  // Private Methods
  // ===========================================================================

  private buildLayer(): Layer.Layer<RpcServices> {
    const effectInterceptors: RpcInterceptor[] = (
      this.config.interceptors ?? []
    ).map((interceptor, index) => ({
      name: `link-interceptor-${index}`,
      intercept: async <T>(ctx: InterceptorContext, next: () => Promise<T>) => {
        const linkCtx: LinkRequestContext<TClientContext> = {
          path: ctx.path,
          input: ctx.input,
          type: ctx.type,
          context: (ctx.meta.clientContext ?? {}) as TClientContext,
          signal: ctx.signal,
          meta: ctx.meta,
        };
        return interceptor(linkCtx, next);
      },
    }));

    return Layer.mergeAll(
      RpcConfigService.layer({
        defaultTimeout: this.config.timeout,
        subscriptionPaths: new Set(this.config.subscriptionPaths ?? []),
      }),
      TauriTransportLayer,
      RpcInterceptorService.withInterceptors(effectInterceptors),
      RpcLoggerService.Default,
    );
  }

  private provideLayer<T>(
    effect: Effect.Effect<T, RpcEffectError, RpcServices>,
  ): Effect.Effect<T, RpcEffectError> {
    return pipe(effect, Effect.provide(this.layer));
  }

  private async runEffect<T>(
    effect: Effect.Effect<T, RpcEffectError>,
    path: string,
    timeoutMs?: number,
  ): Promise<T> {
    try {
      return await Effect.runPromise(effect);
    } catch (error) {
      throw toRpcError(parseEffectError(error, path, timeoutMs));
    }
  }

  private createRequestContext(
    path: string,
    input: unknown,
    type: "query" | "mutation" | "subscription",
    options: LinkCallOptions<TClientContext>,
  ): LinkRequestContext<TClientContext> {
    return {
      path,
      input,
      type,
      context: options.context ?? ({} as TClientContext),
      signal: options.signal,
      meta: options.meta ?? {},
    };
  }

  private callWithLifecycle<T>(
    path: string,
    input: unknown,
    options: LinkCallOptions<TClientContext>,
  ): Effect.Effect<T, RpcEffectError, RpcServices> {
    const timeoutMs = options.timeout ?? this.config.timeout;
    const ctx = this.createRequestContext(path, input, "query", options);

    return Effect.gen(this, function* () {
      yield* validatePath(path);

      if (this.config.onRequest) {
        yield* Effect.promise(() =>
          Promise.resolve(this.config.onRequest!(ctx)),
        );
      }

      const result = yield* pipe(
        callEffect<T>(path, input, {
          signal: options.signal,
          timeout: timeoutMs,
          meta: { ...options.meta, clientContext: options.context },
        }),
        Effect.tapError((error) =>
          Effect.promise(() =>
            Promise.resolve(this.config.onError?.(toRpcError(error), ctx)),
          ),
        ),
      );

      if (this.config.onResponse) {
        yield* Effect.promise(() =>
          Promise.resolve(this.config.onResponse!(result, ctx)),
        );
      }

      return result;
    });
  }

  private subscribeWithLifecycle<T>(
    path: string,
    input: unknown,
    options: LinkSubscribeOptions<TClientContext>,
  ): Effect.Effect<AsyncIterable<T>, RpcEffectError, RpcServices> {
    const ctx = this.createRequestContext(path, input, "subscription", options);

    return Effect.gen(this, function* () {
      yield* validatePath(path);

      if (this.config.onRequest) {
        yield* Effect.promise(() =>
          Promise.resolve(this.config.onRequest!(ctx)),
        );
      }

      const iterator = yield* pipe(
        subscribeEffect<T>(path, input, {
          signal: options.signal,
          lastEventId: options.lastEventId,
          meta: { ...options.meta, clientContext: options.context },
        }),
        Effect.tapError((error) =>
          Effect.promise(() =>
            Promise.resolve(this.config.onError?.(toRpcError(error), ctx)),
          ),
        ),
      );

      return iterator;
    });
  }

  // ===========================================================================
  // Public API
  // ===========================================================================

  /**
   * Make an RPC call.
   */
  async call<T>(
    path: string,
    input: unknown,
    options: LinkCallOptions<TClientContext> = {},
  ): Promise<T> {
    const timeoutMs = options.timeout ?? this.config.timeout;
    const effect = this.provideLayer(
      this.callWithLifecycle<T>(path, input, options),
    );
    return this.runEffect(effect, path, timeoutMs);
  }

  /**
   * Subscribe to a streaming procedure.
   */
  async subscribe<T>(
    path: string,
    input: unknown,
    options: LinkSubscribeOptions<TClientContext> = {},
  ): Promise<AsyncIterable<T>> {
    const effect = this.provideLayer(
      this.subscribeWithLifecycle<T>(path, input, options),
    );
    return this.runEffect(effect, path);
  }

  /**
   * Check if a path is a subscription.
   */
  isSubscription(path: string): boolean {
    return this.config.subscriptionPaths?.includes(path) ?? false;
  }

  /**
   * Get the current configuration.
   */
  getConfig(): TauriLinkConfig<TClientContext> {
    return this.config;
  }

  /**
   * Get the Effect layer for advanced usage.
   */
  getLayer(): Layer.Layer<RpcServices> {
    return this.layer;
  }

  /**
   * Create a new link with additional interceptors.
   */
  withInterceptors(
    interceptors: TauriLinkConfig<TClientContext>["interceptors"],
  ): TauriLink<TClientContext> {
    return new TauriLink({
      ...this.config,
      interceptors: [
        ...(this.config.interceptors ?? []),
        ...(interceptors ?? []),
      ],
    });
  }

  /**
   * Create a new link with a different timeout.
   */
  withTimeout(timeout: number): TauriLink<TClientContext> {
    return new TauriLink({ ...this.config, timeout });
  }
}
