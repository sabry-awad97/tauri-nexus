// =============================================================================
// Interceptor Factory
// =============================================================================
// Generic factory for creating interceptors.

import type { RpcInterceptor, InterceptorContext } from "../core/types";

// =============================================================================
// Types
// =============================================================================

export interface InterceptorOptions {
  readonly name?: string;
}

/** Handler function type for interceptor logic */
export type InterceptorHandler<TOptions> = (
  options: TOptions,
) => <T>(ctx: InterceptorContext, next: () => Promise<T>) => Promise<T>;

// =============================================================================
// Factory Functions
// =============================================================================

/**
 * Create an interceptor factory with typed options.
 *
 * @example
 * const myInterceptor = createInterceptorFactory(
 *   "myInterceptor",
 *   (options: { prefix: string }) => async (ctx, next) => {
 *     console.log(options.prefix, ctx.path);
 *     return next();
 *   }
 * );
 */
export function createInterceptorFactory<TOptions extends InterceptorOptions>(
  defaultName: string,
  handler: InterceptorHandler<TOptions>,
): (options: TOptions) => RpcInterceptor {
  return (options: TOptions): RpcInterceptor => ({
    name: options.name ?? defaultName,
    intercept: handler(options),
  });
}

/**
 * Create a simple interceptor without options.
 */
export function createSimpleInterceptor(
  name: string,
  intercept: <T>(ctx: InterceptorContext, next: () => Promise<T>) => Promise<T>,
): RpcInterceptor {
  return { name, intercept };
}

/**
 * Compose multiple interceptors into a single interceptor.
 */
export function composeInterceptors(
  name: string,
  interceptors: readonly RpcInterceptor[],
): RpcInterceptor {
  return {
    name,
    intercept: async <T>(ctx: InterceptorContext, next: () => Promise<T>) => {
      let current = next;
      for (let i = interceptors.length - 1; i >= 0; i--) {
        const interceptor = interceptors[i];
        const prev = current;
        current = () => interceptor.intercept(ctx, prev);
      }
      return current();
    },
  };
}
