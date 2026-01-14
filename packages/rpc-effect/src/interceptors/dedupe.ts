// =============================================================================
// Deduplication Interceptor
// =============================================================================

import type { RpcInterceptor, InterceptorContext } from "../core/types";
import type { InterceptorOptions } from "./factory";

export interface DedupeInterceptorOptions extends InterceptorOptions {
  readonly getKey?: (ctx: InterceptorContext) => string;
}

export function dedupeInterceptor(
  options: DedupeInterceptorOptions = {},
): RpcInterceptor {
  const pending = new Map<string, Promise<unknown>>();

  const getKey =
    options.getKey ??
    ((ctx: InterceptorContext) => `${ctx.path}:${JSON.stringify(ctx.input)}`);

  return {
    name: options.name ?? "dedupe",
    intercept: async <T>(ctx: InterceptorContext, next: () => Promise<T>) => {
      const key = getKey(ctx);

      const existing = pending.get(key);
      if (existing) {
        return existing as Promise<T>;
      }

      const promise = next().finally(() => {
        pending.delete(key);
      });

      pending.set(key, promise);
      return promise;
    },
  };
}
