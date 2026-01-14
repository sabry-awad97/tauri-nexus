// =============================================================================
// TC009: EffectLink Client Integration Tests
// =============================================================================
// Test high-level EffectLink client integration with layers and interceptors.

import { describe, it, expect, vi, beforeEach } from "vitest";
import { Effect } from "effect";
import {
  EffectLink,
  createEffectClient,
  createEffectClientWithTransport,
  type EffectClient,
  type RpcInterceptor,
  type EventIterator,
  RpcConfigService,
  RpcTransportService,
  RpcInterceptorService,
  RpcLoggerService,
} from "../index";

// Helper to create a properly typed mock transport
const createMockTransport = () => {
  const callSpy = vi.fn();
  const subscribeSpy = vi.fn();
  return {
    callSpy,
    subscribeSpy,
    call: async <T>(_path: string, _input: unknown): Promise<T> => {
      callSpy(_path, _input);
      return { path: _path, input: _input, result: "success" } as T;
    },
    callBatch: async () => ({ results: [] }) as any,
    subscribe: async <T>(): Promise<EventIterator<T>> => {
      subscribeSpy();
      return {
        return: async () => {},
        [Symbol.asyncIterator]: () => ({
          next: async () => ({ done: true as const, value: undefined as T }),
        }),
      };
    },
  };
};

// Helper to create a properly typed mock interceptor
const createMockInterceptor = (name: string): RpcInterceptor => ({
  name,
  intercept: async <T>(_ctx: unknown, next: () => Promise<T>) => next(),
});

