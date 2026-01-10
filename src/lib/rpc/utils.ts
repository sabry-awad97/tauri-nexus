// =============================================================================
// Utility Functions
// =============================================================================

import { invoke } from '@tauri-apps/api/core';
import type { RpcError } from './types';

/** Get list of available procedures from backend */
export async function getProcedures(): Promise<string[]> {
  return invoke<string[]>('plugin:rpc|rpc_procedures');
}

/** Sleep utility for retry logic */
export function sleep(ms: number): Promise<void> {
  return new Promise(resolve => setTimeout(resolve, ms));
}

/** Calculate exponential backoff with jitter */
export function calculateBackoff(
  attempt: number,
  baseDelay: number = 1000,
  maxDelay: number = 30000,
  jitter: boolean = true
): number {
  const exponentialDelay = baseDelay * Math.pow(2, attempt);
  const cappedDelay = Math.min(exponentialDelay, maxDelay);
  
  if (jitter) {
    return cappedDelay * (0.5 + Math.random() * 0.5);
  }
  
  return cappedDelay;
}

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
  retryableCodes: ['INTERNAL_ERROR', 'TIMEOUT', 'UNAVAILABLE'],
  jitter: true,
};

/** Execute a function with retry logic */
export async function withRetry<T>(
  fn: () => Promise<T>,
  config: Partial<RetryConfig> = {}
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

/** Deduplication key generator */
export function deduplicationKey(path: string, input: unknown): string {
  return `${path}:${JSON.stringify(input)}`;
}

/** Pending request tracker for deduplication */
const pendingRequests = new Map<string, Promise<unknown>>();

/** Execute a function with deduplication */
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
