// =============================================================================
// React Hooks Tests
// =============================================================================
// Tests for useQuery, useMutation, useSubscription, and createHooks.

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { renderHook, act, waitFor } from "@testing-library/react";
import * as fc from "fast-check";
import {
  useQuery,
  useMutation,
  useSubscription,
  createHooks,
  useIsMounted,
  useDebounce,
} from "../hooks";
import type {
  RpcError,
  ContractRouter,
  RouterClient,
  EventIterator,
} from "../types";

// =============================================================================
// Test Utilities
// =============================================================================

function createMockEventIterator<T>(
  events: T[],
  error?: RpcError,
): EventIterator<T> {
  let index = 0;
  let returned = false;

  return {
    async return(): Promise<void> {
      returned = true;
    },
    [Symbol.asyncIterator](): AsyncIterator<T> {
      return {
        async next(): Promise<IteratorResult<T>> {
          if (returned) {
            return { done: true, value: undefined };
          }
          if (error && index === events.length) {
            throw error;
          }
          if (index < events.length) {
            return { done: false, value: events[index++] };
          }
          return { done: true, value: undefined };
        },
        async return(): Promise<IteratorResult<T>> {
          returned = true;
          return { done: true, value: undefined };
        },
      };
    },
  };
}

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
// useQuery Tests
// =============================================================================

describe("useQuery", () => {
  it("should fetch data on mount", async () => {
    const queryFn = vi.fn().mockResolvedValue({ id: 1, name: "Test" });

    const { result } = renderHook(() => useQuery(queryFn, []));

    expect(result.current.isLoading).toBe(true);

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });

    expect(result.current.data).toEqual({ id: 1, name: "Test" });
    expect(result.current.isLoading).toBe(false);
    expect(result.current.error).toBeNull();
  });

  it("should handle errors", async () => {
    const error: RpcError = { code: "NOT_FOUND", message: "Not found" };
    const queryFn = vi.fn().mockRejectedValue(error);

    const { result } = renderHook(() => useQuery(queryFn, []));

    await waitFor(() => {
      expect(result.current.isError).toBe(true);
    });

    expect(result.current.error).toMatchObject({ code: "NOT_FOUND" });
    expect(result.current.data).toBeUndefined();
  });

  it("should not fetch when disabled", async () => {
    const queryFn = vi.fn().mockResolvedValue("data");

    const { result } = renderHook(() =>
      useQuery(queryFn, [], { enabled: false }),
    );

    expect(queryFn).not.toHaveBeenCalled();
    expect(result.current.isLoading).toBe(false);
  });

  it("should refetch when deps change", async () => {
    const queryFn = vi.fn().mockResolvedValue("data");

    const { result, rerender } = renderHook(
      ({ id }) => useQuery(() => queryFn(id), [id]),
      { initialProps: { id: 1 } },
    );

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });

    expect(queryFn).toHaveBeenCalledWith(1);

    rerender({ id: 2 });

    await waitFor(() => {
      expect(queryFn).toHaveBeenCalledWith(2);
    });

    expect(queryFn).toHaveBeenCalledTimes(2);
  });

  it("should support refetch interval", async () => {
    const queryFn = vi.fn().mockResolvedValue("data");

    renderHook(() => useQuery(queryFn, [], { refetchInterval: 100 }));

    await waitFor(() => {
      expect(queryFn).toHaveBeenCalledTimes(1);
    });

    await new Promise((resolve) => setTimeout(resolve, 250));

    expect(queryFn.mock.calls.length).toBeGreaterThanOrEqual(2);
  });

  it("should support manual refetch", async () => {
    const queryFn = vi.fn().mockResolvedValue("data");

    const { result } = renderHook(() => useQuery(queryFn, []));

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });

    expect(queryFn).toHaveBeenCalledTimes(1);

    await act(async () => {
      await result.current.refetch();
    });

    expect(queryFn).toHaveBeenCalledTimes(2);
  });

  it("should keep previous data while refetching", async () => {
    let callCount = 0;
    const queryFn = vi.fn().mockImplementation(async () => {
      callCount++;
      return `data-${callCount}`;
    });

    const { result } = renderHook(() =>
      useQuery(queryFn, [], { keepPreviousData: true }),
    );

    await waitFor(() => {
      expect(result.current.data).toBe("data-1");
    });

    await act(async () => {
      await result.current.refetch();
    });

    // Should still have data while refetching
    expect(result.current.data).toBe("data-2");
  });

  it("should use initial data", async () => {
    const queryFn = vi.fn().mockResolvedValue("fetched");

    const { result } = renderHook(() =>
      useQuery(queryFn, [], { initialData: "initial" }),
    );

    expect(result.current.data).toBe("initial");
    expect(result.current.isSuccess).toBe(true);
    expect(result.current.isLoading).toBe(false);
  });
});

