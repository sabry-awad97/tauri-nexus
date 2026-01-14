// =============================================================================
// Error Handler Interceptor
// =============================================================================

import type { RpcInterceptor, InterceptorContext } from "../core/types";
import type { InterceptorOptions } from "./factory";

export function errorHandlerInterceptor(
  handler: (error: unknown, ctx: InterceptorContext) => void | Promise<void>,
  options: InterceptorOptions = {},
): RpcInterceptor {
  return {
    name: options.name ?? "errorHandler",
    intercept: async <T>(ctx: InterceptorContext, next: () => Promise<T>) => {
      try {
        return await next();
      } catch (error) {
        await handler(error, ctx);
        throw error;
      }
    },
  };
}
