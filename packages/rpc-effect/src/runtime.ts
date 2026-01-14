// =============================================================================
// @tauri-nexus/rpc-effect - Effect Runtime Management
// =============================================================================
// Manages the Effect runtime and provides service layers for RPC operations.

import { Effect, Layer, ManagedRuntime } from "effect";
import {
  RpcConfigService,
  RpcTransportService,
  RpcInterceptorService,
  RpcLoggerService,
  type RpcConfig,
  type RpcTransport,
  type RpcInterceptorChain,
  type RpcLogger,
} from "./types";

// =============================================================================
// Default Service Implementations
// =============================================================================

const defaultConfig: RpcConfig = {
  defaultTimeout: undefined,
  subscriptionPaths: new Set(),
  validateInput: false,
  validateOutput: false,
};

export const makeConfigLayer = (config: Partial<RpcConfig> = {}) =>
  Layer.succeed(RpcConfigService, {
    ...defaultConfig,
    ...config,
    subscriptionPaths: new Set([
      ...defaultConfig.subscriptionPaths,
      ...(config.subscriptionPaths ?? []),
    ]),
  });

/**
 * Create a custom transport layer.
 */
export const makeTransportLayer = (transport: RpcTransport) =>
  Layer.succeed(RpcTransportService, transport);

const defaultInterceptorChain: RpcInterceptorChain = {
  interceptors: [],
};

export const makeInterceptorLayer = (chain: RpcInterceptorChain) =>
  Layer.succeed(RpcInterceptorService, chain);

const noopLogger: RpcLogger = {
  debug: () => {},
  info: () => {},
  warn: () => {},
  error: () => {},
};

export const consoleLogger: RpcLogger = {
  debug: (msg, data) => console.debug(`[RPC] ${msg}`, data ?? ""),
  info: (msg, data) => console.info(`[RPC] ${msg}`, data ?? ""),
  warn: (msg, data) => console.warn(`[RPC] ${msg}`, data ?? ""),
  error: (msg, data) => console.error(`[RPC] ${msg}`, data ?? ""),
};

export const makeLoggerLayer = (logger: RpcLogger = noopLogger) =>
  Layer.succeed(RpcLoggerService, logger);

// =============================================================================
// Combined Layers
// =============================================================================

/**
 * Full service requirements for RPC operations.
 */
export type RpcServices =
  | RpcConfigService
  | RpcTransportService
  | RpcInterceptorService
  | RpcLoggerService;

/**
 * Create a layer stack with custom transport.
 */
export const makeRpcLayer = (
  transport: RpcTransport,
  config?: Partial<RpcConfig>,
) =>
  Layer.mergeAll(
    makeConfigLayer(config),
    makeTransportLayer(transport),
    makeInterceptorLayer(defaultInterceptorChain),
    makeLoggerLayer(noopLogger),
  );

/**
 * Create a layer with console logging enabled.
 */
export const makeDebugLayer = (
  transport: RpcTransport,
  config?: Partial<RpcConfig>,
) =>
  Layer.mergeAll(
    makeConfigLayer(config),
    makeTransportLayer(transport),
    makeInterceptorLayer(defaultInterceptorChain),
    makeLoggerLayer(consoleLogger),
  );

// =============================================================================
// Runtime Management
// =============================================================================

let globalRuntime: ManagedRuntime.ManagedRuntime<RpcServices, never> | null =
  null;

export const getRuntime = (
  layer?: Layer.Layer<RpcServices>,
): ManagedRuntime.ManagedRuntime<RpcServices, never> => {
  if (!globalRuntime && layer) {
    globalRuntime = ManagedRuntime.make(layer);
  }
  if (!globalRuntime) {
    throw new Error("Runtime not initialized. Call initializeRuntime first.");
  }
  return globalRuntime;
};

export const initializeRuntime = (
  layer: Layer.Layer<RpcServices>,
): ManagedRuntime.ManagedRuntime<RpcServices, never> => {
  if (globalRuntime) {
    globalRuntime.dispose();
  }
  globalRuntime = ManagedRuntime.make(layer);
  return globalRuntime;
};

export const disposeRuntime = async (): Promise<void> => {
  if (globalRuntime) {
    await globalRuntime.dispose();
    globalRuntime = null;
  }
};

// =============================================================================
// Effect Execution Helpers
// =============================================================================

export const runEffect = async <A, E>(
  effect: Effect.Effect<A, E, RpcServices>,
): Promise<A> => {
  const runtime = getRuntime();
  return runtime.runPromise(effect);
};

// =============================================================================
// Service Access Helpers
// =============================================================================

export const getConfig = Effect.gen(function* () {
  return yield* RpcConfigService;
});

export const getTransport = Effect.gen(function* () {
  return yield* RpcTransportService;
});

export const getInterceptors = Effect.gen(function* () {
  return yield* RpcInterceptorService;
});

export const getLogger = Effect.gen(function* () {
  return yield* RpcLoggerService;
});