describe("TC009: EffectLink Client Integration", () => {
  describe("EffectLink", () => {
    it("should create link with default config", () => {
      const link = new EffectLink();
      expect(link).toBeInstanceOf(EffectLink);
    });

    it("should create link with custom config", () => {
      const link = new EffectLink({
        timeout: 5000,
        subscriptionPaths: ["events.stream"],
        debug: true,
      });
      expect(link.isSubscription("events.stream")).toBe(true);
      expect(link.isSubscription("other.path")).toBe(false);
    });

    it("should set transport and make calls", async () => {
      const transport = createMockTransport();
      const link = new EffectLink({ timeout: 5000 });
      link.setTransport(() => transport);

      const result = await link.runCall<{ path: string }>("users.get", {
        id: 1,
      });

      expect(result.path).toBe("users.get");
      expect(transport.callSpy).toHaveBeenCalledWith("users.get", { id: 1 });
    });

    it("should throw when transport not configured", () => {
      const link = new EffectLink();
      expect(() => link.getLayer()).toThrow("Transport not configured");
    });

    it("should return Effect for call operation", () => {
      const link = new EffectLink();
      const effect = link.call("users.get", { id: 1 });
      expect(Effect.isEffect(effect)).toBe(true);
    });

    it("should return Effect for subscribe operation", () => {
      const link = new EffectLink();
      const effect = link.subscribe("events.stream", {});
      expect(Effect.isEffect(effect)).toBe(true);
    });

    it("should build and cache layer", () => {
      const transport = createMockTransport();
      const link = new EffectLink();
      link.setTransport(() => transport);

      const layer1 = link.getLayer();
      const layer2 = link.getLayer();

      expect(layer1).toBe(layer2);
    });

    it("should create new link with additional interceptors", () => {
      const transport = createMockTransport();
      const interceptor = createMockInterceptor("test");

      const link = new EffectLink();
      link.setTransport(() => transport);

      const newLink = link.withInterceptors([interceptor]);

      expect(newLink).not.toBe(link);
      expect(newLink).toBeInstanceOf(EffectLink);
    });

    it("should create new link with different timeout", () => {
      const transport = createMockTransport();
      const link = new EffectLink({ timeout: 1000 });
      link.setTransport(() => transport);

      const newLink = link.withTimeout(5000);

      expect(newLink).not.toBe(link);
    });
  });

  describe("createEffectClient", () => {
    it("should create client with default config", () => {
      const client = createEffectClient();
      expect(client.__link).toBeInstanceOf(EffectLink);
    });

    it("should create client with custom config", () => {
      const client = createEffectClient({
        timeout: 5000,
        subscriptionPaths: ["events.stream"],
        debug: true,
      });

      expect(client.isSubscription("events.stream")).toBe(true);
    });

    it("should create client with interceptors", () => {
      const interceptor = createMockInterceptor("test");

      const client = createEffectClient({
        interceptors: [interceptor],
      });

      expect(client.__link).toBeDefined();
    });
  });

  describe("createEffectClientWithTransport", () => {
    it("should create client with pre-configured transport", async () => {
      const transport = createMockTransport();
      const client = createEffectClientWithTransport({
        transport,
        timeout: 5000,
      });

      const result = await client.call<{ path: string }>("users.get", {
        id: 1,
      });

      expect(result.path).toBe("users.get");
      expect(transport.callSpy).toHaveBeenCalled();
    });

    it("should support subscription paths", () => {
      const transport = createMockTransport();
      const client = createEffectClientWithTransport({
        transport,
        subscriptionPaths: ["events.stream", "notifications.listen"],
      });

      expect(client.isSubscription("events.stream")).toBe(true);
      expect(client.isSubscription("notifications.listen")).toBe(true);
      expect(client.isSubscription("users.get")).toBe(false);
    });
  });

  describe("EffectClient Methods", () => {
    let client: EffectClient<unknown>;
    let transport: ReturnType<typeof createMockTransport>;

    beforeEach(() => {
      transport = createMockTransport();
      client = createEffectClientWithTransport({ transport });
    });

    it("should make call requests", async () => {
      await client.call("users.get", { id: 1 });
      expect(transport.callSpy).toHaveBeenCalledWith("users.get", { id: 1 });
    });

    it("should make subscribe requests", async () => {
      await client.subscribe("events.stream", {});
      expect(transport.subscribeSpy).toHaveBeenCalled();
    });

    it("should create new client with interceptors", () => {
      const interceptor = createMockInterceptor("test");
      const newClient = client.withInterceptors([interceptor]);
      expect(newClient).not.toBe(client);
    });

    it("should create new client with timeout", () => {
      const newClient = client.withTimeout(10000);
      expect(newClient).not.toBe(client);
    });
  });

  describe("Interceptor Integration", () => {
    it("should execute interceptors in order", async () => {
      const order: string[] = [];

      const interceptor1: RpcInterceptor = {
        name: "interceptor1",
        intercept: async (_ctx, next) => {
          order.push("before-1");
          const result = await next();
          order.push("after-1");
          return result;
        },
      };

      const interceptor2: RpcInterceptor = {
        name: "interceptor2",
        intercept: async (_ctx, next) => {
          order.push("before-2");
          const result = await next();
          order.push("after-2");
          return result;
        },
      };

      const transport = createMockTransport();
      const client = createEffectClientWithTransport({
        transport,
        interceptors: [interceptor1, interceptor2],
      });

      await client.call("users.get", {});

      expect(order).toEqual(["before-1", "before-2", "after-2", "after-1"]);
    });

    it("should pass context through interceptors", async () => {
      let capturedPath: string | undefined;

      const interceptor: RpcInterceptor = {
        name: "pathCapture",
        intercept: async (ctx, next) => {
          capturedPath = ctx.path;
          return next();
        },
      };

      const transport = createMockTransport();
      const client = createEffectClientWithTransport({
        transport,
        interceptors: [interceptor],
      });

      await client.call("users.get", { id: 1 });

      expect(capturedPath).toBe("users.get");
    });
  });

  describe("Layer Integration", () => {
    it("should provide all required services via layer", async () => {
      const transport = createMockTransport();
      const link = new EffectLink({ timeout: 5000, debug: true });
      link.setTransport(() => transport);

      const layer = link.getLayer();

      const program = Effect.gen(function* () {
        const config = yield* RpcConfigService;
        const transportService = yield* RpcTransportService;
        const interceptors = yield* RpcInterceptorService;
        const logger = yield* RpcLoggerService;

        return {
          hasConfig: config !== undefined,
          hasTransport: transportService !== undefined,
          hasInterceptors: interceptors !== undefined,
          hasLogger: logger !== undefined,
          timeout: config.defaultTimeout,
        };
      });

      const result = await Effect.runPromise(
        program.pipe(Effect.provide(layer)),
      );

      expect(result.hasConfig).toBe(true);
      expect(result.hasTransport).toBe(true);
      expect(result.hasInterceptors).toBe(true);
      expect(result.hasLogger).toBe(true);
      expect(result.timeout).toBe(5000);
    });

    it("should use console logger when debug is true", async () => {
      const transport = createMockTransport();
      const link = new EffectLink({ debug: true });
      link.setTransport(() => transport);

      const layer = link.getLayer();

      const program = Effect.gen(function* () {
        const logger = yield* RpcLoggerService;
        return typeof logger.debug === "function";
      });

      const hasDebug = await Effect.runPromise(
        program.pipe(Effect.provide(layer)),
      );

      expect(hasDebug).toBe(true);
    });

    it("should use noop logger when debug is false", async () => {
      const transport = createMockTransport();
      const link = new EffectLink({ debug: false });
      link.setTransport(() => transport);

      const layer = link.getLayer();

      const program = Effect.gen(function* () {
        const logger = yield* RpcLoggerService;
        logger.debug("test");
        logger.info("test");
        return true;
      });

      const result = await Effect.runPromise(
        program.pipe(Effect.provide(layer)),
      );

      expect(result).toBe(true);
    });
  });

  describe("Error Handling", () => {
    it("should propagate transport errors", async () => {
      const transport = {
        call: async () => {
          throw new Error("Network error");
        },
        callBatch: async () => ({ results: [] }),
        subscribe: async () => ({
          return: async () => {},
          [Symbol.asyncIterator]: () => ({
            next: async () => ({ done: true, value: undefined }),
          }),
        }),
      };

      const client = createEffectClientWithTransport({ transport } as any);

      await expect(client.call("users.get", {})).rejects.toThrow();
    });
  });
});
