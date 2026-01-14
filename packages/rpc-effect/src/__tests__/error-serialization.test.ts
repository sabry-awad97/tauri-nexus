// =============================================================================
// TC008: Error Serialization and Parsing Tests
// =============================================================================
// Test error serialization and parsing utilities for Promise-based API consumers.

import { describe, it, expect } from "vitest";
import {
  toRpcError,
  fromRpcError,
  isRpcError,
  hasErrorCode,
  createRpcError,
  isRateLimitError,
  getRateLimitRetryAfter,
  isRpcErrorShape,
  parseJsonError,
  createCallErrorFromShape,
  parseToEffectError,
  fromTransportError,
  parseEffectError,
  parseError,
  createCallError,
  createTimeoutError,
  createCancelledError,
  createValidationError,
  createNetworkError,
  type RpcError,
} from "../index";

describe("TC008: Error Serialization and Parsing", () => {
  describe("toRpcError - Effect to Serializable", () => {
    it("should convert RpcCallError to serializable form", () => {
      const effectError = createCallError("NOT_FOUND", "User not found", {
        userId: 123,
      });
      const rpcError = toRpcError(effectError);

      expect(rpcError.code).toBe("NOT_FOUND");
      expect(rpcError.message).toBe("User not found");
      expect(rpcError.details).toEqual({ userId: 123 });
    });

    it("should convert RpcTimeoutError to serializable form", () => {
      const effectError = createTimeoutError("users.get", 5000);
      const rpcError = toRpcError(effectError);

      expect(rpcError.code).toBe("TIMEOUT");
      expect(rpcError.message).toContain("timed out");
      expect(rpcError.details).toEqual({ timeoutMs: 5000, path: "users.get" });
    });

    it("should convert RpcCancelledError to serializable form", () => {
      const effectError = createCancelledError("users.get", "User cancelled");
      const rpcError = toRpcError(effectError);

      expect(rpcError.code).toBe("CANCELLED");
      expect(rpcError.message).toBe("User cancelled");
      expect(rpcError.details).toEqual({ path: "users.get" });
    });

    it("should convert RpcValidationError to serializable form", () => {
      const issues = [
        { path: ["name"], message: "Required", code: "required" },
      ];
      const effectError = createValidationError("users.create", issues);
      const rpcError = toRpcError(effectError);

      expect(rpcError.code).toBe("VALIDATION_ERROR");
      expect(rpcError.message).toBe("Required");
      expect(rpcError.details).toEqual({ issues });
    });

    it("should convert RpcNetworkError to serializable form", () => {
      const originalError = new Error("Connection refused");
      const effectError = createNetworkError("users.get", originalError);
      const rpcError = toRpcError(effectError);

      expect(rpcError.code).toBe("INTERNAL_ERROR");
      expect(rpcError.message).toContain("Network error");
      expect(rpcError.details).toHaveProperty("originalError");
    });
  });

  describe("fromRpcError - Serializable to Effect", () => {
    it("should convert TIMEOUT code to RpcTimeoutError", () => {
      const rpcError: RpcError = {
        code: "TIMEOUT",
        message: "Request timed out",
        details: { timeoutMs: 5000 },
      };
      const effectError = fromRpcError(rpcError, "users.get");

      expect(effectError._tag).toBe("RpcTimeoutError");
      if (effectError._tag === "RpcTimeoutError") {
        expect(effectError.path).toBe("users.get");
        expect(effectError.timeoutMs).toBe(5000);
      }
    });

    it("should convert CANCELLED code to RpcCancelledError", () => {
      const rpcError: RpcError = {
        code: "CANCELLED",
        message: "Request cancelled",
      };
      const effectError = fromRpcError(rpcError, "users.get");

      expect(effectError._tag).toBe("RpcCancelledError");
      if (effectError._tag === "RpcCancelledError") {
        expect(effectError.path).toBe("users.get");
      }
    });

    it("should convert VALIDATION_ERROR code to RpcValidationError", () => {
      const issues = [
        { path: ["name"], message: "Required", code: "required" },
      ];
      const rpcError: RpcError = {
        code: "VALIDATION_ERROR",
        message: "Validation failed",
        details: { issues },
      };
      const effectError = fromRpcError(rpcError, "users.create");

      expect(effectError._tag).toBe("RpcValidationError");
      if (effectError._tag === "RpcValidationError") {
        expect(effectError.issues).toEqual(issues);
      }
    });

    it("should convert other codes to RpcCallError", () => {
      const rpcError: RpcError = {
        code: "NOT_FOUND",
        message: "User not found",
        details: { userId: 123 },
      };
      const effectError = fromRpcError(rpcError, "users.get");

      expect(effectError._tag).toBe("RpcCallError");
      if (effectError._tag === "RpcCallError") {
        expect(effectError.code).toBe("NOT_FOUND");
        expect(effectError.message).toBe("User not found");
      }
    });
  });

  describe("isRpcError", () => {
    it("should identify valid RpcError objects", () => {
      expect(isRpcError({ code: "TEST", message: "Test" })).toBe(true);
      expect(isRpcError({ code: "ERR", message: "Error", details: {} })).toBe(
        true,
      );
    });

    it("should reject invalid objects", () => {
      expect(isRpcError(null)).toBe(false);
      expect(isRpcError(undefined)).toBe(false);
      expect(isRpcError({})).toBe(false);
      expect(isRpcError({ code: "TEST" })).toBe(false);
      expect(isRpcError({ message: "Test" })).toBe(false);
      expect(isRpcError({ code: 123, message: "Test" })).toBe(false);
    });
  });

  describe("hasErrorCode", () => {
    it("should check error code correctly", () => {
      const error = { code: "NOT_FOUND", message: "Not found" };
      expect(hasErrorCode(error, "NOT_FOUND")).toBe(true);
      expect(hasErrorCode(error, "OTHER")).toBe(false);
    });

    it("should return false for non-RpcError", () => {
      expect(hasErrorCode(null, "TEST")).toBe(false);
      expect(hasErrorCode({}, "TEST")).toBe(false);
    });
  });

  describe("createRpcError", () => {
    it("should create RpcError with all fields", () => {
      const error = createRpcError("TEST", "Test message", { key: "value" });

      expect(error.code).toBe("TEST");
      expect(error.message).toBe("Test message");
      expect(error.details).toEqual({ key: "value" });
    });

    it("should create RpcError without details", () => {
      const error = createRpcError("TEST", "Test message");

      expect(error.code).toBe("TEST");
      expect(error.message).toBe("Test message");
      expect(error.details).toBeUndefined();
    });
  });

  describe("Rate Limit Utilities", () => {
    it("should identify rate limit errors", () => {
      const rateLimitError = {
        code: "RATE_LIMITED",
        message: "Too many requests",
      };
      const otherError = { code: "NOT_FOUND", message: "Not found" };

      expect(isRateLimitError(rateLimitError)).toBe(true);
      expect(isRateLimitError(otherError)).toBe(false);
    });

    it("should extract retry-after from rate limit error", () => {
      const error: RpcError = {
        code: "RATE_LIMITED",
        message: "Too many requests",
        details: { retry_after_ms: 5000 },
      };

      expect(getRateLimitRetryAfter(error)).toBe(5000);
    });

    it("should return undefined for non-rate-limit errors", () => {
      const error: RpcError = { code: "NOT_FOUND", message: "Not found" };
      expect(getRateLimitRetryAfter(error)).toBeUndefined();
    });

    it("should return undefined when retry_after_ms is missing", () => {
      const error: RpcError = {
        code: "RATE_LIMITED",
        message: "Too many requests",
      };
      expect(getRateLimitRetryAfter(error)).toBeUndefined();
    });
  });

  describe("isRpcErrorShape", () => {
    it("should identify valid error shapes", () => {
      expect(isRpcErrorShape({ code: "TEST", message: "Test" })).toBe(true);
      expect(
        isRpcErrorShape({ code: "ERR", message: "Error", details: {} }),
      ).toBe(true);
    });

    it("should reject invalid shapes", () => {
      expect(isRpcErrorShape(null)).toBe(false);
      expect(isRpcErrorShape(undefined)).toBe(false);
      expect(isRpcErrorShape({})).toBe(false);
      expect(isRpcErrorShape({ code: "TEST" })).toBe(false);
    });
  });

  describe("parseJsonError", () => {
    it("should parse valid JSON error string", () => {
      const json = JSON.stringify({ code: "TEST", message: "Test error" });
      const result = parseJsonError(json);

      expect(result).not.toBeNull();
      expect(result?.code).toBe("TEST");
      expect(result?.message).toBe("Test error");
    });

    it("should return null for invalid JSON", () => {
      expect(parseJsonError("not json")).toBeNull();
      expect(parseJsonError("{}")).toBeNull();
      expect(parseJsonError('{"code": "TEST"}')).toBeNull();
    });
  });

  describe("createCallErrorFromShape", () => {
    it("should create RpcCallError from shape", () => {
      const shape = {
        code: "NOT_FOUND",
        message: "Not found",
        details: { id: 1 },
      };
      const error = createCallErrorFromShape(shape);

      expect(error._tag).toBe("RpcCallError");
      expect(error.code).toBe("NOT_FOUND");
      expect(error.message).toBe("Not found");
    });
  });

  describe("parseToEffectError", () => {
    it("should return Effect error as-is", () => {
      const effectError = createCallError("TEST", "Test");
      const result = parseToEffectError(effectError, "path");

      expect(result).toBe(effectError);
    });

    it("should convert AbortError to timeout error when timeoutMs provided", () => {
      const abortError = new Error("Aborted");
      abortError.name = "AbortError";
      const result = parseToEffectError(abortError, "users.get", 5000);

      expect(result._tag).toBe("RpcTimeoutError");
    });

    it("should convert AbortError to cancelled error when no timeoutMs", () => {
      const abortError = new Error("Aborted");
      abortError.name = "AbortError";
      const result = parseToEffectError(abortError, "users.get");

      expect(result._tag).toBe("RpcCancelledError");
    });

    it("should parse error shape objects", () => {
      const shape = { code: "NOT_FOUND", message: "Not found" };
      const result = parseToEffectError(shape, "users.get");

      expect(result._tag).toBe("RpcCallError");
      if (result._tag === "RpcCallError") {
        expect(result.code).toBe("NOT_FOUND");
      }
    });

    it("should parse JSON string errors", () => {
      const json = JSON.stringify({ code: "TEST", message: "Test" });
      const result = parseToEffectError(json, "path", undefined, {
        parseJson: true,
      });

      expect(result._tag).toBe("RpcCallError");
    });

    it("should handle plain string errors", () => {
      const result = parseToEffectError("Something went wrong", "path");

      expect(result._tag).toBe("RpcCallError");
      if (result._tag === "RpcCallError") {
        expect(result.code).toBe("UNKNOWN");
        expect(result.message).toBe("Something went wrong");
      }
    });

    it("should handle Error instances", () => {
      const error = new Error("Test error");
      const result = parseToEffectError(error, "path");

      expect(result._tag).toBe("RpcCallError");
      if (result._tag === "RpcCallError") {
        expect(result.message).toBe("Test error");
      }
    });
  });

  describe("fromTransportError", () => {
    it("should parse transport errors", () => {
      const error = { code: "NETWORK_ERROR", message: "Connection failed" };
      const result = fromTransportError(error, "users.get");

      expect(result._tag).toBe("RpcCallError");
    });

    it("should handle timeout with timeoutMs", () => {
      const abortError = new Error("Aborted");
      abortError.name = "AbortError";
      const result = fromTransportError(abortError, "users.get", 5000);

      expect(result._tag).toBe("RpcTimeoutError");
    });
  });

  describe("parseEffectError", () => {
    it("should parse with all options enabled", () => {
      const error = { code: "TEST", message: "Test" };
      const result = parseEffectError(error, "path");

      expect(result._tag).toBe("RpcCallError");
    });

    it("should unwrap nested errors", () => {
      const nested = { error: { code: "INNER", message: "Inner error" } };
      const result = parseEffectError(nested, "path");

      expect(result._tag).toBe("RpcCallError");
      if (result._tag === "RpcCallError") {
        expect(result.code).toBe("INNER");
      }
    });
  });

  describe("parseError - Full Pipeline", () => {
    it("should convert any error to serializable RpcError", () => {
      const error = new Error("Something failed");
      const result = parseError(error, "users.get");

      expect(isRpcError(result)).toBe(true);
      expect(result.code).toBe("UNKNOWN");
    });

    it("should preserve error code from shape", () => {
      const error = { code: "NOT_FOUND", message: "User not found" };
      const result = parseError(error, "users.get");

      expect(result.code).toBe("NOT_FOUND");
      expect(result.message).toBe("User not found");
    });

    it("should handle Effect errors", () => {
      const effectError = createCallError("FORBIDDEN", "Access denied");
      const result = parseError(effectError, "users.get");

      expect(result.code).toBe("FORBIDDEN");
      expect(result.message).toBe("Access denied");
    });
  });

  describe("Round-Trip Conversion", () => {
    it("should preserve data through toRpcError -> fromRpcError", () => {
      const original = createCallError("NOT_FOUND", "User not found", {
        id: 123,
      });
      const serialized = toRpcError(original);
      const restored = fromRpcError(serialized, "users.get");

      expect(restored._tag).toBe("RpcCallError");
      if (restored._tag === "RpcCallError") {
        expect(restored.code).toBe(original.code);
        expect(restored.message).toBe(original.message);
      }
    });

    it("should handle timeout round-trip", () => {
      const original = createTimeoutError("users.get", 5000);
      const serialized = toRpcError(original);
      const restored = fromRpcError(serialized, "users.get");

      expect(restored._tag).toBe("RpcTimeoutError");
      if (restored._tag === "RpcTimeoutError") {
        expect(restored.timeoutMs).toBe(5000);
      }
    });

    it("should handle validation error round-trip", () => {
      const issues = [
        { path: ["name"], message: "Required", code: "required" },
      ];
      const original = createValidationError("users.create", issues);
      const serialized = toRpcError(original);
      const restored = fromRpcError(serialized, "users.create");

      expect(restored._tag).toBe("RpcValidationError");
      if (restored._tag === "RpcValidationError") {
        expect(restored.issues).toEqual(issues);
      }
    });
  });
});
