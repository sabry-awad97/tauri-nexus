// =============================================================================
// Batch Tests
// =============================================================================
// Tests for type-safe batch operations including TypedBatchBuilder,
// TypedBatchResponseWrapper, and the rpc.batch() method.

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import * as fc from "fast-check";
import { invoke } from "@tauri-apps/api/core";
import {
  createClient,
  configureRpc,
  TypedBatchBuilder,
  TypedBatchResponseWrapper,
  type RpcClient,
  type BatchResponse,
} from "@tauri-nexus/rpc-core";

// Mock the invoke function
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const mockInvoke = vi.mocked(invoke);

// =============================================================================
// Test Contract
// =============================================================================

interface TestContract {
  health: {
    type: "query";
    input: void;
    output: { status: string; version: string };
  };
  greet: { type: "query"; input: { name: string }; output: string };
  user: {
    get: {
      type: "query";
      input: { id: number };
      output: { id: number; name: string };
    };
    list: {
      type: "query";
      input: void;
      output: { id: number; name: string }[];
    };
    create: {
      type: "mutation";
      input: { name: string };
      output: { id: number; name: string };
    };
  };
  stream: {
    counter: { type: "subscription"; input: { start: number }; output: number };
  };
}

// =============================================================================
// Setup & Teardown
// =============================================================================

let client: RpcClient<TestContract>;

beforeEach(() => {
  vi.clearAllMocks();
  configureRpc({
    middleware: [],
    subscriptionPaths: ["stream.counter"],
    timeout: undefined,
  });
  client = createClient<TestContract>();
});

afterEach(() => {
  vi.restoreAllMocks();
});

// =============================================================================
// TypedBatchBuilder Tests
// =============================================================================

describe("TypedBatchBuilder", () => {
  describe("add()", () => {
    it("should add requests to the batch", () => {
      const builder = new TypedBatchBuilder<TestContract>();

      builder
        .add("h", "health", undefined)
        .add("g", "greet", { name: "World" });

      const requests = builder.getRequests();
      expect(requests).toHaveLength(2);
      expect(requests[0]).toEqual({
        id: "h",
        path: "health",
        input: undefined,
      });
      expect(requests[1]).toEqual({
        id: "g",
        path: "greet",
        input: { name: "World" },
      });
    });

    it("should support nested paths", () => {
      const builder = new TypedBatchBuilder<TestContract>();

      builder
        .add("u1", "user.get", { id: 1 })
        .add("u2", "user.list", undefined);

      const requests = builder.getRequests();
      expect(requests).toHaveLength(2);
      expect(requests[0]).toEqual({
        id: "u1",
        path: "user.get",
        input: { id: 1 },
      });
      expect(requests[1]).toEqual({
        id: "u2",
        path: "user.list",
        input: undefined,
      });
    });

    it("should return a new builder with updated type map", () => {
      const builder = new TypedBatchBuilder<TestContract>();

      const builder2 = builder.add("h", "health", undefined);
      const builder3 = builder2.add("g", "greet", { name: "Test" });

      // All should be the same instance (fluent API)
      expect(builder.size()).toBe(2);
      expect(builder2.size()).toBe(2);
      expect(builder3.size()).toBe(2);
    });
  });

  describe("size()", () => {
    it("should return the number of requests", () => {
      const builder = new TypedBatchBuilder<TestContract>();

      expect(builder.size()).toBe(0);

      builder.add("h", "health", undefined);
      expect(builder.size()).toBe(1);

      builder.add("g", "greet", { name: "Test" });
      expect(builder.size()).toBe(2);
    });
  });

  describe("clear()", () => {
    it("should remove all requests", () => {
      const builder = new TypedBatchBuilder<TestContract>();

      builder.add("h", "health", undefined).add("g", "greet", { name: "Test" });

      expect(builder.size()).toBe(2);

      builder.clear();
      expect(builder.size()).toBe(0);
      expect(builder.getRequests()).toEqual([]);
    });
  });

  describe("getRequests()", () => {
    it("should return a copy of requests", () => {
      const builder = new TypedBatchBuilder<TestContract>();

      builder.add("h", "health", undefined);

      const requests1 = builder.getRequests();
      const requests2 = builder.getRequests();

      expect(requests1).toEqual(requests2);
      expect(requests1).not.toBe(requests2); // Different array instances
    });
  });

  describe("execute()", () => {
    it("should call the batch endpoint and return typed response", async () => {
      const mockResponse: BatchResponse = {
        results: [
          { id: "h", data: { status: "ok", version: "1.0" } },
          { id: "g", data: "Hello, World!" },
        ],
      };
      mockInvoke.mockResolvedValueOnce(mockResponse);

      const builder = new TypedBatchBuilder<TestContract>();
      const response = await builder
        .add("h", "health", undefined)
        .add("g", "greet", { name: "World" })
        .execute();

      expect(mockInvoke).toHaveBeenCalledWith("plugin:rpc|rpc_call_batch", {
        batch: {
          requests: [
            { id: "h", path: "health", input: null },
            { id: "g", path: "greet", input: { name: "World" } },
          ],
        },
      });

      expect(response).toBeInstanceOf(TypedBatchResponseWrapper);
      expect(response.results).toHaveLength(2);
    });

    it("should handle timeout option", async () => {
      const mockResponse: BatchResponse = {
        results: [{ id: "h", data: { status: "ok", version: "1.0" } }],
      };
      mockInvoke.mockResolvedValueOnce(mockResponse);

      const builder = new TypedBatchBuilder<TestContract>();
      await builder.add("h", "health", undefined).execute({ timeout: 5000 });

      expect(mockInvoke).toHaveBeenCalled();
    });

    it("should throw on batch failure", async () => {
      mockInvoke.mockRejectedValueOnce(
        JSON.stringify({
          code: "INTERNAL_ERROR",
          message: "Batch processing failed",
        }),
      );

      const builder = new TypedBatchBuilder<TestContract>();

      await expect(
        builder.add("h", "health", undefined).execute(),
      ).rejects.toMatchObject({
        code: "INTERNAL_ERROR",
        message: "Batch processing failed",
      });
    });
  });
});

