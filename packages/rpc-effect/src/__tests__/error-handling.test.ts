// =============================================================================
// TC001: Type-Safe Error Handling Tests
// =============================================================================
// Test that all RPC calls and subscriptions return Effect types that correctly
// model success and the full range of typed error cases.

import { describe, it, expect } from "vitest";
import { Effect, Exit, Cause } from "effect";
import {
  RpcCallError,
  RpcTimeoutError,
  RpcCancelledError,
  RpcValidationError,
  RpcNetworkError,
  createCallError,
  createTimeoutError,
  createCancelledError,
  createValidationError,
  createNetworkError,
  isEffectRpcError,
  isRpcCallError,
  isRpcTimeoutError,
  isRpcCancelledError,
  isRpcValidationError,
  isRpcNetworkError,
  getErrorCode,
  hasCode,
  hasAnyCode,
  isRetryableError,
  matchError,
  failWithCallError,
  failWithTimeout,
  failWithValidation,
  failWithNetwork,
  failWithCancelled,
} from "../index";

describe("TC001: Type-Safe Error Handling", () => {
  describe("Error Construction", () => {
    it("should create RpcCallError with all properties", () => {
      const error = createCallError("NOT_FOUND", "Resource not found", {
        id: 123,
      });

      expect(error._tag).toBe("RpcCallError");
      expect(error.code).toBe("NOT_FOUND");
      expect(error.message).toBe("Resource not found");
      expect(error.details).toEqual({ id: 123 });
    });

    it("should create RpcTimeoutError with path and timeout", () => {
      const error = createTimeoutError("users.get", 5000);

      expect(error._tag).toBe("RpcTimeoutError");
      expect(error.path).toBe("users.get");
      expect(error.timeoutMs).toBe(5000);
    });

    it("should create RpcCancelledError with optional reason", () => {
      const error = createCancelledError("users.list", "User cancelled");

      expect(error._tag).toBe("RpcCancelledError");
      expect(error.path).toBe("users.list");
      expect(error.reason).toBe("User cancelled");
    });

    it("should create RpcValidationError with issues array", () => {
      const issues = [
        { path: "email", message: "Invalid email format" },
        { path: "age", message: "Must be positive" },
      ];
      const error = createValidationError("users.create", issues);

      expect(error._tag).toBe("RpcValidationError");
      expect(error.path).toBe("users.create");
      expect(error.issues).toEqual(issues);
    });

    it("should create RpcNetworkError with original error", () => {
      const originalError = new Error("Connection refused");
      const error = createNetworkError("users.get", originalError);

      expect(error._tag).toBe("RpcNetworkError");
      expect(error.path).toBe("users.get");
      expect(error.originalError).toBe(originalError);
    });
  });

  describe("Type Guards", () => {
    it("should correctly identify RpcCallError", () => {
      const error = createCallError("ERROR", "test");
      expect(isRpcCallError(error)).toBe(true);
      expect(isRpcTimeoutError(error)).toBe(false);
      expect(isEffectRpcError(error)).toBe(true);
    });

    it("should correctly identify RpcTimeoutError", () => {
      const error = createTimeoutError("path", 1000);
      expect(isRpcTimeoutError(error)).toBe(true);
      expect(isRpcCallError(error)).toBe(false);
      expect(isEffectRpcError(error)).toBe(true);
    });

    it("should correctly identify RpcCancelledError", () => {
      const error = createCancelledError("path");
      expect(isRpcCancelledError(error)).toBe(true);
      expect(isEffectRpcError(error)).toBe(true);
    });

    it("should correctly identify RpcValidationError", () => {
      const error = createValidationError("path", []);
      expect(isRpcValidationError(error)).toBe(true);
      expect(isEffectRpcError(error)).toBe(true);
    });

    it("should correctly identify RpcNetworkError", () => {
      const error = createNetworkError("path", new Error());
      expect(isRpcNetworkError(error)).toBe(true);
      expect(isEffectRpcError(error)).toBe(true);
    });

    it("should return false for non-RPC errors", () => {
      expect(isEffectRpcError(new Error("test"))).toBe(false);
      expect(isEffectRpcError(null)).toBe(false);
      expect(isEffectRpcError(undefined)).toBe(false);
      expect(isEffectRpcError({ _tag: "Unknown" })).toBe(false);
    });
  });

  describe("Error Code Utilities", () => {
    it("should get error code from RpcCallError", () => {
      const error = createCallError("UNAUTHORIZED", "Not authorized");
      expect(getErrorCode(error)).toBe("UNAUTHORIZED");
    });

    it("should get virtual error codes for other error types", () => {
      expect(getErrorCode(createTimeoutError("path", 1000))).toBe("TIMEOUT");
      expect(getErrorCode(createCancelledError("path"))).toBe("CANCELLED");
      expect(getErrorCode(createValidationError("path", []))).toBe(
        "VALIDATION_ERROR",
      );
      expect(getErrorCode(createNetworkError("path", new Error()))).toBe(
        "NETWORK_ERROR",
      );
    });

    it("should check if error has specific code", () => {
      const error = createCallError("NOT_FOUND", "Not found");
      expect(hasCode(error, "NOT_FOUND")).toBe(true);
      expect(hasCode(error, "UNAUTHORIZED")).toBe(false);
    });

    it("should check if error has any of the specified codes", () => {
      const error = createCallError("NOT_FOUND", "Not found");
      expect(hasAnyCode(error, ["NOT_FOUND", "UNAUTHORIZED"])).toBe(true);
      expect(hasAnyCode(error, ["UNAUTHORIZED", "FORBIDDEN"])).toBe(false);
    });
  });

  describe("Retryable Error Detection", () => {
    it("should identify retryable errors", () => {
      expect(isRetryableError(createCallError("INTERNAL_ERROR", ""))).toBe(
        true,
      );
      expect(isRetryableError(createNetworkError("path", new Error()))).toBe(
        true,
      );
      expect(isRetryableError(createTimeoutError("path", 1000))).toBe(true);
    });

    it("should identify non-retryable errors", () => {
      expect(isRetryableError(createCallError("UNAUTHORIZED", ""))).toBe(false);
      expect(isRetryableError(createCallError("FORBIDDEN", ""))).toBe(false);
      expect(isRetryableError(createCallError("BAD_REQUEST", ""))).toBe(false);
      expect(isRetryableError(createCallError("NOT_FOUND", ""))).toBe(false);
      expect(isRetryableError(createCancelledError("path"))).toBe(false);
      expect(isRetryableError(createValidationError("path", []))).toBe(false);
    });
  });

  describe("Pattern Matching", () => {
    it("should match all error types exhaustively", () => {
      const handlers = {
        onCallError: (e: RpcCallError) => `call:${e.code}`,
        onTimeoutError: (e: RpcTimeoutError) => `timeout:${e.timeoutMs}`,
        onCancelledError: (e: RpcCancelledError) => `cancelled:${e.path}`,
        onValidationError: (e: RpcValidationError) =>
          `validation:${e.issues.length}`,
        onNetworkError: (e: RpcNetworkError) => `network:${e.path}`,
      };

      expect(matchError(createCallError("ERR", ""), handlers)).toBe("call:ERR");
      expect(matchError(createTimeoutError("p", 5000), handlers)).toBe(
        "timeout:5000",
      );
      expect(matchError(createCancelledError("p"), handlers)).toBe(
        "cancelled:p",
      );
      expect(
        matchError(
          createValidationError("p", [{ path: "", message: "" }]),
          handlers,
        ),
      ).toBe("validation:1");
      expect(matchError(createNetworkError("p", new Error()), handlers)).toBe(
        "network:p",
      );
    });
  });

  describe("Effect Combinators", () => {
    it("should fail with RpcCallError", async () => {
      const effect = failWithCallError("ERROR", "Test error", { key: "value" });
      const exit = await Effect.runPromiseExit(effect);

      expect(Exit.isFailure(exit)).toBe(true);
      if (Exit.isFailure(exit)) {
        const error = Cause.failureOption(exit.cause);
        expect(error._tag).toBe("Some");
        if (error._tag === "Some") {
          expect(error.value._tag).toBe("RpcCallError");
          expect((error.value as RpcCallError).code).toBe("ERROR");
        }
      }
    });

    it("should fail with RpcTimeoutError", async () => {
      const effect = failWithTimeout("users.get", 3000);
      const exit = await Effect.runPromiseExit(effect);

      expect(Exit.isFailure(exit)).toBe(true);
      if (Exit.isFailure(exit)) {
        const error = Cause.failureOption(exit.cause);
        expect(error._tag).toBe("Some");
        if (error._tag === "Some") {
          expect(error.value._tag).toBe("RpcTimeoutError");
        }
      }
    });

    it("should fail with RpcValidationError", async () => {
      const effect = failWithValidation("users.create", [
        { path: "email", message: "Invalid" },
      ]);
      const exit = await Effect.runPromiseExit(effect);

      expect(Exit.isFailure(exit)).toBe(true);
    });

    it("should fail with RpcNetworkError", async () => {
      const effect = failWithNetwork("users.get", new Error("Network down"));
      const exit = await Effect.runPromiseExit(effect);

      expect(Exit.isFailure(exit)).toBe(true);
    });

    it("should fail with RpcCancelledError", async () => {
      const effect = failWithCancelled("users.list", "User action");
      const exit = await Effect.runPromiseExit(effect);

      expect(Exit.isFailure(exit)).toBe(true);
    });
  });

  describe("Effect.catchTag Integration", () => {
    it("should catch specific error types with catchTag", async () => {
      const effect = failWithCallError("NOT_FOUND", "Not found").pipe(
        Effect.catchTag("RpcCallError", (e) =>
          Effect.succeed(`Caught: ${e.code}`),
        ),
      );

      const result = await Effect.runPromise(effect);
      expect(result).toBe("Caught: NOT_FOUND");
    });

    it("should not catch unmatched error types", async () => {
      const effect = failWithTimeout("path", 1000).pipe(
        Effect.catchTag("RpcCallError", () => Effect.succeed("Caught call")),
      );

      const exit = await Effect.runPromiseExit(effect);
      expect(Exit.isFailure(exit)).toBe(true);
    });

    it("should chain multiple catchTag handlers", async () => {
      const effect = failWithTimeout("path", 1000).pipe(
        Effect.catchTag("RpcCallError", () => Effect.succeed("call")),
        Effect.catchTag("RpcTimeoutError", () => Effect.succeed("timeout")),
      );

      const result = await Effect.runPromise(effect);
      expect(result).toBe("timeout");
    });
  });
});
