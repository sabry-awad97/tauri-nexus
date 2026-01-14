// =============================================================================
// Subscription Stream - Effect-idiomatic streaming primitives
// =============================================================================

import {
  Effect,
  Stream,
  Ref,
  Queue,
  Scope,
  Option,
  Fiber,
  Cause,
  PubSub,
} from "effect";
import type { RpcEffectError } from "../core/errors";
import { createCallError } from "../core/error-utils";
import {
  type SubscriptionState,
  type SubscriptionEvent,
  type SubscriptionError,
  type ReconnectConfig,
  type QueueItem,
  SHUTDOWN_SENTINEL,
} from "./types";
import {
  markCompleted,
  updateLastEventId,
  incrementConsumers,
  decrementConsumers,
} from "./state";
import { sendShutdownSentinels } from "./queue";
import {
  shouldReconnect,
  prepareReconnect,
  waitForReconnect,
  maxReconnectsExceededError,
} from "./reconnect";
import { extractSubscriptionError } from "./events";
import type { Event } from "../core/types";

// =============================================================================
// Stream Creation
// =============================================================================

/**
 * Configuration for creating a subscription stream.
 */
export interface SubscriptionStreamConfig<T, S extends SubscriptionState> {
  readonly stateRef: Ref.Ref<S>;
  readonly eventQueue: Queue.Queue<QueueItem<T>>;
  readonly path: string;
  readonly reconnectConfig: ReconnectConfig;
  readonly connect: Effect.Effect<void, RpcEffectError>;
  readonly disconnect: Effect.Effect<void>;
  readonly reconnect: (newId: string) => Effect.Effect<void, RpcEffectError>;
}

/**
 * Process a single queue item and return the data or signal completion/error.
 */
const processQueueItem = <T, S extends SubscriptionState>(
  item: QueueItem<T>,
  config: SubscriptionStreamConfig<T, S>,
): Effect.Effect<Option.Option<T>, RpcEffectError> =>
  Effect.gen(function* () {
    if (item === SHUTDOWN_SENTINEL) {
      return Option.none();
    }

    const event = item as SubscriptionEvent<T>;

    switch (event.type) {
      case "data": {
        const eventData = event.payload as Event<T>;
        if (eventData.id) {
          yield* updateLastEventId(config.stateRef, eventData.id);
        }
        return Option.some(eventData.data);
      }

      case "error": {
        yield* markCompleted(config.stateRef);

        const canReconnect = yield* shouldReconnect(
          config.stateRef,
          config.reconnectConfig,
        );

        if (canReconnect) {
          yield* handleReconnection(config);
          // After reconnection, signal to continue streaming
          return Option.none();
        }

        const state = yield* Ref.get(config.stateRef);
        return yield* Effect.fail(
          maxReconnectsExceededError(
            config.path,
            state.reconnectAttempts,
            config.reconnectConfig.maxReconnects,
          ),
        );
      }

      case "completed": {
        yield* markCompleted(config.stateRef);
        return Option.none();
      }

      default:
        return Option.none();
    }
  });

/**
 * Handle reconnection logic.
 */
const handleReconnection = <T, S extends SubscriptionState>(
  config: SubscriptionStreamConfig<T, S>,
): Effect.Effect<void, RpcEffectError> =>
  Effect.gen(function* () {
    const delay = yield* prepareReconnect(
      config.stateRef,
      config.reconnectConfig,
    );
    yield* waitForReconnect(delay);

    const newId = yield* Effect.sync(() => {
      if (typeof crypto !== "undefined" && crypto.randomUUID) {
        return crypto.randomUUID();
      }
      return "xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx".replace(/[xy]/g, (c) => {
        const r = (Math.random() * 16) | 0;
        const v = c === "x" ? r : (r & 0x3) | 0x8;
        return v.toString(16);
      });
    });

    yield* config.reconnect(newId);
  });

/**
 * Create a subscription stream from queue events.
 *
 * This is the core Effect-idiomatic streaming primitive. It:
 * - Takes items from the event queue
 * - Processes data/error/completed events
 * - Handles reconnection automatically
 * - Properly manages consumer count for graceful shutdown
 */
