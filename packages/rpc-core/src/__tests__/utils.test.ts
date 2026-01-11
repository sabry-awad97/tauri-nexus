// =============================================================================
// Utility Functions Tests
// =============================================================================
// Tests for retry logic, deduplication, and other utilities.

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import * as fc from "fast-check";
import { invoke } from "@tauri-apps/api/core";
import {
  getProcedures,
  sleep,
  calculateBackoff,
  withRetry,
  withDedup,
  deduplicationKey,
  stableStringify,
  defaultRetryConfig,
} from "@tauri-nexus/rpc-core";
import type { RpcError } from "@tauri-nexus/rpc-core";

// Mock invoke
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const mockInvoke = vi.mocked(invoke);

// =============================================================================
// Setup & Teardown
// =============================================================================

beforeEach(() => {
  vi.clearAllMocks();
});

afterEach(() => {
  vi.restoreAllMocks();
});

// =============================================================================
// getProcedures Tests
// =============================================================================

describe("getProcedures()", () => {
  it("should return list of procedures from backend", async () => {
    const procedures = ["health", "user.get", "user.create"];
    mockInvoke.mockResolvedValueOnce(procedures);

    const result = await getProcedures();

    expect(mockInvoke).toHaveBeenCalledWith("plugin:rpc|rpc_procedures");
    expect(result).toEqual(procedures);
  });
});

// =============================================================================
// sleep Tests
// =============================================================================

describe("sleep()", () => {
  it("should resolve after specified time", async () => {
    vi.useFakeTimers();

    const promise = sleep(1000);

    vi.advanceTimersByTime(999);
    expect(vi.getTimerCount()).toBe(1);

    vi.advanceTimersByTime(1);
    await promise;

    expect(vi.getTimerCount()).toBe(0);

    vi.useRealTimers();
  });

  it("should resolve immediately for 0ms", async () => {
    const start = Date.now();
    await sleep(0);
    const elapsed = Date.now() - start;

    expect(elapsed).toBeLessThan(50);
  });
});

// =============================================================================
// calculateBackoff Tests
// =============================================================================

describe("calculateBackoff()", () => {
  it("should calculate exponential backoff", () => {
    // Without jitter for predictable testing
    const delay0 = calculateBackoff(0, 1000, 30000, false);
    const delay1 = calculateBackoff(1, 1000, 30000, false);
    const delay2 = calculateBackoff(2, 1000, 30000, false);
    const delay3 = calculateBackoff(3, 1000, 30000, false);

    expect(delay0).toBe(1000); // 1000 * 2^0 = 1000
    expect(delay1).toBe(2000); // 1000 * 2^1 = 2000
    expect(delay2).toBe(4000); // 1000 * 2^2 = 4000
    expect(delay3).toBe(8000); // 1000 * 2^3 = 8000
  });

  it("should cap at maxDelay", () => {
    const delay = calculateBackoff(10, 1000, 5000, false);
    expect(delay).toBe(5000);
  });

  it("should apply jitter when enabled", () => {
    const delays = new Set<number>();

    // Run multiple times to verify jitter produces different values
    for (let i = 0; i < 10; i++) {
      delays.add(calculateBackoff(1, 1000, 30000, true));
    }

    // With jitter, we should get different values
    // The delay should be between 1000 (50% of 2000) and 2000 (100% of 2000)
    for (const delay of delays) {
      expect(delay).toBeGreaterThanOrEqual(1000);
      expect(delay).toBeLessThanOrEqual(2000);
    }
  });

  it("should use default values", () => {
    const delay = calculateBackoff(0);
    expect(delay).toBeGreaterThanOrEqual(500); // With jitter: 1000 * 0.5
    expect(delay).toBeLessThanOrEqual(1000); // With jitter: 1000 * 1.0
  });

  // Property: backoff is always positive and bounded
  it("property: backoff is always positive and bounded by maxDelay", () => {
    fc.assert(
      fc.property(
        fc.nat(20), // attempt
        fc.integer({ min: 1, max: 10000 }), // baseDelay
        fc.integer({ min: 1, max: 60000 }), // maxDelay
        fc.boolean(), // jitter
        (attempt, baseDelay, maxDelay, jitter) => {
          const delay = calculateBackoff(attempt, baseDelay, maxDelay, jitter);

          expect(delay).toBeGreaterThan(0);
          expect(delay).toBeLessThanOrEqual(maxDelay);
        },
      ),
      { numRuns: 100 },
    );
  });

  // Property: without jitter, backoff is deterministic
  it("property: without jitter, backoff is deterministic", () => {
    fc.assert(
      fc.property(
        fc.nat(10),
        fc.integer({ min: 1, max: 5000 }),
        fc.integer({ min: 1, max: 30000 }),
        (attempt, baseDelay, maxDelay) => {
          const delay1 = calculateBackoff(attempt, baseDelay, maxDelay, false);
          const delay2 = calculateBackoff(attempt, baseDelay, maxDelay, false);

          expect(delay1).toBe(delay2);
        },
      ),
      { numRuns: 50 },
    );
  });

  // Property: backoff increases with attempt (up to max)
  it("property: backoff increases with attempt until capped", () => {
    fc.assert(
      fc.property(
        fc.nat(5),
        fc.integer({ min: 100, max: 1000 }),
        (attempt, baseDelay) => {
          const maxDelay = 100000; // High max to avoid capping
          const delay1 = calculateBackoff(attempt, baseDelay, maxDelay, false);
          const delay2 = calculateBackoff(
            attempt + 1,
            baseDelay,
            maxDelay,
            false,
          );

          expect(delay2).toBeGreaterThanOrEqual(delay1);
        },
      ),
      { numRuns: 50 },
    );
  });
});

