// =============================================================================
// Error Tests
// =============================================================================

import { describe, it, expect } from "vitest";
import {
  makeCallError,
  makeTimeoutError,
  makeCancelledError,
  makeValidationError,
  makeNetworkError,
  isEffectRpcError,
  isRpcCallError,
  isRpcTimeoutError,
  isRpcCancelledError,
  isRpcValidationError,
  isRpcNetworkError,
  hasCode,
  matchError,
  failWithCallError,
  failWithTimeout,
  failWithValidation,
  failWithNetwork,
  failWithCancelled,
} from "../errors";
import {
  RpcCallError,
  RpcTimeoutError,
  RpcCancelledError,
  RpcValidationError,
  RpcNetworkError,
} from "../types";
import { Effect } from "effect";

describe("Error Constructors", () => {
  it("should create RpcCallError", () => {
    const error = makeCallError("NOT_FOUND", "User not found", { id: 1 });
    expect(error).toBeInstanceOf(RpcCallError);
    expect(error._tag).toBe("RpcCallError");
    expect(error.code).toBe("NOT_FOUND");
    expect(error.message).toBe("User not found");
    expect(error.details).toEqual({ id: 1 });
  });

  it("should create RpcTimeoutError", () => {
    const error = makeTimeoutError("user.get", 5000);
    expect(error).toBeInstanceOf(RpcTimeoutError);
    expect(error._tag).toBe("RpcTimeoutError");
    expect(error.path).toBe("user.get");
    expect(error.timeoutMs).toBe(5000);
  });

  it("should create RpcCancelledError", () => {
    const error = makeCancelledError("user.get", "User cancelled");
    expect(error).toBeInstanceOf(RpcCancelledError);
    expect(error._tag).toBe("RpcCancelledError");
    expect(error.path).toBe("user.get");
    expect(error.reason).toBe("User cancelled");
  });

  it("should create RpcValidationError", () => {
    const issues = [{ path: ["name"], message: "Required", code: "required" }];
    const error = makeValidationError("user.create", issues);
    expect(error).toBeInstanceOf(RpcValidationError);
    expect(error._tag).toBe("RpcValidationError");
    expect(error.path).toBe("user.create");
    expect(error.issues).toEqual(issues);
  });

  it("should create RpcNetworkError", () => {
    const originalError = new Error("Connection refused");
    const error = makeNetworkError("user.get", originalError);
    expect(error).toBeInstanceOf(RpcNetworkError);
    expect(error._tag).toBe("RpcNetworkError");
    expect(error.path).toBe("user.get");
    expect(error.originalError).toBe(originalError);
  });
});

describe("isEffectRpcError", () => {
  it("should identify Effect RPC errors", () => {
    expect(isEffectRpcError(makeCallError("TEST", "Test"))).toBe(true);
    expect(isEffectRpcError(makeTimeoutError("path", 1000))).toBe(true);
    expect(isEffectRpcError(makeCancelledError("path"))).toBe(true);
    expect(isEffectRpcError(makeValidationError("path", []))).toBe(true);
    expect(isEffectRpcError(makeNetworkError("path", new Error()))).toBe(true);
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
    const error = makeCallError("TEST", "Test");
    expect(isRpcCallError(error)).toBe(true);
    expect(isRpcTimeoutError(error)).toBe(false);
  });

  it("should identify RpcTimeoutError", () => {
    const error = makeTimeoutError("path", 1000);
    expect(isRpcTimeoutError(error)).toBe(true);
    expect(isRpcCallError(error)).toBe(false);
  });

  it("should identify RpcCancelledError", () => {
    const error = makeCancelledError("path");
    expect(isRpcCancelledError(error)).toBe(true);
    expect(isRpcCallError(error)).toBe(false);
  });

  it("should identify RpcValidationError", () => {
    const error = makeValidationError("path", []);
    expect(isRpcValidationError(error)).toBe(true);
    expect(isRpcCallError(error)).toBe(false);
  });

  it("should identify RpcNetworkError", () => {
    const error = makeNetworkError("path", new Error());
    expect(isRpcNetworkError(error)).toBe(true);
    expect(isRpcCallError(error)).toBe(false);
  });
});

describe("hasCode", () => {
  it("should check code for RpcCallError", () => {
    const error = makeCallError("NOT_FOUND", "Not found");
    expect(hasCode(error, "NOT_FOUND")).toBe(true);
    expect(hasCode(error, "OTHER")).toBe(false);
  });

  it("should return TIMEOUT for RpcTimeoutError", () => {
    const error = makeTimeoutError("path", 1000);
    expect(hasCode(error, "TIMEOUT")).toBe(true);
  });

  it("should return CANCELLED for RpcCancelledError", () => {
    const error = makeCancelledError("path");
    expect(hasCode(error, "CANCELLED")).toBe(true);
  });

  it("should return VALIDATION_ERROR for RpcValidationError", () => {
    const error = makeValidationError("path", []);
    expect(hasCode(error, "VALIDATION_ERROR")).toBe(true);
  });

  it("should return INTERNAL_ERROR for RpcNetworkError", () => {
    const error = makeNetworkError("path", new Error());
    expect(hasCode(error, "INTERNAL_ERROR")).toBe(true);
  });
});

describe("matchError", () => {
  it("should match RpcCallError", () => {
    const error = makeCallError("TEST", "Test");
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
    const error = makeTimeoutError("path", 1000);
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
      makeCallError("TEST", "Test"),
      makeTimeoutError("path", 1000),
      makeCancelledError("path"),
      makeValidationError("path", []),
      makeNetworkError("path", new Error()),
    ];

    const results = errors.map((error) =>
      matchError(error, {
        onCallError: () => "call",
        onTimeoutError: () => "timeout",
        onCancelledError: () => "cancelled",
        onValidationError: () => "validation",
        onNetworkError: () => "network",
      })
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
      makeCallError("TEST", "Test"),
      makeTimeoutError("path", 1000),
      makeCancelledError("path"),
      makeValidationError("path", []),
      makeNetworkError("path", new Error()),
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