export const createSubscriptionStream = <T, S extends SubscriptionState>(
  config: SubscriptionStreamConfig<T, S>,
): Stream.Stream<T, RpcEffectError> =>
  Stream.unwrap(
    Effect.gen(function* () {
      return Stream.repeatEffectOption(
        Effect.gen(function* () {
          const state = yield* Ref.get(config.stateRef);

          if (state.completed) {
            return yield* Effect.fail(Option.none());
          }

          yield* incrementConsumers(config.stateRef);
          const item = yield* Queue.take(config.eventQueue);
          yield* decrementConsumers(config.stateRef);

          const result = yield* processQueueItem(item, config).pipe(
            Effect.mapError(Option.some),
          );

          if (Option.isNone(result)) {
            // Check if we should continue (reconnected) or stop
            const newState = yield* Ref.get(config.stateRef);
            if (newState.completed) {
              return yield* Effect.fail(Option.none());
            }
            // Reconnected, continue streaming by recursing
            return yield* Effect.fail(Option.none());
          }

          return result.value;
        }),
      );
    }),
  );

// =============================================================================
// Scoped Connection
// =============================================================================

/**
 * Create a scoped connection that automatically disconnects on scope close.
 *
 * This uses Effect's resource management to ensure cleanup happens
 * even if the stream is interrupted or fails.
 */
export const scopedConnection = <T, S extends SubscriptionState>(
  config: SubscriptionStreamConfig<T, S>,
): Effect.Effect<void, RpcEffectError, Scope.Scope> =>
  Effect.acquireRelease(config.connect, () => config.disconnect);

/**
 * Create a fully managed subscription stream with automatic resource cleanup.
 *
 * This combines:
 * - Scoped connection (auto-disconnect on completion/error/interruption)
 * - Event streaming with reconnection support
 * - Proper consumer tracking
 */
export const createManagedSubscriptionStream = <T, S extends SubscriptionState>(
  config: SubscriptionStreamConfig<T, S>,
): Stream.Stream<T, RpcEffectError> =>
  Stream.unwrapScoped(
    Effect.gen(function* () {
      yield* scopedConnection(config);
      return createSubscriptionStream(config);
    }),
  );

// =============================================================================
// Stream Utilities
// =============================================================================

/**
 * Run a stream and collect all values into an array.
 * Useful for testing or when you need all values at once.
 */
export const collectStream = <T, E>(
  stream: Stream.Stream<T, E>,
): Effect.Effect<T[], E> =>
  Stream.runCollect(stream).pipe(Effect.map((chunk) => [...chunk]));

/**
 * Run a stream with callbacks for each event.
 * Returns an Effect that completes when the stream ends.
 */
export const runStreamWithCallbacks = <T, E>(
  stream: Stream.Stream<T, E>,
  onData: (value: T) => void,
  onError?: (error: E) => void,
  onComplete?: () => void,
): Effect.Effect<void, E> =>
  stream.pipe(
    Stream.tap((value) => Effect.sync(() => onData(value))),
    Stream.runDrain,
    Effect.tapError((error) =>
      Effect.sync(() => {
        onError?.(error);
      }),
    ),
    Effect.tap(() => Effect.sync(() => onComplete?.())),
    // Use ensuring for guaranteed cleanup
    Effect.ensuring(Effect.sync(() => onComplete?.())),
  );

/**
 * Create an interruptible stream runner.
 * Returns a fiber that can be interrupted to stop the stream.
 */
export const runStreamInterruptible = <T, E>(
  stream: Stream.Stream<T, E>,
  onData: (value: T) => void,
  onError?: (error: E) => void,
  onComplete?: () => void,
): Effect.Effect<Fiber.RuntimeFiber<void, E>> =>
  Effect.fork(runStreamWithCallbacks(stream, onData, onError, onComplete));

// =============================================================================
// AsyncIterator Conversion
// =============================================================================

/**
 * Configuration for converting a stream to an AsyncIterator.
 */
export interface AsyncIteratorConfig<T, S extends SubscriptionState> {
  readonly stateRef: Ref.Ref<S>;
  readonly eventQueue: Queue.Queue<QueueItem<T>>;
  readonly disconnect: Effect.Effect<void>;
  readonly reconnectConfig: ReconnectConfig;
  readonly path: string;
  readonly reconnect: (newId: string) => Effect.Effect<void, RpcEffectError>;
}

