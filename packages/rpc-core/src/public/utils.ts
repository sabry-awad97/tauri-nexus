// =============================================================================
// @tauri-nexus/rpc-core - Utilities (Public Promise API)
// =============================================================================
// Promise-based wrappers for utility functions.

import { Effect } from "effect";
import type { RpcError } from "../core/types";
import {
  getProceduresEffect,
  getSubscriptionCountEffect,
  sleepEffect,
  calculateBackoffEffect,
  stableStringifyEffect,
  deduplicationKeyEffect,
  defaultEffectRetryConfig,
} from "../utils";

// =============================================================================
// Backend Utilities
// =============================================================================

/**
 * Get list of available procedures from backend.
 */
export async function getProcedures(): Promise<string[]> {
  return Effect.runPromise(getProceduresEffect());
}

/**
 * Get current subscription count from backend.
 */
export async function getSubscriptionCount(): Promise<number> {
  return Effect.runPromise(getSubscriptionCountEffect());
}

// =============================================================================
// Timing Utilities
// =============================================================================

/**
 * Sleep utility for retry logic.
 */
export function sleep(ms: number): Promise<void> {
  return Effect.runPromise(sleepEffect(ms));
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
  return Effect.runSync(
    calculateBackoffEffect(attempt, baseDelay, maxDelay, jitter),
  );
}

// =============================================================================
// Retry Logic
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
  maxRetries: defaultEffectRetryConfig.maxRetries,
  baseDelay: defaultEffectRetryConfig.baseDelay,
  maxDelay: defaultEffectRetryConfig.maxDelay,
  retryableCodes: [...defaultEffectRetryConfig.retryableCodes],
  jitter: defaultEffectRetryConfig.jitter,
};

/**
 * Execute a function with retry logic.
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
// Serialization Utilities
// =============================================================================

/**
 * JSON.stringify with sorted keys for consistent output.
 */
export function stableStringify(value: unknown): string {
  return Effect.runSync(stableStringifyEffect(value));
}

/**
 * Deduplication key generator with stable object serialization.
 */
export function deduplicationKey(path: string, input: unknown): string {
  return Effect.runSync(deduplicationKeyEffect(path, input));
}

// =============================================================================
// Deduplication
// =============================================================================

/** Pending request tracker for deduplication */
const pendingRequests = new Map<string, Promise<unknown>>();

/**
 * Execute a function with deduplication.
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
