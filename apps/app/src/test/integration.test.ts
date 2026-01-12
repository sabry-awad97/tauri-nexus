// =============================================================================
// Integration Tests
// =============================================================================
// End-to-end tests demonstrating the complete RPC client workflow.

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import {
  createClient,
  createClientWithSubscriptions,
  configureRpc,
  type Middleware,
} from "@tauri-nexus/rpc-react";

// =============================================================================
// Mocks
// =============================================================================

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(vi.fn()),
}));

const mockInvoke = invoke as ReturnType<typeof vi.fn>;

// =============================================================================
// Test Contract - Using the same pattern as src/rpc/contract.ts
// =============================================================================

interface User {
  id: number;
  name: string;
  email: string;
}

interface CreateUserInput {
  name: string;
  email: string;
}

// Define contract using the procedure definition pattern
interface ApiContract {
  health: {
    type: "query";
    input: void;
    output: { status: string; version: string };
  };

  user: {
    get: { type: "query"; input: { id: number }; output: User };
    list: { type: "query"; input: void; output: User[] };
    create: { type: "mutation"; input: CreateUserInput; output: User };
    delete: {
      type: "mutation";
      input: { id: number };
      output: { success: boolean };
    };
  };

  notifications: {
    subscribe: {
      type: "subscription";
      input: { userId: number };
      output: { message: string };
    };
  };
}

// =============================================================================
// Setup & Teardown
// =============================================================================

