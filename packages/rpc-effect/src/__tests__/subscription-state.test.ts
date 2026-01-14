// =============================================================================
// TC004: Subscription Stream State Management Tests
// =============================================================================
// Test that subscription streams maintain consistent state, handle reconnection
// attempts transparently, emit events correctly, and support async iteration.

import { describe, it, expect, beforeEach } from "vitest";
import { Effect, Ref, Queue } from "effect";
import {
  createSubscriptionState,
  createSubscriptionStateRef,
  createEventQueue,
  markCompleted,
  updateLastEventId,
  incrementConsumers,
  decrementConsumers,
  resetForReconnect,
  incrementReconnectAttempts,
  resetReconnectAttempts,
  incrementAndGetConsumers,
  decrementAndGetConsumers,
  incrementAndGetReconnectAttempts,
  markCompletedOnce,
  updateAndGetLastEventId,
  getState,
  offerEvent,
  sendShutdownSentinels,
  takeFromQueue,
  SHUTDOWN_SENTINEL,
  type SubscriptionState,
  type SubscriptionEvent,
  type QueueItem,
} from "../index";

describe("TC004: Subscription State Management", () => {
  describe("State Creation", () => {
    it("should create initial subscription state with defaults", () => {
      const state = createSubscriptionState("sub-123");

      expect(state.id).toBe("sub-123");
      expect(state.reconnectAttempts).toBe(0);
      expect(state.lastEventId).toBeUndefined();
      expect(state.completed).toBe(false);
      expect(state.pendingConsumers).toBe(0);
    });

    it("should create state with optional lastEventId", () => {
      const state = createSubscriptionState("sub-123", "event-456");

      expect(state.lastEventId).toBe("event-456");
    });

    it("should create managed state ref", async () => {
      const program = Effect.gen(function* () {
        const stateRef = yield* createSubscriptionStateRef("sub-123");
        const state = yield* Ref.get(stateRef);
        return state;
      });

      const state = await Effect.runPromise(program);
      expect(state.id).toBe("sub-123");
    });
  });

  describe("State Operations", () => {
    let stateRef: Ref.Ref<SubscriptionState>;

    beforeEach(async () => {
      stateRef = await Effect.runPromise(createSubscriptionStateRef("sub-123"));
    });

    it("should mark subscription as completed", async () => {
      await Effect.runPromise(markCompleted(stateRef));
      const state = await Effect.runPromise(Ref.get(stateRef));

      expect(state.completed).toBe(true);
    });

    it("should update last event ID", async () => {
      await Effect.runPromise(updateLastEventId(stateRef, "event-789"));
      const state = await Effect.runPromise(Ref.get(stateRef));

      expect(state.lastEventId).toBe("event-789");
    });

    it("should increment and decrement consumers", async () => {
      await Effect.runPromise(incrementConsumers(stateRef));
      let state = await Effect.runPromise(Ref.get(stateRef));
      expect(state.pendingConsumers).toBe(1);

      await Effect.runPromise(incrementConsumers(stateRef));
      state = await Effect.runPromise(Ref.get(stateRef));
      expect(state.pendingConsumers).toBe(2);

      await Effect.runPromise(decrementConsumers(stateRef));
      state = await Effect.runPromise(Ref.get(stateRef));
      expect(state.pendingConsumers).toBe(1);
    });

    it("should not decrement below zero", async () => {
      await Effect.runPromise(decrementConsumers(stateRef));
      const state = await Effect.runPromise(Ref.get(stateRef));

      expect(state.pendingConsumers).toBe(0);
    });

    it("should reset for reconnect with new ID", async () => {
      await Effect.runPromise(markCompleted(stateRef));
      await Effect.runPromise(resetForReconnect(stateRef, "new-sub-456"));

      const state = await Effect.runPromise(Ref.get(stateRef));
      expect(state.id).toBe("new-sub-456");
      expect(state.completed).toBe(false);
    });

    it("should increment and reset reconnect attempts", async () => {
      await Effect.runPromise(incrementReconnectAttempts(stateRef));
      await Effect.runPromise(incrementReconnectAttempts(stateRef));

      let state = await Effect.runPromise(Ref.get(stateRef));
      expect(state.reconnectAttempts).toBe(2);

      await Effect.runPromise(resetReconnectAttempts(stateRef));
      state = await Effect.runPromise(Ref.get(stateRef));
      expect(state.reconnectAttempts).toBe(0);
    });
  });

  describe("Atomic State Operations (Ref.modify)", () => {
    let stateRef: Ref.Ref<SubscriptionState>;

    beforeEach(async () => {
      stateRef = await Effect.runPromise(createSubscriptionStateRef("sub-123"));
    });

    it("should atomically increment and return previous consumer count", async () => {
      const prevCount = await Effect.runPromise(
        incrementAndGetConsumers(stateRef),
      );
      expect(prevCount).toBe(0);

      const nextPrevCount = await Effect.runPromise(
        incrementAndGetConsumers(stateRef),
      );
      expect(nextPrevCount).toBe(1);

      const state = await Effect.runPromise(Ref.get(stateRef));
      expect(state.pendingConsumers).toBe(2);
    });

    it("should atomically decrement and return new consumer count", async () => {
      await Effect.runPromise(incrementConsumers(stateRef));
      await Effect.runPromise(incrementConsumers(stateRef));

      const newCount = await Effect.runPromise(
        decrementAndGetConsumers(stateRef),
      );
      expect(newCount).toBe(1);
    });

    it("should atomically increment and return new reconnect attempts", async () => {
      const count1 = await Effect.runPromise(
        incrementAndGetReconnectAttempts(stateRef),
      );
      expect(count1).toBe(1);

      const count2 = await Effect.runPromise(
        incrementAndGetReconnectAttempts(stateRef),
      );
      expect(count2).toBe(2);
    });

    it("should atomically mark completed and return previous state", async () => {
      const wasCompleted1 = await Effect.runPromise(
        markCompletedOnce(stateRef),
      );
      expect(wasCompleted1).toBe(false);

      const wasCompleted2 = await Effect.runPromise(
        markCompletedOnce(stateRef),
      );
      expect(wasCompleted2).toBe(true);
    });

    it("should atomically update and return previous event ID", async () => {
      const prevId1 = await Effect.runPromise(
        updateAndGetLastEventId(stateRef, "event-1"),
      );
      expect(prevId1).toBeUndefined();

      const prevId2 = await Effect.runPromise(
        updateAndGetLastEventId(stateRef, "event-2"),
      );
      expect(prevId2).toBe("event-1");
    });

    it("should get current state snapshot", async () => {
      await Effect.runPromise(incrementConsumers(stateRef));
      await Effect.runPromise(updateLastEventId(stateRef, "event-123"));

      const state = await Effect.runPromise(getState(stateRef));
      expect(state.pendingConsumers).toBe(1);
      expect(state.lastEventId).toBe("event-123");
    });
  });

  describe("Event Queue Operations", () => {
    let queue: Queue.Queue<QueueItem<string>>;

    beforeEach(async () => {
      queue = await Effect.runPromise(createEventQueue<string>());
    });

    it("should offer events to queue", async () => {
      const event: SubscriptionEvent<string> = {
        type: "data",
        payload: { id: "1", data: "test" },
      };

      const offered = await Effect.runPromise(offerEvent(queue, event));
      expect(offered).toBe(true);

      const size = await Effect.runPromise(Queue.size(queue));
      expect(size).toBe(1);
    });

    it("should take events from queue", async () => {
      const event: SubscriptionEvent<string> = {
        type: "data",
        payload: { id: "1", data: "test" },
      };

      await Effect.runPromise(offerEvent(queue, event));
      const taken = await Effect.runPromise(takeFromQueue(queue));

      expect(taken).toEqual(event);
    });

    it("should send shutdown sentinels", async () => {
      await Effect.runPromise(sendShutdownSentinels(queue, 3));

      const size = await Effect.runPromise(Queue.size(queue));
      expect(size).toBe(4); // count + 1

      const item = await Effect.runPromise(takeFromQueue(queue));
      expect(item).toBe(SHUTDOWN_SENTINEL);
    });

    it("should handle different event types", async () => {
      const dataEvent: SubscriptionEvent<string> = {
        type: "data",
        payload: { id: "1", data: "test" },
      };
      const errorEvent: SubscriptionEvent<string> = {
        type: "error",
        payload: { code: "ERROR", message: "Test error" },
      };
      const completedEvent: SubscriptionEvent<string> = {
        type: "completed",
      };

      await Effect.runPromise(offerEvent(queue, dataEvent));
      await Effect.runPromise(offerEvent(queue, errorEvent));
      await Effect.runPromise(offerEvent(queue, completedEvent));

      const item1 = await Effect.runPromise(takeFromQueue(queue));
      const item2 = await Effect.runPromise(takeFromQueue(queue));
      const item3 = await Effect.runPromise(takeFromQueue(queue));

      expect((item1 as SubscriptionEvent<string>).type).toBe("data");
      expect((item2 as SubscriptionEvent<string>).type).toBe("error");
      expect((item3 as SubscriptionEvent<string>).type).toBe("completed");
    });
  });

  describe("Concurrent State Access", () => {
    it("should handle concurrent consumer increments correctly", async () => {
      const stateRef = await Effect.runPromise(
        createSubscriptionStateRef("sub-123"),
      );

      // Simulate concurrent increments
      const increments = Array.from({ length: 100 }, () =>
        incrementConsumers(stateRef),
      );

      await Effect.runPromise(Effect.all(increments, { concurrency: 10 }));

      const state = await Effect.runPromise(Ref.get(stateRef));
      expect(state.pendingConsumers).toBe(100);
    });

    it("should handle concurrent increment/decrement correctly", async () => {
      const stateRef = await Effect.runPromise(
        createSubscriptionStateRef("sub-123"),
      );

      // Start with 50 consumers
      const initialIncrements = Array.from({ length: 50 }, () =>
        incrementConsumers(stateRef),
      );
      await Effect.runPromise(Effect.all(initialIncrements));

      // Concurrent increments and decrements
      const operations = [
        ...Array.from({ length: 30 }, () => incrementConsumers(stateRef)),
        ...Array.from({ length: 20 }, () => decrementConsumers(stateRef)),
      ];

      await Effect.runPromise(Effect.all(operations, { concurrency: 10 }));

      const state = await Effect.runPromise(Ref.get(stateRef));
      expect(state.pendingConsumers).toBe(60); // 50 + 30 - 20
    });
  });
});
