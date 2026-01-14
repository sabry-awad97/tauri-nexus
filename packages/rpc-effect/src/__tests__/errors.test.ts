// =============================================================================
// Error Tests
// =============================================================================

import { describe, it, expect } from "vitest";
import {
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
  hasCode,
  hasAnyCode,
  getErrorCode,
  isRetryableError,
  matchError,
  failWithCallError,
  failWithTimeout,
  failWithValidation,
  failWithNetwork,
  failWithCancelled,
  RpcCallError,
  RpcTimeoutError,
  RpcCancelledError,
  RpcValidationError,
  RpcNetworkError,
} from "../index";
import { Effect } from "effect";

describe("Error Constructors", () => {
  it("should create RpcCallError", () => {
    const error = createCallError("NOT_FOUND", "User not found", { id: 1 });
    expect(error).toBeInstanceOf(RpcCallError);
    expect(error._tag).toBe("RpcCallError");
    expect(error.code).toBe("NOT_FOUND");
    expect(error.message).toBe("User not found");
    expect(error.details).toEqual({ id: 1 });
  });

  it("should create RpcTimeoutError", () => {
    const error = createTimeoutError("user.get", 5000);
    expect(error).toBeInstanceOf(RpcTimeoutError);
    expect(error._tag).toBe("RpcTimeoutError");
    expect(error.path).toBe("user.get");
    expect(error.timeoutMs).toBe(5000);
  });

  it("should create RpcCancelledError", () => {
    const error = createCancelledError("user.get", "User cancelled");
    expect(error).toBeInstanceOf(RpcCancelledError);
    expect(error._tag).toBe("RpcCancelledError");
    expect(error.path).toBe("user.get");
    expect(error.reason).toBe("User cancelled");
  });

  it("should create RpcValidationError", () => {
    const issues = [{ path: ["name"], message: "Required", code: "required" }];
    const error = createValidationError("user.create", issues);
    expect(error).toBeInstanceOf(RpcValidationError);
    expect(error._tag).toBe("RpcValidationError");
    expect(error.path).toBe("user.create");
    expect(error.issues).toEqual(issues);
  });

  it("should create RpcNetworkError", () => {
    const originalError = new Error("Connection refused");
    const error = createNetworkError("user.get", originalError);
    expect(error).toBeInstanceOf(RpcNetworkError);
    expect(error._tag).toBe("RpcNetworkError");
    expect(error.path).toBe("user.get");
    expect(error.originalError).toBe(originalError);
  });
});

describe("isEffectRpcError", () => {
  it("should identify Effect RPC errors", () => {
    expect(isEffectRpcError(createCallError("TEST", "Test"))).toBe(true);
    expect(isEffectRpcError(createTimeoutError("path", 1000))).toBe(true);
    expect(isEffectRpcError(createCancelledError("path"))).toBe(true);
    expect(isEffectRpcError(createValidationError("path", []))).toBe(true);
    expect(isEffectRpcError(createNetworkError("path", new Error()))).toBe(
      true,
    );
  });

  it("should reject non-Effect errors", () => {
    expect(isEffectRpcError(new Error("test"))).toBe(false);
    expect(isEffectRpcError({ code: "TEST", message: "test" })).toBe(false);
    expect(isEffectRpcError(null)).toBe(false);
    expect(isEffectRpcError(undefined)).toBe(false);
  });
});

describe("Error Type Guards", () => {
  it("should identify RpcCallError", () => {
    const error = createCallError("TEST", "Test");
    expect(isRpcCallError(error)).toBe(true);
    expect(isRpcTimeoutError(error)).toBe(false);
  });

  it("should identify RpcTimeoutError", () => {
    const error = createTimeoutError("path", 1000);
    expect(isRpcTimeoutError(error)).toBe(true);
    expect(isRpcCallError(error)).toBe(false);
  });

  it("should identify RpcCancelledError", () => {
    const error = createCancelledError("path");
    expect(isRpcCancelledError(error)).toBe(true);
    expect(isRpcCallError(error)).toBe(false);
  });

  it("should identify RpcValidationError", () => {
    const error = createValidationError("path", []);
    expect(isRpcValidationError(error)).toBe(true);
    expect(isRpcCallError(error)).toBe(false);
  });

  it("should identify RpcNetworkError", () => {
    const error = createNetworkError("path", new Error());
    expect(isRpcNetworkError(error)).toBe(true);
    expect(isRpcCallError(error)).toBe(false);
  });
});