// =============================================================================
// Cause Utilities
// =============================================================================

/**
 * Extract error from Cause using idiomatic Effect utilities.
 */
const extractErrorFromCause = (
  cause: Cause.Cause<RpcEffectError>,
): RpcEffectError | null => Option.getOrNull(Cause.failureOption(cause));

/**
 * Convert RpcEffectError to plain SubscriptionError for throwing.
 */
const toSubscriptionError = (error: RpcEffectError): SubscriptionError => ({
  code: error._tag === "RpcCallError" ? error.code : error._tag,
  message: error.message,
  details: "details" in error ? error.details : undefined,
});

/**
 * Create an AsyncIterator from subscription configuration.
 *
 * This bridges the Effect world to the Promise/AsyncIterator world
 * for consumers who don't want to use Effect directly.
 */
export const createAsyncIterator = <T, S extends SubscriptionState>(
  config: AsyncIteratorConfig<T, S>,
): AsyncIterator<T> => {
  const processNext = (): Effect.Effect<IteratorResult<T>, RpcEffectError> =>
    Effect.gen(function* () {
      const state = yield* Ref.get(config.stateRef);

      if (state.completed) {
        return { done: true, value: undefined } as IteratorResult<T>;
      }

      yield* incrementConsumers(config.stateRef);
      const item = yield* Queue.take(config.eventQueue);
      yield* decrementConsumers(config.stateRef);

      if (item === SHUTDOWN_SENTINEL) {
        return { done: true, value: undefined } as IteratorResult<T>;
      }

      const event = item as SubscriptionEvent<T>;

      switch (event.type) {
        case "data": {
          const eventData = event.payload as Event<T>;
          if (eventData.id) {
            yield* updateLastEventId(config.stateRef, eventData.id);
          }
          return { done: false, value: eventData.data };
        }

        case "error": {
          const errorPayload = extractSubscriptionError(event.payload);
          yield* markCompleted(config.stateRef);

          const canReconnect = yield* shouldReconnect(
            config.stateRef,
            config.reconnectConfig,
          );

          if (canReconnect) {
            const delay = yield* prepareReconnect(
              config.stateRef,
              config.reconnectConfig,
            );
            yield* waitForReconnect(delay);

            const newId = yield* Effect.sync(() => {
              if (typeof crypto !== "undefined" && crypto.randomUUID) {
                return crypto.randomUUID();
              }
              return "xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx".replace(
                /[xy]/g,
                (c) => {
                  const r = (Math.random() * 16) | 0;
                  const v = c === "x" ? r : (r & 0x3) | 0x8;
                  return v.toString(16);
                },
              );
            });

            // Try to reconnect with idiomatic error handling
            const sendErrorToPendingConsumers = (
              code: string,
              message: string,
              details?: unknown,
            ) =>
              Effect.gen(function* () {
                const state = yield* Ref.get(config.stateRef);
                const errorEvent: SubscriptionEvent<T> = {
                  type: "error",
                  payload: { code, message, details },
                };
                for (let i = 0; i < state.pendingConsumers + 5; i++) {
                  yield* Queue.offer(config.eventQueue, errorEvent);
                }
              });

            const reconnectResult = yield* config.reconnect(newId).pipe(
              Effect.map(() => true),
              // Handle RpcCallError specifically - preserve the original error code
              Effect.catchTag("RpcCallError", (error) =>
                sendErrorToPendingConsumers(
                  error.code,
                  error.message,
                  error.details,
                ).pipe(Effect.flatMap(() => Effect.fail(error))),
              ),
              // Handle all other RPC errors with their tag as the code
              Effect.catchAll((error) =>
                sendErrorToPendingConsumers(
                  error._tag,
                  error.message,
                  "details" in error ? error.details : undefined,
                ).pipe(Effect.flatMap(() => Effect.fail(error))),
              ),
            );

            if (reconnectResult) {
              // Continue iteration after reconnection
              return yield* processNext();
            }
          }

          // If autoReconnect is disabled, throw the original error
          if (!config.reconnectConfig.autoReconnect) {
            // Send shutdown sentinels to pending consumers before failing
            yield* sendShutdownSentinels(config.eventQueue, 10);
            return yield* Effect.fail(
              createCallError(
                errorPayload.code,
                errorPayload.message,
                errorPayload.details,
              ),
            );
          }

          // Max reconnects exceeded - send shutdown sentinels to pending consumers
          yield* sendShutdownSentinels(config.eventQueue, 10);
          const currentState = yield* Ref.get(config.stateRef);
          return yield* Effect.fail(
            maxReconnectsExceededError(
              config.path,
              currentState.reconnectAttempts,
              config.reconnectConfig.maxReconnects,
            ),
          );
        }

        case "completed": {
          yield* markCompleted(config.stateRef);
          return { done: true, value: undefined } as IteratorResult<T>;
        }

        default:
          return { done: true, value: undefined } as IteratorResult<T>;
      }
    });

  return {
    async next(): Promise<IteratorResult<T>> {
      const exit = await Effect.runPromiseExit(processNext());
      if (exit._tag === "Success") {
        return exit.value;
      }
      // Extract error from Cause using idiomatic utilities
      const error = extractErrorFromCause(exit.cause);
      if (error) {
        throw toSubscriptionError(error);
      }
      throw { code: "UNKNOWN", message: "Unknown error" };
    },

    async return(): Promise<IteratorResult<T>> {
      await Effect.runPromise(config.disconnect);
      return { done: true, value: undefined };
    },
  };
};

