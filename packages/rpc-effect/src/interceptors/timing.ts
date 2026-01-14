// =============================================================================
// Timing Interceptor
// =============================================================================

import type { RpcInterceptor, InterceptorContext } from "../core/types";
import type { InterceptorOptions } from "./factory";

export function timingInterceptor(
  onTiming: (path: string, durationMs: number) => void,
  options: InterceptorOptions = {},
): RpcInterceptor {
  return {
    name: options.name ?? "timing",
    intercept: async <T>(ctx: InterceptorContext, next: () => Promise<T>) => {
      const start = Date.now();
      try {
        return await next();
      } finally {
        onTiming(ctx.path, Date.now() - start);
      }
    },
  };
}
