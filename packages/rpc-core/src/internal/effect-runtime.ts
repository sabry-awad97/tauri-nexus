// =============================================================================
// @tauri-nexus/rpc-core - Effect Runtime Management
// =============================================================================
// Manages the Effect runtime and provides service layers for RPC operations.

import { Effect, Layer, Runtime, Scope, ManagedRuntime } from "effect";
import { invoke } from "@tauri-apps/api/core";
import {
  RpcConfigService,
  RpcTransportService,
  RpcInterceptorService,
  RpcLoggerService,
  type RpcConfig,
  type RpcTransport,
  type RpcInterceptorChain,
  type RpcLogger,
  type SubscribeTransportOptions,
} from "./effect-types";
import { createEventIteratorEffect } from "../subscription/effect-iterator";

// =============================================================================
// Default Service Implementations
// =============================================================================

/**
 * Default configuration.
 */
const defaultConfig: RpcConfig = {
  defaultTimeout: undefined,
  subscriptionPaths: new Set(),
  validateInput: false,
  validateOutput: false,
};

/**
 * Create a config layer with custom settings.
 */
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
 * Default Tauri transport implementation.
 */
const tauriTransport: RpcTransport = {
  call: async <T>(path: string, input: unknown): Promise<T> => {
    return invoke<T>("plugin:rpc|rpc_call", { path, input });
  },
  subscribe: async <T>(
    path: string,
    input: unknown,
    options?: SubscribeTransportOptions,
  ): Promise<AsyncIterable<T>> => {
    return Effect.runPromise(createEventIteratorEffect<T>(path, input, options));
  },
};

/**
 * Create the default Tauri transport layer.
 */
export const TauriTransportLayer = Layer.succeed(
  RpcTransportService,
  tauriTransport,
);

/**
 * Create a custom transport layer (useful for testing or alternative backends).
 */
export const makeTransportLayer = (transport: RpcTransport) =>
  Layer.succeed(RpcTransportService, transport);

/**
 * Default empty interceptor chain.
 */
const defaultInterceptorChain: RpcInterceptorChain = {
  interceptors: [],
};

/**
 * Create an interceptor layer with custom interceptors.
 */
export const makeInterceptorLayer = (chain: RpcInterceptorChain) =>
  Layer.succeed(RpcInterceptorService, chain);

/**
 * Default no-op logger.
 */
const noopLogger: RpcLogger = {
  debug: () => {},
  info: () => {},
  warn: () => {},
  error: () => {},
};

/**
 * Console logger implementation.
 */
export const consoleLogger: RpcLogger = {
  debug: (msg, data) => console.debug(`[RPC] ${msg}`, data ?? ""),
  info: (msg, data) => console.info(`[RPC] ${msg}`, data ?? ""),
  warn: (msg, data) => console.warn(`[RPC] ${msg}`, data ?? ""),
  error: (msg, data) => console.error(`[RPC] ${msg}`, data ?? ""),
};

/**
 * Create a logger layer.
 */
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
 * Create the default layer stack for Tauri RPC.
 */
export const makeDefaultLayer = (config?: Partial<RpcConfig>) =>
  Layer.mergeAll(
    makeConfigLayer(config),
    TauriTransportLayer,
    makeInterceptorLayer(defaultInterceptorChain),
    makeLoggerLayer(noopLogger),
  );

/**
 * Create a layer with console logging enabled.
 */
export const makeDebugLayer = (config?: Partial<RpcConfig>) =>
  Layer.mergeAll(
    makeConfigLayer(config),
    TauriTransportLayer,
    makeInterceptorLayer(defaultInterceptorChain),
    makeLoggerLayer(consoleLogger),
  );

// =============================================================================
// Runtime Management
// =============================================================================

/**
 * Global runtime instance (lazily initialized).
 */
let globalRuntime: ManagedRuntime.ManagedRuntime<RpcServices, never> | null =
  null;

/**
 * Get or create the global runtime.
 */
export const getRuntime = (
  layer?: Layer.Layer<RpcServices>,
): ManagedRuntime.ManagedRuntime<RpcServices, never> => {
  if (!globalRuntime) {
    globalRuntime = ManagedRuntime.make(layer ?? makeDefaultLayer());
  }
  return globalRuntime;
};

/**
 * Initialize the runtime with custom configuration.
 * Should be called once at application startup.
 */
export const initializeRuntime = (
  layer: Layer.Layer<RpcServices>,
): ManagedRuntime.ManagedRuntime<RpcServices, never> => {
  if (globalRuntime) {
    // Dispose existing runtime
    Effect.runPromise(globalRuntime.dispose());
  }
  globalRuntime = ManagedRuntime.make(layer);
  return globalRuntime;
};

/**
 * Dispose the global runtime (cleanup).
 */
export const disposeRuntime = async (): Promise<void> => {
  if (globalRuntime) {
    await Effect.runPromise(globalRuntime.dispose());
    globalRuntime = null;
  }
};

// =============================================================================
// Effect Execution Helpers
// =============================================================================

/**
 * Run an Effect with the global runtime.
 * This is the bridge between Effect internals and Promise-based public API.
 */
export const runEffect = async <A, E>(
  effect: Effect.Effect<A, E, RpcServices>,
): Promise<A> => {
  const runtime = getRuntime();
  return runtime.runPromise(effect);
};

/**
 * Run an Effect and convert errors to public format.
 */
export const runEffectSafe = async <A>(
  effect: Effect.Effect<A, unknown, RpcServices>,
  onError: (error: unknown) => Error,
): Promise<A> => {
  try {
    return await runEffect(effect as Effect.Effect<A, never, RpcServices>);
  } catch (error) {
    throw onError(error);
  }
};

// =============================================================================
// Service Access Helpers
// =============================================================================

/**
 * Get the current config from the runtime.
 */
export const getConfig = Effect.gen(function* () {
  return yield* RpcConfigService;
});

/**
 * Get the transport from the runtime.
 */
export const getTransport = Effect.gen(function* () {
  return yield* RpcTransportService;
});

/**
 * Get the interceptor chain from the runtime.
 */
export const getInterceptors = Effect.gen(function* () {
  return yield* RpcInterceptorService;
});

/**
 * Get the logger from the runtime.
 */
export const getLogger = Effect.gen(function* () {
  return yield* RpcLoggerService;
});
