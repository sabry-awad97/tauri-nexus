// =============================================================================
// RPC Interceptor Service
// =============================================================================

import { Context, Layer } from "effect";
import type { RpcInterceptorChain, RpcInterceptor } from "../core/types";

const defaultInterceptorChain: RpcInterceptorChain = {
  interceptors: [],
};

/**
 * Interceptor chain service for middleware-like functionality.
 *
 * @example
 * ```ts
 * // Use default (empty chain)
 * Effect.provide(program, RpcInterceptorService.Default)
 *
 * // With interceptors
 * Effect.provide(program, RpcInterceptorService.withInterceptors([
 *   loggingInterceptor(),
 *   retryInterceptor()
 * ]))
 * ```
 */
export class RpcInterceptorService extends Context.Tag("RpcInterceptorService")<
  RpcInterceptorService,
  RpcInterceptorChain
>() {
  /** Default layer with empty interceptor chain */
  static Default = Layer.succeed(
    RpcInterceptorService,
    defaultInterceptorChain,
  );

  /** Create a layer with custom interceptor chain */
  static layer(chain: RpcInterceptorChain) {
    return Layer.succeed(RpcInterceptorService, chain);
  }

  /** Create a layer with interceptor array */
  static withInterceptors(interceptors: readonly RpcInterceptor[]) {
    return Layer.succeed(RpcInterceptorService, { interceptors });
  }
}
