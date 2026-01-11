// =============================================================================
// TauriLink Tests
// =============================================================================

import { describe, it, expect, vi, beforeEach } from "vitest";
import {
  TauriLink,
  createClientFromLink,
  onError,
  logging,
  retry,
  type LinkRequestContext,
  type LinkInterceptor,
} from "../link";

// Mock Tauri invoke
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

// Mock event iterator
vi.mock("../event-iterator", () => ({
  createEventIterator: vi.fn(),
}));

import { invoke } from "@tauri-apps/api/core";
import { createEventIterator } from "../event-iterator";

const mockInvoke = invoke as ReturnType<typeof vi.fn>;
const mockCreateEventIterator = createEventIterator as ReturnType<typeof vi.fn>;

describe("TauriLink", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("basic calls", () => {
    it("should make a basic call", async () => {
      mockInvoke.mockResolvedValue({ status: "ok" });

      const link = new TauriLink();
      const result = await link.call("health", null);

      expect(result).toEqual({ status: "ok" });
      expect(mockInvoke).toHaveBeenCalledWith("plugin:rpc|rpc_call", {
        path: "health",
        input: null,
      });
    });

    it("should make a call with input", async () => {
      mockInvoke.mockResolvedValue({ id: 1, name: "Test" });

      const link = new TauriLink();
      const result = await link.call("user.get", { id: 1 });

      expect(result).toEqual({ id: 1, name: "Test" });
      expect(mockInvoke).toHaveBeenCalledWith("plugin:rpc|rpc_call", {
        path: "user.get",
        input: { id: 1 },
      });
    });

    it("should validate path", async () => {
      const link = new TauriLink();

      await expect(link.call("", null)).rejects.toMatchObject({
        code: "VALIDATION_ERROR",
        message: "Procedure path cannot be empty",
      });

      await expect(link.call(".path", null)).rejects.toMatchObject({
        code: "VALIDATION_ERROR",
      });

      await expect(link.call("path.", null)).rejects.toMatchObject({
        code: "VALIDATION_ERROR",
      });

      await expect(link.call("path..name", null)).rejects.toMatchObject({
        code: "VALIDATION_ERROR",
      });
    });
  });

  describe("client context", () => {
    it("should pass context through interceptors", async () => {
      mockInvoke.mockResolvedValue({ success: true });

      interface Context {
        token: string;
        userId: number;
      }

      const capturedContext: Context[] = [];

      const link = new TauriLink<Context>({
        interceptors: [
          async (ctx, next) => {
            capturedContext.push(ctx.context);
            return next();
          },
        ],
      });

      await link.call("test", null, {
        context: { token: "abc123", userId: 42 },
      });

      expect(capturedContext).toHaveLength(1);
      expect(capturedContext[0]).toEqual({ token: "abc123", userId: 42 });
    });

    it("should use empty context when not provided", async () => {
      mockInvoke.mockResolvedValue({ success: true });

      let capturedContext: unknown;

      const link = new TauriLink({
        interceptors: [
          async (ctx, next) => {
            capturedContext = ctx.context;
            return next();
          },
        ],
      });

      await link.call("test", null);

      expect(capturedContext).toEqual({});
    });
  });

  describe("interceptors", () => {
    it("should execute interceptors in order", async () => {
      mockInvoke.mockResolvedValue("result");

      const order: number[] = [];

      const link = new TauriLink({
        interceptors: [
          async (_ctx, next) => {
            order.push(1);
            const result = await next();
            order.push(4);
            return result;
          },
          async (_ctx, next) => {
            order.push(2);
            const result = await next();
            order.push(3);
            return result;
          },
        ],
      });

      await link.call("test", null);

      expect(order).toEqual([1, 2, 3, 4]);
    });

    it("should allow interceptors to modify context", async () => {
      mockInvoke.mockResolvedValue("result");

      let finalMeta: Record<string, unknown> = {};

      const link = new TauriLink({
        interceptors: [
          async (ctx, next) => {
            ctx.meta.interceptor1 = true;
            return next();
          },
          async (ctx, next) => {
            ctx.meta.interceptor2 = true;
            finalMeta = { ...ctx.meta };
            return next();
          },
        ],
      });

      await link.call("test", null);

      expect(finalMeta).toEqual({ interceptor1: true, interceptor2: true });
    });

    it("should allow interceptors to transform result", async () => {
      mockInvoke.mockResolvedValue({ value: 10 });

      const link = new TauriLink({
        interceptors: [
          // Type assertion needed because interceptor transforms the result type
          (async (_ctx, next) => {
            const result = await next();
            return { value: (result as { value: number }).value * 2 };
          }) as LinkInterceptor<unknown>,
        ],
      });

      const result = await link.call<{ value: number }>("test", null);

      expect(result.value).toBe(20);
    });
  });

  describe("lifecycle hooks", () => {
    it("should call onRequest before the call", async () => {
      mockInvoke.mockResolvedValue("result");

      const onRequest = vi.fn();

      const link = new TauriLink({ onRequest });

      await link.call("test.path", { foo: "bar" });

      expect(onRequest).toHaveBeenCalledWith(
        expect.objectContaining({
          path: "test.path",
          input: { foo: "bar" },
          type: "query",
        }),
      );
    });

    it("should call onResponse after successful call", async () => {
      mockInvoke.mockResolvedValue({ data: "test" });

      const onResponse = vi.fn();

      const link = new TauriLink({ onResponse });

      await link.call("test", null);

      expect(onResponse).toHaveBeenCalledWith(
        { data: "test" },
        expect.objectContaining({ path: "test" }),
      );
    });

    it("should call onError on failure", async () => {
      mockInvoke.mockRejectedValue(
        JSON.stringify({ code: "NOT_FOUND", message: "Not found" }),
      );

      const onError = vi.fn();

      const link = new TauriLink({ onError });

      await expect(link.call("test", null)).rejects.toMatchObject({
        code: "NOT_FOUND",
      });

      expect(onError).toHaveBeenCalledWith(
        expect.objectContaining({ code: "NOT_FOUND" }),
        expect.objectContaining({ path: "test" }),
      );
    });
  });

  describe("subscriptions", () => {
    it("should identify subscription paths", () => {
      const link = new TauriLink({
        subscriptionPaths: ["stream.counter", "stream.events"],
      });

      expect(link.isSubscription("stream.counter")).toBe(true);
      expect(link.isSubscription("stream.events")).toBe(true);
      expect(link.isSubscription("user.get")).toBe(false);
    });

    it("should call createEventIterator for subscriptions", async () => {
      const mockIterator = {
        [Symbol.asyncIterator]: () => mockIterator,
        next: vi.fn(),
        return: vi.fn(),
      };
      mockCreateEventIterator.mockResolvedValue(mockIterator);

      const link = new TauriLink({
        subscriptionPaths: ["stream.counter"],
      });

      const result = await link.subscribe("stream.counter", { start: 0 });

      expect(mockCreateEventIterator).toHaveBeenCalledWith(
        "stream.counter",
        { start: 0 },
        expect.any(Object),
      );
      expect(result).toBe(mockIterator);
    });
  });
});

