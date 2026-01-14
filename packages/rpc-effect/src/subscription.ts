// =============================================================================
// @tauri-nexus/rpc-effect - Subscription Primitives
// =============================================================================
// Effect-based primitives for managing subscription state and event queues.
// These are transport-agnostic building blocks for subscription implementations.

import { Effect, Ref, Queue, Duration } from "effect";
import { makeCallError } from "./errors";
import type { Event, RpcEffectError } from "./types";

// =============================================================================
// Types
// =============================================================================

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

/** Base subscription state (can be extended) */
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

// =============================================================================
// Default Configuration
// =============================================================================

export const defaultReconnectConfig: ReconnectConfig = {
  autoReconnect: false,
  maxReconnects: 5,
  reconnectDelay: 1000,
};

// =============================================================================
// State Management
// =============================================================================

/**
 * Create initial subscription state.
 */
export const createSubscriptionState = (
  id: string,
  lastEventId?: string,
): SubscriptionState => ({
  id,
  reconnectAttempts: 0,
  lastEventId,
  completed: false,
  pendingConsumers: 0,
});

/**
 * Create a managed subscription state ref.
 */
export const makeSubscriptionStateRef = (
  id: string,
  lastEventId?: string,
): Effect.Effect<Ref.Ref<SubscriptionState>> =>
  Ref.make(createSubscriptionState(id, lastEventId));

/**
 * Create an unbounded event queue.
 */
export const makeEventQueue = <T>(): Effect.Effect<Queue.Queue<QueueItem<T>>> =>
  Queue.unbounded<QueueItem<T>>();

// =============================================================================
// State Operations (Generic to support extended state types)
// =============================================================================

/**
 * Mark subscription as completed.
 */
export const markCompleted = <S extends SubscriptionState>(
  stateRef: Ref.Ref<S>,
): Effect.Effect<void> =>
  Ref.update(stateRef, (s) => ({ ...s, completed: true }));

/**
 * Update last event ID.
 */
export const updateLastEventId = <S extends SubscriptionState>(
  stateRef: Ref.Ref<S>,
  eventId: string,
): Effect.Effect<void> =>
  Ref.update(stateRef, (s) => ({ ...s, lastEventId: eventId }));

/**
 * Increment pending consumers count.
 */
export const incrementConsumers = <S extends SubscriptionState>(
  stateRef: Ref.Ref<S>,
): Effect.Effect<void> =>
  Ref.update(stateRef, (s) => ({
    ...s,
    pendingConsumers: s.pendingConsumers + 1,
  }));

/**
 * Decrement pending consumers count.
 */
export const decrementConsumers = <S extends SubscriptionState>(
  stateRef: Ref.Ref<S>,
): Effect.Effect<void> =>
  Ref.update(stateRef, (s) => ({
    ...s,
    pendingConsumers: Math.max(0, s.pendingConsumers - 1),
  }));

/**
 * Reset subscription for reconnection with new ID.
 */
export const resetForReconnect = <S extends SubscriptionState>(
  stateRef: Ref.Ref<S>,
  newId: string,
): Effect.Effect<void> =>
  Ref.update(stateRef, (s) => ({
    ...s,
    id: newId,
    completed: false,
  }));

/**
 * Increment reconnect attempts.
 */
export const incrementReconnectAttempts = <S extends SubscriptionState>(
  stateRef: Ref.Ref<S>,
): Effect.Effect<void> =>
  Ref.update(stateRef, (s) => ({
    ...s,
    reconnectAttempts: s.reconnectAttempts + 1,
  }));

/**
 * Reset reconnect attempts counter.
 */
export const resetReconnectAttempts = <S extends SubscriptionState>(
  stateRef: Ref.Ref<S>,
): Effect.Effect<void> =>
  Ref.update(stateRef, (s) => ({ ...s, reconnectAttempts: 0 }));

// =============================================================================
// Queue Operations
// =============================================================================

/**
 * Offer an event to the queue.
 */
export const offerEvent = <T>(
  queue: Queue.Queue<QueueItem<T>>,
  event: SubscriptionEvent<T>,
): Effect.Effect<boolean> => Queue.offer(queue, event);

/**
 * Send shutdown sentinels to terminate consumers.
 */
export const sendShutdownSentinels = <T>(
  queue: Queue.Queue<QueueItem<T>>,
  count: number,
): Effect.Effect<void> =>
  Effect.gen(function* () {
    const sentinelCount = Math.max(1, count + 1);
    for (let i = 0; i < sentinelCount; i++) {
      yield* Queue.offer(queue, SHUTDOWN_SENTINEL);
    }
  });

/**
 * Take next item from queue.
 */
export const takeFromQueue = <T>(
  queue: Queue.Queue<QueueItem<T>>,
): Effect.Effect<QueueItem<T>> => Queue.take(queue);

