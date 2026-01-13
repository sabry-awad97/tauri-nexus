// =============================================================================
// @tauri-nexus/rpc-core - Client Configuration
// =============================================================================
// Global configuration for the RPC client.

import type { Middleware, RequestContext } from "../core/types";
import type { RpcError } from "../core/types";

// =============================================================================
// Configuration Types
// =============================================================================

export interface RpcClientConfig {
  /** Middleware stack - executed in order */
  middleware?: Middleware[];
  /** Paths that are subscriptions (for runtime detection) */
  subscriptionPaths?: string[];
  /** Global request timeout in milliseconds */
  timeout?: number;
  /** Called before each request */
  onRequest?: (ctx: RequestContext) => void;
  /** Called after successful response */
  onResponse?: <R>(ctx: RequestContext, data: R) => void;
  /** Called on error */
  onError?: (ctx: RequestContext, error: RpcError) => void;
}

// =============================================================================
// Global Configuration
// =============================================================================

/** Global configuration store */
let globalConfig: RpcClientConfig = {};

/**
 * Configure the RPC client globally.
 */
export function configureRpc(config: RpcClientConfig): void {
  globalConfig = { ...globalConfig, ...config };
}

/**
 * Get current configuration.
 */
export function getConfig(): RpcClientConfig {
  return globalConfig;
}

/**
 * Check if path is a subscription.
 */
export function isSubscriptionPath(path: string): boolean {
  const paths = globalConfig.subscriptionPaths ?? [];
  return paths.includes(path);
}
