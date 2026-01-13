// =============================================================================
// @tauri-nexus/rpc-core - Public Effect Link
// =============================================================================
// A simplified Effect-based link for advanced users who want more control.

import { Effect, pipe } from "effect";
import { EffectLink } from "../internal/effect-link";
import { toPublicError } from "../internal/effect-errors";
import type { RpcInterceptor, RpcEffectError } from "../internal/effect-types";
import type { RpcError } from "../core/types";

// =============================================================================
// Configuration
// =============================================================================

/**
 * Configuration for the public Effect RPC link.
 */
export interface EffectRpcLinkConfig {
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
// Public Effect Link
// =============================================================================

/**
 * EffectRpcLink - A composable link for advanced Effect-based usage.
 *
 * Use this when you want to compose RPC calls with other Effect operations,
 * or when you need fine-grained control over error handling.
 *
 * @example
 * ```typescript
 * import { EffectRpcLink } from '@tauri-nexus/rpc-core/effect';
 * import { Effect, pipe } from 'effect';
 *
 * const link = new EffectRpcLink({
 *   subscriptionPaths: ['stream.events'],
 *   timeout: 5000,
 * });
 *
 * // Get raw Effect for composition
 * const getUserEffect = link.call<User>('user.get', { id: 1 });
 *
 * // Compose with other Effects
 * const program = pipe(
 *   getUserEffect,
 *   Effect.flatMap(user => updateCache(user)),
 *   Effect.catchTag('RpcTimeoutError', () => Effect.succeed(cachedUser)),
 * );
 *
 * // Or use the simple Promise API
 * const user = await link.execute('user.get', { id: 1 });
 * ```
 */
export class EffectRpcLink {
  private readonly internal: EffectLink;

  constructor(config: EffectRpcLinkConfig = {}) {
    this.internal = new EffectLink(config);
  }

  /**
   * Get an Effect for making an RPC call.
   * Use this for Effect composition.
   */
  call<T>(
    path: string,
    input?: unknown,
    options?: { signal?: AbortSignal; timeout?: number },
  ): Effect.Effect<T, RpcEffectError> {
    return pipe(
      this.internal.call<T>(path, input, options),
      Effect.provide(this.internal.getLayer()),
    );
  }

  /**
   * Get an Effect for subscribing to a stream.
   * Use this for Effect composition.
   */
  subscribe<T>(
    path: string,
    input?: unknown,
    options?: { signal?: AbortSignal; lastEventId?: string },
  ): Effect.Effect<AsyncIterable<T>, RpcEffectError> {
    return pipe(
      this.internal.subscribe<T>(path, input, options),
      Effect.provide(this.internal.getLayer()),
    );
  }

  /**
   * Execute an RPC call and return a Promise.
   * Errors are converted to the public RpcError format.
   */
  async execute<T>(
    path: string,
    input?: unknown,
    options?: { signal?: AbortSignal; timeout?: number },
  ): Promise<T> {
    try {
      return await this.internal.runCall<T>(path, input, options);
    } catch (error) {
      throw this.convertError(error);
    }
  }

  /**
   * Execute a subscription and return a Promise.
   * Errors are converted to the public RpcError format.
   */
  async executeSubscribe<T>(
    path: string,
    input?: unknown,
    options?: { signal?: AbortSignal; lastEventId?: string },
  ): Promise<AsyncIterable<T>> {
    try {
      return await this.internal.runSubscribe<T>(path, input, options);
    } catch (error) {
      throw this.convertError(error);
    }
  }

  /**
   * Check if a path is a subscription.
   */
  isSubscription(path: string): boolean {
    return this.internal.isSubscription(path);
  }

  /**
   * Create a new link with additional interceptors.
   */
  withInterceptors(interceptors: readonly RpcInterceptor[]): EffectRpcLink {
    const newInternal = this.internal.withInterceptors(interceptors);
    const link = new EffectRpcLink();
    (link as unknown as { internal: EffectLink }).internal = newInternal;
    return link;
  }

  /**
   * Create a new link with a different timeout.
   */
  withTimeout(timeout: number): EffectRpcLink {
    const newInternal = this.internal.withTimeout(timeout);
    const link = new EffectRpcLink();
    (link as unknown as { internal: EffectLink }).internal = newInternal;
    return link;
  }

  /**
   * Convert internal errors to public format.
   */
  private convertError(error: unknown): RpcError {
    if (typeof error === "object" && error !== null && "_tag" in error) {
      return toPublicError(error as Parameters<typeof toPublicError>[0]);
    }

    // Already a public error or unknown
    if (
      typeof error === "object" &&
      error !== null &&
      "code" in error &&
      "message" in error
    ) {
      return error as RpcError;
    }

    return {
      code: "UNKNOWN",
      message: String(error),
    };
  }
}