// =============================================================================
// useMutation Tests
// =============================================================================

describe("useMutation", () => {
  it("should start in idle state", () => {
    const mutationFn = vi.fn();

    const { result } = renderHook(() => useMutation(mutationFn));

    expect(result.current.isIdle).toBe(true);
    expect(result.current.isLoading).toBe(false);
    expect(result.current.data).toBeUndefined();
  });

  it("should execute mutation with mutate()", async () => {
    const mutationFn = vi.fn().mockResolvedValue({ id: 1 });

    const { result } = renderHook(() => useMutation(mutationFn));

    act(() => {
      result.current.mutate({ name: "Test" });
    });

    expect(result.current.isLoading).toBe(true);

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });

    expect(result.current.data).toEqual({ id: 1 });
    expect(mutationFn).toHaveBeenCalledWith({ name: "Test" });
  });

  it("should execute mutation with mutateAsync()", async () => {
    const mutationFn = vi.fn().mockResolvedValue({ id: 1 });

    const { result } = renderHook(() => useMutation(mutationFn));

    let returnedData: any;
    await act(async () => {
      returnedData = await result.current.mutateAsync({ name: "Test" });
    });

    expect(returnedData).toEqual({ id: 1 });
    expect(result.current.isSuccess).toBe(true);
  });

  it("should handle mutation errors", async () => {
    const error: RpcError = { code: "VALIDATION_ERROR", message: "Invalid" };
    const mutationFn = vi.fn().mockRejectedValue(error);

    const { result } = renderHook(() => useMutation(mutationFn));

    act(() => {
      result.current.mutate({ name: "" });
    });

    await waitFor(() => {
      expect(result.current.isError).toBe(true);
    });

    expect(result.current.error).toMatchObject({ code: "VALIDATION_ERROR" });
  });

  it("should call onSuccess callback", async () => {
    const mutationFn = vi.fn().mockResolvedValue({ id: 1 });
    const onSuccess = vi.fn();

    const { result } = renderHook(() => useMutation(mutationFn, { onSuccess }));

    await act(async () => {
      await result.current.mutateAsync({ name: "Test" });
    });

    expect(onSuccess).toHaveBeenCalledWith({ id: 1 }, { name: "Test" });
  });

  it("should call onError callback", async () => {
    const error: RpcError = { code: "ERROR", message: "Failed" };
    const mutationFn = vi.fn().mockRejectedValue(error);
    const onError = vi.fn();

    const { result } = renderHook(() => useMutation(mutationFn, { onError }));

    await act(async () => {
      try {
        await result.current.mutateAsync({ name: "Test" });
      } catch {}
    });

    expect(onError).toHaveBeenCalledWith(
      expect.objectContaining({ code: "ERROR" }),
      { name: "Test" },
    );
  });

  it("should call onSettled callback on success", async () => {
    const mutationFn = vi.fn().mockResolvedValue({ id: 1 });
    const onSettled = vi.fn();

    const { result } = renderHook(() => useMutation(mutationFn, { onSettled }));

    await act(async () => {
      await result.current.mutateAsync({ name: "Test" });
    });

    expect(onSettled).toHaveBeenCalledWith({ id: 1 }, null, { name: "Test" });
  });

  it("should call onSettled callback on error", async () => {
    const error: RpcError = { code: "ERROR", message: "Failed" };
    const mutationFn = vi.fn().mockRejectedValue(error);
    const onSettled = vi.fn();

    const { result } = renderHook(() => useMutation(mutationFn, { onSettled }));

    await act(async () => {
      try {
        await result.current.mutateAsync({ name: "Test" });
      } catch {}
    });

    expect(onSettled).toHaveBeenCalledWith(
      undefined,
      expect.objectContaining({ code: "ERROR" }),
      { name: "Test" },
    );
  });

  it("should reset state", async () => {
    const mutationFn = vi.fn().mockResolvedValue({ id: 1 });

    const { result } = renderHook(() => useMutation(mutationFn));

    await act(async () => {
      await result.current.mutateAsync({ name: "Test" });
    });

    expect(result.current.isSuccess).toBe(true);

    act(() => {
      result.current.reset();
    });

    expect(result.current.isIdle).toBe(true);
    expect(result.current.data).toBeUndefined();
  });
});

// =============================================================================
// useSubscription Tests
// =============================================================================