// =============================================================================
// withRetry Tests
// =============================================================================

describe("withRetry()", () => {
  it("should return result on first success", async () => {
    const fn = vi.fn().mockResolvedValue("success");

    const result = await withRetry(fn, { maxRetries: 3 });

    expect(result).toBe("success");
    expect(fn).toHaveBeenCalledTimes(1);
  });

  it("should retry on retryable errors", async () => {
    const fn = vi
      .fn()
      .mockRejectedValueOnce({
        code: "INTERNAL_ERROR",
        message: "Error",
      } as RpcError)
      .mockRejectedValueOnce({
        code: "TIMEOUT",
        message: "Timeout",
      } as RpcError)
      .mockResolvedValue("success");

    const result = await withRetry(fn, {
      maxRetries: 3,
      baseDelay: 10,
      retryableCodes: ["INTERNAL_ERROR", "TIMEOUT"],
    });

    expect(result).toBe("success");
    expect(fn).toHaveBeenCalledTimes(3);
  });

  it("should not retry on non-retryable errors", async () => {
    const error: RpcError = { code: "NOT_FOUND", message: "Not found" };
    const fn = vi.fn().mockRejectedValue(error);

    await expect(
      withRetry(fn, {
        maxRetries: 3,
        retryableCodes: ["INTERNAL_ERROR"],
      }),
    ).rejects.toMatchObject({ code: "NOT_FOUND" });

    expect(fn).toHaveBeenCalledTimes(1);
  });

  it("should throw after max retries", async () => {
    const error: RpcError = { code: "INTERNAL_ERROR", message: "Error" };
    const fn = vi.fn().mockRejectedValue(error);

    await expect(
      withRetry(fn, {
        maxRetries: 2,
        baseDelay: 10,
        retryableCodes: ["INTERNAL_ERROR"],
      }),
    ).rejects.toMatchObject({ code: "INTERNAL_ERROR" });

    expect(fn).toHaveBeenCalledTimes(3); // Initial + 2 retries
  });

  it("should use default config", async () => {
    const fn = vi.fn().mockResolvedValue("success");

    await withRetry(fn);

    expect(fn).toHaveBeenCalledTimes(1);
  });

  // Property: withRetry always returns or throws
  it("property: withRetry always returns result or throws error", async () => {
    await fc.assert(
      fc.asyncProperty(
        fc.boolean(),
        fc.nat(3),
        async (shouldSucceed, failCount) => {
          let callCount = 0;
          const fn = vi.fn().mockImplementation(async () => {
            callCount++;
            if (!shouldSucceed || callCount <= failCount) {
              throw { code: "INTERNAL_ERROR", message: "Error" } as RpcError;
            }
            return "success";
          });

          try {
            const result = await withRetry(fn, {
              maxRetries: 5,
              baseDelay: 1,
              retryableCodes: ["INTERNAL_ERROR"],
            });
            expect(result).toBe("success");
          } catch (error) {
            expect((error as RpcError).code).toBe("INTERNAL_ERROR");
          }
        },
      ),
      { numRuns: 20 },
    );
  });
});