// =============================================================================
// Reconnection Logic
// =============================================================================

/**
 * Calculate reconnection delay with exponential backoff and jitter.
 */
export const calculateReconnectDelay = (
  attempt: number,
  baseDelay: number,
): Effect.Effect<number> =>
  Effect.sync(() => {
    const delay = baseDelay * Math.pow(2, attempt - 1);
    return delay * (0.5 + Math.random() * 0.5);
  });

/**
 * Check if reconnection should be attempted.
 */
export const shouldReconnect = <S extends SubscriptionState>(
  stateRef: Ref.Ref<S>,
  config: ReconnectConfig,
): Effect.Effect<boolean> =>
  Effect.gen(function* () {
    if (!config.autoReconnect) return false;
    const state = yield* Ref.get(stateRef);
    return state.reconnectAttempts < config.maxReconnects;
  });

/**
 * Prepare for reconnection attempt.
 */
export const prepareReconnect = <S extends SubscriptionState>(
  stateRef: Ref.Ref<S>,
  config: ReconnectConfig,
): Effect.Effect<number> =>
  Effect.gen(function* () {
    yield* incrementReconnectAttempts(stateRef);
    const state = yield* Ref.get(stateRef);
    return yield* calculateReconnectDelay(
      state.reconnectAttempts,
      config.reconnectDelay,
    );
  });

/**
 * Wait for reconnection delay.
 */
export const waitForReconnect = (delayMs: number): Effect.Effect<void> =>
  Effect.sleep(Duration.millis(delayMs));

/**
 * Create max reconnects exceeded error.
 */
export const maxReconnectsExceededError = (
  path: string,
  attempts: number,
  maxReconnects: number,
): RpcEffectError =>
  makeCallError(
    "MAX_RECONNECTS_EXCEEDED",
    `Maximum reconnection attempts (${maxReconnects}) exceeded`,
    { attempts, maxReconnects, path },
  );

// =============================================================================
// Event Processing
// =============================================================================

/**
 * Process a data event.
 */
export const processDataEvent = <T, S extends SubscriptionState>(
  stateRef: Ref.Ref<S>,
  event: Event<T>,
): Effect.Effect<T> =>
  Effect.gen(function* () {
    if (event.id) {
      yield* updateLastEventId(stateRef, event.id);
    }
    return event.data;
  });

/**
 * Process an error event.
 */
export const processErrorEvent = <T, S extends SubscriptionState>(
  stateRef: Ref.Ref<S>,
  _queue: Queue.Queue<QueueItem<T>>,
  error: SubscriptionError,
  config: ReconnectConfig,
  path: string,
): Effect.Effect<
  { shouldRetry: boolean; error: RpcEffectError },
  RpcEffectError
> =>
  Effect.gen(function* () {
    yield* markCompleted(stateRef);

    const canReconnect = yield* shouldReconnect(stateRef, config);

    if (!canReconnect) {
      const state = yield* Ref.get(stateRef);
      if (state.reconnectAttempts >= config.maxReconnects) {
        return {
          shouldRetry: false,
          error: maxReconnectsExceededError(
            path,
            state.reconnectAttempts,
            config.maxReconnects,
          ),
        };
      }
    }

    return {
      shouldRetry: canReconnect,
      error: makeCallError(error.code, error.message, error.details),
    };
  });

// =============================================================================
// ID Generation
// =============================================================================

/**
 * Generate a unique subscription ID.
 */
export const generateSubscriptionId: Effect.Effect<string> = Effect.sync(() => {
  if (typeof crypto !== "undefined" && crypto.randomUUID) {
    return crypto.randomUUID();
  }
  return "xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx".replace(/[xy]/g, (c) => {
    const r = (Math.random() * 16) | 0;
    const v = c === "x" ? r : (r & 0x3) | 0x8;
    return v.toString(16);
  });
});

// =============================================================================
// Error Extraction
// =============================================================================

/**
 * Extract error from various error formats.
 */
export const extractSubscriptionError = (error: unknown): SubscriptionError => {
  // Check for RpcError shape
  if (
    error &&
    typeof error === "object" &&
    "code" in error &&
    "message" in error
  ) {
    const e = error as { code: string; message: string; details?: unknown };
    return { code: e.code, message: e.message, details: e.details };
  }

  // Check for Error with JSON message
  if (error instanceof Error) {
    const message = error.message;
    if (message.startsWith("{") && message.includes('"code"')) {
      try {
        const parsed = JSON.parse(message);
        if (parsed && typeof parsed === "object" && "code" in parsed) {
          return parsed as SubscriptionError;
        }
      } catch {
        // Not valid JSON
      }
    }
    return { code: "UNKNOWN", message: error.message };
  }

  return { code: "UNKNOWN", message: String(error) };
};