// =============================================================================
// TypedBatchResponseWrapper Tests
// =============================================================================

describe("TypedBatchResponseWrapper", () => {
  describe("results", () => {
    it("should return all results in order", () => {
      const response: BatchResponse = {
        results: [
          { id: "a", data: "result-a" },
          { id: "b", data: "result-b" },
          { id: "c", data: "result-c" },
        ],
      };

      const wrapper = new TypedBatchResponseWrapper<{
        a: string;
        b: string;
        c: string;
      }>(response);

      expect(wrapper.results).toHaveLength(3);
      expect(wrapper.results[0].id).toBe("a");
      expect(wrapper.results[1].id).toBe("b");
      expect(wrapper.results[2].id).toBe("c");
    });
  });

  describe("getResult()", () => {
    it("should return typed result by ID", () => {
      const response: BatchResponse = {
        results: [
          { id: "health", data: { status: "ok", version: "1.0" } },
          { id: "user", data: { id: 1, name: "John" } },
        ],
      };

      const wrapper = new TypedBatchResponseWrapper<{
        health: { status: string; version: string };
        user: { id: number; name: string };
      }>(response);

      const healthResult = wrapper.getResult("health");
      expect(healthResult.id).toBe("health");
      expect(healthResult.data).toEqual({ status: "ok", version: "1.0" });

      const userResult = wrapper.getResult("user");
      expect(userResult.id).toBe("user");
      expect(userResult.data).toEqual({ id: 1, name: "John" });
    });

    it("should return error result for failed requests", () => {
      const response: BatchResponse = {
        results: [
          { id: "success", data: "ok" },
          { id: "fail", error: { code: "NOT_FOUND", message: "Not found" } },
        ],
      };

      const wrapper = new TypedBatchResponseWrapper<{
        success: string;
        fail: string;
      }>(response);

      const failResult = wrapper.getResult("fail");
      expect(failResult.id).toBe("fail");
      expect(failResult.data).toBeUndefined();
      expect(failResult.error).toEqual({
        code: "NOT_FOUND",
        message: "Not found",
      });
    });

    it("should return NOT_FOUND error for missing ID", () => {
      const response: BatchResponse = {
        results: [{ id: "exists", data: "ok" }],
      };

      const wrapper = new TypedBatchResponseWrapper<{
        exists: string;
        missing: string;
      }>(response);

      const missingResult = wrapper.getResult("missing");
      expect(missingResult.id).toBe("missing");
      expect(missingResult.error?.code).toBe("NOT_FOUND");
    });
  });

  describe("isSuccess() / isError()", () => {
    it("should correctly identify success and error states", () => {
      const response: BatchResponse = {
        results: [
          { id: "ok", data: "success" },
          { id: "fail", error: { code: "ERROR", message: "Failed" } },
        ],
      };

      const wrapper = new TypedBatchResponseWrapper<{
        ok: string;
        fail: string;
      }>(response);

      expect(wrapper.isSuccess("ok")).toBe(true);
      expect(wrapper.isError("ok")).toBe(false);

      expect(wrapper.isSuccess("fail")).toBe(false);
      expect(wrapper.isError("fail")).toBe(true);
    });

    it("should return false/true for non-existent IDs", () => {
      const response: BatchResponse = { results: [] };
      const wrapper = new TypedBatchResponseWrapper<Record<string, never>>(
        response,
      );

      expect(wrapper.isSuccess("nonexistent")).toBe(false);
      expect(wrapper.isError("nonexistent")).toBe(true);
    });
  });

  describe("getSuccessful() / getFailed()", () => {
    it("should filter results by success/failure", () => {
      const response: BatchResponse = {
        results: [
          { id: "s1", data: "success1" },
          { id: "f1", error: { code: "E1", message: "Error 1" } },
          { id: "s2", data: "success2" },
          { id: "f2", error: { code: "E2", message: "Error 2" } },
          { id: "s3", data: "success3" },
        ],
      };

      const wrapper = new TypedBatchResponseWrapper<Record<string, string>>(
        response,
      );

      const successful = wrapper.getSuccessful();
      expect(successful).toHaveLength(3);
      expect(successful.map((r) => r.id)).toEqual(["s1", "s2", "s3"]);

      const failed = wrapper.getFailed();
      expect(failed).toHaveLength(2);
      expect(failed.map((r) => r.id)).toEqual(["f1", "f2"]);
    });
  });

  describe("successCount / errorCount", () => {
    it("should return correct counts", () => {
      const response: BatchResponse = {
        results: [
          { id: "s1", data: "ok" },
          { id: "s2", data: "ok" },
          { id: "f1", error: { code: "E", message: "Error" } },
        ],
      };

      const wrapper = new TypedBatchResponseWrapper<Record<string, string>>(
        response,
      );

      expect(wrapper.successCount).toBe(2);
      expect(wrapper.errorCount).toBe(1);
    });

    it("should handle all success", () => {
      const response: BatchResponse = {
        results: [
          { id: "a", data: "ok" },
          { id: "b", data: "ok" },
        ],
      };

      const wrapper = new TypedBatchResponseWrapper<Record<string, string>>(
        response,
      );

      expect(wrapper.successCount).toBe(2);
      expect(wrapper.errorCount).toBe(0);
    });

    it("should handle all failures", () => {
      const response: BatchResponse = {
        results: [
          { id: "a", error: { code: "E", message: "Error" } },
          { id: "b", error: { code: "E", message: "Error" } },
        ],
      };

      const wrapper = new TypedBatchResponseWrapper<Record<string, string>>(
        response,
      );

      expect(wrapper.successCount).toBe(0);
      expect(wrapper.errorCount).toBe(2);
    });
  });
});