// =============================================================================
// stableStringify Tests
// =============================================================================

describe("stableStringify()", () => {
  it("should handle primitive values", () => {
    expect(stableStringify(null)).toBe("null");
    expect(stableStringify(undefined)).toBe(undefined);
    expect(stableStringify(42)).toBe("42");
    expect(stableStringify("hello")).toBe('"hello"');
    expect(stableStringify(true)).toBe("true");
    expect(stableStringify(false)).toBe("false");
  });

  it("should handle arrays", () => {
    expect(stableStringify([])).toBe("[]");
    expect(stableStringify([1, 2, 3])).toBe("[1,2,3]");
    expect(stableStringify(["a", "b"])).toBe('["a","b"]');
  });

  it("should handle empty objects", () => {
    expect(stableStringify({})).toBe("{}");
  });

  it("should sort object keys", () => {
    const obj1 = { b: 1, a: 2, c: 3 };
    const obj2 = { a: 2, c: 3, b: 1 };

    expect(stableStringify(obj1)).toBe('{"a":2,"b":1,"c":3}');
    expect(stableStringify(obj2)).toBe('{"a":2,"b":1,"c":3}');
    expect(stableStringify(obj1)).toBe(stableStringify(obj2));
  });

  it("should handle nested objects with sorted keys", () => {
    const obj = { z: { b: 1, a: 2 }, y: 3 };
    expect(stableStringify(obj)).toBe('{"y":3,"z":{"a":2,"b":1}}');
  });

  it("should handle arrays of objects", () => {
    const arr = [
      { b: 1, a: 2 },
      { d: 3, c: 4 },
    ];
    expect(stableStringify(arr)).toBe('[{"a":2,"b":1},{"c":4,"d":3}]');
  });

  // Property: stableStringify is deterministic for any JSON value
  it("property: stableStringify is deterministic", () => {
    fc.assert(
      fc.property(fc.jsonValue(), (value) => {
        const str1 = stableStringify(value);
        const str2 = stableStringify(value);
        expect(str1).toBe(str2);
      }),
      { numRuns: 100 },
    );
  });

  // Property: objects with same properties produce same string regardless of order
  it("property: object key order does not affect output", () => {
    fc.assert(
      fc.property(
        fc.dictionary(
          fc.string({ minLength: 1, maxLength: 5 }),
          fc.jsonValue(),
        ),
        (obj) => {
          const keys = Object.keys(obj);
          if (keys.length < 2) return;

          // Shuffle keys
          const shuffledKeys = [...keys].sort(() => Math.random() - 0.5);
          const shuffledObj: Record<string, unknown> = {};
          for (const key of shuffledKeys) {
            shuffledObj[key] = obj[key];
          }

          expect(stableStringify(obj)).toBe(stableStringify(shuffledObj));
        },
      ),
      { numRuns: 100 },
    );
  });
});

// =============================================================================
// deduplicationKey Tests
// =============================================================================

