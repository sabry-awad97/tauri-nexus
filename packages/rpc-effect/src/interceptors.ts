// =============================================================================
// @tauri-nexus/rpc-effect - Pre-built Interceptors
// =============================================================================
// Common interceptors for logging, retry, error handling, and authentication.

import type { RpcInterceptor, InterceptorContext } from "./types";

// =============================================================================
// Types
// =============================================================================

export interface InterceptorOptions {
  readonly name?: string;
}

export interface RetryInterceptorOptions extends InterceptorOptions {
  readonly maxRetries?: number;
  readonly delay?: number;
  readonly backoff?: "linear" | "exponential";
  readonly retryOn?: (error: unknown) => boolean;
}

export interface AuthInterceptorOptions extends InterceptorOptions {
  readonly getToken: () => string | null | Promise<string | null>;
  readonly headerName?: string;
  readonly prefix?: string;
}

// =============================================================================
// Logging Interceptor
// =============================================================================

export function loggingInterceptor(
  options: InterceptorOptions & { prefix?: string } = {},
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

// =============================================================================
// Retry Interceptor
// =============================================================================

export function retryInterceptor(
  options: RetryInterceptorOptions = {},
): RpcInterceptor {
  const { maxRetries = 3, delay = 1000, backoff = "linear", retryOn } = options;

  const shouldRetry = (error: unknown): boolean => {
    if (retryOn) return retryOn(error);

    if (typeof error === "object" && error !== null && "code" in error) {
      const code = (error as { code: string }).code;
      return ![
        "VALIDATION_ERROR",
        "UNAUTHORIZED",
        "FORBIDDEN",
        "CANCELLED",
        "BAD_REQUEST",
      ].includes(code);
    }
    return true;
  };

  const getDelay = (attempt: number): number => {
    if (backoff === "exponential") {
      return delay * Math.pow(2, attempt);
    }
    return delay * (attempt + 1);
  };

  return {
    name: options.name ?? "retry",
    intercept: async <T>(_ctx: InterceptorContext, next: () => Promise<T>) => {
      let lastError: unknown;

      for (let attempt = 0; attempt <= maxRetries; attempt++) {
        try {
          return await next();
        } catch (error) {
          lastError = error;

          if (!shouldRetry(error) || attempt === maxRetries) {
            throw error;
          }

          const retryDelay = getDelay(attempt);
          await sleep(retryDelay);
        }
      }

      throw lastError;
    },
  };
}

// =============================================================================
// Error Handler Interceptor
// =============================================================================

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

// =============================================================================
// Auth Interceptor
// =============================================================================

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

// =============================================================================
// Timing Interceptor
// =============================================================================

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

// =============================================================================
// Deduplication Interceptor
// =============================================================================

export function dedupeInterceptor(
  options: InterceptorOptions & {
    getKey?: (ctx: InterceptorContext) => string;
  } = {},
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

// =============================================================================
// Helpers
// =============================================================================

const sleep = (ms: number): Promise<void> =>
  new Promise((resolve) => setTimeout(resolve, ms));
