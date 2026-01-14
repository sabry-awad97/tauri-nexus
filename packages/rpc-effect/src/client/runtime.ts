// =============================================================================
// Runtime Management
// =============================================================================

import { Effect, Layer, ManagedRuntime } from "effect";
import {
  RpcConfigService,
  RpcTransportService,
  RpcInterceptorService,
  RpcLoggerService,
  type RpcServices,
} from "../services";
import type { RpcConfig, RpcTransport } from "../core/types";

// =============================================================================
// Combined Layers
// =============================================================================

/**
 * Create a layer stack with custom transport.
 * Uses default config, no interceptors, and no logging.
 */
export const createRpcLayer = (
  transport: RpcTransport,
  config?: Partial<RpcConfig>,
) =>
  Layer.mergeAll(
    RpcConfigService.layer(config),
    RpcTransportService.layer(transport),
    RpcInterceptorService.Default,
    RpcLoggerService.Default,
  );

/**
 * Create a layer with console logging enabled.
 */
export const createDebugLayer = (
  transport: RpcTransport,
  config?: Partial<RpcConfig>,
) =>
  Layer.mergeAll(
    RpcConfigService.layer(config),
    RpcTransportService.layer(transport),
    RpcInterceptorService.Default,
    RpcLoggerService.Console,
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
