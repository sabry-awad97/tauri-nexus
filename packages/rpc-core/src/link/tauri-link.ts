// =============================================================================
// @tauri-nexus/rpc-core - TauriLink Implementation
// =============================================================================
// Configurable link for Tauri RPC calls with interceptor support.
// Uses Effect throughout for type-safe error handling and composition.

import { Effect, pipe, Layer } from "effect";
import {
  callEffect,
  subscribeEffect,
  validatePath,
} from "../internal/effect-call";
import { toPublicError, parseEffectError } from "../internal/effect-errors";
import {
  makeConfigLayer,
  makeInterceptorLayer,
  makeLoggerLayer,
  TauriTransportLayer,
  type RpcServices,
} from "../internal/effect-runtime";
import type {
  RpcEffectError,
  RpcInterceptor,
  InterceptorContext,
} from "../internal/effect-types";
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
 * Uses Effect throughout for robust error handling and composition.
 */
export class TauriLink<TClientContext = unknown> {
  private config: TauriLinkConfig<TClientContext>;
  private layer: Layer.Layer<RpcServices>;

  constructor(config: TauriLinkConfig<TClientContext> = {}) {
    this.config = config;
    this.layer = this.buildLayer();
  }

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
      makeConfigLayer({
        defaultTimeout: this.config.timeout,
        subscriptionPaths: new Set(this.config.subscriptionPaths ?? []),
      }),
      TauriTransportLayer,
      makeInterceptorLayer({ interceptors: effectInterceptors }),
      makeLoggerLayer(),
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
      throw toPublicError(parseEffectError(error, path, timeoutMs));
    }
  }

  private callWithLifecycle<T>(
    path: string,
    input: unknown,
    options: LinkCallOptions<TClientContext>,
    link: TauriLink<TClientContext>,
  ): Effect.Effect<T, RpcEffectError, RpcServices> {
    const timeoutMs = options.timeout ?? link.config.timeout;

    return Effect.gen(function* () {
      yield* validatePath(path);

      const ctx: LinkRequestContext<TClientContext> = {
        path,
        input,
        type: "query",
        context: options.context ?? ({} as TClientContext),
        signal: options.signal,
        meta: options.meta ?? {},
      };

      if (link.config.onRequest) {
        yield* Effect.promise(() =>
          Promise.resolve(link.config.onRequest!(ctx)),
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
            Promise.resolve(link.config.onError?.(toPublicError(error), ctx)),
          ),
        ),
      );

      if (link.config.onResponse) {
        yield* Effect.promise(() =>
          Promise.resolve(link.config.onResponse!(result, ctx)),
        );
      }

      return result;
    });
  }

  private subscribeWithLifecycle<T>(
    path: string,
    input: unknown,
    options: LinkSubscribeOptions<TClientContext>,
    link: TauriLink<TClientContext>,
  ): Effect.Effect<AsyncIterable<T>, RpcEffectError, RpcServices> {
    return Effect.gen(function* () {
      yield* validatePath(path);

      const ctx: LinkRequestContext<TClientContext> = {
        path,
        input,
        type: "subscription",
        context: options.context ?? ({} as TClientContext),
        signal: options.signal,
        meta: options.meta ?? {},
      };

      if (link.config.onRequest) {
        yield* Effect.promise(() =>
          Promise.resolve(link.config.onRequest!(ctx)),
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
            Promise.resolve(link.config.onError?.(toPublicError(error), ctx)),
          ),
        ),
      );

      return iterator;
    });
  }

  async call<T>(
    path: string,
    input: unknown,
    options: LinkCallOptions<TClientContext> = {},
  ): Promise<T> {
    const timeoutMs = options.timeout ?? this.config.timeout;
    const effect = this.provideLayer(
      this.callWithLifecycle<T>(path, input, options, this),
    );
    return this.runEffect(effect, path, timeoutMs);
  }

  async subscribe<T>(
    path: string,
    input: unknown,
    options: LinkSubscribeOptions<TClientContext> = {},
  ): Promise<AsyncIterable<T>> {
    const effect = this.provideLayer(
      this.subscribeWithLifecycle<T>(path, input, options, this),
    );
    return this.runEffect(effect, path);
  }

  isSubscription(path: string): boolean {
    return this.config.subscriptionPaths?.includes(path) ?? false;
  }

  getConfig(): TauriLinkConfig<TClientContext> {
    return this.config;
  }

  getLayer(): Layer.Layer<RpcServices> {
    return this.layer;
  }

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

  withTimeout(timeout: number): TauriLink<TClientContext> {
    return new TauriLink({ ...this.config, timeout });
  }
}
