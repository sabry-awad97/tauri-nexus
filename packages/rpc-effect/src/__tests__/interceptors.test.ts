// =============================================================================
// Interceptors Tests
// =============================================================================

import { describe, it, expect, vi } from "vitest";
import * as fc from "fast-check";
import {
  loggingInterceptor,
  retryInterceptor,
  errorHandlerInterceptor,
  authInterceptor,
  timingInterceptor,
  dedupeInterceptor,
  type InterceptorContext,
} from "../index";

const createMockContext = (
  path: string = "test.path",
  input: unknown = null,
): InterceptorContext => ({
  path,
  input,
  type: "query",
  meta: {},
});

describe("loggingInterceptor", () => {
  it("should log requests and responses", async () => {
    const consoleSpy = vi.spyOn(console, "log").mockImplementation(() => {});

    const interceptor = loggingInterceptor({ prefix: "[TEST]" });
    const ctx = createMockContext("user.get", { id: 1 });

    await interceptor.intercept(ctx, () => Promise.resolve({ name: "John" }));

    expect(consoleSpy).toHaveBeenCalledWith("[TEST] → user.get", { id: 1 });
    expect(consoleSpy).toHaveBeenCalledWith(
      expect.stringContaining("[TEST] ← user.get"),
      { name: "John" },
    );

    consoleSpy.mockRestore();
  });

  it("should log errors", async () => {
    const consoleErrorSpy = vi
      .spyOn(console, "error")
      .mockImplementation(() => {});
    vi.spyOn(console, "log").mockImplementation(() => {});

    const interceptor = loggingInterceptor();
    const ctx = createMockContext();

    await expect(
      interceptor.intercept(ctx, () => Promise.reject(new Error("fail"))),
    ).rejects.toThrow("fail");

    expect(consoleErrorSpy).toHaveBeenCalled();

    consoleErrorSpy.mockRestore();
  });
});

