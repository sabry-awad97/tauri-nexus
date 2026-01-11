// =============================================================================
// React Hooks Tests
// =============================================================================
// Tests for useSubscription (the only custom hook - queries/mutations use TanStack Query)

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { renderHook, act, waitFor } from "@testing-library/react";
import * as fc from "fast-check";
import { useSubscription, useIsMounted } from "../hooks";
import type { RpcError, EventIterator } from "../types";

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

// =============================================================================
// Property-Based Tests
// =============================================================================

describe("Property-Based Tests", () => {
  // Property: Single Active Connection
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
          expect(activeConnections.size).toBeLessThanOrEqual(1);

          // Clean up
          unmount();
        },
      ),
      { numRuns: 5 },
    );
  });

  // Property: Event buffer respects maxEvents limit
  it("property: useSubscription respects maxEvents limit", async () => {
    await fc.assert(
      fc.asyncProperty(
        fc.integer({ min: 1, max: 10 }),
        fc.integer({ min: 5, max: 20 }),
        async (maxEvents, numEvents) => {
          const events = Array.from({ length: numEvents }, (_, i) => i);
          const subscribeFn = vi
            .fn()
            .mockResolvedValue(createMockEventIterator(events));

          const { result } = renderHook(() =>
            useSubscription(subscribeFn, [], { maxEvents }),
          );

          await waitFor(
            () => {
              // Either all events received or maxEvents limit reached
              expect(result.current.data.length).toBe(
                Math.min(numEvents, maxEvents),
              );
            },
            { timeout: 2000 },
          );

          // Buffer should never exceed maxEvents
          expect(result.current.data.length).toBeLessThanOrEqual(maxEvents);

          // If more events than maxEvents, should have the latest ones
          if (numEvents > maxEvents) {
            const expectedLastEvent = numEvents - 1;
            expect(result.current.latestEvent).toBe(expectedLastEvent);
          }
        },
      ),
      { numRuns: 10 },
    );
  });
});
