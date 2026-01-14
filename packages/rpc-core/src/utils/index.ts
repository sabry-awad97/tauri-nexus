// =============================================================================
// @tauri-nexus/rpc-core - Utilities
// =============================================================================
// Re-exports utilities from rpc-effect with Promise wrappers.

import { Effect } from "effect";
import { invoke } from "@tauri-apps/api/core";
import {
  sleep as sleepEffect,
  calculateBackoff as calculateBackoffEffect,
  stableStringify,
  type RetryConfig,
} from "@tauri-nexus/rpc-effect";

// =============================================================================
// Timing Utilities
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
// Retry Logic
// =============================================================================

export { type RetryConfig } from "@tauri-nexus/rpc-effect";

export const defaultRetryConfig: RetryConfig = {
  maxRetries: 3,
  baseDelay: 1000,
  maxDelay: 30000,
  retryableCodes: ["INTERNAL_ERROR", "TIMEOUT", "UNAVAILABLE"],
  jitter: true,
  backoff: "exponential",
};

export async function withRetry<T>(
  fn: () => Promise<T>,
  config: Partial<RetryConfig> = {},
): Promise<T> {
  const {
    maxRetries = 3,
    baseDelay = 1000,
    retryableCodes = ["INTERNAL_ERROR", "TIMEOUT"],
  } = {
    ...defaultRetryConfig,
    ...config,
  };

  let lastError: unknown;

  for (let attempt = 0; attempt <= maxRetries; attempt++) {
    try {
      return await fn();
    } catch (error) {
      lastError = error;

      const shouldRetry =
        typeof error === "object" &&
        error !== null &&
        "code" in error &&
        retryableCodes.includes((error as { code: string }).code);

      if (!shouldRetry || attempt === maxRetries) {
        throw error;
      }

      const delay = calculateBackoff(attempt, baseDelay);
      await sleep(delay);
    }
  }

  throw lastError;
}

// =============================================================================
// Serialization
// =============================================================================

export { stableStringify };

export const deduplicationKey = (path: string, input: unknown): string =>
  `${path}:${stableStringify(input)}`;

// =============================================================================
// Deduplication
// =============================================================================

const pendingRequests = new Map<string, Promise<unknown>>();

export async function withDedup<T>(
  key: string,
  fn: () => Promise<T>,
): Promise<T> {
  const existing = pendingRequests.get(key);
  if (existing) {
    return existing as Promise<T>;
  }

  const promise = fn().finally(() => {
    pendingRequests.delete(key);
  });

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
