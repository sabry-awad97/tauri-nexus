// =============================================================================
// @tauri-nexus/rpc-core - Link Types
// =============================================================================
// Type definitions for the TauriLink abstraction.

import type { ProcedureType, RpcError } from "../core/types";

// =============================================================================
// Request Context
// =============================================================================

/** Request context passed through interceptors */
export interface LinkRequestContext<TClientContext = unknown> {
  /** Procedure path (e.g., "user.get") */
  readonly path: string;
  /** Input data */
  input: unknown;
  /** Procedure type */
  readonly type: ProcedureType;
  /** Client context provided at call time */
  readonly context: TClientContext;
  /** Abort signal for cancellation */
  readonly signal?: AbortSignal;
  /** Custom metadata */
  meta: Record<string, unknown>;
}

/** Response from a link call */
export interface LinkResponse<TOutput = unknown> {
  /** Response data */
  readonly data: TOutput;
  /** Response metadata */
  readonly meta?: Record<string, unknown>;
}

// =============================================================================
// Interceptor Types
// =============================================================================

/** Interceptor function type */
export type LinkInterceptor<TClientContext = unknown> = <T>(
  ctx: LinkRequestContext<TClientContext>,
  next: () => Promise<T>,
) => Promise<T>;

/** Error handler function */
export type ErrorHandler<TClientContext = unknown> = (
  error: RpcError,
  ctx: LinkRequestContext<TClientContext>,
) => void | Promise<void>;

/** Request handler function */
export type RequestHandler<TClientContext = unknown> = (
  ctx: LinkRequestContext<TClientContext>,
) => void | Promise<void>;

/** Response handler function */
export type ResponseHandler<TClientContext = unknown> = <T>(
  data: T,
  ctx: LinkRequestContext<TClientContext>,
) => void | Promise<void>;

// =============================================================================
// Link Configuration
// =============================================================================

export interface TauriLinkConfig<TClientContext = unknown> {
  /** Interceptors - executed in order, wrapping the request */
  interceptors?: LinkInterceptor<TClientContext>[];
  /** Called before each request */
  onRequest?: RequestHandler<TClientContext>;
  /** Called after successful response */
  onResponse?: ResponseHandler<TClientContext>;
  /** Called on error */
  onError?: ErrorHandler<TClientContext>;
  /** Global request timeout in milliseconds */
  timeout?: number;
  /** Paths that are subscriptions */
  subscriptionPaths?: string[];
}

// =============================================================================
// Call Options
// =============================================================================

export interface LinkCallOptions<TClientContext = unknown> {
  /** Client context for this call */
  context?: TClientContext;
  /** Abort signal */
  signal?: AbortSignal;
  /** Request timeout in milliseconds */
  timeout?: number;
  /** Custom metadata */
  meta?: Record<string, unknown>;
}

export interface LinkSubscribeOptions<
  TClientContext = unknown,
> extends LinkCallOptions<TClientContext> {
  /** Last event ID for resumption */
  lastEventId?: string;
  /** Auto-reconnect on disconnect */
  autoReconnect?: boolean;
  /** Reconnect delay in milliseconds */
  reconnectDelay?: number;
  /** Maximum reconnect attempts */
  maxReconnects?: number;
}
