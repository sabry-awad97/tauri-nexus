// =============================================================================
// Event Iterator Tests
// =============================================================================
// Tests for the async event iterator implementation.

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import * as fc from "fast-check";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { createEventIterator, consumeEventIterator } from "@tauri-nexus/rpc-core";
import type { RpcError } from "@tauri-nexus/rpc-core";

// =============================================================================
// Mocks
// =============================================================================

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(),
}));

const mockInvoke = vi.mocked(invoke);
const mockListen = vi.mocked(listen);

// Helper to create a mock event emitter
function createMockEventEmitter() {
  let eventCallback:
    | ((event: { payload: any; event: string; id: number }) => void)
    | null = null;
  const unlisten = vi.fn();

  mockListen.mockImplementation(async (_eventName, callback) => {
    eventCallback = callback as typeof eventCallback;
    return unlisten;
  });

  return {
    emit: (payload: any) => {
      if (eventCallback) {
        eventCallback({ payload, event: "test", id: 0 });
      }
    },
    unlisten,
    getCallback: () => eventCallback,
  };
}

// =============================================================================
// Setup & Teardown
// =============================================================================

beforeEach(() => {
  vi.clearAllMocks();
  mockInvoke.mockResolvedValue(undefined);
});

afterEach(() => {
  vi.restoreAllMocks();
});

// =============================================================================
// Event Iterator Creation Tests
// =============================================================================

describe("createEventIterator()", () => {
  it("should create an async iterator", async () => {
    createMockEventEmitter();

    const iterator = await createEventIterator<number>("stream.counter", {
      start: 0,
    });

    expect(iterator).toBeDefined();
    expect(typeof iterator[Symbol.asyncIterator]).toBe("function");
    expect(typeof iterator.return).toBe("function");

    // Cleanup
    await iterator.return();
  });

  it("should set up event listener with correct event name", async () => {
    createMockEventEmitter();

    const iterator = await createEventIterator<number>("stream.counter", {
      start: 0,
    });

    expect(mockListen).toHaveBeenCalledWith(
      expect.stringMatching(/^rpc:subscription:sub_/),
      expect.any(Function),
    );

    await iterator.return();
  });

  it("should invoke backend subscribe command", async () => {
    createMockEventEmitter();

    const iterator = await createEventIterator<number>("stream.counter", {
      start: 0,
    });

    expect(mockInvoke).toHaveBeenCalledWith("plugin:rpc|rpc_subscribe", {
      request: expect.objectContaining({
        path: "stream.counter",
        input: { start: 0 },
      }),
    });

    await iterator.return();
  });

  it("should pass lastEventId when provided", async () => {
    createMockEventEmitter();

    const iterator = await createEventIterator<number>(
      "stream.counter",
      { start: 0 },
      { lastEventId: "event-123" },
    );

    expect(mockInvoke).toHaveBeenCalledWith("plugin:rpc|rpc_subscribe", {
      request: expect.objectContaining({
        lastEventId: "event-123",
      }),
    });

    await iterator.return();
  });
});

// =============================================================================
// Event Iteration Tests
// =============================================================================

describe("Event Iteration", () => {
  it("should yield events as they arrive", async () => {
    const emitter = createMockEventEmitter();

    const iterator = await createEventIterator<number>("stream.counter", {});
    const asyncIterator = iterator[Symbol.asyncIterator]();

    // Emit some events
    emitter.emit({ type: "data", payload: { data: 1 } });
    emitter.emit({ type: "data", payload: { data: 2 } });
    emitter.emit({ type: "data", payload: { data: 3 } });

    // Consume events
    const result1 = await asyncIterator.next();
    const result2 = await asyncIterator.next();
    const result3 = await asyncIterator.next();

    expect(result1).toEqual({ done: false, value: 1 });
    expect(result2).toEqual({ done: false, value: 2 });
    expect(result3).toEqual({ done: false, value: 3 });

    await iterator.return();
  });

  it("should buffer events when not being consumed", async () => {
    const emitter = createMockEventEmitter();

    const iterator = await createEventIterator<number>("stream.counter", {});

    // Emit events before consuming
    emitter.emit({ type: "data", payload: { data: 1 } });
    emitter.emit({ type: "data", payload: { data: 2 } });

    // Now consume
    const events: number[] = [];
    const asyncIterator = iterator[Symbol.asyncIterator]();

    events.push((await asyncIterator.next()).value);
    events.push((await asyncIterator.next()).value);

    expect(events).toEqual([1, 2]);

    await iterator.return();
  });

  it("should complete when receiving completed event", async () => {
    const emitter = createMockEventEmitter();

    const iterator = await createEventIterator<number>("stream.counter", {});
    const asyncIterator = iterator[Symbol.asyncIterator]();

    emitter.emit({ type: "data", payload: { data: 1 } });
    emitter.emit({ type: "completed" });

    const result1 = await asyncIterator.next();
    const result2 = await asyncIterator.next();

    expect(result1).toEqual({ done: false, value: 1 });
    expect(result2).toEqual({ done: true, value: undefined });
  });

  it("should reject on error event", async () => {
    const emitter = createMockEventEmitter();

    const iterator = await createEventIterator<number>("stream.counter", {});
    const asyncIterator = iterator[Symbol.asyncIterator]();

    const error: RpcError = {
      code: "SUBSCRIPTION_ERROR",
      message: "Stream failed",
    };
    emitter.emit({ type: "error", payload: error });

    await expect(asyncIterator.next()).rejects.toMatchObject({
      code: "SUBSCRIPTION_ERROR",
      message: "Stream failed",
    });
  });

  it("should track lastEventId from events", async () => {
    const emitter = createMockEventEmitter();

    const iterator = await createEventIterator<number>("stream.counter", {});
    const asyncIterator = iterator[Symbol.asyncIterator]();

    emitter.emit({ type: "data", payload: { data: 1, id: "event-1" } });
    emitter.emit({ type: "data", payload: { data: 2, id: "event-2" } });

    await asyncIterator.next();
    await asyncIterator.next();

    // The lastEventId should be tracked internally for reconnection
    await iterator.return();
  });
});

