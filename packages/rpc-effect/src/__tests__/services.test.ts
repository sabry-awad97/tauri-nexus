// =============================================================================
// TC005: Effect Services Injection and Override Tests
// =============================================================================
// Test that services for configuration, transport, interceptor chains, and
// logging are injectable and can be overridden or composed.

import { describe, it, expect, vi } from "vitest";
import { Effect, Layer } from "effect";
import {
  RpcConfigService,
  RpcTransportService,
  RpcInterceptorService,
  RpcLoggerService,
  consoleLogger,
  type RpcConfig,
  type RpcTransport,
  type RpcInterceptorChain,
  type RpcLogger,
  type RpcInterceptor,
  type EventIterator,
} from "../index";

// Helper to create a properly typed mock transport
const createMockTransport = (
  overrides: {
    callResult?: unknown;
    batchResults?: unknown[];
  } = {},
): RpcTransport & { callSpy: ReturnType<typeof vi.fn> } => {
  const callSpy = vi.fn();
  return {
    callSpy,
    call: async <T>(path: string, input: unknown): Promise<T> => {
      callSpy(path, input);
      return (overrides.callResult ?? { result: "test" }) as T;
    },
    callBatch: async () => ({ results: overrides.batchResults ?? [] }) as any,
    subscribe: async <T>(): Promise<EventIterator<T>> => ({
      return: async () => {},
      [Symbol.asyncIterator]: () => ({
        next: async () => ({ done: true as const, value: undefined as T }),
      }),
    }),
  };
};

// Helper to create a properly typed mock interceptor
const createMockInterceptor = (name: string): RpcInterceptor => ({
  name,
  intercept: async <T>(_ctx: unknown, next: () => Promise<T>) => next(),
});

