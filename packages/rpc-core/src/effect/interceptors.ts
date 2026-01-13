// =============================================================================
// @tauri-nexus/rpc-core - Public Interceptor Factories
// =============================================================================
// Pre-built interceptors for common use cases.

import type { RpcInterceptor, InterceptorContext } from "../internal/effect-types";

// =============================================================================
// Types
// =============================================================================

/**
 * Base interceptor options.
 */
export interface InterceptorOptions {
  /** Name for debugging */
  readonly name?: string;
}

/**
 * Options for the retry interceptor.
 */
export interface RetryInterceptorOptions extends InterceptorOptions {
  /** Maximum number of retries (default: 3) */
  readonly maxRetries?: number;
  /** Base delay in milliseconds (default: 1000) */
  readonly delay?: number;
  /** Backoff strategy (default: 'linear') */
  readonly backoff?: "linear" | "exponential";
  /** Custom retry condition */
  readonly retryOn?: (error: unknown) => boolean;
}

/**
 * Options for the auth interceptor.
 */
export interface AuthInterceptorOptions extends InterceptorOptions {
  /** Function to get the auth token */
  readonly getToken: () => string | null | Promise<string | null>;
  /** Header name (default: 'authorization') */
  readonly headerName?: string;
  /** Token prefix (default: 'Bearer') */
  readonly prefix?: string;
}

// =============================================================================
// Logging Interceptor
// =============================================================================

/**
 * Create a logging interceptor that logs all RPC calls.
 *
 * @example
 * ```typescript
 * const rpc = createEffectClient<AppContract>({
 *   interceptors: [loggingInterceptor({ prefix: '[MyApp]' })],
 * });
 * ```
 */
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

/**
 * Create a retry interceptor with configurable backoff.
 *
 * @example
 * ```typescript
 * const rpc = createEffectClient<AppContract>({
 *   interceptors: [
 *     retryInterceptor({
 *       maxRetries: 3,
 *       delay: 1000,
 *       backoff: 'exponential',
 *     }),
 *   ],
 * });
 * ```
 */
export function retryInterceptor(
  options: RetryInterceptorOptions = {},
): RpcInterceptor {
  const {
    maxRetries = 3,
    delay = 1000,
    backoff = "linear",
    retryOn,
  } = options;

  const shouldRetry = (error: unknown): boolean => {
    if (retryOn) return retryOn(error);

    // Default: don't retry validation, auth, or cancellation errors
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

/**
 * Create an error handler interceptor for logging or analytics.
 *
 * @example
 * ```typescript
 * const rpc = createEffectClient<AppContract>({
 *   interceptors: [
 *     errorHandlerInterceptor((error, ctx) => {
 *       analytics.track('rpc_error', {
 *         path: ctx.path,
 *         error: error.code,
 *       });
 *     }),
 *   ],
 * });
 * ```
 */
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

/**
 * Create an auth interceptor that adds authentication to requests.
 *
 * @example
 * ```typescript
 * const rpc = createEffectClient<AppContract>({
 *   interceptors: [
 *     authInterceptor({
 *       getToken: () => localStorage.getItem('token'),
 *     }),
 *   ],
 * });
 * ```
 */
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

/**
 * Create a timing interceptor that tracks request duration.
 *
 * @example
 * ```typescript
 * const rpc = createEffectClient<AppContract>({
 *   interceptors: [
 *     timingInterceptor((path, duration) => {
 *       metrics.recordLatency(path, duration);
 *     }),
 *   ],
 * });
 * ```
 */
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

/**
 * Create a deduplication interceptor that prevents duplicate concurrent requests.
 *
 * @example
 * ```typescript
 * const rpc = createEffectClient<AppContract>({
 *   interceptors: [dedupeInterceptor()],
 * });
 *
 * // These will share the same request
 * const [user1, user2] = await Promise.all([
 *   rpc.user.get({ id: 1 }),
 *   rpc.user.get({ id: 1 }),
 * ]);
 * ```
 */
export function dedupeInterceptor(
  options: InterceptorOptions & {
    /** Custom key generator */
    getKey?: (ctx: InterceptorContext) => string;
  } = {},
): RpcInterceptor {
  const pending = new Map<string, Promise<unknown>>();

  const getKey =
    options.getKey ??
    ((_ctx: InterceptorContext) =>
      `${_ctx.path}:${JSON.stringify(_ctx.input)}`);

  return {
    name: options.name ?? "dedupe",
    intercept: async <T>(_ctx: InterceptorContext, next: () => Promise<T>) => {
      const key = getKey(_ctx);

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