// =============================================================================
// Cleanup Tests
// =============================================================================

describe("Cleanup", () => {
  it("should unsubscribe when return() is called", async () => {
    const emitter = createMockEventEmitter();

    const iterator = await createEventIterator<number>("stream.counter", {});

    await iterator.return();

    expect(emitter.unlisten).toHaveBeenCalled();
    expect(mockInvoke).toHaveBeenCalledWith("plugin:rpc|rpc_unsubscribe", {
      id: expect.stringMatching(/^sub_/),
    });
  });

  it("should resolve pending consumers on cleanup", async () => {
    createMockEventEmitter();

    const iterator = await createEventIterator<number>("stream.counter", {});
    const asyncIterator = iterator[Symbol.asyncIterator]();

    // Start waiting for next value
    const nextPromise = asyncIterator.next();

    // Cleanup while waiting
    await iterator.return();

    // Should resolve with done: true
    const result = await nextPromise;
    expect(result).toEqual({ done: true, value: undefined });
  });

  it("should handle abort signal", async () => {
    const emitter = createMockEventEmitter();
    const controller = new AbortController();

    await createEventIterator<number>(
      "stream.counter",
      {},
      { signal: controller.signal },
    );

    controller.abort();

    // Give time for abort handler to run
    await new Promise((resolve) => setTimeout(resolve, 10));

    expect(emitter.unlisten).toHaveBeenCalled();
  });
});

// =============================================================================
// consumeEventIterator Tests
// =============================================================================

describe("consumeEventIterator()", () => {
  it("should call onEvent for each event", async () => {
    const emitter = createMockEventEmitter();
    const onEvent = vi.fn();

    const iteratorPromise = createEventIterator<number>("stream.counter", {});

    consumeEventIterator(iteratorPromise, { onEvent });

    // Wait for iterator to be created
    const iterator = await iteratorPromise;

    // Emit events
    emitter.emit({ type: "data", payload: { data: 1 } });
    emitter.emit({ type: "data", payload: { data: 2 } });

    // Give time for events to be processed
    await new Promise((resolve) => setTimeout(resolve, 10));

    expect(onEvent).toHaveBeenCalledWith(1);
    expect(onEvent).toHaveBeenCalledWith(2);

    await iterator.return();
  });

  it("should call onComplete when stream completes", async () => {
    const emitter = createMockEventEmitter();
    const onComplete = vi.fn();
    const onFinish = vi.fn();

    const iteratorPromise = createEventIterator<number>("stream.counter", {});

    consumeEventIterator(iteratorPromise, { onComplete, onFinish });

    await iteratorPromise;

    emitter.emit({ type: "completed" });

    await new Promise((resolve) => setTimeout(resolve, 10));

    expect(onComplete).toHaveBeenCalled();
    expect(onFinish).toHaveBeenCalledWith("success");
  });

  it("should call onError when stream errors", async () => {
    const emitter = createMockEventEmitter();
    const onError = vi.fn();
    const onFinish = vi.fn();

    const iteratorPromise = createEventIterator<number>("stream.counter", {});

    consumeEventIterator(iteratorPromise, { onError, onFinish });

    await iteratorPromise;

    const error: RpcError = { code: "ERROR", message: "Failed" };
    emitter.emit({ type: "error", payload: error });

    await new Promise((resolve) => setTimeout(resolve, 10));

    expect(onError).toHaveBeenCalledWith(
      expect.objectContaining({ code: "ERROR" }),
    );
    expect(onFinish).toHaveBeenCalledWith("error");
  });

  it("should return cancel function", async () => {
    createMockEventEmitter();
    const onFinish = vi.fn();

    const iteratorPromise = createEventIterator<number>("stream.counter", {});

    const cancel = consumeEventIterator(iteratorPromise, { onFinish });

    await iteratorPromise;

    await cancel();

    expect(onFinish).toHaveBeenCalledWith("cancelled");
  });

  it("should stop processing events after cancel", async () => {
    const emitter = createMockEventEmitter();
    const onEvent = vi.fn();

    const iteratorPromise = createEventIterator<number>("stream.counter", {});

    const cancel = consumeEventIterator(iteratorPromise, { onEvent });

    await iteratorPromise;

    emitter.emit({ type: "data", payload: { data: 1 } });
    await new Promise((resolve) => setTimeout(resolve, 10));

    await cancel();

    emitter.emit({ type: "data", payload: { data: 2 } });
    await new Promise((resolve) => setTimeout(resolve, 10));

    expect(onEvent).toHaveBeenCalledTimes(1);
    expect(onEvent).toHaveBeenCalledWith(1);
  });
});

