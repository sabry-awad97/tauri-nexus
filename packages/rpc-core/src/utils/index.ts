// =============================================================================
// @tauri-nexus/rpc-core - Utilities
// =============================================================================
// Pure utility functions without Effect dependencies.

import { invoke } from "@tauri-apps/api/core";
import { stableStringify as stableStringifyImpl } from "@tauri-nexus/rpc-effect";

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

// =============================================================================
// Retry Logic
// =============================================================================

export interface RetryConfig {
  readonly maxRetries: number;
  readonly baseDelay: number;
  readonly maxDelay: number;
  readonly retryableCodes: readonly string[];
  readonly jitter: boolean;
  readonly backoff: "linear" | "exponential";
}

export const defaultRetryConfig: RetryConfig = {
  maxRetries: 3,
  baseDelay: 1000,
  maxDelay: 30000,
  retryableCodes: ["INTERNAL_ERROR", "TIMEOUT", "UNAVAILABLE"],
  jitter: true,
  backoff: "exponential",
};

/**
 * Execute a function with retry logic.
 */
export async function withRetry<T>(
  fn: () => Promise<T>,
  config: Partial<RetryConfig> = {}
): Promise<T> {
  const {
    maxRetries = 3,
    baseDelay = 1000,
    maxDelay = 30000,
    retryableCodes = ["INTERNAL_ERROR", "TIMEOUT"],
    jitter = true,
  } = { ...defaultRetryConfig, ...config };

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

      const delay = calculateBackoff(attempt, baseDelay, maxDelay, jitter);
      await sleep(delay);
    }
  }

  throw lastError;
}

// =============================================================================
// Serialization
// =============================================================================

/**
 * JSON.stringify with sorted keys for consistent output.
 */
export const stableStringify = stableStringifyImpl;

/**
 * Generate a deduplication key from path and input.
 */
export const deduplicationKey = (path: string, input: unknown): string =>
  `${path}:${stableStringify(input)}`;

// =============================================================================
// Deduplication
// =============================================================================

const pendingRequests = new Map<string, Promise<unknown>>();

/**
 * Execute a function with deduplication.
 * Concurrent calls with the same key will share the same Promise.
 */
export async function withDedup<T>(
  key: string,
  fn: () => Promise<T>
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