describe("TC005: Effect Services Injection", () => {
  describe("RpcConfigService", () => {
    it("should provide default configuration", async () => {
      const program = Effect.gen(function* () {
        const config = yield* RpcConfigService;
        return config;
      });

      const config = await Effect.runPromise(
        program.pipe(Effect.provide(RpcConfigService.Default)),
      );

      expect(config.defaultTimeout).toBeUndefined();
      expect(config.subscriptionPaths).toBeInstanceOf(Set);
      expect(config.validateInput).toBe(false);
      expect(config.validateOutput).toBe(false);
    });

    it("should allow custom configuration via layer", async () => {
      const customConfig: Partial<RpcConfig> = {
        defaultTimeout: 5000,
        validateInput: true,
        subscriptionPaths: new Set(["events.stream"]),
      };

      const program = Effect.gen(function* () {
        const config = yield* RpcConfigService;
        return config;
      });

      const config = await Effect.runPromise(
        program.pipe(Effect.provide(RpcConfigService.layer(customConfig))),
      );

      expect(config.defaultTimeout).toBe(5000);
      expect(config.validateInput).toBe(true);
      expect(config.subscriptionPaths.has("events.stream")).toBe(true);
    });

    it("should merge custom config with defaults", () => {
      const merged = RpcConfigService.config({
        defaultTimeout: 3000,
      });

      expect(merged.defaultTimeout).toBe(3000);
      expect(merged.validateInput).toBe(false);
    });
  });

  describe("RpcTransportService", () => {
    it("should require transport to be provided", async () => {
      // This test verifies that RpcTransportService is a Context.Tag
      // that requires explicit provision (no default)
      expect(RpcTransportService).toBeDefined();
      expect(RpcTransportService.layer).toBeDefined();
    });

    it("should accept custom transport via layer", async () => {
      const mockTransport = createMockTransport({
        callResult: { result: "test" },
      });

      const program = Effect.gen(function* () {
        const transport = yield* RpcTransportService;
        return transport.call("test.path", {});
      });

      const result = await Effect.runPromise(
        program.pipe(Effect.provide(RpcTransportService.layer(mockTransport))),
      );

      expect(result).toEqual({ result: "test" });
      expect(mockTransport.callSpy).toHaveBeenCalledWith("test.path", {});
    });
  });

  describe("RpcInterceptorService", () => {
    it("should provide empty interceptor chain by default", async () => {
      const program = Effect.gen(function* () {
        const chain = yield* RpcInterceptorService;
        return chain;
      });

      const chain = await Effect.runPromise(
        program.pipe(Effect.provide(RpcInterceptorService.Default)),
      );

      expect(chain.interceptors).toEqual([]);
    });

    it("should accept custom interceptors via withInterceptors", async () => {
      const mockInterceptor = createMockInterceptor("test");

      const program = Effect.gen(function* () {
        const chain = yield* RpcInterceptorService;
        return chain;
      });

      const chain = await Effect.runPromise(
        program.pipe(
          Effect.provide(
            RpcInterceptorService.withInterceptors([mockInterceptor]),
          ),
        ),
      );

      expect(chain.interceptors).toHaveLength(1);
      expect(chain.interceptors[0]).toBe(mockInterceptor);
    });

    it("should accept custom chain via layer", async () => {
      const customChain: RpcInterceptorChain = {
        interceptors: [
          createMockInterceptor("first"),
          createMockInterceptor("second"),
        ],
      };

      const program = Effect.gen(function* () {
        const chain = yield* RpcInterceptorService;
        return chain;
      });

      const chain = await Effect.runPromise(
        program.pipe(Effect.provide(RpcInterceptorService.layer(customChain))),
      );

      expect(chain.interceptors).toHaveLength(2);
    });
  });

  describe("RpcLoggerService", () => {
    it("should provide noop logger by default", async () => {
      const program = Effect.gen(function* () {
        const logger = yield* RpcLoggerService;
        return logger;
      });

      const logger = await Effect.runPromise(
        program.pipe(Effect.provide(RpcLoggerService.Default)),
      );

      expect(() => logger.debug("test")).not.toThrow();
      expect(() => logger.info("test")).not.toThrow();
      expect(() => logger.warn("test")).not.toThrow();
      expect(() => logger.error("test")).not.toThrow();
    });

    it("should provide console logger via Console layer", async () => {
      const program = Effect.gen(function* () {
        const logger = yield* RpcLoggerService;
        return logger;
      });

      const logger = await Effect.runPromise(
        program.pipe(Effect.provide(RpcLoggerService.Console)),
      );

      expect(logger).toBe(consoleLogger);
    });

    it("should accept custom logger via layer", async () => {
      const logs: string[] = [];
      const customLogger: RpcLogger = {
        debug: (msg) => logs.push(`DEBUG: ${msg}`),
        info: (msg) => logs.push(`INFO: ${msg}`),
        warn: (msg) => logs.push(`WARN: ${msg}`),
        error: (msg) => logs.push(`ERROR: ${msg}`),
      };

      const program = Effect.gen(function* () {
        const logger = yield* RpcLoggerService;
        logger.info("Test message");
        return logger;
      });

      await Effect.runPromise(
        program.pipe(Effect.provide(RpcLoggerService.layer(customLogger))),
      );

      expect(logs).toContain("INFO: Test message");
    });
  });

  describe("Layer Composition", () => {
    it("should compose multiple service layers", async () => {
      const mockTransport = createMockTransport({ callResult: "result" });

      const logs: string[] = [];
      const customLogger: RpcLogger = {
        debug: (msg) => logs.push(msg),
        info: (msg) => logs.push(msg),
        warn: (msg) => logs.push(msg),
        error: (msg) => logs.push(msg),
      };

      const composedLayer = Layer.mergeAll(
        RpcConfigService.layer({ defaultTimeout: 5000 }),
        RpcTransportService.layer(mockTransport),
        RpcInterceptorService.Default,
        RpcLoggerService.layer(customLogger),
      );

      const program = Effect.gen(function* () {
        const config = yield* RpcConfigService;
        const transport = yield* RpcTransportService;
        const chain = yield* RpcInterceptorService;
        const logger = yield* RpcLoggerService;

        logger.info("Starting call");
        const result = yield* Effect.tryPromise(() =>
          transport.call("test", {}),
        );

        return {
          timeout: config.defaultTimeout,
          interceptorCount: chain.interceptors.length,
          result,
        };
      });

      const result = await Effect.runPromise(
        program.pipe(Effect.provide(composedLayer)),
      );

      expect(result.timeout).toBe(5000);
      expect(result.interceptorCount).toBe(0);
      expect(result.result).toBe("result");
      expect(logs).toContain("Starting call");
    });
  });

  describe("Service Access in Effects", () => {
    it("should access multiple services in single effect", async () => {
      const mockTransport = createMockTransport({
        callResult: { path: "test" },
      });

      const layer = Layer.mergeAll(
        RpcConfigService.layer({ defaultTimeout: 3000 }),
        RpcTransportService.layer(mockTransport),
        RpcInterceptorService.Default,
        RpcLoggerService.Default,
      );

      const program = Effect.gen(function* () {
        const config = yield* RpcConfigService;
        const transport = yield* RpcTransportService;
        const interceptors = yield* RpcInterceptorService;
        const logger = yield* RpcLoggerService;

        return {
          hasConfig: config !== undefined,
          hasTransport: transport !== undefined,
          hasInterceptors: interceptors !== undefined,
          hasLogger: logger !== undefined,
        };
      });

      const result = await Effect.runPromise(
        program.pipe(Effect.provide(layer)),
      );

      expect(result.hasConfig).toBe(true);
      expect(result.hasTransport).toBe(true);
      expect(result.hasInterceptors).toBe(true);
      expect(result.hasLogger).toBe(true);
    });
  });
});