describe("useSubscription", () => {
  it("should connect and receive events", async () => {
    const events = [1, 2, 3];
    const subscribeFn = vi
      .fn()
      .mockResolvedValue(createMockEventIterator(events));

    const { result } = renderHook(() => useSubscription(subscribeFn, []));

    await waitFor(
      () => {
        expect(result.current.data.length).toBe(3);
      },
      { timeout: 1000 },
    );

    expect(result.current.data).toEqual([1, 2, 3]);
    expect(result.current.latestEvent).toBe(3);
  });

  it("should track connection state", async () => {
    const subscribeFn = vi.fn().mockResolvedValue(createMockEventIterator([1]));

    const { result } = renderHook(() => useSubscription(subscribeFn, []));

    await waitFor(() => {
      expect(result.current.connectionCount).toBeGreaterThan(0);
    });
  });

  it("should handle errors", async () => {
    const error: RpcError = { code: "SUBSCRIPTION_ERROR", message: "Failed" };
    const subscribeFn = vi
      .fn()
      .mockResolvedValue(createMockEventIterator([], error));

    const { result } = renderHook(() => useSubscription(subscribeFn, []));

    await waitFor(
      () => {
        expect(result.current.isError).toBe(true);
      },
      { timeout: 1000 },
    );

    expect(result.current.error).toMatchObject({ code: "SUBSCRIPTION_ERROR" });
  });

  it("should not connect when disabled", async () => {
    const subscribeFn = vi.fn().mockResolvedValue(createMockEventIterator([]));

    renderHook(() => useSubscription(subscribeFn, [], { enabled: false }));

    expect(subscribeFn).not.toHaveBeenCalled();
  });

  it("should call onEvent callback", async () => {
    const events = [1, 2];
    const subscribeFn = vi
      .fn()
      .mockResolvedValue(createMockEventIterator(events));
    const onEvent = vi.fn();

    renderHook(() => useSubscription(subscribeFn, [], { onEvent }));

    await waitFor(
      () => {
        expect(onEvent).toHaveBeenCalledTimes(2);
      },
      { timeout: 1000 },
    );

    expect(onEvent).toHaveBeenCalledWith(1);
    expect(onEvent).toHaveBeenCalledWith(2);
  });

  it("should clear data", async () => {
    const events = [1, 2, 3];
    const subscribeFn = vi
      .fn()
      .mockResolvedValue(createMockEventIterator(events));

    const { result } = renderHook(() => useSubscription(subscribeFn, []));

    await waitFor(
      () => {
        expect(result.current.data.length).toBe(3);
      },
      { timeout: 1000 },
    );

    act(() => {
      result.current.clear();
    });

    expect(result.current.data).toEqual([]);
    expect(result.current.latestEvent).toBeUndefined();
  });

  it("should limit events to maxEvents", async () => {
    const events = [1, 2, 3, 4, 5];
    const subscribeFn = vi
      .fn()
      .mockResolvedValue(createMockEventIterator(events));

    const { result } = renderHook(() =>
      useSubscription(subscribeFn, [], { maxEvents: 3 }),
    );

    await waitFor(
      () => {
        expect(result.current.data.length).toBe(3);
      },
      { timeout: 1000 },
    );

    // Should keep only the last 3 events
    expect(result.current.data).toEqual([3, 4, 5]);
  });
});

// =============================================================================
// createHooks Tests
// =============================================================================

describe("createHooks", () => {
  interface TestContract extends ContractRouter {
    getUser: { type: "query"; input: { id: number }; output: { name: string } };
    createUser: {
      type: "mutation";
      input: { name: string };
      output: { id: number };
    };
    events: { type: "subscription"; input: void; output: string };
  }

  it("should create typed hooks from client", () => {
    const mockClient = {
      getUser: vi.fn().mockResolvedValue({ name: "Test" }),
      createUser: vi.fn().mockResolvedValue({ id: 1 }),
      events: vi.fn().mockResolvedValue(createMockEventIterator([])),
    } as unknown as RouterClient<TestContract>;

    const hooks = createHooks(mockClient);

    expect(hooks.useRpcQuery).toBeDefined();
    expect(hooks.useRpcMutation).toBeDefined();
    expect(hooks.useRpcSubscription).toBeDefined();
    expect(hooks.client).toBe(mockClient);
  });

  it("should work with useRpcQuery", async () => {
    const mockClient = {
      getUser: vi.fn().mockResolvedValue({ name: "Test" }),
    } as unknown as RouterClient<TestContract>;

    const { useRpcQuery } = createHooks(mockClient);

    const { result } = renderHook(() =>
      useRpcQuery((c) => c.getUser({ id: 1 }), [1]),
    );

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });

    expect(result.current.data).toEqual({ name: "Test" });
  });

  it("should work with useRpcMutation", async () => {
    const mockClient = {
      createUser: vi.fn().mockResolvedValue({ id: 1 }),
    } as unknown as RouterClient<TestContract>;

    const { useRpcMutation } = createHooks(mockClient);

    const { result } = renderHook(() => useRpcMutation((c) => c.createUser));

    await act(async () => {
      await result.current.mutateAsync({ name: "Test" });
    });

    expect(result.current.data).toEqual({ id: 1 });
  });
});

