// =============================================================================
// @tauri-nexus/rpc-core - TauriLink Implementation
// =============================================================================
// Configurable link for Tauri RPC calls with interceptor support.
// Uses Effect internally for type-safe error handling and composition.

import { Effect, pipe, Layer } from "effect";
import type { RpcError } from "../core/types";
import { callEffect, subscribeEffect } from "../internal/effect-call";
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
// Path Validation (standalone, no Effect services)
// =============================================================================

const PATH_REGEX = /^[a-zA-Z0-9_.]+$/;

function validatePathSync(path: string): void {
  if (!path) {
    throw {
      code: "VALIDATION_ERROR",
      message: "Procedure path cannot be empty",
    } as RpcError;
  }

  if (path.startsWith(".") || path.endsWith(".")) {
    throw {
      code: "VALIDATION_ERROR",
      message: "Procedure path cannot start or end with a dot",
    } as RpcError;
  }

  if (path.includes("..")) {
    throw {
      code: "VALIDATION_ERROR",
      message: "Procedure path cannot contain consecutive dots",
    } as RpcError;
  }

  if (!PATH_REGEX.test(path)) {
    throw {
      code: "VALIDATION_ERROR",
      message: "Procedure path contains invalid characters",
    } as RpcError;
  }
}

// =============================================================================
// TauriLink Class
// =============================================================================

/**
 * TauriLink - A configurable link for Tauri RPC calls.
 * Uses Effect internally for robust error handling and composition.
 *
 * @example
 * ```typescript
 * const link = new TauriLink<{ token?: string }>({
 *   interceptors: [
 *     async (ctx, next) => {
 *       console.log(`[RPC] ${ctx.path}`);
 *       return next();
 *     },
 *   ],
 * });
 *
 * const client = createClientFromLink<AppContract, { token?: string }>(link);
 * ```
 */
export class TauriLink<TClientContext = unknown> {
  private config: TauriLinkConfig<TClientContext>;
  private layer: Layer.Layer<RpcServices>;

  constructor(config: TauriLinkConfig<TClientContext> = {}) {
    this.config = config;
    this.layer = this.buildLayer();
  }

  /**
   * Build Effect layer from link configuration.
   */
  private buildLayer(): Layer.Layer<RpcServices> {
    // Convert link interceptors to Effect interceptors
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

  /**
   * Run an Effect with this link's layer.
   */
  private async runEffect<T>(
    effect: Effect.Effect<T, RpcEffectError, RpcServices>,
  ): Promise<T> {
    const provided = pipe(effect, Effect.provide(this.layer));
    return Effect.runPromise(provided);
  }

  /**
   * Make an RPC call.
   */
  async call<T>(
    path: string,
    input: unknown,
    options: LinkCallOptions<TClientContext> = {},
  ): Promise<T> {
    validatePathSync(path);

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
      // Lifecycle hook: before request
      await this.config.onRequest?.(ctx);

      const result = await this.runEffect(
        callEffect<T>(path, input, {
          signal: options.signal,
          timeout: timeoutMs,
          meta: {
            ...options.meta,
            clientContext: options.context,
          },
        }),
      );

      // Lifecycle hook: after response
      await this.config.onResponse?.(result, ctx);
      return result;
    } catch (error) {
      const rpcError = this.convertError(error, timeoutMs);
      await this.config.onError?.(rpcError, ctx);
      throw rpcError;
    }
  }

  /**
   * Subscribe to a streaming procedure.
   */
  async subscribe<T>(
    path: string,
    input: unknown,
    options: LinkSubscribeOptions<TClientContext> = {},
  ): Promise<AsyncIterable<T>> {
    validatePathSync(path);

    const ctx: LinkRequestContext<TClientContext> = {
      path,
      input,
      type: "subscription",
      context: options.context ?? ({} as TClientContext),
      signal: options.signal,
      meta: options.meta ?? {},
    };

    try {
      // Lifecycle hook: before request
      await this.config.onRequest?.(ctx);

      const iterator = await this.runEffect(
        subscribeEffect<T>(path, input, {
          signal: options.signal,
          lastEventId: options.lastEventId,
          meta: {
            ...options.meta,
            clientContext: options.context,
          },
        }),
      );

      return iterator;
    } catch (error) {
      const rpcError = this.convertError(error);
      await this.config.onError?.(rpcError, ctx);
      throw rpcError;
    }
  }

  /**
   * Check if path is a subscription.
   */
  isSubscription(path: string): boolean {
    return this.config.subscriptionPaths?.includes(path) ?? false;
  }

  /**
   * Get configuration.
   */
  getConfig(): TauriLinkConfig<TClientContext> {
    return this.config;
  }

  /**
   * Convert errors to public RpcError format.
   */
  private convertError(error: unknown, timeoutMs?: number): RpcError {
    // Effect-based errors
    if (this.isEffectError(error)) {
      return toPublicError(error);
    }

    // Already an RpcError
    if (this.isRpcError(error)) {
      return error;
    }

    // Parse unknown errors
    return toPublicError(parseEffectError(error, "unknown", timeoutMs));
  }

  /**
   * Check if error is an Effect-based RPC error.
   */
  private isEffectError(error: unknown): error is RpcEffectError {
    return (
      typeof error === "object" &&
      error !== null &&
      "_tag" in error &&
      typeof (error as { _tag: string })._tag === "string"
    );
  }

  /**
   * Check if error is already an RpcError.
   */
  private isRpcError(error: unknown): error is RpcError {
    return (
      typeof error === "object" &&
      error !== null &&
      "code" in error &&
      "message" in error &&
      typeof (error as RpcError).code === "string" &&
      typeof (error as RpcError).message === "string"
    );
  }
}