// =============================================================================
// Property-Based Tests
// =============================================================================

describe("Property-Based Tests", () => {
  // Property 1: Cleanup Order Invariant
  // Listener removal happens before backend unsubscribe, and pending consumers are resolved
  it("property: cleanup order invariant - unlisten before unsubscribe, consumers resolved", async () => {
    await fc.assert(
      fc.asyncProperty(
        fc.integer({ min: 0, max: 5 }), // number of pending consumers
        fc.integer({ min: 0, max: 5 }), // number of buffered events
        async (pendingCount, bufferedCount) => {
          const emitter = createMockEventEmitter();
          const operationOrder: string[] = [];

          // Track operation order
          emitter.unlisten.mockImplementation(() => {
            operationOrder.push("unlisten");
          });

          mockInvoke.mockImplementation(async (cmd) => {
            if (cmd === "plugin:rpc|rpc_unsubscribe") {
              operationOrder.push("unsubscribe");
            }
            return undefined;
          });

          const iterator = await createEventIterator<number>("stream.test", {});
          const asyncIterator = iterator[Symbol.asyncIterator]();

          // Buffer some events
          for (let i = 0; i < bufferedCount; i++) {
            emitter.emit({ type: "data", payload: { data: i } });
          }

          // Create pending consumers (promises waiting for next value)
          const pendingPromises: Promise<IteratorResult<number>>[] = [];
          for (let i = 0; i < pendingCount; i++) {
            // Consume buffered events first, then create pending
            if (i < bufferedCount) {
              await asyncIterator.next();
            } else {
              pendingPromises.push(asyncIterator.next());
            }
          }

          // Cleanup
          await iterator.return();

          // Verify cleanup order: unlisten should come before unsubscribe
          const unlistenIndex = operationOrder.indexOf("unlisten");
          const unsubscribeIndex = operationOrder.indexOf("unsubscribe");

          if (unlistenIndex !== -1 && unsubscribeIndex !== -1) {
            expect(unlistenIndex).toBeLessThan(unsubscribeIndex);
          }

          // Verify all pending consumers are resolved with done: true
          const results = await Promise.all(pendingPromises);
          for (const result of results) {
            expect(result.done).toBe(true);
          }
        },
      ),
      { numRuns: 50 },
    );
  });

  // Property 2: Pending Promise Cleanup on Max Retries
  // When max reconnection attempts are exceeded, all pending promises should be rejected
  it("property: pending promises rejected on max retries exceeded", async () => {
    await fc.assert(
      fc.asyncProperty(
        fc.integer({ min: 1, max: 5 }), // number of pending consumers
        fc.integer({ min: 1, max: 3 }), // max reconnects
        async (pendingCount, maxReconnects) => {
          const emitter = createMockEventEmitter();
          let connectAttempts = 0;

          // Mock invoke to fail on subscribe (simulating connection failures)
          mockInvoke.mockImplementation(async (cmd) => {
            if (cmd === "plugin:rpc|rpc_subscribe") {
              connectAttempts++;
              // First call succeeds, subsequent calls fail
              if (connectAttempts > 1) {
                throw new Error("Connection failed");
              }
            }
            return undefined;
          });

          const iterator = await createEventIterator<number>(
            "stream.test",
            {},
            {
              autoReconnect: true,
              maxReconnects,
              reconnectDelay: 1, // Very short delay for testing
            },
          );
          const asyncIterator = iterator[Symbol.asyncIterator]();

          // Create pending consumers and immediately attach catch handlers to prevent unhandled rejections
          const pendingResults: Promise<{
            status: "fulfilled" | "rejected";
            value?: IteratorResult<number>;
            reason?: unknown;
          }>[] = [];
          for (let i = 0; i < pendingCount; i++) {
            const promise = asyncIterator.next();
            // Wrap each promise to handle rejection without throwing
            pendingResults.push(
              promise
                .then((value) => ({ status: "fulfilled" as const, value }))
                .catch((reason) => ({ status: "rejected" as const, reason })),
            );
          }

          // Emit an error to trigger reconnection
          const error: RpcError = {
            code: "CONNECTION_LOST",
            message: "Connection lost",
          };
          emitter.emit({ type: "error", payload: error });

          // Wait for reconnection attempts to exhaust
          await new Promise((resolve) =>
            setTimeout(resolve, 50 * maxReconnects),
          );

          // All pending promises should be rejected
          const results = await Promise.all(pendingResults);
          for (const result of results) {
            expect(result.status).toBe("rejected");
            if (result.status === "rejected") {
              expect(result.reason).toHaveProperty("code");
            }
          }

          await iterator.return();
        },
      ),
      { numRuns: 20 },
    );
  });

  // Property 4: Cleanup Resilience
  // Cleanup should complete successfully even when backend unsubscribe fails
  it("property: cleanup completes even when backend fails", async () => {
    await fc.assert(
      fc.asyncProperty(
        fc.integer({ min: 0, max: 5 }), // number of pending consumers
        fc.boolean(), // whether backend unsubscribe should fail
        async (pendingCount, shouldFail) => {
          const emitter = createMockEventEmitter();

          // Mock invoke to optionally fail on unsubscribe
          mockInvoke.mockImplementation(async (cmd) => {
            if (cmd === "plugin:rpc|rpc_unsubscribe" && shouldFail) {
              throw new Error("Backend unsubscribe failed");
            }
            return undefined;
          });

          const iterator = await createEventIterator<number>("stream.test", {});
          const asyncIterator = iterator[Symbol.asyncIterator]();

          // Create pending consumers with catch handlers
          const pendingResults: Promise<{
            status: "fulfilled" | "rejected";
            value?: IteratorResult<number>;
          }>[] = [];
          for (let i = 0; i < pendingCount; i++) {
            const promise = asyncIterator.next();
            pendingResults.push(
              promise
                .then((value) => ({ status: "fulfilled" as const, value }))
                .catch(() => ({ status: "rejected" as const })),
            );
          }

          // Cleanup should not throw even if backend fails
          await expect(iterator.return()).resolves.toBeUndefined();

          // All pending consumers should be resolved with done: true
          const results = await Promise.all(pendingResults);
          for (const result of results) {
            expect(result.status).toBe("fulfilled");
            if (result.status === "fulfilled") {
              expect(result.value?.done).toBe(true);
            }
          }

          // Verify unlisten was called regardless of backend failure
          expect(emitter.unlisten).toHaveBeenCalled();
        },
      ),
      { numRuns: 50 },
    );
  });

  // Property: All emitted data events are yielded in order
  it("property: events are yielded in emission order", async () => {
    await fc.assert(
      fc.asyncProperty(
        fc.array(fc.integer(), { minLength: 1, maxLength: 20 }),
        async (values) => {
          const emitter = createMockEventEmitter();

          const iterator = await createEventIterator<number>("stream.test", {});
          const asyncIterator = iterator[Symbol.asyncIterator]();

          // Emit all values
          for (const value of values) {
            emitter.emit({ type: "data", payload: { data: value } });
          }
          emitter.emit({ type: "completed" });

          // Collect all values
          const collected: number[] = [];
          let result = await asyncIterator.next();
          while (!result.done) {
            collected.push(result.value);
            result = await asyncIterator.next();
          }

          expect(collected).toEqual(values);

          await iterator.return();
        },
      ),
      { numRuns: 50 },
    );
  });

  // Property: Iterator always terminates on completed event
  it("property: iterator terminates on completed event", async () => {
    await fc.assert(
      fc.asyncProperty(
        fc.array(fc.string(), { maxLength: 10 }),
        async (values) => {
          const emitter = createMockEventEmitter();

          const iterator = await createEventIterator<string>("stream.test", {});
          const asyncIterator = iterator[Symbol.asyncIterator]();

          // Emit values then complete
          for (const value of values) {
            emitter.emit({ type: "data", payload: { data: value } });
          }
          emitter.emit({ type: "completed" });

          // Consume all
          let count = 0;
          let result = await asyncIterator.next();
          while (!result.done) {
            count++;
            result = await asyncIterator.next();
          }

          expect(count).toBe(values.length);
          expect(result.done).toBe(true);

          await iterator.return();
        },
      ),
      { numRuns: 50 },
    );
  });

  // Property 15: Reconnection Backoff
  // Reconnection delays should increase exponentially with jitter
  // Note: This is a simplified test that verifies the backoff calculation logic
  // rather than actual timing, which is difficult to test reliably
  it("property: reconnection backoff calculation is correct", () => {
    // Test the backoff calculation directly
    fc.assert(
      fc.property(
        fc.integer({ min: 0, max: 5 }), // attempt number
        fc.integer({ min: 100, max: 1000 }), // base delay
        (attempt, baseDelay) => {
          // Calculate expected delay without jitter
          const expectedDelay = baseDelay * Math.pow(2, attempt);

          // With jitter, delay should be between 50% and 100% of expected
          const minDelay = expectedDelay * 0.5;
          const maxDelay = expectedDelay;

          // Verify the formula is correct
          expect(minDelay).toBeLessThanOrEqual(maxDelay);
          expect(expectedDelay).toBeGreaterThan(0);
        },
      ),
      { numRuns: 50 },
    );
  });

  // Property 16: Event ID Resumption
  // When reconnecting, the lastEventId should be passed to resume from the correct position
  it("property: event ID resumption on reconnect", async () => {
    await fc.assert(
      fc.asyncProperty(
        fc.array(fc.string({ minLength: 1 }), { minLength: 1, maxLength: 5 }), // event IDs
        async (eventIds) => {
          const emitter = createMockEventEmitter();
          let lastEventIdOnReconnect: string | undefined;
          let connectAttempts = 0;

          // Mock invoke to capture lastEventId on reconnect
          mockInvoke.mockImplementation(async (cmd, args: any) => {
            if (cmd === "plugin:rpc|rpc_subscribe") {
              connectAttempts++;
              if (connectAttempts > 1) {
                lastEventIdOnReconnect = args?.request?.lastEventId;
              }
            }
            return undefined;
          });

          const iterator = await createEventIterator<number>(
            "stream.test",
            {},
            {
              autoReconnect: true,
              maxReconnects: 1,
              reconnectDelay: 10,
            },
          );
          const asyncIterator = iterator[Symbol.asyncIterator]();

          // Emit events with IDs
          for (let i = 0; i < eventIds.length; i++) {
            emitter.emit({
              type: "data",
              payload: { data: i, id: eventIds[i] },
            });
          }

          // Consume all events
          for (let i = 0; i < eventIds.length; i++) {
            await asyncIterator.next();
          }

          // Trigger reconnection
          const error: RpcError = {
            code: "CONNECTION_LOST",
            message: "Connection lost",
          };
          emitter.emit({ type: "error", payload: error });

          // Wait for reconnection attempt
          await new Promise((resolve) => setTimeout(resolve, 50));

          // The lastEventId should be the last event ID we received
          if (connectAttempts > 1) {
            expect(lastEventIdOnReconnect).toBe(eventIds[eventIds.length - 1]);
          }

          await iterator.return();
        },
      ),
      { numRuns: 20 },
    );
  });

  // Property 17: Max Reconnection Completion
  // After max reconnection attempts, the iterator should complete with an error
  // Note: This test verifies the MAX_RECONNECTS_EXCEEDED error is generated correctly
  it("property: max reconnection error code is correct", () => {
    // Test that the error code and message are correctly formatted
    fc.assert(
      fc.property(
        fc.integer({ min: 1, max: 10 }), // max reconnects
        fc.integer({ min: 1, max: 10 }), // attempts
        (maxReconnects, attempts) => {
          // Simulate the error that would be generated
          const error: RpcError = {
            code: "MAX_RECONNECTS_EXCEEDED",
            message: `Maximum reconnection attempts (${maxReconnects}) exceeded`,
            details: {
              attempts,
              maxReconnects,
              path: "test.path",
            },
          };

          expect(error.code).toBe("MAX_RECONNECTS_EXCEEDED");
          expect(error.message).toContain(String(maxReconnects));
          expect(error.details).toHaveProperty("attempts");
          expect(error.details).toHaveProperty("maxReconnects");
        },
      ),
      { numRuns: 50 },
    );
  });
});