// =============================================================================
// Utility Hooks Tests
// =============================================================================

describe("useIsMounted", () => {
  it("should return true when mounted", () => {
    const { result } = renderHook(() => useIsMounted());

    expect(result.current()).toBe(true);
  });

  it("should return false after unmount", () => {
    const { result, unmount } = renderHook(() => useIsMounted());

    const isMounted = result.current;
    unmount();

    expect(isMounted()).toBe(false);
  });
});

describe("useDebounce", () => {
  it("should debounce value changes", async () => {
    const { result, rerender } = renderHook(
      ({ value }) => useDebounce(value, 100),
      { initialProps: { value: "initial" } },
    );

    expect(result.current).toBe("initial");

    rerender({ value: "updated" });
    expect(result.current).toBe("initial");

    await new Promise((resolve) => setTimeout(resolve, 150));

    expect(result.current).toBe("updated");
  });

  it("should cancel pending debounce on unmount", () => {
    const { result, unmount, rerender } = renderHook(
      ({ value }) => useDebounce(value, 100),
      { initialProps: { value: "initial" } },
    );

    rerender({ value: "updated" });
    unmount();

    // Should not throw or cause issues
    expect(result.current).toBe("initial");
  });
});

// =============================================================================
// Property-Based Tests
// =============================================================================