describe("hasCode", () => {
  it("should check code for RpcCallError", () => {
    const error = createCallError("NOT_FOUND", "Not found");
    expect(hasCode(error, "NOT_FOUND")).toBe(true);
    expect(hasCode(error, "OTHER")).toBe(false);
  });

  it("should return TIMEOUT for RpcTimeoutError", () => {
    const error = createTimeoutError("path", 1000);
    expect(hasCode(error, "TIMEOUT")).toBe(true);
  });

  it("should return CANCELLED for RpcCancelledError", () => {
    const error = createCancelledError("path");
    expect(hasCode(error, "CANCELLED")).toBe(true);
  });

  it("should return VALIDATION_ERROR for RpcValidationError", () => {
    const error = createValidationError("path", []);
    expect(hasCode(error, "VALIDATION_ERROR")).toBe(true);
  });

  it("should return NETWORK_ERROR for RpcNetworkError", () => {
    const error = createNetworkError("path", new Error());
    expect(hasCode(error, "NETWORK_ERROR")).toBe(true);
  });
});

describe("getErrorCode", () => {
  it("should return code for RpcCallError", () => {
    expect(getErrorCode(createCallError("NOT_FOUND", "Not found"))).toBe(
      "NOT_FOUND",
    );
  });

  it("should return TIMEOUT for RpcTimeoutError", () => {
    expect(getErrorCode(createTimeoutError("path", 1000))).toBe("TIMEOUT");
  });

  it("should return CANCELLED for RpcCancelledError", () => {
    expect(getErrorCode(createCancelledError("path"))).toBe("CANCELLED");
  });

  it("should return VALIDATION_ERROR for RpcValidationError", () => {
    expect(getErrorCode(createValidationError("path", []))).toBe(
      "VALIDATION_ERROR",
    );
  });

  it("should return NETWORK_ERROR for RpcNetworkError", () => {
    expect(getErrorCode(createNetworkError("path", new Error()))).toBe(
      "NETWORK_ERROR",
    );
  });
});

describe("hasAnyCode", () => {
  it("should match any of the given codes", () => {
    const error = createCallError("NOT_FOUND", "Not found");
    expect(hasAnyCode(error, ["NOT_FOUND", "BAD_REQUEST"])).toBe(true);
    expect(hasAnyCode(error, ["BAD_REQUEST", "FORBIDDEN"])).toBe(false);
  });

  it("should work with virtual codes", () => {
    const timeout = createTimeoutError("path", 1000);
    expect(hasAnyCode(timeout, ["TIMEOUT", "CANCELLED"])).toBe(true);
  });
});

describe("isRetryableError", () => {
  it("should return false for non-retryable errors", () => {
    expect(isRetryableError(createValidationError("path", []))).toBe(false);
    expect(isRetryableError(createCancelledError("path"))).toBe(false);
    expect(
      isRetryableError(createCallError("UNAUTHORIZED", "Unauthorized")),
    ).toBe(false);
    expect(isRetryableError(createCallError("FORBIDDEN", "Forbidden"))).toBe(
      false,
    );
    expect(
      isRetryableError(createCallError("BAD_REQUEST", "Bad request")),
    ).toBe(false);
    expect(isRetryableError(createCallError("NOT_FOUND", "Not found"))).toBe(
      false,
    );
  });

  it("should return true for retryable errors", () => {
    expect(isRetryableError(createTimeoutError("path", 1000))).toBe(true);
    expect(isRetryableError(createNetworkError("path", new Error()))).toBe(
      true,
    );
    expect(
      isRetryableError(createCallError("INTERNAL_ERROR", "Internal error")),
    ).toBe(true);
    expect(
      isRetryableError(createCallError("SERVICE_UNAVAILABLE", "Unavailable")),
    ).toBe(true);
  });
});

