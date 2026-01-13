// =============================================================================
// @tauri-nexus/rpc-core - Utility Functions
// =============================================================================
// Common utilities for retry logic, deduplication, and serialization.
// This module provides Promise-based wrappers around Effect-based implementations.

import type { RpcError } from "../core/types";

// Re-export Effect-based utilities
export {
  // Effect-based functions
  getProceduresEffect,
  getSubscriptionCountEffect,
  sleepEffect,
  calculateBackoffEffect,
  withRetryEffect,
  withRetryEffectDetailed,
  createRetrySchedule,
  createDedupCache,
  deduplicationKeyEffect,
  stableStringifyEffect,
  withDedupEffect,
  // Promise-based wrappers
  getProcedures,
  getSubscriptionCount,
  // Types
  type EffectRetryConfig,
  defaultEffectRetryConfig,
} from "./effect-utils";

// =============================================================================
// Timing Utilities (Promise-based wrappers)
// =============================================================================

/**
 * Sleep utility for retry logic.
 */
export function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

/**
 * Calculate exponential backoff with jitter.
 */
export function calculateBackoff(
  attempt: number,
  baseDelay: number = 1000,
  maxDelay: number = 30000,
  jitter: boolean = true,
): number {
  const exponentialDelay = baseDelay * Math.pow(2, attempt);
  const cappedDelay = Math.min(exponentialDelay, maxDelay);

  if (jitter) {
    return cappedDelay * (0.5 + Math.random() * 0.5);
  }

  return cappedDelay;
}

// =============================================================================
// Retry Logic (Promise-based - uses Effect internally)
// =============================================================================

/** Retry configuration */
export interface RetryConfig {
  maxRetries: number;
  baseDelay: number;
  maxDelay: number;
  retryableCodes: string[];
  jitter: boolean;
}

export const defaultRetryConfig: RetryConfig = {
  maxRetries: 3,
  baseDelay: 1000,
  maxDelay: 30000,
  retryableCodes: ["INTERNAL_ERROR", "TIMEOUT", "UNAVAILABLE"],
  jitter: true,
};

/**
 * Execute a function with retry logic.
 * This is a Promise-based wrapper; for Effect-based retry, use withRetryEffect.
 */
export async function withRetry<T>(
  fn: () => Promise<T>,
  config: Partial<RetryConfig> = {},
): Promise<T> {
  const { maxRetries, baseDelay, maxDelay, retryableCodes, jitter } = {
    ...defaultRetryConfig,
    ...config,
  };

  let lastError: RpcError | undefined;

  for (let attempt = 0; attempt <= maxRetries; attempt++) {
    try {
      return await fn();
    } catch (error) {
      lastError = error as RpcError;

      if (!retryableCodes.includes(lastError.code)) {
        throw lastError;
      }

      if (attempt < maxRetries) {
        const delay = calculateBackoff(attempt, baseDelay, maxDelay, jitter);
        await sleep(delay);
      }
    }
  }

  throw lastError;
}

// =============================================================================
// Serialization Utilities (Promise-based wrappers)
// =============================================================================

/**
 * JSON.stringify with sorted keys for consistent output.
 * Ensures objects with the same properties produce identical strings
 * regardless of property insertion order.
 * For Effect-based version, use stableStringifyEffect.
 */
export function stableStringify(value: unknown): string {
  if (value === null || value === undefined) {
    return JSON.stringify(value);
  }

  if (typeof value !== "object") {
    return JSON.stringify(value);
  }

  if (Array.isArray(value)) {
    return "[" + value.map(stableStringify).join(",") + "]";
  }

  const obj = value as Record<string, unknown>;
  const keys = Object.keys(obj).sort();

  if (keys.length === 0) {
    return "{}";
  }

  const pairs = keys.map(
    (key) => `${JSON.stringify(key)}:${stableStringify(obj[key])}`,
  );
  return "{" + pairs.join(",") + "}";
}

/**
 * Deduplication key generator with stable object serialization.
 * For Effect-based version, use deduplicationKeyEffect.
 */
export function deduplicationKey(path: string, input: unknown): string {
  return `${path}:${stableStringify(input)}`;
}

// =============================================================================
// Deduplication (Promise-based wrapper)
// =============================================================================

/** Pending request tracker for deduplication */
const pendingRequests = new Map<string, Promise<unknown>>();

/**
 * Execute a function with deduplication.
 * For Effect-based version, use withDedupEffect or createDedupCache.
 */
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