describe("Property-Based Tests", () => {
  // Property 5: Request Staleness Detection
  // When multiple requests are triggered rapidly, only the latest request's result should update state
  it("property: useQuery request staleness detection", async () => {
    await fc.assert(
      fc.asyncProperty(fc.integer({ min: 2, max: 5 }), async (numRequests) => {
        let callCount = 0;
        const delays = Array.from(
          { length: numRequests },
          () => Math.random() * 50,
        );

        // Create a query function where earlier requests may resolve after later ones
        const queryFn = vi.fn().mockImplementation(async () => {
          const myCallNumber = ++callCount;
          const delay = delays[myCallNumber - 1] || 0;
          await new Promise((resolve) => setTimeout(resolve, delay));
          return `result-${myCallNumber}`;
        });

        const { result, rerender } = renderHook(
          ({ dep }) => useQuery(queryFn, [dep]),
          { initialProps: { dep: 0 } },
        );

        // Trigger multiple rapid refetches by changing deps
        for (let i = 1; i < numRequests; i++) {
          rerender({ dep: i });
        }

        // Wait for all requests to settle
        await new Promise((resolve) => setTimeout(resolve, 200));

        await waitFor(
          () => {
            expect(result.current.isFetching).toBe(false);
          },
          { timeout: 1000 },
        );

        // The final state should reflect the LAST request's result
        // (result-{numRequests}), not an earlier one that may have resolved later
        expect(result.current.data).toBe(`result-${numRequests}`);
        expect(result.current.isSuccess).toBe(true);
      }),
      { numRuns: 10 },
    );
  });

  // Property 6: Single Active Connection
  // When deps change rapidly, only the latest connection should be active
  it("property: useSubscription single active connection", async () => {
    await fc.assert(
      fc.asyncProperty(
        fc.integer({ min: 2, max: 4 }),
        async (numConnections) => {
          let connectionCount = 0;
          const activeConnections = new Set<number>();
          const returnedConnections = new Set<number>();

          const subscribeFn = vi.fn().mockImplementation(async () => {
            const myConnectionId = ++connectionCount;
            activeConnections.add(myConnectionId);

            // Simulate connection delay
            await new Promise((resolve) =>
              setTimeout(resolve, Math.random() * 30),
            );

            let eventIndex = 0;
            return {
              async return(): Promise<void> {
                activeConnections.delete(myConnectionId);
                returnedConnections.add(myConnectionId);
              },
              [Symbol.asyncIterator](): AsyncIterator<number> {
                return {
                  async next(): Promise<IteratorResult<number>> {
                    if (returnedConnections.has(myConnectionId)) {
                      return { done: true, value: undefined };
                    }
                    if (eventIndex < 3) {
                      await new Promise((resolve) => setTimeout(resolve, 20));
                      return {
                        done: false,
                        value: myConnectionId * 100 + eventIndex++,
                      };
                    }
                    return { done: true, value: undefined };
                  },
                  async return(): Promise<IteratorResult<number>> {
                    activeConnections.delete(myConnectionId);
                    returnedConnections.add(myConnectionId);
                    return { done: true, value: undefined };
                  },
                };
              },
            };
          });

          const { rerender, unmount } = renderHook(
            ({ dep }) => useSubscription(subscribeFn, [dep]),
            { initialProps: { dep: 0 } },
          );

          // Trigger multiple rapid reconnections by changing deps
          for (let i = 1; i < numConnections; i++) {
            rerender({ dep: i });
            await new Promise((resolve) => setTimeout(resolve, 10));
          }

          // Wait for connections to settle
          await new Promise((resolve) => setTimeout(resolve, 300));

          // At most one connection should be active at any time
          // (the latest one, or zero if all completed)
          expect(activeConnections.size).toBeLessThanOrEqual(1);

          // Clean up
          unmount();
        },
      ),
      { numRuns: 5 },
    );
  });

  // Property 7: Concurrent Mutation Independence
  // Multiple concurrent mutations should not interfere with each other
  it("property: useMutation concurrent mutation independence", async () => {
    await fc.assert(
      fc.asyncProperty(fc.integer({ min: 2, max: 5 }), async (numMutations) => {
        const results: string[] = [];
        const mutationFn = vi.fn().mockImplementation(async (input: string) => {
          // Random delay to simulate varying response times
          await new Promise((resolve) =>
            setTimeout(resolve, Math.random() * 50),
          );
          results.push(input);
          return `result-${input}`;
        });

        const { result } = renderHook(() => useMutation(mutationFn));

        // Fire multiple mutations concurrently
        const promises: Promise<string>[] = [];
        for (let i = 0; i < numMutations; i++) {
          promises.push(
            result.current.mutateAsync(`input-${i}`).catch(() => `error-${i}`),
          );
        }

        // Wait for all mutations to complete
        const mutationResults = await Promise.all(promises);

        // Each mutation should have been called
        expect(mutationFn).toHaveBeenCalledTimes(numMutations);

        // Each mutation should return its own result (not interfere with others)
        for (let i = 0; i < numMutations; i++) {
          expect(mutationResults[i]).toBe(`result-input-${i}`);
        }

        // All inputs should have been processed
        expect(results.length).toBe(numMutations);
      }),
      { numRuns: 10 },
    );
  });

  // Property: useQuery always returns valid state
  it("property: useQuery state is always consistent", async () => {
    await fc.assert(
      fc.asyncProperty(
        fc.boolean(),
        fc.option(fc.string(), { nil: undefined }),
        async (shouldSucceed, errorMessage) => {
          const queryFn = shouldSucceed
            ? vi.fn().mockResolvedValue("data")
            : vi
                .fn()
                .mockRejectedValue({
                  code: "ERROR",
                  message: errorMessage || "Error",
                });

          const { result } = renderHook(() => useQuery(queryFn, []));

          await waitFor(
            () => {
              expect(result.current.isLoading).toBe(false);
            },
            { timeout: 1000 },
          );

          // State consistency checks
          if (result.current.isSuccess) {
            expect(result.current.isError).toBe(false);
            expect(result.current.error).toBeNull();
            expect(result.current.data).toBeDefined();
          }

          if (result.current.isError) {
            expect(result.current.isSuccess).toBe(false);
            expect(result.current.error).not.toBeNull();
          }

          // isLoading and isFetching should be false when settled
          expect(result.current.isLoading).toBe(false);
        },
      ),
      { numRuns: 20 },
    );
  });

  // Property: useMutation state transitions are valid
  it("property: useMutation state transitions are valid", async () => {
    await fc.assert(
      fc.asyncProperty(fc.boolean(), async (shouldSucceed) => {
        const mutationFn = shouldSucceed
          ? vi.fn().mockResolvedValue("result")
          : vi.fn().mockRejectedValue({ code: "ERROR", message: "Failed" });

        const { result } = renderHook(() => useMutation(mutationFn));

        // Initial state
        expect(result.current.isIdle).toBe(true);

        await act(async () => {
          try {
            await result.current.mutateAsync("input");
          } catch {}
        });

        // Final state
        expect(result.current.isIdle).toBe(false);
        expect(result.current.isLoading).toBe(false);

        if (shouldSucceed) {
          expect(result.current.isSuccess).toBe(true);
          expect(result.current.isError).toBe(false);
        } else {
          expect(result.current.isSuccess).toBe(false);
          expect(result.current.isError).toBe(true);
        }
      }),
      { numRuns: 20 },
    );
  });
});