describe("matchError", () => {
  it("should match RpcCallError", () => {
    const error = createCallError("TEST", "Test");
    const result = matchError(error, {
      onCallError: (e) => `call:${e.code}`,
      onTimeoutError: () => "timeout",
      onCancelledError: () => "cancelled",
      onValidationError: () => "validation",
      onNetworkError: () => "network",
    });
    expect(result).toBe("call:TEST");
  });

  it("should match RpcTimeoutError", () => {
    const error = createTimeoutError("path", 1000);
    const result = matchError(error, {
      onCallError: () => "call",
      onTimeoutError: (e) => `timeout:${e.timeoutMs}`,
      onCancelledError: () => "cancelled",
      onValidationError: () => "validation",
      onNetworkError: () => "network",
    });
    expect(result).toBe("timeout:1000");
  });

  it("should match all error types exhaustively", () => {
    const errors = [
      createCallError("TEST", "Test"),
      createTimeoutError("path", 1000),
      createCancelledError("path"),
      createValidationError("path", []),
      createNetworkError("path", new Error()),
    ];

    const results = errors.map((error) =>
      matchError(error, {
        onCallError: () => "call",
        onTimeoutError: () => "timeout",
        onCancelledError: () => "cancelled",
        onValidationError: () => "validation",
        onNetworkError: () => "network",
      }),
    );

    expect(results).toEqual([
      "call",
      "timeout",
      "cancelled",
      "validation",
      "network",
    ]);
  });
});

describe("Effect Combinators", () => {
  it("failWithCallError should create failing Effect", async () => {
    const effect = failWithCallError("TEST", "Test error");
    const result = await Effect.runPromise(Effect.either(effect));
    expect(result._tag).toBe("Left");
    if (result._tag === "Left") {
      expect(result.left._tag).toBe("RpcCallError");
    }
  });

  it("failWithTimeout should create failing Effect", async () => {
    const effect = failWithTimeout("path", 5000);
    const result = await Effect.runPromise(Effect.either(effect));
    expect(result._tag).toBe("Left");
    if (result._tag === "Left") {
      expect(result.left._tag).toBe("RpcTimeoutError");
    }
  });

  it("failWithValidation should create failing Effect", async () => {
    const effect = failWithValidation("path", []);
    const result = await Effect.runPromise(Effect.either(effect));
    expect(result._tag).toBe("Left");
    if (result._tag === "Left") {
      expect(result.left._tag).toBe("RpcValidationError");
    }
  });

  it("failWithNetwork should create failing Effect", async () => {
    const effect = failWithNetwork("path", new Error("Network"));
    const result = await Effect.runPromise(Effect.either(effect));
    expect(result._tag).toBe("Left");
    if (result._tag === "Left") {
      expect(result.left._tag).toBe("RpcNetworkError");
    }
  });

  it("failWithCancelled should create failing Effect", async () => {
    const effect = failWithCancelled("path", "User cancelled");
    const result = await Effect.runPromise(Effect.either(effect));
    expect(result._tag).toBe("Left");
    if (result._tag === "Left") {
      expect(result.left._tag).toBe("RpcCancelledError");
    }
  });
});

describe("Property-Based Tests", () => {
  it("property: type guards are mutually exclusive", () => {
    const errors = [
      createCallError("TEST", "Test"),
      createTimeoutError("path", 1000),
      createCancelledError("path"),
      createValidationError("path", []),
      createNetworkError("path", new Error()),
    ];

    for (const error of errors) {
      const guards = [
        isRpcCallError(error),
        isRpcTimeoutError(error),
        isRpcCancelledError(error),
        isRpcValidationError(error),
        isRpcNetworkError(error),
      ];
      const trueCount = guards.filter(Boolean).length;
      expect(trueCount).toBe(1);
    }
  });
});
