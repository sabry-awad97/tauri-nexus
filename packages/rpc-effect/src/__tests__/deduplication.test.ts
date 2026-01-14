// =============================================================================
// TC007: Request Deduplication Tests
// =============================================================================
// Test request deduplication utilities with cache management and key generation.

import { describe, it, expect } from "vitest";
import { Effect } from "effect";
import {
  createDedupCache,
  deduplicationKey,
  withDedup,
  stableStringify,
} from "../index";

describe("TC007: Request Deduplication", () => {
  describe("Stable Stringify", () => {
    it("should produce consistent output for same object", () => {
      const obj = { b: 2, a: 1, c: 3 };
      const result1 = stableStringify(obj);
      const result2 = stableStringify(obj);

      expect(result1).toBe(result2);
    });

    it("should produce same output regardless of key order", () => {
      const obj1 = { a: 1, b: 2, c: 3 };
      const obj2 = { c: 3, b: 2, a: 1 };

      expect(stableStringify(obj1)).toBe(stableStringify(obj2));
    });

    it("should handle nested objects", () => {
      const obj1 = { outer: { b: 2, a: 1 }, x: 1 };
      const obj2 = { x: 1, outer: { a: 1, b: 2 } };

      expect(stableStringify(obj1)).toBe(stableStringify(obj2));
    });

    it("should handle arrays", () => {
      const obj = { arr: [1, 2, 3], name: "test" };
      const result = stableStringify(obj);

      expect(result).toContain("[1,2,3]");
    });

    it("should handle null and undefined", () => {
      expect(stableStringify(null)).toBe("null");
      expect(stableStringify(undefined)).toBe(undefined);
    });

    it("should handle primitive values", () => {
      expect(stableStringify("string")).toBe('"string"');
      expect(stableStringify(123)).toBe("123");
      expect(stableStringify(true)).toBe("true");
    });
  });

  describe("Deduplication Key Generation", () => {
    it("should generate consistent keys for same path and input", async () => {
      const key1 = await Effect.runPromise(
        deduplicationKey("users.get", { id: 1 }),
      );
      const key2 = await Effect.runPromise(
        deduplicationKey("users.get", { id: 1 }),
      );

      expect(key1).toBe(key2);
    });

    it("should generate different keys for different paths", async () => {
      const key1 = await Effect.runPromise(
        deduplicationKey("users.get", { id: 1 }),
      );
      const key2 = await Effect.runPromise(
        deduplicationKey("users.list", { id: 1 }),
      );

      expect(key1).not.toBe(key2);
    });

    it("should generate different keys for different inputs", async () => {
      const key1 = await Effect.runPromise(
        deduplicationKey("users.get", { id: 1 }),
      );
      const key2 = await Effect.runPromise(
        deduplicationKey("users.get", { id: 2 }),
      );

      expect(key1).not.toBe(key2);
    });

    it("should generate same key regardless of input key order", async () => {
      const key1 = await Effect.runPromise(
        deduplicationKey("users.get", { a: 1, b: 2 }),
      );
      const key2 = await Effect.runPromise(
        deduplicationKey("users.get", { b: 2, a: 1 }),
      );

      expect(key1).toBe(key2);
    });

    it("should handle empty input", async () => {
      const key1 = await Effect.runPromise(deduplicationKey("users.list", {}));
      const key2 = await Effect.runPromise(deduplicationKey("users.list", {}));

      expect(key1).toBe(key2);
    });

    it("should handle undefined input", async () => {
      const key1 = await Effect.runPromise(
        deduplicationKey("users.list", undefined),
      );
      const key2 = await Effect.runPromise(
        deduplicationKey("users.list", undefined),
      );

      expect(key1).toBe(key2);
    });
  });

  describe("Dedup Cache (Effect-based)", () => {
    it("should create cache with correct methods", async () => {
      const cache = await Effect.runPromise(createDedupCache<string>());

      expect(cache.withDedup).toBeDefined();
      expect(cache.clear).toBeDefined();
      expect(cache.clearKey).toBeDefined();
      expect(cache.size).toBeDefined();
    });

    it("should start with empty cache", async () => {
      const cache = await Effect.runPromise(createDedupCache<string>());
      const size = await Effect.runPromise(cache.size());

      expect(size).toBe(0);
    });

    it("should clear all entries", async () => {
      const cache = await Effect.runPromise(createDedupCache<string>());

      // Add some entries via withDedup
      await Effect.runPromise(
        cache.withDedup("key1", Effect.succeed("result1")),
      );

      // Clear
      await Effect.runPromise(cache.clear());

      const size = await Effect.runPromise(cache.size());
      expect(size).toBe(0);
    });
  });

  describe("withDedup (Global)", () => {
    it("should deduplicate concurrent identical requests", async () => {
      let callCount = 0;
      const effect = Effect.gen(function* () {
        callCount++;
        yield* Effect.sleep("50 millis");
        return "result";
      });

      // Start multiple concurrent requests with same key
      const results = await Promise.all([
        Effect.runPromise(withDedup("same-key-1", effect)),
        Effect.runPromise(withDedup("same-key-1", effect)),
        Effect.runPromise(withDedup("same-key-1", effect)),
      ]);

      // All should get same result
      expect(results).toEqual(["result", "result", "result"]);
      // But effect should only run once
      expect(callCount).toBe(1);
    });

    it("should not deduplicate requests with different keys", async () => {
      let callCount = 0;
      const createEffect = (id: number) =>
        Effect.gen(function* () {
          callCount++;
          return `result-${id}`;
        });

      const results = await Promise.all([
        Effect.runPromise(withDedup("key-a", createEffect(1))),
        Effect.runPromise(withDedup("key-b", createEffect(2))),
        Effect.runPromise(withDedup("key-c", createEffect(3))),
      ]);

      expect(results).toEqual(["result-1", "result-2", "result-3"]);
      expect(callCount).toBe(3);
    });

    it("should allow new request after previous completes", async () => {
      let callCount = 0;
      const effect = Effect.gen(function* () {
        callCount++;
        return `result-${callCount}`;
      });

      const result1 = await Effect.runPromise(
        withDedup("sequential-key", effect),
      );
      const result2 = await Effect.runPromise(
        withDedup("sequential-key", effect),
      );

      expect(result1).toBe("result-1");
      expect(result2).toBe("result-2");
      expect(callCount).toBe(2);
    });
  });

  describe("Cache-based withDedup", () => {
    it("should deduplicate concurrent requests using cache", async () => {
      const cache = await Effect.runPromise(createDedupCache<string>());
      let callCount = 0;

      const effect = Effect.gen(function* () {
        callCount++;
        yield* Effect.sleep("50 millis");
        return "cached-result";
      });

      // Start multiple concurrent requests with same key
      const results = await Promise.all([
        Effect.runPromise(cache.withDedup("cache-key", effect)),
        Effect.runPromise(cache.withDedup("cache-key", effect)),
        Effect.runPromise(cache.withDedup("cache-key", effect)),
      ]);

      expect(results).toEqual([
        "cached-result",
        "cached-result",
        "cached-result",
      ]);
      expect(callCount).toBe(1);
    });

    it("should not deduplicate different keys in cache", async () => {
      const cache = await Effect.runPromise(createDedupCache<string>());
      let callCount = 0;

      const createEffect = (id: number) =>
        Effect.gen(function* () {
          callCount++;
          return `result-${id}`;
        });

      const results = await Promise.all([
        Effect.runPromise(cache.withDedup("k1", createEffect(1))),
        Effect.runPromise(cache.withDedup("k2", createEffect(2))),
        Effect.runPromise(cache.withDedup("k3", createEffect(3))),
      ]);

      expect(results).toEqual(["result-1", "result-2", "result-3"]);
      expect(callCount).toBe(3);
    });

    it("should clear specific key from cache", async () => {
      const cache = await Effect.runPromise(createDedupCache<string>());

      await Effect.runPromise(
        cache.withDedup("to-clear", Effect.succeed("value")),
      );
      await Effect.runPromise(cache.clearKey("to-clear"));

      const size = await Effect.runPromise(cache.size());
      expect(size).toBe(0);
    });
  });

  describe("Integration with RPC-like Patterns", () => {
    it("should deduplicate RPC calls with same path and input", async () => {
      let callCount = 0;

      const rpcCall = async (path: string, input: unknown) => {
        const key = await Effect.runPromise(deduplicationKey(path, input));
        return Effect.runPromise(
          withDedup(
            key,
            Effect.gen(function* () {
              callCount++;
              yield* Effect.sleep("50 millis");
              return { path, input, timestamp: Date.now() };
            }),
          ),
        );
      };

      // Concurrent calls with same path and input
      const results = await Promise.all([
        rpcCall("users.get", { id: 1 }),
        rpcCall("users.get", { id: 1 }),
        rpcCall("users.get", { id: 1 }),
      ]);

      // All should get same result (same timestamp)
      expect(results[0].timestamp).toBe(results[1].timestamp);
      expect(results[1].timestamp).toBe(results[2].timestamp);
      expect(callCount).toBe(1);
    });

    it("should not deduplicate calls with different inputs", async () => {
      let callCount = 0;

      const rpcCall = async (path: string, input: unknown) => {
        const key = await Effect.runPromise(deduplicationKey(path, input));
        return Effect.runPromise(
          withDedup(
            key,
            Effect.gen(function* () {
              callCount++;
              return { path, input };
            }),
          ),
        );
      };

      const results = await Promise.all([
        rpcCall("users.get", { id: 1 }),
        rpcCall("users.get", { id: 2 }),
        rpcCall("users.get", { id: 3 }),
      ]);

      expect(results[0].input).toEqual({ id: 1 });
      expect(results[1].input).toEqual({ id: 2 });
      expect(results[2].input).toEqual({ id: 3 });
      expect(callCount).toBe(3);
    });
  });
});
