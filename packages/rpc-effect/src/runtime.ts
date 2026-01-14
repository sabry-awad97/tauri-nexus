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
  consoleLogger,
  type RpcConfig,
  type RpcTransport,
  type RpcInterceptorChain,
  type RpcLogger,
} from "./types";

// =============================================================================
// Layer Factories (Convenience wrappers)
// =============================================================================

/**
 * Create a config layer with custom settings.
 * @deprecated Use RpcConfigService.layer() instead
 */
export const makeConfigLayer = (config: Partial<RpcConfig> = {}) =>
  RpcConfigService.layer(config);

/**
 * Create a custom transport layer.
 * @deprecated Use RpcTransportService.layer() instead
 */
export const makeTransportLayer = (transport: RpcTransport) =>
  RpcTransportService.layer(transport);

/**
 * Create an interceptor layer.
 * @deprecated Use RpcInterceptorService.layer() instead
 */
export const makeInterceptorLayer = (chain: RpcInterceptorChain) =>
  RpcInterceptorService.layer(chain);

/**
 * Create a logger layer.
 * @deprecated Use RpcLoggerService.layer() or RpcLoggerService.Default instead
 */
export const makeLoggerLayer = (logger?: RpcLogger) =>
  logger ? RpcLoggerService.layer(logger) : RpcLoggerService.Default;

// Re-export consoleLogger for backward compatibility
export { consoleLogger };

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
 * Uses default config, no interceptors, and no logging.
 */
export const makeRpcLayer = (
  transport: RpcTransport,
  config?: Partial<RpcConfig>
) =>
  Layer.mergeAll(
    RpcConfigService.layer(config),
    RpcTransportService.layer(transport),
    RpcInterceptorService.Default,
    RpcLoggerService.Default
  );

/**
 * Create a layer with console logging enabled.
 */
export const makeDebugLayer = (
  transport: RpcTransport,
  config?: Partial<RpcConfig>
) =>
  Layer.mergeAll(
    RpcConfigService.layer(config),
    RpcTransportService.layer(transport),
    RpcInterceptorService.Default,
    RpcLoggerService.Console
  );

// =============================================================================
// Runtime Management
// =============================================================================

let globalRuntime: ManagedRuntime.ManagedRuntime<RpcServices, never> | null =
  null;

export const getRuntime = (
  layer?: Layer.Layer<RpcServices>
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
  layer: Layer.Layer<RpcServices>
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
  effect: Effect.Effect<A, E, RpcServices>
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