// =============================================================================
// Client batch() Method Tests
// =============================================================================

describe("Client batch() Method", () => {
  it("should return a TypedBatchBuilder", () => {
    const builder = client.batch();
    expect(builder).toBeInstanceOf(TypedBatchBuilder);
  });

  it("should execute batch with correct paths", async () => {
    const mockResponse: BatchResponse = {
      results: [
        { id: "h", data: { status: "ok", version: "1.0" } },
        { id: "g", data: "Hello!" },
      ],
    };
    mockInvoke.mockResolvedValueOnce(mockResponse);

    const response = await client
      .batch()
      .add("h", "health", undefined)
      .add("g", "greet", { name: "Test" })
      .execute();

    expect(mockInvoke).toHaveBeenCalledWith("plugin:rpc|rpc_call_batch", {
      batch: {
        requests: [
          { id: "h", path: "health", input: null },
          { id: "g", path: "greet", input: { name: "Test" } },
        ],
      },
    });

    expect(response.successCount).toBe(2);
  });

  it("should support nested procedure paths", async () => {
    const mockResponse: BatchResponse = {
      results: [
        { id: "u1", data: { id: 1, name: "User 1" } },
        { id: "u2", data: { id: 2, name: "User 2" } },
        { id: "list", data: [] },
      ],
    };
    mockInvoke.mockResolvedValueOnce(mockResponse);

    const response = await client
      .batch()
      .add("u1", "user.get", { id: 1 })
      .add("u2", "user.get", { id: 2 })
      .add("list", "user.list", undefined)
      .execute();

    expect(mockInvoke).toHaveBeenCalledWith("plugin:rpc|rpc_call_batch", {
      batch: {
        requests: [
          { id: "u1", path: "user.get", input: { id: 1 } },
          { id: "u2", path: "user.get", input: { id: 2 } },
          { id: "list", path: "user.list", input: null },
        ],
      },
    });

    const u1Result = response.getResult("u1");
    expect(u1Result.data).toEqual({ id: 1, name: "User 1" });
  });

  it("should handle mixed success and failure results", async () => {
    const mockResponse: BatchResponse = {
      results: [
        { id: "ok", data: { status: "ok", version: "1.0" } },
        { id: "fail", error: { code: "NOT_FOUND", message: "User not found" } },
      ],
    };
    mockInvoke.mockResolvedValueOnce(mockResponse);

    const response = await client
      .batch()
      .add("ok", "health", undefined)
      .add("fail", "user.get", { id: 999 })
      .execute();

    expect(response.isSuccess("ok")).toBe(true);
    expect(response.isError("fail")).toBe(true);
    expect(response.getResult("fail").error?.code).toBe("NOT_FOUND");
  });
});

