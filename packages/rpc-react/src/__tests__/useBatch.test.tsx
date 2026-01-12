// =============================================================================
// useBatch Hook Tests
// =============================================================================

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { renderHook, act, waitFor } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import {
  createClient,
  configureRpc,
  TypedBatchResponseWrapper,
  type RpcClient,
  type BatchResponse,
} from "@tauri-nexus/rpc-core";
import { useBatch } from "@tauri-nexus/rpc-react";

// Mock the invoke function
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const mockInvoke = invoke as ReturnType<typeof vi.fn>;

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
// useBatch Hook Tests
// =============================================================================

describe("useBatch Hook", () => {
  describe("initial state", () => {
    it("should have correct initial state", () => {
      const { result } = renderHook(() =>
        useBatch(() => client.batch().add("h", "health", undefined)),
      );

      expect(result.current.isLoading).toBe(false);
      expect(result.current.isSuccess).toBe(false);
      expect(result.current.isError).toBe(false);
      expect(result.current.error).toBeNull();
      expect(result.current.response).toBeNull();
      expect(result.current.duration).toBeNull();
    });
  });

  describe("execute()", () => {
    it("should execute batch and update state", async () => {
      const mockResponse: BatchResponse = {
        results: [{ id: "h", data: { status: "ok", version: "1.0" } }],
      };
      mockInvoke.mockResolvedValueOnce(mockResponse);

      const { result } = renderHook(() =>
        useBatch(() => client.batch().add("h", "health", undefined)),
      );

      await act(async () => {
        await result.current.execute();
      });

      expect(result.current.isLoading).toBe(false);
      expect(result.current.isSuccess).toBe(true);
      expect(result.current.isError).toBe(false);
      expect(result.current.response).not.toBeNull();
      expect(result.current.duration).toBeGreaterThanOrEqual(0);
    });

    it("should set isLoading during execution", async () => {
      let resolvePromise: (value: BatchResponse) => void;
      const promise = new Promise<BatchResponse>((resolve) => {
        resolvePromise = resolve;
      });
      mockInvoke.mockReturnValueOnce(promise);

      const { result } = renderHook(() =>
        useBatch(() => client.batch().add("h", "health", undefined)),
      );

      // Start execution
      let executePromise: Promise<unknown>;
      act(() => {
        executePromise = result.current.execute();
      });

      // Should be loading
      expect(result.current.isLoading).toBe(true);

      // Resolve the promise
      await act(async () => {
        resolvePromise!({
          results: [{ id: "h", data: { status: "ok", version: "1.0" } }],
        });
        await executePromise;
      });

      // Should no longer be loading
      expect(result.current.isLoading).toBe(false);
      expect(result.current.isSuccess).toBe(true);
    });

    it("should handle errors", async () => {
      mockInvoke.mockRejectedValueOnce(
        JSON.stringify({
          code: "INTERNAL_ERROR",
          message: "Batch failed",
        }),
      );

      const { result } = renderHook(() =>
        useBatch(() => client.batch().add("h", "health", undefined)),
      );

      await act(async () => {
        try {
          await result.current.execute();
        } catch {
          // Expected to throw
        }
      });

      expect(result.current.isLoading).toBe(false);
      expect(result.current.isSuccess).toBe(false);
      expect(result.current.isError).toBe(true);
      expect(result.current.error?.code).toBe("INTERNAL_ERROR");
      expect(result.current.response).toBeNull();
    });
  });

  describe("executeOnMount option", () => {
    it("should execute on mount when executeOnMount is true", async () => {
      const mockResponse: BatchResponse = {
        results: [{ id: "h", data: { status: "ok", version: "1.0" } }],
      };
      mockInvoke.mockResolvedValueOnce(mockResponse);

      const { result } = renderHook(() =>
        useBatch(() => client.batch().add("h", "health", undefined), {
          executeOnMount: true,
        }),
      );

      await waitFor(() => {
        expect(result.current.isSuccess).toBe(true);
      });

      expect(mockInvoke).toHaveBeenCalled();
    });

    it("should not execute on mount when executeOnMount is false", async () => {
      const { result } = renderHook(() =>
        useBatch(() => client.batch().add("h", "health", undefined), {
          executeOnMount: false,
        }),
      );

      // Wait a bit to ensure no execution
      await new Promise((resolve) => setTimeout(resolve, 50));

      expect(result.current.isLoading).toBe(false);
      expect(result.current.isSuccess).toBe(false);
      expect(mockInvoke).not.toHaveBeenCalled();
    });
  });

  describe("callbacks", () => {
    it("should call onSuccess callback", async () => {
      const mockResponse: BatchResponse = {
        results: [{ id: "h", data: { status: "ok", version: "1.0" } }],
      };
      mockInvoke.mockResolvedValueOnce(mockResponse);

      const onSuccess = vi.fn();

      const { result } = renderHook(() =>
        useBatch(() => client.batch().add("h", "health", undefined), {
          onSuccess,
        }),
      );

      await act(async () => {
        await result.current.execute();
      });

      expect(onSuccess).toHaveBeenCalledTimes(1);
      expect(onSuccess).toHaveBeenCalledWith(
        expect.any(TypedBatchResponseWrapper),
      );
    });

    it("should call onError callback", async () => {
      mockInvoke.mockRejectedValueOnce(
        JSON.stringify({
          code: "ERROR",
          message: "Failed",
        }),
      );

      const onError = vi.fn();

      const { result } = renderHook(() =>
        useBatch(() => client.batch().add("h", "health", undefined), {
          onError,
        }),
      );

      await act(async () => {
        try {
          await result.current.execute();
        } catch {
          // Expected
        }
      });

      expect(onError).toHaveBeenCalledTimes(1);
      expect(onError).toHaveBeenCalledWith(
        expect.objectContaining({
          code: "ERROR",
          message: "Failed",
        }),
      );
    });
  });

  describe("reset()", () => {
    it("should reset state to initial values", async () => {
      const mockResponse: BatchResponse = {
        results: [{ id: "h", data: { status: "ok", version: "1.0" } }],
      };
      mockInvoke.mockResolvedValueOnce(mockResponse);

      const { result } = renderHook(() =>
        useBatch(() => client.batch().add("h", "health", undefined)),
      );

      // Execute first
      await act(async () => {
        await result.current.execute();
      });

      expect(result.current.isSuccess).toBe(true);
      expect(result.current.response).not.toBeNull();

      // Reset
      act(() => {
        result.current.reset();
      });

      expect(result.current.isLoading).toBe(false);
      expect(result.current.isSuccess).toBe(false);
      expect(result.current.isError).toBe(false);
      expect(result.current.error).toBeNull();
      expect(result.current.response).toBeNull();
      expect(result.current.duration).toBeNull();
    });
  });

  describe("getResult()", () => {
    it("should return typed result by ID", async () => {
      const mockResponse: BatchResponse = {
        results: [
          { id: "h", data: { status: "ok", version: "1.0" } },
          { id: "g", data: "Hello!" },
        ],
      };
      mockInvoke.mockResolvedValueOnce(mockResponse);

      const { result } = renderHook(() =>
        useBatch(() =>
          client
            .batch()
            .add("h", "health", undefined)
            .add("g", "greet", { name: "Test" }),
        ),
      );

      await act(async () => {
        await result.current.execute();
      });

      const healthResult = result.current.getResult("h");
      expect(healthResult?.data).toEqual({ status: "ok", version: "1.0" });

      const greetResult = result.current.getResult("g");
      expect(greetResult?.data).toBe("Hello!");
    });

    it("should return undefined before execution", () => {
      const { result } = renderHook(() =>
        useBatch(() => client.batch().add("h", "health", undefined)),
      );

      expect(result.current.getResult("h")).toBeUndefined();
    });
  });

  describe("multiple executions", () => {
    it("should handle multiple sequential executions", async () => {
      const mockResponse1: BatchResponse = {
        results: [{ id: "h", data: { status: "ok", version: "1.0" } }],
      };
      const mockResponse2: BatchResponse = {
        results: [{ id: "h", data: { status: "updated", version: "2.0" } }],
      };
      mockInvoke
        .mockResolvedValueOnce(mockResponse1)
        .mockResolvedValueOnce(mockResponse2);

      const { result } = renderHook(() =>
        useBatch(() => client.batch().add("h", "health", undefined)),
      );

      // First execution
      await act(async () => {
        await result.current.execute();
      });

      expect(result.current.getResult("h")?.data).toEqual({
        status: "ok",
        version: "1.0",
      });

      // Second execution
      await act(async () => {
        await result.current.execute();
      });

      expect(result.current.getResult("h")?.data).toEqual({
        status: "updated",
        version: "2.0",
      });
    });
  });
});
