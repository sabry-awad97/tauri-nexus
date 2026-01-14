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
} from "@tauri-nexus/rpc-effect";

// =============================================================================
// Re-exports
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
  jitter: boolean = true
): number =>
  Effect.runSync(calculateBackoffEffect(attempt, baseDelay, maxDelay, jitter));

// =============================================================================
// Deduplication
// =============================================================================

export const deduplicationKey = (path: string, input: unknown): string =>
  Effect.runSync(deduplicationKeyEffect(path, input));

// =============================================================================
// Retry (Promise-based implementation)
// =============================================================================

const isRetryableError = (
  error: unknown,
  retryableCodes: readonly string[]
): boolean => {
  if (
    typeof error === "object" &&
    error !== null &&
    "code" in error &&
    typeof (error as { code: unknown }).code === "string"
  ) {
    return retryableCodes.includes((error as { code: string }).code);
  }
  return false;
};

export async function withRetry<T>(
  fn: () => Promise<T>,
  config: Partial<RetryConfig> = {}
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
      if (!isRetryableError(error, retryableCodes) || attempt >= maxRetries) {
        throw error;
      }
      const delay = calculateBackoff(attempt, baseDelay, maxDelay, jitter);
      await sleep(delay);
    }
  }

  throw lastError;
}

// =============================================================================
// Deduplication (Promise-based implementation)
// =============================================================================

const pendingRequests = new Map<string, Promise<unknown>>();

export async function withDedup<T>(
  key: string,
  fn: () => Promise<T>
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
