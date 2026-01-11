// =============================================================================
// Client Tests
// =============================================================================
// Tests for the RPC client factory, configuration, and error handling.

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import * as fc from "fast-check";
import { invoke } from "@tauri-apps/api/core";
import {
  createClient,
  createClientWithSubscriptions,
  configureRpc,
  getConfig,
  call,
  isRpcError,
  hasErrorCode,
  createError,
  validatePath,
  type RpcClientConfig,
  type RpcError,
  type Middleware,
} from "@tauri-nexus/rpc-core";

// Mock the invoke function
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const mockInvoke = vi.mocked(invoke);

// =============================================================================
// Test Contract
// =============================================================================

interface TestContract {
  health: { type: "query"; input: void; output: { status: string } };
  greet: { type: "query"; input: { name: string }; output: string };
  user: {
    get: {
      type: "query";
      input: { id: number };
      output: { id: number; name: string };
    };
    create: {
      type: "mutation";
      input: { name: string };
      output: { id: number; name: string };
    };
  };
  stream: {
    counter: { type: "subscription"; input: { start: number }; output: number };
  };
}

// =============================================================================
// Setup & Teardown
// =============================================================================

beforeEach(() => {
  vi.clearAllMocks();
  // Reset global config
  configureRpc({
    middleware: [],
    subscriptionPaths: [],
    timeout: undefined,
    onRequest: undefined,
    onResponse: undefined,
    onError: undefined,
  });
});

afterEach(() => {
  vi.restoreAllMocks();
});

// =============================================================================
// Error Handling Tests
// =============================================================================

describe("Error Handling", () => {
  describe("isRpcError()", () => {
    it("should return true for valid RPC errors", () => {
      const error: RpcError = { code: "NOT_FOUND", message: "User not found" };
      expect(isRpcError(error)).toBe(true);
    });

    it("should return false for non-RPC errors", () => {
      expect(isRpcError(null)).toBe(false);
      expect(isRpcError(undefined)).toBe(false);
      expect(isRpcError("string error")).toBe(false);
      expect(isRpcError(new Error("error"))).toBe(false);
      expect(isRpcError({ code: 123, message: "wrong type" })).toBe(false);
      expect(isRpcError({ code: "VALID" })).toBe(false); // missing message
      expect(isRpcError({ message: "valid" })).toBe(false); // missing code
    });

    it("should return true for RPC errors with details", () => {
      const error: RpcError = {
        code: "VALIDATION_ERROR",
        message: "Invalid input",
        details: { field: "email", reason: "invalid format" },
      };
      expect(isRpcError(error)).toBe(true);
    });

    // Property: isRpcError returns true iff object has string code and message
    it("property: isRpcError correctly identifies RPC error structure", () => {
      fc.assert(
        fc.property(
          fc.record({
            code: fc.string({ minLength: 1 }),
            message: fc.string(),
            details: fc.option(fc.anything(), { nil: undefined }),
          }),
          (error) => {
            expect(isRpcError(error)).toBe(true);
          },
        ),
        { numRuns: 100 },
      );
    });
  });

  describe("hasErrorCode()", () => {
    it("should return true when error has matching code", () => {
      const error: RpcError = { code: "NOT_FOUND", message: "Not found" };
      expect(hasErrorCode(error, "NOT_FOUND")).toBe(true);
    });

    it("should return false when error has different code", () => {
      const error: RpcError = { code: "NOT_FOUND", message: "Not found" };
      expect(hasErrorCode(error, "UNAUTHORIZED")).toBe(false);
    });

    it("should return false for non-RPC errors", () => {
      expect(hasErrorCode(null, "NOT_FOUND")).toBe(false);
      expect(hasErrorCode("error", "NOT_FOUND")).toBe(false);
    });
  });

  describe("createError()", () => {
    it("should create a valid RPC error", () => {
      const error = createError("NOT_FOUND", "User not found");
      expect(error.code).toBe("NOT_FOUND");
      expect(error.message).toBe("User not found");
      expect(error.details).toBeUndefined();
    });

    it("should create error with details", () => {
      const details = { userId: 123 };
      const error = createError("NOT_FOUND", "User not found", details);
      expect(error.details).toEqual(details);
    });

    // Property: createError always produces valid RPC errors
    it("property: createError always produces valid RPC errors", () => {
      fc.assert(
        fc.property(
          fc.string({ minLength: 1 }),
          fc.string(),
          fc.option(fc.anything(), { nil: undefined }),
          (code, message, details) => {
            const error = createError(code, message, details);
            expect(isRpcError(error)).toBe(true);
            expect(error.code).toBe(code);
            expect(error.message).toBe(message);
          },
        ),
        { numRuns: 100 },
      );
    });
  });
});

// =============================================================================
// Configuration Tests
// =============================================================================

