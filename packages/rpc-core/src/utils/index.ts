// =============================================================================
// @tauri-nexus/rpc-core - Utilities
// =============================================================================
// Promise-based utilities. These are standalone implementations that don't
// depend on Effect, providing simple retry and deduplication for Promise-based code.

import { invoke } from "@tauri-apps/api/core";
import {
  stableStringify,
  defaultRetryConfig,
  type RetryConfig,
} from "@tauri-nexus/rpc-effect";

// =============================================================================
// Re-exports (no wrapper needed)
// =============================================================================

export { stableStringify, defaultRetryConfig, type RetryConfig };

// =============================================================================
// Timing Utilities
// =============================================================================

/**
 * Sleep for a specified duration.
 */
export const sleep = (ms: number): Promise<void> =>
  new Promise((resolve) => setTimeout(resolve, ms));

/**
 * Calculate exponential backoff with optional jitter.
 */
export const calculateBackoff = (
  attempt: number,
  baseDelay: number = 1000,
  maxDelay: number = 30000,
  jitter: boolean = true
): number => {
  const exponentialDelay = baseDelay * Math.pow(2, attempt);
  const cappedDelay = Math.min(exponentialDelay, maxDelay);

  if (jitter) {
    return cappedDelay * (0.5 + Math.random() * 0.5);
  }

  return cappedDelay;
};

/**
 * Generate a deduplication key from path and input.
 */
export const deduplicationKey = (path: string, input: unknown): string =>
  `${path}:${stableStringify(input)}`;

// =============================================================================
// Retry Logic
// =============================================================================

/**
 * Check if an error is retryable based on its code.
 */
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

/**
 * Execute a function with retry logic.
 */
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

      // Don't retry if not retryable or if we've exhausted retries
      if (!isRetryableError(error, retryableCodes) || attempt >= maxRetries) {
        throw error;
      }

      // Wait before retrying
      const delay = calculateBackoff(attempt, baseDelay, maxDelay, jitter);
      await sleep(delay);
    }
  }

  throw lastError;
}

// =============================================================================
// Deduplication
// =============================================================================

const globalPendingRequests = new Map<string, Promise<unknown>>();

/**
 * Execute a function with deduplication.
 * Concurrent calls with the same key will share the same Promise.
 */
export async function withDedup<T>(
  key: string,
  fn: () => Promise<T>
): Promise<T> {
  const existing = globalPendingRequests.get(key);
  if (existing) {
    return existing as Promise<T>;
  }

  const promise = fn().finally(() => {
    globalPendingRequests.delete(key);
  });

  globalPendingRequests.set(key, promise);
  return promise;
}

// =============================================================================
// Backend Utilities (Tauri-specific, not in rpc-effect)
// =============================================================================

/**
 * Get list of available procedures from backend.
 */
export const getProcedures = (): Promise<string[]> =>
  invoke<string[]>("plugin:rpc|rpc_procedures");

/**
 * Get current subscription count from backend.
 */
export const getSubscriptionCount = (): Promise<number> =>
  invoke<number>("plugin:rpc|rpc_subscription_count");
