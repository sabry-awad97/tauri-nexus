// =============================================================================
// @tauri-nexus/rpc-core - Link Interceptors
// =============================================================================
// Link-specific interceptors using shared utilities from rpc-effect.

import type { RpcError } from "@tauri-nexus/rpc-effect";
import { isRpcError } from "@tauri-nexus/rpc-effect";
import type { LinkInterceptor, LinkRequestContext } from "./types";

// =============================================================================
// Link-Specific Interceptor Options
// =============================================================================

/**
 * Options for the Link auth interceptor.
 */
export interface AuthInterceptorOptions {
  /** The header name to use for the token. Defaults to "Authorization" */
  headerName?: string;
  /** The property name in context that contains the token. Defaults to "token" */
  tokenProperty?: string;
  /** The token prefix. Defaults to "Bearer" */
  prefix?: string;
}

// =============================================================================
// Link-Specific Interceptors
// =============================================================================

/**
 * Create an error handler interceptor for Link.
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

/**
 * Create a logging interceptor for Link.
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

/**
 * Create a retry interceptor for Link.
 * Uses linear backoff: delay * (attempt + 1)
 */
export function retry<TClientContext = unknown>(
  options: {
    maxRetries?: number;
    delay?: number;
    shouldRetry?: (error: RpcError) => boolean;
  } = {},
): LinkInterceptor<TClientContext> {
  const maxRetries = options.maxRetries ?? 3;
  const baseDelay = options.delay ?? 1000;
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
        const delay = baseDelay * (attempt + 1);
        await new Promise((resolve) => setTimeout(resolve, delay));
      }
    }

    throw lastError;
  };
}

/**
 * Create an authentication interceptor for Link.
 * Adds Bearer token from client context to request metadata.
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
