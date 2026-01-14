// =============================================================================
// Subscription Types
// =============================================================================

import type { Event } from "../core/types";

/** Subscription event types */
export type SubscriptionEventType = "data" | "error" | "completed";

/** Generic subscription event */
export interface SubscriptionEvent<T> {
  readonly type: SubscriptionEventType;
  readonly payload?: Event<T> | SubscriptionError;
}

/** Subscription error structure */
export interface SubscriptionError {
  readonly code: string;
  readonly message: string;
  readonly details?: unknown;
}

/** Base subscription state */
export interface SubscriptionState {
  readonly id: string;
  readonly reconnectAttempts: number;
  readonly lastEventId?: string;
  readonly completed: boolean;
  readonly pendingConsumers: number;
}

/** Reconnection configuration */
export interface ReconnectConfig {
  readonly autoReconnect: boolean;
  readonly maxReconnects: number;
  readonly reconnectDelay: number;
}

/** Shutdown sentinel for queue termination */
export const SHUTDOWN_SENTINEL = Symbol("SHUTDOWN");
export type QueueItem<T> = SubscriptionEvent<T> | typeof SHUTDOWN_SENTINEL;

/** Default reconnection configuration */
export const defaultReconnectConfig: ReconnectConfig = {
  autoReconnect: false,
  maxReconnects: 5,
  reconnectDelay: 1000,
};