// =============================================================================
// Resource Management - acquireUseRelease Pattern
// =============================================================================

/**
 * Execute a function with a managed subscription iterator.
 * Uses Effect.acquireUseRelease for idiomatic resource management.
 */
export const withSubscription = <T, A, S extends SubscriptionState>(
  config: AsyncIteratorConfig<T, S>,
  use: (iterator: AsyncIterator<T>) => Effect.Effect<A, RpcEffectError>,
): Effect.Effect<A, RpcEffectError> =>
  Effect.acquireUseRelease(
    Effect.sync(() => createAsyncIterator(config)),
    use,
    () => config.disconnect,
  );

// =============================================================================
// PubSub for Multi-Consumer Subscriptions
// =============================================================================

/**
 * Broadcast subscription for multiple consumers.
 * Uses Effect's PubSub for idiomatic multi-consumer patterns.
 */
export interface BroadcastSubscription<T> {
  readonly publish: (
    event: SubscriptionEvent<T>,
  ) => Effect.Effect<boolean, never>;
  readonly subscribe: () => Effect.Effect<
    Queue.Dequeue<SubscriptionEvent<T>>,
    never,
    Scope.Scope
  >;
  readonly subscriberCount: Effect.Effect<number, never>;
}

/**
 * Create a broadcast subscription that can have multiple consumers.
 * Each consumer receives all events published after they subscribe.
 */
export const createBroadcastSubscription = <T>(): Effect.Effect<
  BroadcastSubscription<T>
> =>
  Effect.gen(function* () {
    const pubsub = yield* PubSub.unbounded<SubscriptionEvent<T>>();

    return {
      publish: (event: SubscriptionEvent<T>) => PubSub.publish(pubsub, event),
      subscribe: () => PubSub.subscribe(pubsub),
      subscriberCount: PubSub.size(pubsub),
    };
  });

/**
 * Create a scoped broadcast subscription with automatic cleanup.
 */
export const createScopedBroadcastSubscription = <T>(): Effect.Effect<
  BroadcastSubscription<T>,
  never,
  Scope.Scope
> =>
  Effect.acquireRelease(
    createBroadcastSubscription<T>(),
    () =>
      // PubSub cleanup is automatic when scope closes
      Effect.void,
  );

// =============================================================================
// Stream from AsyncIterable (asyncScoped pattern)
// =============================================================================

/**
 * Create a stream from an async iterable source.
 * Uses Stream.fromAsyncIterable for idiomatic event source handling.
 */
export const createEventSourceStream = <T>(
  eventSource: Effect.Effect<AsyncIterable<T>, RpcEffectError>,
): Stream.Stream<T, RpcEffectError> =>
  Stream.unwrap(
    eventSource.pipe(
      Effect.map((source) =>
        Stream.fromAsyncIterable(source, (error) =>
          createCallError(
            "STREAM_ERROR",
            error instanceof Error ? error.message : String(error),
          ),
        ),
      ),
    ),
  );
