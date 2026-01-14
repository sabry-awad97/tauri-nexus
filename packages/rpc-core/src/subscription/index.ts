// =============================================================================
// @tauri-nexus/rpc-core - Subscription Module
// =============================================================================
// Event iterator implementation for Tauri subscriptions.
// Uses subscription primitives from rpc-effect for state management.

import { Effect, Ref, Queue } from "effect";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  Event,
  EventIterator,
  RpcError,
  SubscriptionOptions,
  SubscribeRequest,
} from "../core/types";
import {
  // Types
  type SubscriptionEvent,
  type SubscriptionState,
  type ReconnectConfig,
  type QueueItem,
  SHUTDOWN_SENTINEL,
  // State management
  createEventQueue,
  markCompleted,
  updateLastEventId,
  incrementConsumers,
  decrementConsumers,
  resetForReconnect,
  resetReconnectAttempts,
  // Queue operations
  sendShutdownSentinels,
  // Reconnection
  prepareReconnect,
  waitForReconnect,
  maxReconnectsExceededError,
  // Utilities
  generateSubscriptionId,
  extractSubscriptionError,
  // Errors
  createNetworkError,
  type RpcEffectError,
} from "@tauri-nexus/rpc-effect";

// =============================================================================
// Internal State Type (extends base with Tauri-specific fields)
// =============================================================================

interface TauriSubscriptionState extends SubscriptionState {
  unlisten: UnlistenFn | null;
}

// =============================================================================
// Connection Effects
// =============================================================================

const connectEffect = <T>(
  stateRef: Ref.Ref<TauriSubscriptionState>,
  path: string,
  input: unknown,
  eventQueue: Queue.Queue<QueueItem<T>>,
): Effect.Effect<void, RpcEffectError> =>
  Effect.gen(function* () {
    const state = yield* Ref.get(stateRef);
    const eventName = `rpc:subscription:sub_${state.id}`;

    const unlisten = yield* Effect.tryPromise({
      try: () =>
        listen<SubscriptionEvent<T>>(eventName, (event) => {
          Effect.runPromise(Queue.offer(eventQueue, event.payload));
        }),
      catch: (error) => createNetworkError(path, error),
    });

    yield* Ref.update(stateRef, (s) => ({ ...s, unlisten }));

    const request: SubscribeRequest = {
      id: state.id,
      path,
      input,
      lastEventId: state.lastEventId,
    };

    yield* Effect.tryPromise({
      try: () => invoke("plugin:rpc|rpc_subscribe", { request }),
      catch: (error) => {
        unlisten();
        return createNetworkError(path, error);
      },
    });
  });

const disconnectEffect = <T>(
  stateRef: Ref.Ref<TauriSubscriptionState>,
  eventQueue: Queue.Queue<QueueItem<T>>,
): Effect.Effect<void> =>
  Effect.gen(function* () {
    const state = yield* Ref.get(stateRef);

    if (state.unlisten) {
      state.unlisten();
      yield* Ref.update(stateRef, (s) => ({ ...s, unlisten: null }));
    }

    yield* markCompleted(stateRef);

    const sentinelsToSend = Math.max(1, state.pendingConsumers + 1);
    yield* sendShutdownSentinels(eventQueue, sentinelsToSend - 1);

    yield* Effect.tryPromise(() =>
      invoke("plugin:rpc|rpc_unsubscribe", { id: `sub_${state.id}` }),
    ).pipe(Effect.catchAll(() => Effect.void));
  });

const reconnectEffect = <T>(
  stateRef: Ref.Ref<TauriSubscriptionState>,
  path: string,
  input: unknown,
  config: ReconnectConfig,
  eventQueue: Queue.Queue<QueueItem<T>>,
): Effect.Effect<boolean, RpcEffectError> =>
  Effect.gen(function* () {
    if (!config.autoReconnect) {
      return false;
    }

    const state = yield* Ref.get(stateRef);

    if (state.reconnectAttempts >= config.maxReconnects) {
      yield* markCompleted(stateRef);

      const errorEvent: SubscriptionEvent<T> = {
        type: "error",
        payload: extractSubscriptionError(
          maxReconnectsExceededError(
            path,
            state.reconnectAttempts,
            config.maxReconnects,
          ),
        ),
      };

      const sentinelsToSend = Math.max(1, state.pendingConsumers + 1);
      for (let i = 0; i < sentinelsToSend; i++) {
        yield* Queue.offer(eventQueue, errorEvent);
      }

      yield* Effect.fail(
        maxReconnectsExceededError(
          path,
          state.reconnectAttempts,
          config.maxReconnects,
        ),
      );
      return false;
    }

    const delay = yield* prepareReconnect(stateRef, config);
    yield* waitForReconnect(delay);

    const newId = yield* generateSubscriptionId;
    yield* resetForReconnect(stateRef, newId);
    yield* Ref.update(stateRef, (s) => ({ ...s, unlisten: null }));

    yield* connectEffect(stateRef, path, input, eventQueue).pipe(
      Effect.tap(() => resetReconnectAttempts(stateRef)),
      Effect.catchAll(() =>
        reconnectEffect(stateRef, path, input, config, eventQueue),
      ),
    );

    return true;
  });