// =============================================================================
// Property-Based Tests
// =============================================================================

describe("Property-Based Tests", () => {
  // Property: Batch builder size equals number of add() calls
  it("property: builder size equals number of add() calls", () => {
    fc.assert(
      fc.property(fc.integer({ min: 0, max: 20 }), (count) => {
        const builder = new TypedBatchBuilder<TestContract>();

        for (let i = 0; i < count; i++) {
          builder.add(`id-${i}`, "health", undefined);
        }

        expect(builder.size()).toBe(count);
        expect(builder.getRequests()).toHaveLength(count);
      }),
      { numRuns: 50 },
    );
  });

  // Property: clear() always results in empty builder
  it("property: clear() always results in empty builder", () => {
    fc.assert(
      fc.property(fc.integer({ min: 1, max: 20 }), (count) => {
        const builder = new TypedBatchBuilder<TestContract>();

        for (let i = 0; i < count; i++) {
          builder.add(`id-${i}`, "health", undefined);
        }

        builder.clear();

        expect(builder.size()).toBe(0);
        expect(builder.getRequests()).toEqual([]);
      }),
      { numRuns: 50 },
    );
  });

  // Property: Response wrapper preserves all result IDs
  it("property: response wrapper preserves all result IDs", () => {
    fc.assert(
      fc.property(
        fc.array(
          fc.record({
            id: fc.string({ minLength: 1, maxLength: 20 }),
            data: fc.anything(),
          }),
          { minLength: 1, maxLength: 20 },
        ),
        (results) => {
          // Ensure unique IDs
          const uniqueResults = results.filter(
            (r, i, arr) => arr.findIndex((x) => x.id === r.id) === i,
          );

          const response: BatchResponse = { results: uniqueResults };
          const wrapper = new TypedBatchResponseWrapper<
            Record<string, unknown>
          >(response);

          expect(wrapper.results).toHaveLength(uniqueResults.length);

          for (const result of uniqueResults) {
            const retrieved = wrapper.getResult(result.id as string);
            expect(retrieved.id).toBe(result.id);
          }
        },
      ),
      { numRuns: 50 },
    );
  });

  // Property: successCount + errorCount equals total results
  it("property: successCount + errorCount equals total results", () => {
    fc.assert(
      fc.property(
        fc.array(
          fc.oneof(
            fc.record({
              id: fc.string({ minLength: 1, maxLength: 10 }),
              data: fc.anything(),
            }),
            fc.record({
              id: fc.string({ minLength: 1, maxLength: 10 }),
              error: fc.record({
                code: fc.string({ minLength: 1 }),
                message: fc.string(),
              }),
            }),
          ),
          { minLength: 0, maxLength: 20 },
        ),
        (results) => {
          // Ensure unique IDs
          const uniqueResults = results.filter(
            (r, i, arr) => arr.findIndex((x) => x.id === r.id) === i,
          );

          const response: BatchResponse = { results: uniqueResults };
          const wrapper = new TypedBatchResponseWrapper<
            Record<string, unknown>
          >(response);

          expect(wrapper.successCount + wrapper.errorCount).toBe(
            uniqueResults.length,
          );
        },
      ),
      { numRuns: 100 },
    );
  });

  // Property: getSuccessful() and getFailed() partition all results
  it("property: getSuccessful() and getFailed() partition all results", () => {
    fc.assert(
      fc.property(
        fc.array(
          fc.oneof(
            fc.record({
              id: fc.string({ minLength: 1, maxLength: 10 }),
              data: fc.anything(),
            }),
            fc.record({
              id: fc.string({ minLength: 1, maxLength: 10 }),
              error: fc.record({
                code: fc.string({ minLength: 1 }),
                message: fc.string(),
              }),
            }),
          ),
          { minLength: 0, maxLength: 20 },
        ),
        (results) => {
          // Ensure unique IDs
          const uniqueResults = results.filter(
            (r, i, arr) => arr.findIndex((x) => x.id === r.id) === i,
          );

          const response: BatchResponse = { results: uniqueResults };
          const wrapper = new TypedBatchResponseWrapper<
            Record<string, unknown>
          >(response);

          const successful = wrapper.getSuccessful();
          const failed = wrapper.getFailed();

          // Together they should equal total
          expect(successful.length + failed.length).toBe(uniqueResults.length);

          // No overlap
          const successIds = new Set(successful.map((r) => r.id));
          const failIds = new Set(failed.map((r) => r.id));
          for (const id of successIds) {
            expect(failIds.has(id)).toBe(false);
          }
        },
      ),
      { numRuns: 100 },
    );
  });

  // Property: isSuccess and isError are mutually exclusive for existing IDs
  it("property: isSuccess and isError are mutually exclusive", () => {
    fc.assert(
      fc.property(
        fc.array(
          fc.oneof(
            fc.record({
              id: fc.string({ minLength: 1, maxLength: 10 }),
              data: fc.anything(),
            }),
            fc.record({
              id: fc.string({ minLength: 1, maxLength: 10 }),
              error: fc.record({
                code: fc.string({ minLength: 1 }),
                message: fc.string(),
              }),
            }),
          ),
          { minLength: 1, maxLength: 20 },
        ),
        (results) => {
          // Ensure unique IDs
          const uniqueResults = results.filter(
            (r, i, arr) => arr.findIndex((x) => x.id === r.id) === i,
          );

          const response: BatchResponse = { results: uniqueResults };
          const wrapper = new TypedBatchResponseWrapper<
            Record<string, unknown>
          >(response);

          for (const result of uniqueResults) {
            const isSuccess = wrapper.isSuccess(result.id);
            const isError = wrapper.isError(result.id);

            // Exactly one should be true
            expect(isSuccess !== isError).toBe(true);
          }
        },
      ),
      { numRuns: 100 },
    );
  });
});

