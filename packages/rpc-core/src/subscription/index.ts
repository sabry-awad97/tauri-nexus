// =============================================================================
// @tauri-nexus/rpc-core - Subscription Module
// =============================================================================
// Event iterator implementation for Tauri subscriptions.

import { Effect, pipe, Ref, Queue, Duration, Cause } from "effect";
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
  makeCallError,
  makeNetworkError,
  type RpcEffectError,
} from "@tauri-nexus/rpc-effect";

// =============================================================================
// Types
// =============================================================================

interface SubscriptionEvent<T> {
  type: "data" | "error" | "completed";
  payload?: Event<T> | RpcError;
}

interface SubscriptionState {
  id: string;
  reconnectAttempts: number;
  lastEventId?: string;
  completed: boolean;
  unlisten: UnlistenFn | null;
  pendingConsumers: number;
}

const SHUTDOWN_SENTINEL = Symbol("SHUTDOWN");
type QueueItem<T> = SubscriptionEvent<T> | typeof SHUTDOWN_SENTINEL;

// =============================================================================
// Helpers
// =============================================================================

const generateSubscriptionId = Effect.sync(() => {
  if (typeof crypto !== "undefined" && crypto.randomUUID) {
    return crypto.randomUUID();
  }
  return "xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx".replace(/[xy]/g, (c) => {
    const r = (Math.random() * 16) | 0;
    const v = c === "x" ? r : (r & 0x3) | 0x8;
    return v.toString(16);
  });
});

function extractError(error: unknown): RpcError {
  if (error && typeof error === "object" && "_tag" in error) {
    const cause = error as Cause.Cause<unknown>;
    const failure = Cause.failureOption(cause);
    if (failure._tag === "Some") {
      return failure.value as RpcError;
    }
  }

  if (error && typeof error === "object" && "cause" in error) {
    const fiberFailure = error as { cause: Cause.Cause<unknown> };
    const failure = Cause.failureOption(fiberFailure.cause);
    if (failure._tag === "Some") {
      return failure.value as RpcError;
    }
  }

  if (
    error &&
    typeof error === "object" &&
    "code" in error &&
    "message" in error
  ) {
    return error as RpcError;
  }

  if (error instanceof Error) {
    const message = error.message;
    if (message.startsWith("{") && message.includes('"code"')) {
      try {
        const parsed = JSON.parse(message);
        if (parsed && typeof parsed === "object" && "code" in parsed) {
          return parsed as RpcError;
        }
      } catch {
        // Not valid JSON
      }
    }
  }

  return {
    code: "UNKNOWN",
    message: String(error),
  };
}

// =============================================================================
// Connection Effects
// =============================================================================

const connectEffect = <T>(
  stateRef: Ref.Ref<SubscriptionState>,
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
      catch: (error) => makeNetworkError(path, error),
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
        return makeNetworkError(path, error);
      },
    });
  });

const disconnectEffect = <T>(
  stateRef: Ref.Ref<SubscriptionState>,
  eventQueue: Queue.Queue<QueueItem<T>>,
): Effect.Effect<void> =>
  Effect.gen(function* () {
    const state = yield* Ref.get(stateRef);

    if (state.unlisten) {
      state.unlisten();
      yield* Ref.update(stateRef, (s) => ({ ...s, unlisten: null }));
    }

    yield* Ref.update(stateRef, (s) => ({ ...s, completed: true }));

    const sentinelsToSend = Math.max(1, state.pendingConsumers + 1);
    for (let i = 0; i < sentinelsToSend; i++) {
      yield* Queue.offer(eventQueue, SHUTDOWN_SENTINEL);
    }

    yield* pipe(
      Effect.tryPromise(() =>
        invoke("plugin:rpc|rpc_unsubscribe", { id: `sub_${state.id}` }),
      ),
      Effect.catchAll(() => Effect.void),
    );
  });