// =============================================================================
// Event Iterator Factory
// =============================================================================

const createEventIteratorEffect = <T>(
  path: string,
  input: unknown = null,
  options: SubscriptionOptions = {},
): Effect.Effect<EventIterator<T>, RpcEffectError> =>
  Effect.gen(function* () {
    const subscriptionId = yield* generateSubscriptionId;

    const stateRef = yield* Ref.make<TauriSubscriptionState>({
      id: subscriptionId,
      reconnectAttempts: 0,
      lastEventId: options.lastEventId,
      completed: false,
      pendingConsumers: 0,
      unlisten: null,
    });

    const eventQueue = yield* createEventQueue<T>();

    yield* connectEffect(stateRef, path, input, eventQueue);

    if (options.signal) {
      options.signal.addEventListener("abort", () => {
        Effect.runPromise(disconnectEffect(stateRef, eventQueue));
      });
    }

    const reconnectConfig: ReconnectConfig = {
      autoReconnect: options.autoReconnect ?? false,
      maxReconnects: options.maxReconnects ?? 5,
      reconnectDelay: options.reconnectDelay ?? 1000,
    };

    const iterator: EventIterator<T> = {
      async return(): Promise<void> {
        await Effect.runPromise(disconnectEffect(stateRef, eventQueue));
      },

      [Symbol.asyncIterator](): AsyncIterator<T> {
        return {
          async next(): Promise<IteratorResult<T>> {
            try {
              return await Effect.runPromise(
                Effect.gen(function* () {
                  const state = yield* Ref.get(stateRef);

                  if (state.completed) {
                    return {
                      done: true,
                      value: undefined,
                    } as IteratorResult<T>;
                  }

                  yield* incrementConsumers(stateRef);
                  const item = yield* Queue.take(eventQueue);
                  yield* decrementConsumers(stateRef);

                  if (item === SHUTDOWN_SENTINEL) {
                    return {
                      done: true,
                      value: undefined,
                    } as IteratorResult<T>;
                  }

                  const event = item as SubscriptionEvent<T>;

                  switch (event.type) {
                    case "data": {
                      const eventData = event.payload as Event<T>;
                      if (eventData.id) {
                        yield* updateLastEventId(stateRef, eventData.id);
                      }
                      return { done: false, value: eventData.data };
                    }

                    case "error": {
                      const error = event.payload as RpcError;
                      yield* markCompleted(stateRef);

                      if (reconnectConfig.autoReconnect) {
                        const reconnected = yield* reconnectEffect(
                          stateRef,
                          path,
                          input,
                          reconnectConfig,
                          eventQueue,
                        ).pipe(Effect.catchAll(() => Effect.succeed(false)));

                        if (reconnected) {
                          return yield* Effect.fail(error);
                        }
                      }
                      return yield* Effect.fail(error);
                    }

                    case "completed": {
                      yield* markCompleted(stateRef);
                      return {
                        done: true,
                        value: undefined,
                      } as IteratorResult<T>;
                    }

                    default:
                      return {
                        done: true,
                        value: undefined,
                      } as IteratorResult<T>;
                  }
                }),
              );
            } catch (error) {
              throw extractSubscriptionError(error);
            }
          },

          async return(): Promise<IteratorResult<T>> {
            await Effect.runPromise(disconnectEffect(stateRef, eventQueue));
            return { done: true, value: undefined };
          },
        };
      },
    };

    return iterator;
  });

// =============================================================================
// Public API
// =============================================================================

/**
 * Create an async event iterator for a subscription.
 */
export async function createEventIterator<T>(
  path: string,
  input: unknown = null,
  options: SubscriptionOptions = {},
): Promise<EventIterator<T>> {
  return Effect.runPromise(createEventIteratorEffect<T>(path, input, options));
}

/**
 * Options for consuming an event iterator.
 */
export interface ConsumeOptions<T> {
  onEvent?: (event: T) => void;
  onError?: (error: RpcError) => void;
  onComplete?: () => void;
  onFinish?: (state: "success" | "error" | "cancelled") => void;
}

/**
 * Consume an event iterator with lifecycle callbacks.
 * Returns a cancel function.
 */
export function consumeEventIterator<T>(
  iteratorPromise: Promise<EventIterator<T>>,
  options: ConsumeOptions<T>,
): () => Promise<void> {
  let cancelled = false;
  let iterator: EventIterator<T> | null = null;

  (async () => {
    try {
      iterator = await iteratorPromise;

      for await (const event of iterator) {
        if (cancelled) break;
        options.onEvent?.(event);
      }

      if (!cancelled) {
        options.onComplete?.();
        options.onFinish?.("success");
      }
    } catch (error) {
      if (!cancelled) {
        options.onError?.(error as RpcError);
        options.onFinish?.("error");
      }
    }
  })();

  return async () => {
    cancelled = true;
    if (iterator) {
      await iterator.return();
    }
    options.onFinish?.("cancelled");
  };
}
