// =============================================================================
// @tauri-nexus/rpc-core - TauriLink Implementation
// =============================================================================
// Configurable link for Tauri RPC calls with interceptor support.

import { invoke } from "@tauri-apps/api/core";
import type { RpcError, RpcErrorCode } from "../core/types";
import { createEventIterator } from "../subscription/event-iterator";
import type {
  TauriLinkConfig,
  LinkRequestContext,
  LinkCallOptions,
  LinkSubscribeOptions,
} from "./types";

// =============================================================================
// Helper Functions
// =============================================================================

/** Create an RPC error */
function createRpcError(
  code: RpcErrorCode | string,
  message: string,
  details?: unknown,
): RpcError {
  return { code, message, details };
}

/** Check if error is an RPC error */
function isRpcError(error: unknown): error is RpcError {
  return (
    typeof error === "object" &&
    error !== null &&
    "code" in error &&
    "message" in error &&
    typeof (error as RpcError).code === "string" &&
    typeof (error as RpcError).message === "string"
  );
}

/** Parse error from backend response */
function parseError(error: unknown, timeoutMs?: number): RpcError {
  if (error instanceof Error) {
    if (error.name === "AbortError") {
      if (timeoutMs !== undefined) {
        return createRpcError(
          "TIMEOUT",
          `Request timed out after ${timeoutMs}ms`,
          { timeoutMs },
        );
      }
      return createRpcError("CANCELLED", "Request was cancelled");
    }
    return createRpcError("UNKNOWN", error.message);
  }

  if (typeof error === "string") {
    try {
      const parsed = JSON.parse(error);
      if (isRpcError(parsed)) return parsed;
      return createRpcError("UNKNOWN", error);
    } catch {
      return createRpcError("UNKNOWN", error);
    }
  }

  if (isRpcError(error)) return error;
  return createRpcError("UNKNOWN", String(error));
}

/** Validate procedure path */
function validatePath(path: string): void {
  if (!path) {
    throw createRpcError("VALIDATION_ERROR", "Procedure path cannot be empty");
  }
  if (path.startsWith(".") || path.endsWith(".")) {
    throw createRpcError(
      "VALIDATION_ERROR",
      "Procedure path cannot start or end with a dot",
    );
  }
  if (path.includes("..")) {
    throw createRpcError(
      "VALIDATION_ERROR",
      "Procedure path cannot contain consecutive dots",
    );
  }
  for (const ch of path) {
    if (!/[a-zA-Z0-9_.]/.test(ch)) {
      throw createRpcError(
        "VALIDATION_ERROR",
        `Procedure path contains invalid character: '${ch}'`,
      );
    }
  }
}

// =============================================================================
// TauriLink Class
// =============================================================================

/**
 * TauriLink - A configurable link for Tauri RPC calls.
 *
 * @example
 * ```typescript
 * const link = new TauriLink<{ token?: string }>({
 *   interceptors: [
 *     async (ctx, next) => {
 *       console.log(`[RPC] ${ctx.path}`);
 *       return next();
 *     },
 *   ],
 * });
 *
 * const client = createClientFromLink<AppContract, { token?: string }>(link);
 * ```
 */
export class TauriLink<TClientContext = unknown> {
  private config: TauriLinkConfig<TClientContext>;

  constructor(config: TauriLinkConfig<TClientContext> = {}) {
    this.config = config;
  }

  /** Execute interceptor chain */
  private async executeInterceptors<T>(
    ctx: LinkRequestContext<TClientContext>,
    fn: () => Promise<T>,
  ): Promise<T> {
    const interceptors = this.config.interceptors ?? [];

    let next = fn;
    for (let i = interceptors.length - 1; i >= 0; i--) {
      const interceptor = interceptors[i];
      const currentNext = next;
      next = () => interceptor(ctx, currentNext);
    }

    return next();
  }

  /** Make an RPC call */
  async call<T>(
    path: string,
    input: unknown,
    options: LinkCallOptions<TClientContext> = {},
  ): Promise<T> {
    validatePath(path);

    const ctx: LinkRequestContext<TClientContext> = {
      path,
      input,
      type: "query",
      context: options.context ?? ({} as TClientContext),
      signal: options.signal,
      meta: options.meta ?? {},
    };

    const timeoutMs = options.timeout ?? this.config.timeout;

    try {
      await this.config.onRequest?.(ctx);

      const result = await this.executeInterceptors(ctx, async () => {
        if (timeoutMs) {
          const controller = new AbortController();
          const timeoutId = setTimeout(() => controller.abort(), timeoutMs);

          try {
            const result = await invoke<T>("plugin:rpc|rpc_call", {
              path,
              input,
            });
            clearTimeout(timeoutId);
            return result;
          } catch (error) {
            clearTimeout(timeoutId);
            throw error;
          }
        }

        return invoke<T>("plugin:rpc|rpc_call", { path, input });
      });

      await this.config.onResponse?.(result, ctx);
      return result;
    } catch (error) {
      const rpcError = parseError(error, timeoutMs);
      await this.config.onError?.(rpcError, ctx);
      throw rpcError;
    }
  }

  /** Subscribe to a streaming procedure */
  async subscribe<T>(
    path: string,
    input: unknown,
    options: LinkSubscribeOptions<TClientContext> = {},
  ): Promise<AsyncIterable<T>> {
    validatePath(path);

    const ctx: LinkRequestContext<TClientContext> = {
      path,
      input,
      type: "subscription",
      context: options.context ?? ({} as TClientContext),
      signal: options.signal,
      meta: options.meta ?? {},
    };

    try {
      await this.config.onRequest?.(ctx);
      return await createEventIterator<T>(path, input, options);
    } catch (error) {
      const rpcError = parseError(error);
      await this.config.onError?.(rpcError, ctx);
      throw rpcError;
    }
  }

  /** Check if path is a subscription */
  isSubscription(path: string): boolean {
    return this.config.subscriptionPaths?.includes(path) ?? false;
  }

  /** Get configuration */
  getConfig(): TauriLinkConfig<TClientContext> {
    return this.config;
  }
}