// =============================================================================
// Edge Cases
// =============================================================================

describe("Edge Cases", () => {
  it("should handle empty batch", async () => {
    const mockResponse: BatchResponse = { results: [] };
    mockInvoke.mockResolvedValueOnce(mockResponse);

    const response = await client.batch().execute();

    expect(response.results).toHaveLength(0);
    expect(response.successCount).toBe(0);
    expect(response.errorCount).toBe(0);
  });

  it("should handle large batch", async () => {
    const results = Array.from({ length: 100 }, (_, i) => ({
      id: `req-${i}`,
      data: { index: i },
    }));
    const mockResponse: BatchResponse = { results };
    mockInvoke.mockResolvedValueOnce(mockResponse);

    const builder = client.batch();
    for (let i = 0; i < 100; i++) {
      builder.add(`req-${i}`, "health", undefined);
    }

    const response = await builder.execute();

    expect(response.results).toHaveLength(100);
    expect(response.successCount).toBe(100);
  });

  it("should handle special characters in IDs", async () => {
    const mockResponse: BatchResponse = {
      results: [
        { id: "id-with-dash", data: "ok" },
        { id: "id_with_underscore", data: "ok" },
        { id: "id.with.dots", data: "ok" },
        { id: "id:with:colons", data: "ok" },
      ],
    };
    mockInvoke.mockResolvedValueOnce(mockResponse);

    const builder = new TypedBatchBuilder<TestContract>();
    builder
      .add("id-with-dash", "health", undefined)
      .add("id_with_underscore", "health", undefined)
      .add("id.with.dots", "health", undefined)
      .add("id:with:colons", "health", undefined);

    const response = await builder.execute();

    expect(response.getResult("id-with-dash").data).toBe("ok");
    expect(response.getResult("id_with_underscore").data).toBe("ok");
    expect(response.getResult("id.with.dots").data).toBe("ok");
    expect(response.getResult("id:with:colons").data).toBe("ok");
  });

  it("should handle null and undefined data values", async () => {
    const mockResponse: BatchResponse = {
      results: [
        { id: "null-data", data: null },
        { id: "undefined-data", data: undefined },
        { id: "empty-object", data: {} },
        { id: "empty-array", data: [] },
      ],
    };
    mockInvoke.mockResolvedValueOnce(mockResponse);

    const builder = new TypedBatchBuilder<TestContract>();
    builder
      .add("null-data", "health", undefined)
      .add("undefined-data", "health", undefined)
      .add("empty-object", "health", undefined)
      .add("empty-array", "health", undefined);

    const response = await builder.execute();

    expect(response.getResult("null-data").data).toBeNull();
    expect(response.getResult("undefined-data").data).toBeUndefined();
    expect(response.getResult("empty-object").data).toEqual({});
    expect(response.getResult("empty-array").data).toEqual([]);
  });

  it("should handle duplicate request IDs (last wins in map)", async () => {
    const mockResponse: BatchResponse = {
      results: [
        { id: "dup", data: "first" },
        { id: "dup", data: "second" },
      ],
    };
    mockInvoke.mockResolvedValueOnce(mockResponse);

    const builder = new TypedBatchBuilder<TestContract>();
    builder.add("dup", "health", undefined).add("dup", "health", undefined);

    const response = await builder.execute();

    // The map will have the last value for duplicate keys
    const result = response.getResult("dup");
    expect(result.data).toBe("second");

    // But results array preserves order
    expect(response.results).toHaveLength(2);
  });
});


