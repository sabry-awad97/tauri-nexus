// =============================================================================
// TC003: Batch RPC Operations Tests
// =============================================================================
// Test batch RPC operations with concurrency control and result aggregation.

import { describe, it, expect, vi } from "vitest";
import { Effect, Layer, Either } from "effect";
import {
  batchCall,
  batchCallParallel,
  batchCallParallelCollect,
  batchCallParallelFailFast,
  batchCallSequential,
  validateBatchRequests,
  type BatchRequestItem,
  type BatchResponse,
  RpcConfigService,
  RpcTransportService,
  RpcInterceptorService,
  RpcLoggerService,
  createCallError,
} from "../index";

describe("TC003: Batch RPC Operations", () => {
  // Mock transport for testing
  const createMockTransport = (
    callBatchImpl: (
      requests: readonly BatchRequestItem[],
    ) => Promise<BatchResponse>,
  ) => ({
    call: vi.fn(),
    callBatch: callBatchImpl,
    subscribe: vi.fn(),
  });

  const createTestLayer = (transport: ReturnType<typeof createMockTransport>) =>
    Layer.mergeAll(
      RpcConfigService.Default,
      RpcTransportService.layer(transport as any),
      RpcInterceptorService.Default,
      RpcLoggerService.Default,
    );

  describe("Batch Request Validation", () => {
    it("should validate all request paths", async () => {
      const requests: BatchRequestItem[] = [
        { id: "1", path: "users.get", input: { id: 1 } },
        { id: "2", path: "users.list", input: {} },
      ];

      const result = await Effect.runPromise(validateBatchRequests(requests));
      expect(result).toEqual(requests);
    });

    it("should fail on invalid path", async () => {
      const requests: BatchRequestItem[] = [
        { id: "1", path: "", input: {} }, // Invalid empty path
      ];

      const exit = await Effect.runPromiseExit(validateBatchRequests(requests));
      expect(exit._tag).toBe("Failure");
    });

    it("should fail on path with invalid characters", async () => {
      const requests: BatchRequestItem[] = [
        { id: "1", path: "users/get", input: {} }, // Invalid slash
      ];

      const exit = await Effect.runPromiseExit(validateBatchRequests(requests));
      expect(exit._tag).toBe("Failure");
    });
  });

  describe("batchCall (Transport-based)", () => {
    it("should execute batch through transport", async () => {
      const mockResponse: BatchResponse = {
        results: [
          { id: "1", data: { name: "Alice" } },
          { id: "2", data: { name: "Bob" } },
        ],
      };

      const transport = createMockTransport(async () => mockResponse);
      const layer = createTestLayer(transport);

      const requests: BatchRequestItem[] = [
        { id: "1", path: "users.get", input: { id: 1 } },
        { id: "2", path: "users.get", input: { id: 2 } },
      ];

      const result = await Effect.runPromise(
        batchCall(requests).pipe(Effect.provide(layer)),
      );

      expect(result.results).toHaveLength(2);
      expect(result.results[0].data).toEqual({ name: "Alice" });
    });

    it("should handle transport errors", async () => {
      const transport = createMockTransport(async () => {
        throw new Error("Network error");
      });
      const layer = createTestLayer(transport);

      const requests: BatchRequestItem[] = [
        { id: "1", path: "users.get", input: {} },
      ];

      const exit = await Effect.runPromiseExit(
        batchCall(requests).pipe(Effect.provide(layer)),
      );

      expect(exit._tag).toBe("Failure");
    });
  });

  describe("batchCallParallel (Effect.all based)", () => {
    it("should execute requests in parallel and return Either results", async () => {
      let callCount = 0;
      const transport = {
        call: vi.fn(async (path: string, input: unknown) => {
          callCount++;
          if ((input as { fail?: boolean }).fail) {
            throw createCallError("ERROR", "Failed");
          }
          return { path, success: true };
        }),
        callBatch: vi.fn(),
        subscribe: vi.fn(),
      };
      const layer = createTestLayer(transport);

      const requests: BatchRequestItem[] = [
        { id: "1", path: "test.success", input: {} },
        { id: "2", path: "test.fail", input: { fail: true } },
        { id: "3", path: "test.success2", input: {} },
      ];

      const results = await Effect.runPromise(
        batchCallParallel(requests, 5).pipe(Effect.provide(layer)),
      );

      expect(results).toHaveLength(3);
      expect(Either.isRight(results[0])).toBe(true);
      expect(Either.isLeft(results[1])).toBe(true);
      expect(Either.isRight(results[2])).toBe(true);
    });

    it("should respect concurrency limit", async () => {
      let maxConcurrent = 0;
      let currentConcurrent = 0;

      const transport = {
        call: vi.fn(async () => {
          currentConcurrent++;
          maxConcurrent = Math.max(maxConcurrent, currentConcurrent);
          await new Promise((r) => setTimeout(r, 10));
          currentConcurrent--;
          return { success: true };
        }),
        callBatch: vi.fn(),
        subscribe: vi.fn(),
      };
      const layer = createTestLayer(transport);

      const requests: BatchRequestItem[] = Array.from(
        { length: 10 },
        (_, i) => ({
          id: String(i),
          path: "test.call",
          input: {},
        }),
      );

      await Effect.runPromise(
        batchCallParallel(requests, 3).pipe(Effect.provide(layer)),
      );

      expect(maxConcurrent).toBeLessThanOrEqual(3);
    });
  });

  describe("batchCallParallelCollect", () => {
    it("should collect results into BatchResponse format", async () => {
      const transport = {
        call: vi.fn(async (_path: string, input: { id: number }) => ({
          id: input.id,
          name: `User ${input.id}`,
        })),
        callBatch: vi.fn(),
        subscribe: vi.fn(),
      };
      const layer = createTestLayer(transport);

      const requests: BatchRequestItem[] = [
        { id: "req1", path: "users.get", input: { id: 1 } },
        { id: "req2", path: "users.get", input: { id: 2 } },
      ];

      const response = await Effect.runPromise(
        batchCallParallelCollect(requests).pipe(Effect.provide(layer)),
      );

      expect(response.results).toHaveLength(2);
      expect(response.results[0].id).toBe("req1");
      expect(response.results[0].data).toEqual({ id: 1, name: "User 1" });
      expect(response.results[1].id).toBe("req2");
    });

    it("should include errors in results without failing batch", async () => {
      const transport = {
        call: vi.fn(async (_path: string, input: { shouldFail?: boolean }) => {
          if (input.shouldFail) {
            throw createCallError("NOT_FOUND", "Not found");
          }
          return { success: true };
        }),
        callBatch: vi.fn(),
        subscribe: vi.fn(),
      };
      const layer = createTestLayer(transport);

      const requests: BatchRequestItem[] = [
        { id: "success", path: "test.call", input: {} },
        { id: "fail", path: "test.call", input: { shouldFail: true } },
      ];

      const response = await Effect.runPromise(
        batchCallParallelCollect(requests).pipe(Effect.provide(layer)),
      );

      expect(response.results).toHaveLength(2);
      expect(response.results[0].data).toBeDefined();
      expect(response.results[0].error).toBeUndefined();
      expect(response.results[1].error).toBeDefined();
      expect(response.results[1].error?.code).toBe("NOT_FOUND");
    });
  });

  describe("batchCallParallelFailFast", () => {
    it("should return all results on success", async () => {
      const transport = {
        call: vi.fn(async (_path: string, input: { id: number }) => ({
          id: input.id,
        })),
        callBatch: vi.fn(),
        subscribe: vi.fn(),
      };
      const layer = createTestLayer(transport);

      const requests: BatchRequestItem[] = [
        { id: "1", path: "test.call", input: { id: 1 } },
        { id: "2", path: "test.call", input: { id: 2 } },
      ];

      const results = await Effect.runPromise(
        batchCallParallelFailFast(requests).pipe(Effect.provide(layer)),
      );

      expect(results).toHaveLength(2);
      expect(results[0]).toEqual({ id: 1 });
      expect(results[1]).toEqual({ id: 2 });
    });

    it("should fail fast on first error", async () => {
      let callCount = 0;
      const transport = {
        call: vi.fn(async (_path: string, input: { id: number }) => {
          callCount++;
          await new Promise((r) => setTimeout(r, input.id * 10));
          if (input.id === 1) {
            throw createCallError("ERROR", "First failed");
          }
          return { id: input.id };
        }),
        callBatch: vi.fn(),
        subscribe: vi.fn(),
      };
      const layer = createTestLayer(transport);

      const requests: BatchRequestItem[] = [
        { id: "1", path: "test.call", input: { id: 1 } },
        { id: "2", path: "test.call", input: { id: 2 } },
        { id: "3", path: "test.call", input: { id: 3 } },
      ];

      const exit = await Effect.runPromiseExit(
        batchCallParallelFailFast(requests, 3).pipe(Effect.provide(layer)),
      );

      expect(exit._tag).toBe("Failure");
    });
  });

  describe("batchCallSequential", () => {
    it("should execute requests one at a time", async () => {
      const executionOrder: number[] = [];
      const transport = {
        call: vi.fn(async (_path: string, input: { id: number }) => {
          executionOrder.push(input.id);
          return { id: input.id };
        }),
        callBatch: vi.fn(),
        subscribe: vi.fn(),
      };
      const layer = createTestLayer(transport);

      const requests: BatchRequestItem[] = [
        { id: "1", path: "test.call", input: { id: 1 } },
        { id: "2", path: "test.call", input: { id: 2 } },
        { id: "3", path: "test.call", input: { id: 3 } },
      ];

      await Effect.runPromise(
        batchCallSequential(requests).pipe(Effect.provide(layer)),
      );

      expect(executionOrder).toEqual([1, 2, 3]);
    });
  });

  describe("Mixed Success/Failure Scenarios", () => {
    it("should handle all successes", async () => {
      const transport = {
        call: vi.fn(async () => ({ success: true })),
        callBatch: vi.fn(),
        subscribe: vi.fn(),
      };
      const layer = createTestLayer(transport);

      const requests: BatchRequestItem[] = Array.from(
        { length: 5 },
        (_, i) => ({
          id: String(i),
          path: "test.call",
          input: {},
        }),
      );

      const response = await Effect.runPromise(
        batchCallParallelCollect(requests).pipe(Effect.provide(layer)),
      );

      const successes = response.results.filter((r) => r.data !== undefined);
      expect(successes).toHaveLength(5);
    });

    it("should handle all failures", async () => {
      const transport = {
        call: vi.fn(async () => {
          throw createCallError("ERROR", "All fail");
        }),
        callBatch: vi.fn(),
        subscribe: vi.fn(),
      };
      const layer = createTestLayer(transport);

      const requests: BatchRequestItem[] = Array.from(
        { length: 5 },
        (_, i) => ({
          id: String(i),
          path: "test.call",
          input: {},
        }),
      );

      const response = await Effect.runPromise(
        batchCallParallelCollect(requests).pipe(Effect.provide(layer)),
      );

      const failures = response.results.filter((r) => r.error !== undefined);
      expect(failures).toHaveLength(5);
    });

    it("should handle mixed results correctly", async () => {
      const transport = {
        call: vi.fn(async (_path: string, input: { id: number }) => {
          if (input.id % 2 === 0) {
            throw createCallError("EVEN_ERROR", "Even IDs fail");
          }
          return { id: input.id, success: true };
        }),
        callBatch: vi.fn(),
        subscribe: vi.fn(),
      };
      const layer = createTestLayer(transport);

      const requests: BatchRequestItem[] = Array.from(
        { length: 6 },
        (_, i) => ({
          id: String(i),
          path: "test.call",
          input: { id: i },
        }),
      );

      const response = await Effect.runPromise(
        batchCallParallelCollect(requests).pipe(Effect.provide(layer)),
      );

      const successes = response.results.filter((r) => r.data !== undefined);
      const failures = response.results.filter((r) => r.error !== undefined);

      expect(successes).toHaveLength(3); // 1, 3, 5
      expect(failures).toHaveLength(3); // 0, 2, 4
    });
  });
});
