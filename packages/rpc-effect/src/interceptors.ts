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
// Generic Interceptor Factory
// =============================================================================

/**
 * Handler function type for interceptor logic.
 */
export type InterceptorHandler<TOptions> = (
  options: TOptions
) => <T>(ctx: InterceptorContext, next: () => Promise<T>) => Promise<T>;

/**
 * Create an interceptor factory with typed options.
 * Reduces boilerplate when creating custom interceptors.
 *
 * @example
 * const myInterceptor = createInterceptorFactory(
 *   "myInterceptor",
 *   (options: { prefix: string }) => async (ctx, next) => {
 *     console.log(options.prefix, ctx.path);
 *     return next();
 *   }
 * );
 *
 * const interceptor = myInterceptor({ prefix: "[API]" });
 */
export function createInterceptorFactory<TOptions extends InterceptorOptions>(
  defaultName: string,
  handler: InterceptorHandler<TOptions>
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
  intercept: <T>(ctx: InterceptorContext, next: () => Promise<T>) => Promise<T>
): RpcInterceptor {
  return { name, intercept };
}

/**
 * Compose multiple interceptors into a single interceptor.
 */
export function composeInterceptors(
  name: string,
  interceptors: readonly RpcInterceptor[]
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

// =============================================================================
// Helpers (Internal)
// =============================================================================

const wait = (ms: number): Promise<void> =>
  new Promise((resolve) => setTimeout(resolve, ms));

/** Error codes that should NOT be retried */
const NON_RETRYABLE_CODES = [
  "VALIDATION_ERROR",
  "UNAUTHORIZED",
  "FORBIDDEN",
  "CANCELLED",
  "BAD_REQUEST",
] as const;

const defaultShouldRetry = (error: unknown): boolean => {
  if (typeof error === "object" && error !== null && "code" in error) {
    const code = (error as { code: string }).code;
    return !NON_RETRYABLE_CODES.includes(
      code as (typeof NON_RETRYABLE_CODES)[number]
    );
  }
  return true;
};

// =============================================================================
// Logging Interceptor
// =============================================================================

export function loggingInterceptor(
  options: InterceptorOptions & { prefix?: string } = {}
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
  options: RetryInterceptorOptions = {}
): RpcInterceptor {
  const { maxRetries = 3, delay = 1000, backoff = "linear", retryOn } = options;

  const shouldRetry = retryOn ?? defaultShouldRetry;

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

          await wait(getDelay(attempt));
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
  options: InterceptorOptions = {}
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
  options: AuthInterceptorOptions
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
  options: InterceptorOptions = {}
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
  } = {}
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
// Convenience Aliases
// =============================================================================

export const createLoggingInterceptor = (prefix = "[RPC]"): RpcInterceptor =>
  loggingInterceptor({ prefix });

export const createRetryInterceptor = (options: {
  maxRetries?: number;
  delay?: number;
  retryOn?: (error: unknown) => boolean;
}): RpcInterceptor =>
  retryInterceptor({
    maxRetries: options.maxRetries,
    delay: options.delay,
    retryOn: options.retryOn,
  });

export const createErrorInterceptor = (
  handler: (error: unknown, ctx: InterceptorContext) => void
): RpcInterceptor => errorHandlerInterceptor(handler);

export const createAuthInterceptor = (
  getToken: () => string | null | Promise<string | null>
): RpcInterceptor => authInterceptor({ getToken });