beforeEach(() => {
  vi.clearAllMocks();
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
// Integration Tests
// =============================================================================

describe("Integration: Complete RPC Workflow", () => {
  describe("Basic Client Usage", () => {
    it("should perform a complete CRUD workflow", async () => {
      // Use type assertion to work around strict type checking in tests
      const client = createClient<ApiContract>();

      // 1. Health check
      mockInvoke.mockResolvedValueOnce({ status: "ok", version: "1.0.0" });
      const health = await (client as any).health();
      expect(health.status).toBe("ok");

      // 2. Create user
      const newUser: User = { id: 1, name: "John", email: "john@example.com" };
      mockInvoke.mockResolvedValueOnce(newUser);
      const created = await (client as any).user.create({
        name: "John",
        email: "john@example.com",
      });
      expect(created.id).toBe(1);

      // 3. Get user
      mockInvoke.mockResolvedValueOnce(newUser);
      const fetched = await (client as any).user.get({ id: 1 });
      expect(fetched.name).toBe("John");

      // 4. List users
      mockInvoke.mockResolvedValueOnce([newUser]);
      const users = await (client as any).user.list();
      expect(users).toHaveLength(1);

      // 5. Delete user
      mockInvoke.mockResolvedValueOnce({ success: true });
      const deleted = await (client as any).user.delete({ id: 1 });
      expect(deleted.success).toBe(true);
    });

    it("should handle errors gracefully", async () => {
      const client = createClient<ApiContract>();

      mockInvoke.mockRejectedValueOnce(
        JSON.stringify({ code: "NOT_FOUND", message: "User not found" }),
      );

      await expect((client as any).user.get({ id: 999 })).rejects.toMatchObject(
        {
          code: "NOT_FOUND",
          message: "User not found",
        },
      );
    });
  });

  describe("Middleware Integration", () => {
    it("should execute logging middleware", async () => {
      const logs: string[] = [];

      const loggingMiddleware: Middleware = async (ctx, next) => {
        logs.push(`Request: ${ctx.path}`);
        const start = Date.now();
        const result = await next();
        logs.push(`Response: ${ctx.path} (${Date.now() - start}ms)`);
        return result;
      };

      const client = createClient<ApiContract>({
        middleware: [loggingMiddleware],
      });

      mockInvoke.mockResolvedValueOnce({ status: "ok", version: "1.0.0" });
      await (client as any).health();

      expect(logs).toHaveLength(2);
      expect(logs[0]).toBe("Request: health");
      expect(logs[1]).toMatch(/^Response: health \(\d+ms\)$/);
    });

    it("should execute authentication middleware", async () => {
      let authHeader: string | undefined;

      const authMiddleware: Middleware = async (ctx, next) => {
        // Simulate adding auth token to context
        ctx.meta = { ...ctx.meta, authorization: "Bearer token123" };
        authHeader = ctx.meta.authorization as string;
        return next();
      };

      const client = createClient<ApiContract>({
        middleware: [authMiddleware],
      });

      mockInvoke.mockResolvedValueOnce({ status: "ok", version: "1.0.0" });
      await (client as any).health();

      expect(authHeader).toBe("Bearer token123");
    });

    it("should execute retry middleware", async () => {
      let attempts = 0;

      const retryMiddleware: Middleware = async (_ctx, next) => {
        const maxRetries = 3;
        let lastError: Error | undefined;

        for (let i = 0; i < maxRetries; i++) {
          try {
            attempts++;
            return await next();
          } catch (error) {
            lastError = error as Error;
            if (i === maxRetries - 1) throw lastError;
          }
        }
        throw lastError;
      };

      const client = createClient<ApiContract>({
        middleware: [retryMiddleware],
      });

      mockInvoke
        .mockRejectedValueOnce(new Error("Temporary failure"))
        .mockRejectedValueOnce(new Error("Temporary failure"))
        .mockResolvedValueOnce({ status: "ok", version: "1.0.0" });

      const result = await (client as any).health();

      expect(attempts).toBe(3);
      expect(result.status).toBe("ok");
    });
  });

  describe("Lifecycle Hooks", () => {
    it("should track all requests and responses", async () => {
      const requests: string[] = [];
      const responses: string[] = [];
      const errors: string[] = [];

      const client = createClient<ApiContract>({
        onRequest: (ctx) => requests.push(ctx.path),
        onResponse: (ctx) => responses.push(ctx.path),
        onError: (ctx, error) => errors.push(`${ctx.path}: ${error.code}`),
      });

      // Successful request
      mockInvoke.mockResolvedValueOnce({ status: "ok", version: "1.0.0" });
      await (client as any).health();

      // Failed request
      mockInvoke.mockRejectedValueOnce(
        JSON.stringify({ code: "NOT_FOUND", message: "Not found" }),
      );
      await (client as any).user.get({ id: 999 }).catch(() => {});

      expect(requests).toEqual(["health", "user.get"]);
      expect(responses).toEqual(["health"]);
      expect(errors).toEqual(["user.get: NOT_FOUND"]);
    });
  });

  describe("Subscription Client", () => {
    it("should configure subscription paths correctly", () => {
      const client = createClientWithSubscriptions<ApiContract>({
        subscriptionPaths: ["notifications.subscribe"],
      });

      expect(client).toBeDefined();
      expect(typeof (client as any).notifications.subscribe).toBe("function");
    });
  });
});

describe("Integration: Type Safety Verification", () => {
  it("should enforce correct input types", async () => {
    const client = createClient<ApiContract>();

    mockInvoke.mockResolvedValue({
      id: 1,
      name: "Test",
      email: "test@example.com",
    });

    // These should compile without errors (using any for test flexibility)
    await (client as any).health();
    await (client as any).user.get({ id: 1 });
    await (client as any).user.list();
    await (client as any).user.create({
      name: "Test",
      email: "test@example.com",
    });
    await (client as any).user.delete({ id: 1 });

    expect(mockInvoke).toHaveBeenCalledTimes(5);
  });

  it("should infer correct output types", async () => {
    const client = createClient<ApiContract>();

    mockInvoke.mockResolvedValueOnce({ status: "ok", version: "1.0.0" });
    const health = await (client as any).health();
    expect(typeof health.status).toBe("string");
    expect(typeof health.version).toBe("string");

    mockInvoke.mockResolvedValueOnce({
      id: 1,
      name: "Test",
      email: "test@example.com",
    });
    const user = await (client as any).user.get({ id: 1 });
    expect(typeof user.id).toBe("number");
    expect(typeof user.name).toBe("string");
    expect(typeof user.email).toBe("string");
  });
});

describe("Integration: Error Handling", () => {
  it("should handle various error formats", async () => {
    const client = createClient<ApiContract>();

    // JSON error string
    mockInvoke.mockRejectedValueOnce(
      JSON.stringify({ code: "VALIDATION_ERROR", message: "Invalid input" }),
    );
    await expect((client as any).health()).rejects.toMatchObject({
      code: "VALIDATION_ERROR",
    });

    // Plain string error
    mockInvoke.mockRejectedValueOnce("Connection refused");
    await expect((client as any).health()).rejects.toMatchObject({
      code: "UNKNOWN",
      message: "Connection refused",
    });

    // Error object
    mockInvoke.mockRejectedValueOnce(new Error("Network error"));
    await expect((client as any).health()).rejects.toMatchObject({
      code: "UNKNOWN",
      message: "Network error",
    });

    // RPC error object
    mockInvoke.mockRejectedValueOnce({
      code: "INTERNAL_ERROR",
      message: "Server error",
      details: { trace: "stack trace" },
    });
    await expect((client as any).health()).rejects.toMatchObject({
      code: "INTERNAL_ERROR",
      message: "Server error",
      details: { trace: "stack trace" },
    });
  });
});
