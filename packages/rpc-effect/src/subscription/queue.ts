// =============================================================================
// Subscription Queue Operations
// =============================================================================

import { Effect, Queue } from "effect";
import {
  SHUTDOWN_SENTINEL,
  type SubscriptionEvent,
  type QueueItem,
} from "./types";

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
