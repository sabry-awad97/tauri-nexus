// =============================================================================
// @tauri-nexus/rpc-core - Built-in Interceptors
// =============================================================================
// Common interceptors for logging, retry, error handling, and authentication.

import type { RpcError } from "../core/types";
import { isRpcError } from "../core/errors";
import type { LinkInterceptor, LinkRequestContext } from "./types";

// =============================================================================
// Error Handler Interceptor
// =============================================================================

/**
 * Create an error handler interceptor.
 *
 * @example
 * ```typescript
 * const link = new TauriLink({
 *   interceptors: [
 *     onError((error) => {
 *       console.error('RPC Error:', error);
 *     }),
 *   ],
 * });
 * ```
 */
export function onError<TClientContext = unknown>(
  handler: (error: RpcError, ctx: LinkRequestContext<TClientContext>) => void,
): LinkInterceptor<TClientContext> {
  return async (ctx, next) => {
    try {
      return await next();
    } catch (error) {
      if (isRpcError(error)) {
        handler(error, ctx);
      }
      throw error;
    }
  };
}

// =============================================================================
// Logging Interceptor
// =============================================================================

/**
 * Create a logging interceptor.
 *
 * @example
 * ```typescript
 * const link = new TauriLink({
 *   interceptors: [logging({ prefix: '[RPC]' })],
 * });
 * ```
 */
export function logging<TClientContext = unknown>(
  options: { prefix?: string } = {},
): LinkInterceptor<TClientContext> {
  const prefix = options.prefix ?? "[RPC]";
  return async (ctx, next) => {
    const start = performance.now();
    console.log(`${prefix} ${ctx.path}`, ctx.input);
    try {
      const result = await next();
      const duration = (performance.now() - start).toFixed(2);
      console.log(`${prefix} ${ctx.path} completed in ${duration}ms`);
      return result;
    } catch (error) {
      const duration = (performance.now() - start).toFixed(2);
      console.error(`${prefix} ${ctx.path} failed in ${duration}ms`, error);
      throw error;
    }
  };
}

// =============================================================================
// Retry Interceptor
// =============================================================================

/**
 * Create a retry interceptor.
 *
 * @example
 * ```typescript
 * const link = new TauriLink({
 *   interceptors: [
 *     retry({ maxRetries: 3, delay: 1000 }),
 *   ],
 * });
 * ```
 */
export function retry<TClientContext = unknown>(
  options: {
    maxRetries?: number;
    delay?: number;
    shouldRetry?: (error: RpcError) => boolean;
  } = {},
): LinkInterceptor<TClientContext> {
  const maxRetries = options.maxRetries ?? 3;
  const delay = options.delay ?? 1000;
  const shouldRetry =
    options.shouldRetry ??
    ((error) =>
      error.code === "SERVICE_UNAVAILABLE" || error.code === "TIMEOUT");

  return async (_ctx, next) => {
    let lastError: RpcError | undefined;

    for (let attempt = 0; attempt <= maxRetries; attempt++) {
      try {
        return await next();
      } catch (error) {
        if (
          !isRpcError(error) ||
          !shouldRetry(error) ||
          attempt === maxRetries
        ) {
          throw error;
        }
        lastError = error;
        await new Promise((resolve) =>
          setTimeout(resolve, delay * (attempt + 1)),
        );
      }
    }

    throw lastError;
  };
}

// =============================================================================
// Authentication Interceptor
// =============================================================================

/**
 * Configuration options for the auth interceptor.
 */
export interface AuthInterceptorOptions {
  /** The header name to use for the token. Defaults to "Authorization" */
  headerName?: string;
  /** The property name in context that contains the token. Defaults to "token" */
  tokenProperty?: string;
  /** The token prefix. Defaults to "Bearer" */
  prefix?: string;
}

/**
 * Create an authentication interceptor that adds Bearer tokens to requests.
 *
 * @example
 * ```typescript
 * const link = new TauriLink<{ token?: string }>({
 *   interceptors: [authInterceptor()],
 * });
 *
 * const client = createClientFromLink<AppContract, { token?: string }>(link);
 * const user = await client.user.get({ id: 1 }, { context: { token: 'jwt' } });
 * ```
 */
export function authInterceptor<
  TClientContext extends Record<string, unknown> = Record<string, unknown>,
>(options: AuthInterceptorOptions = {}): LinkInterceptor<TClientContext> {
  const headerName = options.headerName ?? "Authorization";
  const tokenProperty = options.tokenProperty ?? "token";
  const prefix = options.prefix ?? "Bearer";

  return async (ctx, next) => {
    const token = ctx.context[tokenProperty];
    if (token) {
      ctx.meta[headerName] = `${prefix} ${token}`;
    }
    return next();
  };
}
