// =============================================================================
// Subscription State Management
// =============================================================================

import { Effect, Ref, Queue } from "effect";
import type { SubscriptionState, QueueItem } from "./types";

// =============================================================================
// State Creation
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
export const createSubscriptionStateRef = (
  id: string,
  lastEventId?: string,
): Effect.Effect<Ref.Ref<SubscriptionState>> =>
  Ref.make(createSubscriptionState(id, lastEventId));

/**
 * Create an unbounded event queue.
 */
export const createEventQueue = <T>(): Effect.Effect<
  Queue.Queue<QueueItem<T>>
> => Queue.unbounded<QueueItem<T>>();

// =============================================================================
// State Operations
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
