// =============================================================================
// Logging Interceptor
// =============================================================================

import type { RpcInterceptor, InterceptorContext } from "../core/types";
import type { InterceptorOptions } from "./factory";

export interface LoggingInterceptorOptions extends InterceptorOptions {
  readonly prefix?: string;
}

export function loggingInterceptor(
  options: LoggingInterceptorOptions = {},
): RpcInterceptor {
  const prefix = options.prefix ?? "[RPC]";

  return {
    name: options.name ?? "logging",
    intercept: async <T>(ctx: InterceptorContext, next: () => Promise<T>) => {
      const start = Date.now();
      console.log(`${prefix} → ${ctx.path}`, ctx.input);

      try {
        const result = await next();
        const duration = Date.now() - start;
        console.log(`${prefix} ← ${ctx.path} (${duration}ms)`, result);
        return result;
      } catch (error) {
        const duration = Date.now() - start;
        console.error(`${prefix} ✗ ${ctx.path} (${duration}ms)`, error);
        throw error;
      }
    },
  };
}
