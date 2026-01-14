// =============================================================================
// Error Tests
// =============================================================================

import { describe, it, expect } from "vitest";
import * as fc from "fast-check";
import {
  makeCallError,
  makeTimeoutError,
  makeCancelledError,
  makeValidationError,
  makeNetworkError,
  toEffectError,
  isEffectRpcError,
  isRpcCallError,
  isRpcTimeoutError,
  isRpcCancelledError,
  isRpcValidationError,
  isRpcNetworkError,
  hasCode,
  matchError,
} from "../errors";
import {
  RpcCallError,
  RpcTimeoutError,
  RpcCancelledError,
  RpcValidationError,
  RpcNetworkError,
} from "../types";

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

describe("toEffectError", () => {
  it("should handle RPC error shape objects", () => {
    const rpcError = { code: "NOT_FOUND", message: "User not found" };
    const error = toEffectError(rpcError, "user.get");
    expect(error._tag).toBe("RpcCallError");
    expect((error as RpcCallError).code).toBe("NOT_FOUND");
  });

  it("should parse JSON string errors", () => {
    const jsonError = JSON.stringify({
      code: "NOT_FOUND",
      message: "User not found",
    });
    const error = toEffectError(jsonError, "user.get");
    expect(error._tag).toBe("RpcCallError");
    expect((error as RpcCallError).code).toBe("NOT_FOUND");
    expect((error as RpcCallError).message).toBe("User not found");
  });

  it("should handle plain string errors", () => {
    const error = toEffectError("Connection failed", "user.get");
    expect(error._tag).toBe("RpcCallError");
    expect((error as RpcCallError).code).toBe("UNKNOWN");
    expect((error as RpcCallError).message).toBe("Connection failed");
  });

  it("should handle Error objects", () => {
    const error = toEffectError(new Error("Network error"), "user.get");
    expect(error._tag).toBe("RpcCallError");
    expect((error as RpcCallError).message).toBe("Network error");
  });

  it("should parse Error with JSON message", () => {
    const jsonMessage = JSON.stringify({
      code: "FORBIDDEN",
      message: "Access denied",
    });
    const error = toEffectError(new Error(jsonMessage), "user.get");
    expect(error._tag).toBe("RpcCallError");
    expect((error as RpcCallError).code).toBe("FORBIDDEN");
    expect((error as RpcCallError).message).toBe("Access denied");
  });

  it("should handle AbortError as timeout when timeoutMs provided", () => {
    const abortError = new DOMException("Aborted", "AbortError");
    const error = toEffectError(abortError, "user.get", 5000);
    expect(error._tag).toBe("RpcTimeoutError");
    expect((error as RpcTimeoutError).timeoutMs).toBe(5000);
  });

  it("should handle AbortError as cancelled when no timeoutMs", () => {
    const abortError = new DOMException("Aborted", "AbortError");
    const error = toEffectError(abortError, "user.get");
    expect(error._tag).toBe("RpcCancelledError");
  });

  it("should pass through existing Effect errors", () => {
    const original = makeCallError("CUSTOM", "Custom error");
    const error = toEffectError(original, "user.get");
    expect(error).toBe(original);
  });

  it("should handle unknown values", () => {
    const error = toEffectError(12345, "user.get");
    expect(error._tag).toBe("RpcCallError");
    expect((error as RpcCallError).code).toBe("UNKNOWN");
    expect((error as RpcCallError).message).toBe("12345");
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
});

describe("Property-Based Tests", () => {
  it("property: toEffectError always returns RpcEffectError", () => {
    fc.assert(
      fc.property(fc.anything(), fc.string(), (error, path) => {
        const result = toEffectError(error, path);
        expect(result._tag).toMatch(
          /^Rpc(Call|Timeout|Cancelled|Validation|Network)Error$/
        );
      }),
      { numRuns: 100 }
    );
  });

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