describe("Configuration", () => {
  describe("configureRpc()", () => {
    it("should set global configuration", () => {
      const config: RpcClientConfig = {
        timeout: 5000,
        subscriptionPaths: ["stream.counter"],
      };

      configureRpc(config);
      const result = getConfig();

      expect(result.timeout).toBe(5000);
      expect(result.subscriptionPaths).toEqual(["stream.counter"]);
    });

    it("should merge with existing configuration", () => {
      configureRpc({ timeout: 5000 });
      configureRpc({ subscriptionPaths: ["stream.counter"] });

      const result = getConfig();
      expect(result.timeout).toBe(5000);
      expect(result.subscriptionPaths).toEqual(["stream.counter"]);
    });
  });

  describe("getConfig()", () => {
    it("should return current configuration", () => {
      configureRpc({ timeout: 3000 });
      expect(getConfig().timeout).toBe(3000);
    });
  });
});

// =============================================================================
// Client Factory Tests
// =============================================================================

describe("Client Factory", () => {
  describe("createClient()", () => {
    it("should create a client with proxy structure", () => {
      const client = createClient<TestContract>();

      expect(client).toBeDefined();
      expect(typeof client.health).toBe("function");
      expect(typeof client.greet).toBe("function");
      expect(client.user).toBeDefined();
      expect(typeof client.user.get).toBe("function");
      expect(typeof client.user.create).toBe("function");
    });

    it("should apply configuration when provided", () => {
      createClient<TestContract>({ timeout: 5000 });
      expect(getConfig().timeout).toBe(5000);
    });
  });

  describe("createClientWithSubscriptions()", () => {
    it("should create client with subscription paths configured", () => {
      const client = createClientWithSubscriptions<TestContract>({
        subscriptionPaths: ["stream.counter"],
      });

      expect(client).toBeDefined();
      expect(getConfig().subscriptionPaths).toContain("stream.counter");
    });
  });
});

// =============================================================================
// Call Function Tests
// =============================================================================

describe("call()", () => {
  it("should invoke backend with correct path and input", async () => {
    mockInvoke.mockResolvedValueOnce({ id: 1, name: "John" });

    const result = await call<{ id: number; name: string }>("user.get", {
      id: 1,
    });

    expect(mockInvoke).toHaveBeenCalledWith("plugin:rpc|rpc_call", {
      path: "user.get",
      input: { id: 1 },
    });
    expect(result).toEqual({ id: 1, name: "John" });
  });

  it("should handle void input", async () => {
    mockInvoke.mockResolvedValueOnce({ status: "ok" });

    const result = await call<{ status: string }>("health");

    expect(mockInvoke).toHaveBeenCalledWith("plugin:rpc|rpc_call", {
      path: "health",
      input: null,
    });
    expect(result).toEqual({ status: "ok" });
  });

  it("should throw RPC error on failure", async () => {
    const errorResponse = JSON.stringify({
      code: "NOT_FOUND",
      message: "User not found",
    });
    mockInvoke.mockRejectedValueOnce(errorResponse);

    await expect(call("user.get", { id: 999 })).rejects.toMatchObject({
      code: "NOT_FOUND",
      message: "User not found",
    });
  });

  it("should handle non-JSON error strings", async () => {
    mockInvoke.mockRejectedValueOnce("Connection failed");

    await expect(call("health")).rejects.toMatchObject({
      code: "UNKNOWN",
      message: "Connection failed",
    });
  });

  // Property 9: Malformed JSON Error Handling
  // Non-JSON strings should produce UNKNOWN errors with original string as message
  it("property: malformed JSON errors produce UNKNOWN with original message", async () => {
    // Generate strings that are definitely not valid JSON
    const nonJsonStringArb = fc.oneof(
      fc.string().filter((s) => {
        try {
          JSON.parse(s);
          return false;
        } catch {
          return true;
        }
      }),
      fc.constantFrom(
        "Connection failed",
        "Network error",
        "Timeout",
        "Server unavailable",
        "{invalid json",
        "not json at all",
        '{"unclosed": "object',
      ),
    );

    await fc.assert(
      fc.asyncProperty(nonJsonStringArb, async (errorString) => {
        mockInvoke.mockReset();
        configureRpc({ middleware: [], subscriptionPaths: [] });
        mockInvoke.mockRejectedValueOnce(errorString);

        try {
          await call("test.path");
          // Should not reach here
          expect(true).toBe(false);
        } catch (error) {
          expect(isRpcError(error)).toBe(true);
          if (isRpcError(error)) {
            expect(error.code).toBe("UNKNOWN");
            expect(error.message).toBe(errorString);
          }
        }
      }),
      { numRuns: 50 },
    );
  });

  it("should handle Error objects", async () => {
    mockInvoke.mockRejectedValueOnce(new Error("Network error"));

    await expect(call("health")).rejects.toMatchObject({
      code: "UNKNOWN",
      message: "Network error",
    });
  });

  // Property: call always returns result or throws RpcError
  it("property: call result is always valid or throws RpcError", async () => {
    // Generate valid paths (alphanumeric, underscore, dot, no leading/trailing dots)
    const validPathArb = fc.stringMatching(
      /^[a-zA-Z][a-zA-Z0-9_]*(\.[a-zA-Z][a-zA-Z0-9_]*)*$/,
    );

    await fc.assert(
      fc.asyncProperty(validPathArb, fc.anything(), async (path, input) => {
        // Reset mock for each iteration
        mockInvoke.mockReset();
        // Mock success case
        mockInvoke.mockResolvedValueOnce({ success: true });

        const result = await call(path, input);
        expect(result).toEqual({ success: true });
      }),
      { numRuns: 50 },
    );
  });
});