describe("retryInterceptor", () => {
  it("should retry on retryable errors", async () => {
    let attempts = 0;

    const interceptor = retryInterceptor({ maxRetries: 2, delay: 10 });
    const ctx = createMockContext();

    const result = await interceptor.intercept(ctx, async () => {
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

    const interceptor = retryInterceptor({ maxRetries: 3, delay: 10 });
    const ctx = createMockContext();

    await expect(
      interceptor.intercept(ctx, async () => {
        attempts++;
        throw { code: "VALIDATION_ERROR", message: "Invalid" };
      }),
    ).rejects.toMatchObject({ code: "VALIDATION_ERROR" });

    expect(attempts).toBe(1);
  });

  it("should respect custom retryOn function", async () => {
    let attempts = 0;

    const interceptor = retryInterceptor({
      maxRetries: 3,
      delay: 10,
      retryOn: (error) => (error as { code: string }).code === "CUSTOM_RETRY",
    });
    const ctx = createMockContext();

    const result = await interceptor.intercept(ctx, async () => {
      attempts++;
      if (attempts < 2) {
        throw { code: "CUSTOM_RETRY", message: "Retry me" };
      }
      return "done";
    });

    expect(result).toBe("done");
    expect(attempts).toBe(2);
  });

  it("should use exponential backoff", async () => {
    const delays: number[] = [];
    const originalSetTimeout = global.setTimeout;
    vi.spyOn(global, "setTimeout").mockImplementation((fn, delay) => {
      delays.push(delay as number);
      return originalSetTimeout(fn, 1);
    });

    let attempts = 0;
    const interceptor = retryInterceptor({
      maxRetries: 3,
      delay: 100,
      backoff: "exponential",
    });
    const ctx = createMockContext();

    await interceptor.intercept(ctx, async () => {
      attempts++;
      if (attempts < 4) {
        throw { code: "INTERNAL_ERROR", message: "Error" };
      }
      return "done";
    });

    expect(delays[0]).toBe(100); // 100 * 2^0
    expect(delays[1]).toBe(200); // 100 * 2^1
    expect(delays[2]).toBe(400); // 100 * 2^2

    vi.restoreAllMocks();
  });
});

describe("errorHandlerInterceptor", () => {
  it("should call handler on error", async () => {
    const handler = vi.fn();
    const interceptor = errorHandlerInterceptor(handler);
    const ctx = createMockContext();
    const error = { code: "ERROR", message: "Failed" };

    await expect(
      interceptor.intercept(ctx, () => Promise.reject(error)),
    ).rejects.toEqual(error);

    expect(handler).toHaveBeenCalledWith(error, ctx);
  });

  it("should not call handler on success", async () => {
    const handler = vi.fn();
    const interceptor = errorHandlerInterceptor(handler);
    const ctx = createMockContext();

    await interceptor.intercept(ctx, () => Promise.resolve("success"));

    expect(handler).not.toHaveBeenCalled();
  });
});

describe("authInterceptor", () => {
  it("should add authorization header when token present", async () => {
    const interceptor = authInterceptor({
      getToken: () => "my-token",
    });
    const ctx = createMockContext();

    await interceptor.intercept(ctx, () => Promise.resolve("result"));

    expect(ctx.meta.authorization).toBe("Bearer my-token");
  });

  it("should not add header when token is null", async () => {
    const interceptor = authInterceptor({
      getToken: () => null,
    });
    const ctx = createMockContext();

    await interceptor.intercept(ctx, () => Promise.resolve("result"));

    expect(ctx.meta.authorization).toBeUndefined();
  });

  it("should use custom header name and prefix", async () => {
    const interceptor = authInterceptor({
      getToken: () => "api-key",
      headerName: "X-API-Key",
      prefix: "Key",
    });
    const ctx = createMockContext();

    await interceptor.intercept(ctx, () => Promise.resolve("result"));

    expect(ctx.meta["X-API-Key"]).toBe("Key api-key");
  });

  it("should handle async getToken", async () => {
    const interceptor = authInterceptor({
      getToken: async () => {
        await new Promise((r) => setTimeout(r, 10));
        return "async-token";
      },
    });
    const ctx = createMockContext();

    await interceptor.intercept(ctx, () => Promise.resolve("result"));

    expect(ctx.meta.authorization).toBe("Bearer async-token");
  });
});

describe("timingInterceptor", () => {
  it("should call onTiming with duration", async () => {
    const onTiming = vi.fn();
    const interceptor = timingInterceptor(onTiming);
    const ctx = createMockContext("user.get");

    await interceptor.intercept(ctx, async () => {
      await new Promise((r) => setTimeout(r, 50));
      return "result";
    });

    expect(onTiming).toHaveBeenCalledWith("user.get", expect.any(Number));
    expect(onTiming.mock.calls[0][1]).toBeGreaterThanOrEqual(50);
  });

  it("should call onTiming even on error", async () => {
    const onTiming = vi.fn();
    const interceptor = timingInterceptor(onTiming);
    const ctx = createMockContext();

    await expect(
      interceptor.intercept(ctx, () => Promise.reject(new Error("fail"))),
    ).rejects.toThrow();

    expect(onTiming).toHaveBeenCalled();
  });
});

describe("dedupeInterceptor", () => {
  it("should deduplicate concurrent requests", async () => {
    let callCount = 0;
    const interceptor = dedupeInterceptor();
    const ctx1 = createMockContext("user.get", { id: 1 });
    const ctx2 = createMockContext("user.get", { id: 1 });

    const next = async () => {
      callCount++;
      await new Promise((r) => setTimeout(r, 50));
      return { name: "John" };
    };

    const [result1, result2] = await Promise.all([
      interceptor.intercept(ctx1, next),
      interceptor.intercept(ctx2, next),
    ]);

    expect(callCount).toBe(1);
    expect(result1).toEqual(result2);
  });

  it("should not deduplicate different requests", async () => {
    let callCount = 0;
    const interceptor = dedupeInterceptor();
    const ctx1 = createMockContext("user.get", { id: 1 });
    const ctx2 = createMockContext("user.get", { id: 2 });

    const next = async () => {
      callCount++;
      return { name: "User" };
    };

    await Promise.all([
      interceptor.intercept(ctx1, next),
      interceptor.intercept(ctx2, next),
    ]);

    expect(callCount).toBe(2);
  });

  it("should use custom key generator", async () => {
    let callCount = 0;
    const interceptor = dedupeInterceptor({
      getKey: (ctx) => ctx.path, // Only dedupe by path, ignore input
    });
    const ctx1 = createMockContext("user.get", { id: 1 });
    const ctx2 = createMockContext("user.get", { id: 2 });

    const next = async () => {
      callCount++;
      await new Promise((r) => setTimeout(r, 50));
      return { name: "User" };
    };

    await Promise.all([
      interceptor.intercept(ctx1, next),
      interceptor.intercept(ctx2, next),
    ]);

    expect(callCount).toBe(1);
  });
});

describe("Property-Based Tests", () => {
  it("property: authInterceptor adds header iff token is truthy", async () => {
    await fc.assert(
      fc.asyncProperty(
        fc.option(fc.string({ minLength: 1 }), { nil: undefined }),
        async (token) => {
          const interceptor = authInterceptor({
            getToken: () => token ?? null,
          });
          const ctx = createMockContext();

          await interceptor.intercept(ctx, () => Promise.resolve("result"));

          if (token) {
            expect(ctx.meta.authorization).toBe(`Bearer ${token}`);
          } else {
            expect(ctx.meta.authorization).toBeUndefined();
          }
        },
      ),
      { numRuns: 100 },
    );
  });
});