describe("deduplicationKey()", () => {
  it("should generate consistent keys", () => {
    const key1 = deduplicationKey("user.get", { id: 1 });
    const key2 = deduplicationKey("user.get", { id: 1 });

    expect(key1).toBe(key2);
  });

  it("should generate different keys for different paths", () => {
    const key1 = deduplicationKey("user.get", { id: 1 });
    const key2 = deduplicationKey("user.create", { id: 1 });

    expect(key1).not.toBe(key2);
  });

  it("should generate different keys for different inputs", () => {
    const key1 = deduplicationKey("user.get", { id: 1 });
    const key2 = deduplicationKey("user.get", { id: 2 });

    expect(key1).not.toBe(key2);
  });

  it("should handle complex inputs", () => {
    const input = { nested: { deep: { value: [1, 2, 3] } } };
    const key = deduplicationKey("complex.path", input);

    expect(key).toBe('complex.path:{"nested":{"deep":{"value":[1,2,3]}}}');
  });

  // Property: same path + input always produces same key
  it("property: deduplicationKey is deterministic", () => {
    fc.assert(
      fc.property(
        fc.string({ minLength: 1 }),
        fc.jsonValue(),
        (path, input) => {
          const key1 = deduplicationKey(path, input);
          const key2 = deduplicationKey(path, input);

          expect(key1).toBe(key2);
        },
      ),
      { numRuns: 100 },
    );
  });

  // Property: different inputs produce different keys
  it("property: different inputs produce different keys", () => {
    fc.assert(
      fc.property(
        fc.string({ minLength: 1 }),
        fc.jsonValue(),
        fc.jsonValue(),
        (path, input1, input2) => {
          fc.pre(JSON.stringify(input1) !== JSON.stringify(input2));

          const key1 = deduplicationKey(path, input1);
          const key2 = deduplicationKey(path, input2);

          expect(key1).not.toBe(key2);
        },
      ),
      { numRuns: 50 },
    );
  });

  // Feature: rpc-library-improvements, Property 14: Deduplication Key Stability
  // **Validates: Requirements 5.4**
  it("property: deduplicationKey produces stable keys regardless of property order", () => {
    // Use alphanumeric keys to avoid special JavaScript properties like __proto__
    const safeKeyArb = fc.stringMatching(/^[a-zA-Z][a-zA-Z0-9]*$/);

    fc.assert(
      fc.property(
        fc.string({ minLength: 1 }),
        fc.dictionary(safeKeyArb, fc.jsonValue()),
        (path, obj) => {
          // Create object with same properties in different order
          const keys = Object.keys(obj);
          if (keys.length < 2) return; // Need at least 2 keys to test order

          const reversedObj: Record<string, unknown> = {};
          const reversedKeys = [...keys].reverse();
          for (const key of reversedKeys) {
            reversedObj[key] = obj[key];
          }

          const key1 = deduplicationKey(path, obj);
          const key2 = deduplicationKey(path, reversedObj);

          expect(key1).toBe(key2);
        },
      ),
      { numRuns: 100 },
    );
  });
});

// =============================================================================
// withDedup Tests
// =============================================================================