// =============================================================================
// Middleware Tests
// =============================================================================

describe("Middleware", () => {
  it("should execute middleware in order", async () => {
    const order: number[] = [];

    const middleware1: Middleware = async (_ctx, next) => {
      order.push(1);
      const result = await next();
      order.push(4);
      return result;
    };

    const middleware2: Middleware = async (_ctx, next) => {
      order.push(2);
      const result = await next();
      order.push(3);
      return result;
    };

    configureRpc({ middleware: [middleware1, middleware2] });
    mockInvoke.mockResolvedValueOnce("result");

    await call("test");

    expect(order).toEqual([1, 2, 3, 4]);
  });

  it("should pass context to middleware", async () => {
    let capturedContext: any = null;

    const middleware: Middleware = async (ctx, next) => {
      capturedContext = ctx;
      return next();
    };

    configureRpc({ middleware: [middleware] });
    mockInvoke.mockResolvedValueOnce("result");

    await call("user.get", { id: 1 }, { meta: { custom: "value" } });

    expect(capturedContext).toMatchObject({
      path: "user.get",
      input: { id: 1 },
      meta: { custom: "value" },
    });
  });

  it("should allow middleware to modify response", async () => {
    // Reset mock and config first
    mockInvoke.mockReset();

    // Use type assertion to allow returning modified object
    const middleware: Middleware = async (_ctx, next) => {
      const result = await next();
      return { ...(result as object), modified: true } as any;
    };

    configureRpc({ middleware: [middleware] });
    mockInvoke.mockResolvedValueOnce({ original: true });

    const result = await call("test");

    expect(result).toEqual({ original: true, modified: true });
  });

  it("should allow middleware to handle errors", async () => {
    // Reset mock and config first
    mockInvoke.mockReset();

    // Use type assertion to allow returning fallback object
    const middleware: Middleware = async (_ctx, next) => {
      try {
        return await next();
      } catch {
        return { fallback: true } as any;
      }
    };

    configureRpc({ middleware: [middleware] });
    mockInvoke.mockRejectedValueOnce(new Error("Failed"));

    const result = await call("test");

    expect(result).toEqual({ fallback: true });
  });
});

// =============================================================================
// Lifecycle Hooks Tests
// =============================================================================

describe("Lifecycle Hooks", () => {
  it("should call onRequest before each request", async () => {
    mockInvoke.mockReset();
    const onRequest = vi.fn();
    configureRpc({ onRequest, middleware: [] });
    mockInvoke.mockResolvedValueOnce("result");

    await call("test.path", { data: "value" });

    expect(onRequest).toHaveBeenCalledWith(
      expect.objectContaining({
        path: "test.path",
        input: { data: "value" },
      }),
    );
  });

  it("should call onResponse after successful response", async () => {
    mockInvoke.mockReset();
    const onResponse = vi.fn();
    configureRpc({ onResponse, middleware: [] });
    mockInvoke.mockResolvedValueOnce({ success: true });

    await call("test");

    expect(onResponse).toHaveBeenCalledWith(
      expect.objectContaining({ path: "test" }),
      { success: true },
    );
  });

  it("should call onError on failure", async () => {
    mockInvoke.mockReset();
    const onError = vi.fn();
    configureRpc({ onError, middleware: [] });
    mockInvoke.mockRejectedValueOnce(
      JSON.stringify({ code: "ERROR", message: "Failed" }),
    );

    await expect(call("test")).rejects.toThrow();

    expect(onError).toHaveBeenCalledWith(
      expect.objectContaining({ path: "test" }),
      expect.objectContaining({ code: "ERROR" }),
    );
  });
});

// =============================================================================
// Client Proxy Tests
// =============================================================================

