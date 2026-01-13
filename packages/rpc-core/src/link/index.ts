// =============================================================================
// @tauri-nexus/rpc-core - Link Module
// =============================================================================

// Types
export type {
  LinkRequestContext,
  LinkResponse,
  LinkInterceptor,
  ErrorHandler,
  RequestHandler,
  ResponseHandler,
  TauriLinkConfig,
  LinkCallOptions,
  LinkSubscribeOptions,
} from "./types";

// TauriLink
export { TauriLink } from "./tauri-link";

// Client factory
export { createClientFromLink, type LinkRouterClient } from "./client-factory";

// Interceptors
export {
  onError,
  logging,
  retry,
  authInterceptor,
  type AuthInterceptorOptions,
} from "./interceptors";

// =============================================================================
// Type Inference for Client Context
// =============================================================================

import type { TauriLink } from "./tauri-link";
import type { LinkRouterClient } from "./client-factory";

/**
 * Infer the client context type from a link.
 */
export type InferLinkContext<T> = T extends TauriLink<infer C> ? C : never;

/**
 * Infer the client context type from a link client.
 */
export type InferLinkClientContext<T> =
  T extends LinkRouterClient<unknown, infer C> ? C : never;