const reconnectEffect = <T>(
  stateRef: Ref.Ref<SubscriptionState>,
  path: string,
  input: unknown,
  options: SubscriptionOptions,
  eventQueue: Queue.Queue<QueueItem<T>>,
): Effect.Effect<boolean, RpcEffectError> =>
  Effect.gen(function* () {
    const {
      autoReconnect = false,
      maxReconnects = 5,
      reconnectDelay = 1000,
    } = options;

    if (!autoReconnect) {
      return false;
    }

    const state = yield* Ref.get(stateRef);

    if (state.reconnectAttempts >= maxReconnects) {
      yield* Ref.update(stateRef, (s) => ({ ...s, completed: true }));

      const errorEvent: SubscriptionEvent<T> = {
        type: "error",
        payload: {
          code: "MAX_RECONNECTS_EXCEEDED",
          message: `Maximum reconnection attempts (${maxReconnects}) exceeded`,
          details: { attempts: state.reconnectAttempts, maxReconnects, path },
        },
      };

      const sentinelsToSend = Math.max(1, state.pendingConsumers + 1);
      for (let i = 0; i < sentinelsToSend; i++) {
        yield* Queue.offer(eventQueue, errorEvent);
      }

      yield* Effect.fail(
        makeCallError(
          "MAX_RECONNECTS_EXCEEDED",
          `Maximum reconnection attempts (${maxReconnects}) exceeded`,
          { attempts: state.reconnectAttempts, maxReconnects, path },
        ),
      );
      return false;
    }

    yield* Ref.update(stateRef, (s) => ({
      ...s,
      reconnectAttempts: s.reconnectAttempts + 1,
    }));

    const currentState = yield* Ref.get(stateRef);
    const delay =
      reconnectDelay * Math.pow(2, currentState.reconnectAttempts - 1);
    const jitteredDelay = delay * (0.5 + Math.random() * 0.5);

    yield* Effect.sleep(Duration.millis(jitteredDelay));

    const newId = yield* generateSubscriptionId;
    yield* Ref.update(stateRef, (s) => ({
      ...s,
      id: newId,
      completed: false,
    }));

    yield* pipe(
      connectEffect(stateRef, path, input, eventQueue),
      Effect.tap(() =>
        Ref.update(stateRef, (s) => ({ ...s, reconnectAttempts: 0 })),
      ),
      Effect.catchAll(() =>
        reconnectEffect(stateRef, path, input, options, eventQueue),
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

    const stateRef = yield* Ref.make<SubscriptionState>({
      id: subscriptionId,
      reconnectAttempts: 0,
      lastEventId: options.lastEventId,
      completed: false,
      unlisten: null,
      pendingConsumers: 0,
    });

    const eventQueue = yield* Queue.unbounded<QueueItem<T>>();

    yield* connectEffect(stateRef, path, input, eventQueue);

    if (options.signal) {
      options.signal.addEventListener("abort", () => {
        Effect.runPromise(disconnectEffect(stateRef, eventQueue));
      });
    }

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

                  yield* Ref.update(stateRef, (s) => ({
                    ...s,
                    pendingConsumers: s.pendingConsumers + 1,
                  }));

                  const item = yield* Queue.take(eventQueue);

                  yield* Ref.update(stateRef, (s) => ({
                    ...s,
                    pendingConsumers: Math.max(0, s.pendingConsumers - 1),
                  }));

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
                        yield* Ref.update(stateRef, (s) => ({
                          ...s,
                          lastEventId: eventData.id,
                        }));
                      }
                      return { done: false, value: eventData.data };
                    }

                    case "error": {
                      const error = event.payload as RpcError;
                      yield* Ref.update(stateRef, (s) => ({
                        ...s,
                        completed: true,
                      }));

                      if (options.autoReconnect) {
                        const reconnected = yield* pipe(
                          reconnectEffect(
                            stateRef,
                            path,
                            input,
                            options,
                            eventQueue,
                          ),
                          Effect.catchAll(() => Effect.succeed(false)),
                        );
                        if (reconnected) {
                          return yield* Effect.fail(error);
                        }
                      }
                      return yield* Effect.fail(error);
                    }

                    case "completed": {
                      yield* Ref.update(stateRef, (s) => ({
                        ...s,
                        completed: true,
                      }));
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
              throw extractError(error);
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

export interface ConsumeOptions<T> {
  onEvent?: (event: T) => void;
  onError?: (error: RpcError) => void;
  onComplete?: () => void;
  onFinish?: (state: "success" | "error" | "cancelled") => void;
}

/**
 * Consume an event iterator with lifecycle callbacks.
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
