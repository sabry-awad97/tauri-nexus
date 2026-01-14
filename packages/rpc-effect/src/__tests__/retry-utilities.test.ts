// =============================================================================
// TC006: Retry Utilities Tests
// =============================================================================
// Test retry utilities with configurable backoff strategies and jitter.

import { describe, it, expect } from "vitest";
import { Effect } from "effect";
import {
  defaultRetryConfig,
  createRetrySchedule,
  withRetry,
  withRetryDetailed,
  type RetryConfig,
  createCallError,
} from "../index";

describe("TC006: Retry Utilities", () => {
  describe("Default Configuration", () => {
    it("should have sensible defaults", () => {
      expect(defaultRetryConfig.maxRetries).toBe(3);
      expect(defaultRetryConfig.baseDelay).toBe(1000);
      expect(defaultRetryConfig.maxDelay).toBe(30000);
      expect(defaultRetryConfig.jitter).toBe(true);
      expect(defaultRetryConfig.backoff).toBe("exponential");
      expect(defaultRetryConfig.retryableCodes).toContain("INTERNAL_ERROR");
      expect(defaultRetryConfig.retryableCodes).toContain("TIMEOUT");
      expect(defaultRetryConfig.retryableCodes).toContain("UNAVAILABLE");
    });

    it("should have readonly retryableCodes array", () => {
      expect(Array.isArray(defaultRetryConfig.retryableCodes)).toBe(true);
    });
  });

  describe("Retry Schedule", () => {
    it("should create schedule with default config", () => {
      const schedule = createRetrySchedule();
      expect(schedule).toBeDefined();
    });

    it("should create schedule with custom config", () => {
      const config: Partial<RetryConfig> = {
        maxRetries: 5,
        baseDelay: 500,
        backoff: "linear",
      };

      const schedule = createRetrySchedule(config);
      expect(schedule).toBeDefined();
    });

    it("should create schedule with exponential backoff", () => {
      const schedule = createRetrySchedule({ backoff: "exponential" });
      expect(schedule).toBeDefined();
    });

    it("should create schedule with linear backoff", () => {
      const schedule = createRetrySchedule({ backoff: "linear" });
      expect(schedule).toBeDefined();
    });

    it("should create schedule with jitter disabled", () => {
      const schedule = createRetrySchedule({ jitter: false });
      expect(schedule).toBeDefined();
    });
  });

  describe("withRetry", () => {
    it("should succeed on first attempt without retry", async () => {
      let attempts = 0;
      const effect = Effect.sync(() => {
        attempts++;
        return "success";
      });

      const result = await Effect.runPromise(withRetry(effect));

      expect(result).toBe("success");
      expect(attempts).toBe(1);
    });

    it("should retry on retryable error and eventually succeed", async () => {
      let attempts = 0;
      const effect = Effect.gen(function* () {
        attempts++;
        if (attempts < 3) {
          return yield* Effect.fail(
            createCallError("INTERNAL_ERROR", "Temporary failure"),
          );
        }
        return "success";
      });

      const config: Partial<RetryConfig> = {
        maxRetries: 5,
        baseDelay: 10,
        jitter: false,
      };

      const result = await Effect.runPromise(withRetry(effect, config));

      expect(result).toBe("success");
      expect(attempts).toBe(3);
    });

    it("should fail after max retries exceeded", async () => {
      let attempts = 0;
      const effect = Effect.gen(function* () {
        attempts++;
        return yield* Effect.fail(
          createCallError("INTERNAL_ERROR", "Permanent failure"),
        );
      });

      const config: Partial<RetryConfig> = {
        maxRetries: 3,
        baseDelay: 10,
        jitter: false,
      };

      const exit = await Effect.runPromiseExit(withRetry(effect, config));

      expect(exit._tag).toBe("Failure");
      expect(attempts).toBe(4); // Initial + 3 retries
    });

    it("should not retry non-retryable errors", async () => {
      let attempts = 0;
      const effect = Effect.gen(function* () {
        attempts++;
        return yield* Effect.fail(
          createCallError("VALIDATION_ERROR", "Invalid input"),
        );
      });

      const config: Partial<RetryConfig> = {
        maxRetries: 5,
        baseDelay: 10,
        retryableCodes: ["INTERNAL_ERROR"], // VALIDATION_ERROR not included
      };

      const exit = await Effect.runPromiseExit(withRetry(effect, config));

      expect(exit._tag).toBe("Failure");
      expect(attempts).toBe(1); // No retries for non-retryable error
    });

    it("should use custom retry config", async () => {
      let attempts = 0;
      const effect = Effect.gen(function* () {
        attempts++;
        if (attempts < 5) {
          return yield* Effect.fail(
            createCallError("TIMEOUT", "Request timeout"),
          );
        }
        return "success";
      });

      const config: Partial<RetryConfig> = {
        maxRetries: 5,
        baseDelay: 10,
        jitter: false,
        retryableCodes: ["TIMEOUT"],
      };

      const result = await Effect.runPromise(withRetry(effect, config));

      expect(result).toBe("success");
      expect(attempts).toBe(5);
    });
  });

  describe("withRetryDetailed", () => {
    it("should return detailed result on success", async () => {
      const effect = Effect.succeed("success");

      const result = await Effect.runPromise(withRetryDetailed(effect));

      expect(result.result).toBe("success");
      expect(result.attempts).toBeGreaterThanOrEqual(1);
    });

    it("should track retry attempts in detailed result", async () => {
      let attempts = 0;
      const effect = Effect.gen(function* () {
        attempts++;
        if (attempts < 3) {
          return yield* Effect.fail(
            createCallError("INTERNAL_ERROR", "Retry needed"),
          );
        }
        return "success";
      });

      const config: Partial<RetryConfig> = {
        maxRetries: 5,
        baseDelay: 10,
        jitter: false,
      };

      const result = await Effect.runPromise(withRetryDetailed(effect, config));

      expect(result.result).toBe("success");
      expect(result.attempts).toBe(3);
    });

    it("should fail with detailed error after max retries", async () => {
      let attempts = 0;
      const effect = Effect.gen(function* () {
        attempts++;
        return yield* Effect.fail(
          createCallError("UNAVAILABLE", "Service unavailable"),
        );
      });

      const config: Partial<RetryConfig> = {
        maxRetries: 2,
        baseDelay: 10,
        jitter: false,
      };

      const exit = await Effect.runPromiseExit(
        withRetryDetailed(effect, config),
      );

      expect(exit._tag).toBe("Failure");
      expect(attempts).toBe(3); // Initial + 2 retries
    });
  });

  describe("Retry with Different Error Types", () => {
    it("should retry RpcCallError with retryable code", async () => {
      let attempts = 0;
      const effect = Effect.gen(function* () {
        attempts++;
        if (attempts < 2) {
          return yield* Effect.fail(
            createCallError("INTERNAL_ERROR", "Server error"),
          );
        }
        return "recovered";
      });

      const config: Partial<RetryConfig> = {
        maxRetries: 3,
        baseDelay: 10,
        jitter: false,
      };

      const result = await Effect.runPromise(withRetry(effect, config));

      expect(result).toBe("recovered");
      expect(attempts).toBe(2);
    });

    it("should handle TIMEOUT error code", async () => {
      let attempts = 0;
      const effect = Effect.gen(function* () {
        attempts++;
        if (attempts < 2) {
          return yield* Effect.fail(createCallError("TIMEOUT", "Timed out"));
        }
        return "success";
      });

      const config: Partial<RetryConfig> = {
        maxRetries: 3,
        baseDelay: 10,
        jitter: false,
        retryableCodes: ["TIMEOUT"],
      };

      const result = await Effect.runPromise(withRetry(effect, config));

      expect(result).toBe("success");
      expect(attempts).toBe(2);
    });
  });

  describe("Concurrent Retries", () => {
    it("should handle multiple concurrent retry operations", async () => {
      const createRetryingEffect = (id: number, failCount: number) => {
        let attempts = 0;
        return Effect.gen(function* () {
          attempts++;
          if (attempts <= failCount) {
            return yield* Effect.fail(
              createCallError("INTERNAL_ERROR", `${id} failed`),
            );
          }
          return { id, attempts };
        });
      };

      const config: Partial<RetryConfig> = {
        maxRetries: 5,
        baseDelay: 10,
        jitter: false,
      };

      const effects = [
        withRetry(createRetryingEffect(1, 2), config),
        withRetry(createRetryingEffect(2, 1), config),
        withRetry(createRetryingEffect(3, 3), config),
      ];

      const results = await Effect.runPromise(
        Effect.all(effects, { concurrency: 3 }),
      );

      expect(results[0]).toEqual({ id: 1, attempts: 3 });
      expect(results[1]).toEqual({ id: 2, attempts: 2 });
      expect(results[2]).toEqual({ id: 3, attempts: 4 });
    });
  });

  describe("Backoff Strategies", () => {
    it("should work with exponential backoff", async () => {
      let attempts = 0;
      const effect = Effect.gen(function* () {
        attempts++;
        if (attempts < 2) {
          return yield* Effect.fail(createCallError("INTERNAL_ERROR", "Retry"));
        }
        return "done";
      });

      const config: Partial<RetryConfig> = {
        maxRetries: 3,
        baseDelay: 10,
        backoff: "exponential",
        jitter: false,
      };

      const result = await Effect.runPromise(withRetry(effect, config));
      expect(result).toBe("done");
    });

    it("should work with linear backoff", async () => {
      let attempts = 0;
      const effect = Effect.gen(function* () {
        attempts++;
        if (attempts < 2) {
          return yield* Effect.fail(createCallError("INTERNAL_ERROR", "Retry"));
        }
        return "done";
      });

      const config: Partial<RetryConfig> = {
        maxRetries: 3,
        baseDelay: 10,
        backoff: "linear",
        jitter: false,
      };

      const result = await Effect.runPromise(withRetry(effect, config));
      expect(result).toBe("done");
    });
  });
});
