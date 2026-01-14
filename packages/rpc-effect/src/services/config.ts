// =============================================================================
// RPC Config Service
// =============================================================================

import { Effect, Layer } from "effect";
import type { RpcConfig } from "../core/types";

const defaultRpcConfig: RpcConfig = {
  defaultTimeout: undefined,
  subscriptionPaths: new Set(),
  validateInput: false,
  validateOutput: false,
};

/**
 * Configuration service for RPC calls.
 * Uses Effect.Service for idiomatic service definition with default.
 *
 * @example
 * ```ts
 * // Use default config
 * Effect.provide(program, RpcConfigService.Default)
 *
 * // Custom config
 * Effect.provide(program, RpcConfigService.layer({ defaultTimeout: 5000 }))
 * ```
 */
export class RpcConfigService extends Effect.Service<RpcConfigService>()(
  "RpcConfigService",
  {
    succeed: defaultRpcConfig,
  },
) {
  /** Create a custom config by merging with defaults */
  static config(config: Partial<RpcConfig> = {}): RpcConfig {
    return {
      ...defaultRpcConfig,
      ...config,
      subscriptionPaths: new Set([
        ...defaultRpcConfig.subscriptionPaths,
        ...(config.subscriptionPaths ?? []),
      ]),
    };
  }

  /** Create a layer with custom config */
  static layer(config: Partial<RpcConfig> = {}) {
    return Layer.succeed(RpcConfigService, RpcConfigService.config(config));
  }
}