describe("createClientFromLink", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("should create a proxy client", async () => {
    mockInvoke.mockResolvedValue({ id: 1, name: "Test" });

    const link = new TauriLink();
    const client = createClientFromLink<{
      user: {
        get: {
          type: "query";
          input: { id: number };
          output: { id: number; name: string };
        };
      };
    }>(link);

    const result = await client.user.get({ id: 1 });

    expect(result).toEqual({ id: 1, name: "Test" });
    expect(mockInvoke).toHaveBeenCalledWith("plugin:rpc|rpc_call", {
      path: "user.get",
      input: { id: 1 },
    });
  });

  it("should pass context to link", async () => {
    mockInvoke.mockResolvedValue({ success: true });

    interface Context {
      token: string;
    }

    let capturedContext: Context | undefined;

    const link = new TauriLink<Context>({
      interceptors: [
        async (ctx, next) => {
          capturedContext = ctx.context;
          return next();
        },
      ],
    });

    const client = createClientFromLink<
      { test: { type: "query"; input: void; output: { success: boolean } } },
      Context
    >(link);

    await (client.test as any)(undefined, { context: { token: "secret" } });

    expect(capturedContext).toEqual({ token: "secret" });
  });

  it("should route subscriptions correctly", async () => {
    const mockIterator = {
      [Symbol.asyncIterator]: () => mockIterator,
      next: vi.fn(),
      return: vi.fn(),
    };
    mockCreateEventIterator.mockResolvedValue(mockIterator);

    const link = new TauriLink({
      subscriptionPaths: ["stream.events"],
    });

    const client = createClientFromLink<{
      stream: {
        events: { type: "subscription"; input: void; output: string };
      };
    }>(link);

    const result = await (client.stream.events as any)();

    expect(mockCreateEventIterator).toHaveBeenCalled();
    expect(result).toBe(mockIterator);
  });
});

