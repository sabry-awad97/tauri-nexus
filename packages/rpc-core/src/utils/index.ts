// =============================================================================
// @tauri-nexus/rpc-core - Utilities
// =============================================================================
// Promise-based utilities. Uses Effect internally, exposes Promise API.

import { Effect } from "effect";
import { invoke } from "@tauri-apps/api/core";
import {
  sleep as sleepEffect,
  calculateBackoff as calculateBackoffEffect,
  stableStringify,
  deduplicationKey as deduplicationKeyEffect,
  defaultRetryConfig,
  type RetryConfig,
  isRpcError,
} from "@tauri-nexus/rpc-effect";

// =============================================================================
// Re-exports (from rpc-effect)
// =============================================================================

export { stableStringify, defaultRetryConfig, type RetryConfig };

// =============================================================================
// Timing
// =============================================================================

export const sleep = (ms: number): Promise<void> =>
  Effect.runPromise(sleepEffect(ms));

export const calculateBackoff = (
  attempt: number,
  baseDelay: number = 1000,
  maxDelay: number = 30000,
  jitter: boolean = true,
): number =>
  Effect.runSync(calculateBackoffEffect(attempt, baseDelay, maxDelay, jitter));

// =============================================================================
// Deduplication Key
// =============================================================================

export const deduplicationKey = (path: string, input: unknown): string =>
  Effect.runSync(deduplicationKeyEffect(path, input));

// =============================================================================
// Retry (Promise-based, uses shared config from rpc-effect)
// =============================================================================

const isRetryableCode = (
  code: string,
  retryableCodes: readonly string[],
): boolean => retryableCodes.includes(code);

const getErrorCode = (error: unknown): string | undefined => {
  if (isRpcError(error)) return error.code;
  if (typeof error === "object" && error !== null && "code" in error) {
    const code = (error as { code: unknown }).code;
    return typeof code === "string" ? code : undefined;
  }
  return undefined;
};

export async function withRetry<T>(
  fn: () => Promise<T>,
  config: Partial<RetryConfig> = {},
): Promise<T> {
  const {
    maxRetries = defaultRetryConfig.maxRetries,
    baseDelay = defaultRetryConfig.baseDelay,
    maxDelay = defaultRetryConfig.maxDelay,
    retryableCodes = defaultRetryConfig.retryableCodes,
    jitter = defaultRetryConfig.jitter,
  } = config;

  let lastError: unknown;

  for (let attempt = 0; attempt <= maxRetries; attempt++) {
    try {
      return await fn();
    } catch (error) {
      lastError = error;
      const code = getErrorCode(error);
      const isRetryable =
        code !== undefined && isRetryableCode(code, retryableCodes);

      if (!isRetryable || attempt >= maxRetries) {
        throw error;
      }
      const delay = calculateBackoff(attempt, baseDelay, maxDelay, jitter);
      await sleep(delay);
    }
  }

  throw lastError;
}

// =============================================================================
// Deduplication (Promise-based)
// =============================================================================

const pendingRequests = new Map<string, Promise<unknown>>();

export async function withDedup<T>(
  key: string,
  fn: () => Promise<T>,
): Promise<T> {
  const existing = pendingRequests.get(key);
  if (existing) return existing as Promise<T>;

  const promise = fn().finally(() => pendingRequests.delete(key));
  pendingRequests.set(key, promise);
  return promise;
}

// =============================================================================
// Backend Utilities
// =============================================================================

export const getProcedures = (): Promise<string[]> =>
  invoke<string[]>("plugin:rpc|rpc_procedures");

export const getSubscriptionCount = (): Promise<number> =>
  invoke<number>("plugin:rpc|rpc_subscription_count");
