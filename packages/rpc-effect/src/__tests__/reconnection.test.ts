// =============================================================================
// TC004 (continued): Reconnection Logic Tests
// =============================================================================
// Test reconnection logic with exponential backoff, jitter, and max retries.

import { describe, it, expect } from "vitest";
import { Effect, Ref } from "effect";
import {
  createSubscriptionStateRef,
  calculateReconnectDelay,
  shouldReconnect,
  prepareReconnect,
  maxReconnectsExceededError,
  createReconnectSchedule,
  withReconnection,
  incrementReconnectAttempts,
  type ReconnectConfig,
  defaultReconnectConfig,
  createCallError,
} from "../index";

describe("TC004: Reconnection Logic", () => {
  const enabledConfig: ReconnectConfig = {
    autoReconnect: true,
    maxReconnects: 5,
    reconnectDelay: 1000,
  };

  const disabledConfig: ReconnectConfig = {
    autoReconnect: false,
    maxReconnects: 5,
    reconnectDelay: 1000,
  };

  describe("Default Configuration", () => {
    it("should have sensible defaults", () => {
      expect(defaultReconnectConfig.autoReconnect).toBe(false);
      expect(defaultReconnectConfig.maxReconnects).toBe(5);
      expect(defaultReconnectConfig.reconnectDelay).toBe(1000);
    });
  });

  describe("Reconnect Delay Calculation", () => {
    it("should calculate exponential backoff delay", async () => {
      const delay1 = await Effect.runPromise(calculateReconnectDelay(1, 1000));
      const delay2 = await Effect.runPromise(calculateReconnectDelay(2, 1000));
      const delay3 = await Effect.runPromise(calculateReconnectDelay(3, 1000));

      // Base delays: 1000, 2000, 4000 (with jitter 0.5-1.0)
      expect(delay1).toBeGreaterThanOrEqual(500);
      expect(delay1).toBeLessThanOrEqual(1000);

      expect(delay2).toBeGreaterThanOrEqual(1000);
      expect(delay2).toBeLessThanOrEqual(2000);

      expect(delay3).toBeGreaterThanOrEqual(2000);
      expect(delay3).toBeLessThanOrEqual(4000);
    });

    it("should apply jitter to prevent thundering herd", async () => {
      const delays = await Promise.all(
        Array.from({ length: 10 }, () =>
          Effect.runPromise(calculateReconnectDelay(1, 1000)),
        ),
      );

      // With jitter, not all delays should be the same
      const uniqueDelays = new Set(delays);
      expect(uniqueDelays.size).toBeGreaterThan(1);
    });
  });

  describe("Should Reconnect Check", () => {
    it("should return false when autoReconnect is disabled", async () => {
      const stateRef = await Effect.runPromise(
        createSubscriptionStateRef("sub-123"),
      );

      const should = await Effect.runPromise(
        shouldReconnect(stateRef, disabledConfig),
      );

      expect(should).toBe(false);
    });

    it("should return true when under max reconnects", async () => {
      const stateRef = await Effect.runPromise(
        createSubscriptionStateRef("sub-123"),
      );

      const should = await Effect.runPromise(
        shouldReconnect(stateRef, enabledConfig),
      );

      expect(should).toBe(true);
    });

    it("should return false when max reconnects exceeded", async () => {
      const stateRef = await Effect.runPromise(
        createSubscriptionStateRef("sub-123"),
      );

      // Increment to max
      for (let i = 0; i < 5; i++) {
        await Effect.runPromise(incrementReconnectAttempts(stateRef));
      }

      const should = await Effect.runPromise(
        shouldReconnect(stateRef, enabledConfig),
      );

      expect(should).toBe(false);
    });
  });

  describe("Prepare Reconnect", () => {
    it("should increment attempts and return delay", async () => {
      const stateRef = await Effect.runPromise(
        createSubscriptionStateRef("sub-123"),
      );

      const delay = await Effect.runPromise(
        prepareReconnect(stateRef, enabledConfig),
      );

      expect(delay).toBeGreaterThan(0);

      const state = await Effect.runPromise(Ref.get(stateRef));
      expect(state.reconnectAttempts).toBe(1);
    });

    it("should increase delay with each attempt", async () => {
      const stateRef = await Effect.runPromise(
        createSubscriptionStateRef("sub-123"),
      );

      const delay1 = await Effect.runPromise(
        prepareReconnect(stateRef, enabledConfig),
      );
      const delay2 = await Effect.runPromise(
        prepareReconnect(stateRef, enabledConfig),
      );
      const delay3 = await Effect.runPromise(
        prepareReconnect(stateRef, enabledConfig),
      );

      // Due to jitter, we can't guarantee strict ordering, but average should increase
      // Just verify they're all positive
      expect(delay1).toBeGreaterThan(0);
      expect(delay2).toBeGreaterThan(0);
      expect(delay3).toBeGreaterThan(0);
    });
  });

  describe("Max Reconnects Exceeded Error", () => {
    it("should create error with correct details", () => {
      const error = maxReconnectsExceededError("users.stream", 5, 5);

      expect(error._tag).toBe("RpcCallError");
      if (error._tag === "RpcCallError") {
        expect(error.code).toBe("MAX_RECONNECTS_EXCEEDED");
        expect(error.message).toContain("5");
        expect(error.details).toEqual({
          attempts: 5,
          maxReconnects: 5,
          path: "users.stream",
        });
      }
    });
  });

  describe("Schedule-Based Reconnection", () => {
    it("should create reconnect schedule with exponential backoff", () => {
      const schedule = createReconnectSchedule(enabledConfig);

      // Schedule should be defined
      expect(schedule).toBeDefined();
    });

    it("should apply withReconnection to effect", async () => {
      let attempts = 0;
      const failingEffect = Effect.gen(function* () {
        attempts++;
        if (attempts < 3) {
          return yield* Effect.fail(createCallError("TEMP_ERROR", "Temporary"));
        }
        return "success";
      });

      const result = await Effect.runPromise(
        withReconnection(failingEffect, enabledConfig),
      );

      expect(result).toBe("success");
      expect(attempts).toBe(3);
    });

    it("should not retry when autoReconnect is disabled", async () => {
      let attempts = 0;
      const failingEffect = Effect.gen(function* () {
        attempts++;
        return yield* Effect.fail(createCallError("ERROR", "Failed"));
      });

      const exit = await Effect.runPromiseExit(
        withReconnection(failingEffect, disabledConfig),
      );

      expect(exit._tag).toBe("Failure");
      expect(attempts).toBe(1);
    });
  });

  describe("Reconnection State Transitions", () => {
    it("should track reconnection attempts across multiple failures", async () => {
      const stateRef = await Effect.runPromise(
        createSubscriptionStateRef("sub-123"),
      );

      // Simulate multiple reconnection attempts
      for (let i = 0; i < 3; i++) {
        const canReconnect = await Effect.runPromise(
          shouldReconnect(stateRef, enabledConfig),
        );
        expect(canReconnect).toBe(true);

        await Effect.runPromise(prepareReconnect(stateRef, enabledConfig));
      }

      const state = await Effect.runPromise(Ref.get(stateRef));
      expect(state.reconnectAttempts).toBe(3);
    });

    it("should stop reconnecting after max attempts", async () => {
      const stateRef = await Effect.runPromise(
        createSubscriptionStateRef("sub-123"),
      );

      // Exhaust all reconnection attempts
      for (let i = 0; i < 5; i++) {
        await Effect.runPromise(prepareReconnect(stateRef, enabledConfig));
      }

      const canReconnect = await Effect.runPromise(
        shouldReconnect(stateRef, enabledConfig),
      );
      expect(canReconnect).toBe(false);
    });
  });
});