describe("interceptor helpers", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("onError", () => {
    it("should catch and report errors", async () => {
      const handler = vi.fn();
      const interceptor = onError(handler);

      const ctx: LinkRequestContext = {
        path: "test",
        input: null,
        type: "query",
        context: {},
        meta: {},
      };

      const error = { code: "NOT_FOUND", message: "Not found" };

      await expect(
        interceptor(ctx, () => Promise.reject(error)),
      ).rejects.toEqual(error);

      expect(handler).toHaveBeenCalledWith(error, ctx);
    });

    it("should not catch non-RpcError", async () => {
      const handler = vi.fn();
      const interceptor = onError(handler);

      const ctx: LinkRequestContext = {
        path: "test",
        input: null,
        type: "query",
        context: {},
        meta: {},
      };

      await expect(
        interceptor(ctx, () => Promise.reject(new Error("plain error"))),
      ).rejects.toThrow("plain error");

      expect(handler).not.toHaveBeenCalled();
    });
  });

  describe("logging", () => {
    it("should log requests and responses", async () => {
      const consoleSpy = vi.spyOn(console, "log").mockImplementation(() => {});

      const interceptor = logging({ prefix: "[TEST]" });

      const ctx: LinkRequestContext = {
        path: "user.get",
        input: { id: 1 },
        type: "query",
        context: {},
        meta: {},
      };

      await interceptor(ctx, () => Promise.resolve({ name: "Test" }));

      expect(consoleSpy).toHaveBeenCalledWith("[TEST] user.get", { id: 1 });
      expect(consoleSpy).toHaveBeenCalledWith(
        expect.stringContaining("[TEST] user.get completed in"),
      );

      consoleSpy.mockRestore();
    });

    it("should log errors", async () => {
      const consoleSpy = vi.spyOn(console, "log").mockImplementation(() => {});
      const consoleErrorSpy = vi
        .spyOn(console, "error")
        .mockImplementation(() => {});

      const interceptor = logging();

      const ctx: LinkRequestContext = {
        path: "test",
        input: null,
        type: "query",
        context: {},
        meta: {},
      };

      await expect(
        interceptor(ctx, () => Promise.reject(new Error("fail"))),
      ).rejects.toThrow("fail");

      expect(consoleErrorSpy).toHaveBeenCalledWith(
        expect.stringContaining("[RPC] test failed in"),
        expect.any(Error),
      );

      consoleSpy.mockRestore();
      consoleErrorSpy.mockRestore();
    });
  });

  describe("retry", () => {
    it("should retry on retryable errors", async () => {
      let attempts = 0;

      const interceptor = retry({ maxRetries: 2, delay: 10 });

      const ctx: LinkRequestContext = {
        path: "test",
        input: null,
        type: "query",
        context: {},
        meta: {},
      };

      const result = await interceptor(ctx, async () => {
        attempts++;
        if (attempts < 3) {
          throw { code: "SERVICE_UNAVAILABLE", message: "Unavailable" };
        }
        return "success";
      });

      expect(result).toBe("success");
      expect(attempts).toBe(3);
    });

    it("should not retry non-retryable errors", async () => {
      let attempts = 0;

      const interceptor = retry({ maxRetries: 3, delay: 10 });

      const ctx: LinkRequestContext = {
        path: "test",
        input: null,
        type: "query",
        context: {},
        meta: {},
      };

      await expect(
        interceptor(ctx, async () => {
          attempts++;
          throw { code: "NOT_FOUND", message: "Not found" };
        }),
      ).rejects.toMatchObject({ code: "NOT_FOUND" });

      expect(attempts).toBe(1);
    });

    it("should respect custom shouldRetry", async () => {
      let attempts = 0;

      const interceptor = retry({
        maxRetries: 3,
        delay: 10,
        shouldRetry: (error) => error.code === "CUSTOM_RETRY",
      });

      const ctx: LinkRequestContext = {
        path: "test",
        input: null,
        type: "query",
        context: {},
        meta: {},
      };

      const result = await interceptor(ctx, async () => {
        attempts++;
        if (attempts < 2) {
          throw { code: "CUSTOM_RETRY", message: "Retry me" };
        }
        return "done";
      });

      expect(result).toBe("done");
      expect(attempts).toBe(2);
    });
  });
});
