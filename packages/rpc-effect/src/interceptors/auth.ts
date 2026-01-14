// =============================================================================
// Auth Interceptor
// =============================================================================

import type { RpcInterceptor, InterceptorContext } from "../core/types";
import type { InterceptorOptions } from "./factory";

export interface AuthInterceptorOptions extends InterceptorOptions {
  readonly getToken: () => string | null | Promise<string | null>;
  readonly headerName?: string;
  readonly prefix?: string;
}

export function authInterceptor(
  options: AuthInterceptorOptions,
): RpcInterceptor {
  const { getToken, headerName = "authorization", prefix = "Bearer" } = options;

  return {
    name: options.name ?? "auth",
    intercept: async <T>(ctx: InterceptorContext, next: () => Promise<T>) => {
      const token = await getToken();

      if (token) {
        ctx.meta[headerName] = prefix ? `${prefix} ${token}` : token;
      }

      return next();
    },
  };
}
