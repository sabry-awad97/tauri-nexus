// =============================================================================
// Retry Interceptor
// =============================================================================

import type { RpcInterceptor, InterceptorContext } from "../core/types";
import type { InterceptorOptions } from "./factory";

export interface RetryInterceptorOptions extends InterceptorOptions {
  readonly maxRetries?: number;
  readonly delay?: number;
  readonly backoff?: "linear" | "exponential";
  readonly retryOn?: (error: unknown) => boolean;
}

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
      code as (typeof NON_RETRYABLE_CODES)[number],
    );
  }
  return true;
};

const wait = (ms: number): Promise<void> =>
  new Promise((resolve) => setTimeout(resolve, ms));

export function retryInterceptor(
  options: RetryInterceptorOptions = {},
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