describe("Client Proxy", () => {
  it("should build correct paths for nested procedures", async () => {
    mockInvoke.mockReset();
    configureRpc({ middleware: [] });
    const client = createClient<TestContract>();
    mockInvoke.mockResolvedValueOnce({ id: 1, name: "John" });

    await client.user.get({ id: 1 });

    expect(mockInvoke).toHaveBeenCalledWith("plugin:rpc|rpc_call", {
      path: "user.get",
      input: { id: 1 },
    });
  });

  it("should build correct paths for root procedures", async () => {
    mockInvoke.mockReset();
    configureRpc({ middleware: [] });
    const client = createClient<TestContract>();
    mockInvoke.mockResolvedValueOnce({ status: "ok" });

    await client.health();

    expect(mockInvoke).toHaveBeenCalledWith("plugin:rpc|rpc_call", {
      path: "health",
      input: null,
    });
  });

  it("should pass input correctly", async () => {
    const client = createClient<TestContract>();
    mockInvoke.mockResolvedValueOnce("Hello, World!");

    await client.greet({ name: "World" });

    expect(mockInvoke).toHaveBeenCalledWith("plugin:rpc|rpc_call", {
      path: "greet",
      input: { name: "World" },
    });
  });

  // Property: Client proxy builds correct paths for any depth
  it("property: proxy builds correct dot-separated paths", async () => {
    interface DeepContract {
      a: {
        b: {
          c: {
            method: { type: "query"; input: void; output: string };
          };
        };
      };
    }

    const client = createClient<DeepContract>();
    mockInvoke.mockResolvedValue("result");

    await client.a.b.c.method();

    expect(mockInvoke).toHaveBeenCalledWith("plugin:rpc|rpc_call", {
      path: "a.b.c.method",
      input: null,
    });
  });
});

// =============================================================================
// Path Validation Tests
// =============================================================================

describe("Path Validation", () => {
  it("should accept valid paths", () => {
    expect(() => validatePath("health")).not.toThrow();
    expect(() => validatePath("user.get")).not.toThrow();
    expect(() => validatePath("api.v1.users.list")).not.toThrow();
    expect(() => validatePath("a_b_c")).not.toThrow();
    expect(() => validatePath("test123")).not.toThrow();
  });

  it("should reject empty paths", () => {
    expect(() => validatePath("")).toThrow("cannot be empty");
  });

  it("should reject paths starting with dot", () => {
    expect(() => validatePath(".path")).toThrow(
      "cannot start or end with a dot",
    );
  });

  it("should reject paths ending with dot", () => {
    expect(() => validatePath("path.")).toThrow(
      "cannot start or end with a dot",
    );
  });

  it("should reject paths with consecutive dots", () => {
    expect(() => validatePath("path..name")).toThrow(
      "cannot contain consecutive dots",
    );
  });

  it("should reject paths with invalid characters", () => {
    expect(() => validatePath("path/name")).toThrow("invalid character");
    expect(() => validatePath("path name")).toThrow("invalid character");
    expect(() => validatePath("path-name")).toThrow("invalid character");
    expect(() => validatePath("path@name")).toThrow("invalid character");
  });

  // Property 11: Path Validation Completeness
  // Valid paths should always pass, invalid paths should always fail
  it("property: path validation completeness", () => {
    // Generate valid paths: alphanumeric + underscore, dot-separated, no leading/trailing dots
    const validSegmentArb = fc.stringMatching(/^[a-zA-Z][a-zA-Z0-9_]*$/);
    const validPathArb = fc
      .array(validSegmentArb, { minLength: 1, maxLength: 5 })
      .map((segments) => segments.join("."));

    // Test that all valid paths pass validation
    fc.assert(
      fc.property(validPathArb, (path) => {
        expect(() => validatePath(path)).not.toThrow();
      }),
      { numRuns: 100 },
    );
  });

  it("property: invalid paths are rejected", () => {
    // Generate invalid paths
    const invalidPathArb = fc.oneof(
      // Empty string
      fc.constant(""),
      // Leading dot
      fc
        .string({ minLength: 1 })
        .map((s) => "." + s.replace(/[^a-zA-Z0-9_.]/g, "")),
      // Trailing dot
      fc
        .string({ minLength: 1 })
        .map((s) => s.replace(/[^a-zA-Z0-9_.]/g, "") + "."),
      // Consecutive dots
      fc
        .tuple(
          fc.stringMatching(/^[a-zA-Z][a-zA-Z0-9_]*$/),
          fc.stringMatching(/^[a-zA-Z][a-zA-Z0-9_]*$/),
        )
        .map(([a, b]) => `${a}..${b}`),
      // Invalid characters
      fc
        .stringMatching(/^[a-zA-Z][a-zA-Z0-9_]*$/)
        .chain((valid) =>
          fc
            .constantFrom("/", " ", "-", "@", "#", "$", "%")
            .map((ch) => valid + ch + "suffix"),
        ),
    );

    fc.assert(
      fc.property(invalidPathArb, (path) => {
        expect(() => validatePath(path)).toThrow();
      }),
      { numRuns: 100 },
    );
  });
});