describe("withDedup()", () => {
  it("should deduplicate concurrent calls", async () => {
    let callCount = 0;
    const fn = vi.fn().mockImplementation(async () => {
      callCount++;
      await sleep(50);
      return `result-${callCount}`;
    });

    // Start multiple concurrent calls with same key
    const promise1 = withDedup("key", fn);
    const promise2 = withDedup("key", fn);
    const promise3 = withDedup("key", fn);

    const [result1, result2, result3] = await Promise.all([
      promise1,
      promise2,
      promise3,
    ]);

    // All should get the same result
    expect(result1).toBe("result-1");
    expect(result2).toBe("result-1");
    expect(result3).toBe("result-1");

    // Function should only be called once
    expect(fn).toHaveBeenCalledTimes(1);
  });

  it("should not deduplicate sequential calls", async () => {
    let callCount = 0;
    const fn = vi.fn().mockImplementation(async () => {
      callCount++;
      return `result-${callCount}`;
    });

    const result1 = await withDedup("key", fn);
    const result2 = await withDedup("key", fn);

    expect(result1).toBe("result-1");
    expect(result2).toBe("result-2");
    expect(fn).toHaveBeenCalledTimes(2);
  });

  it("should not deduplicate different keys", async () => {
    const fn1 = vi.fn().mockResolvedValue("result-1");
    const fn2 = vi.fn().mockResolvedValue("result-2");

    const [result1, result2] = await Promise.all([
      withDedup("key1", fn1),
      withDedup("key2", fn2),
    ]);

    expect(result1).toBe("result-1");
    expect(result2).toBe("result-2");
    expect(fn1).toHaveBeenCalledTimes(1);
    expect(fn2).toHaveBeenCalledTimes(1);
  });

  it("should handle errors correctly", async () => {
    const error = new Error("Failed");
    const fn = vi.fn().mockRejectedValue(error);

    const promise1 = withDedup("key", fn);
    const promise2 = withDedup("key", fn);

    await expect(promise1).rejects.toThrow("Failed");
    await expect(promise2).rejects.toThrow("Failed");

    expect(fn).toHaveBeenCalledTimes(1);
  });

  it("should clean up after completion", async () => {
    const fn = vi.fn().mockResolvedValue("result");

    await withDedup("key", fn);

    // After completion, a new call should execute the function again
    await withDedup("key", fn);

    expect(fn).toHaveBeenCalledTimes(2);
  });

  // Property 12: Deduplication Promise Sharing
  // All concurrent calls with the same key should receive the same result
  it("property: deduplication promise sharing", async () => {
    await fc.assert(
      fc.asyncProperty(
        fc.integer({ min: 2, max: 10 }),
        fc.string({ minLength: 1 }),
        async (numCalls, key) => {
          let callCount = 0;
          const fn = vi.fn().mockImplementation(async () => {
            callCount++;
            await sleep(10);
            return `result-${callCount}`;
          });

          // Start multiple concurrent calls with same key
          const promises = Array.from({ length: numCalls }, () =>
            withDedup(key, fn),
          );
          const results = await Promise.all(promises);

          // All results should be identical
          const firstResult = results[0];
          for (const result of results) {
            expect(result).toBe(firstResult);
          }

          // Function should only be called once
          expect(fn).toHaveBeenCalledTimes(1);
        },
      ),
      { numRuns: 20 },
    );
  });

  // Property 13: Deduplication Error Propagation
  // All concurrent calls should receive the same error when the function fails
  it("property: deduplication error propagation", async () => {
    await fc.assert(
      fc.asyncProperty(
        fc.integer({ min: 2, max: 10 }),
        fc.string({ minLength: 1 }),
        fc.string({ minLength: 1 }),
        async (numCalls, key, errorMessage) => {
          const error = new Error(errorMessage);
          const fn = vi.fn().mockImplementation(async () => {
            await sleep(10);
            throw error;
          });

          // Start multiple concurrent calls with same key
          const promises = Array.from({ length: numCalls }, () =>
            withDedup(key, fn).catch((e) => e),
          );
          const results = await Promise.all(promises);

          // All results should be the same error
          for (const result of results) {
            expect(result).toBeInstanceOf(Error);
            expect((result as Error).message).toBe(errorMessage);
          }

          // Function should only be called once
          expect(fn).toHaveBeenCalledTimes(1);
        },
      ),
      { numRuns: 20 },
    );
  });
});

// =============================================================================
// defaultRetryConfig Tests
// =============================================================================

describe("defaultRetryConfig", () => {
  it("should have sensible defaults", () => {
    expect(defaultRetryConfig.maxRetries).toBe(3);
    expect(defaultRetryConfig.baseDelay).toBe(1000);
    expect(defaultRetryConfig.maxDelay).toBe(30000);
    expect(defaultRetryConfig.jitter).toBe(true);
    expect(defaultRetryConfig.retryableCodes).toContain("INTERNAL_ERROR");
    expect(defaultRetryConfig.retryableCodes).toContain("TIMEOUT");
    expect(defaultRetryConfig.retryableCodes).toContain("UNAVAILABLE");
  });
});
