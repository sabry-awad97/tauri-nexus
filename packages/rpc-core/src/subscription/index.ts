// =============================================================================
// @tauri-nexus/rpc-core - Subscription Module
// =============================================================================
// Event iterator implementation for Tauri subscriptions.
// Uses Effect-idiomatic primitives from rpc-effect for streaming and state.

import { Effect, Ref, Queue } from "effect";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
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
  type AsyncIteratorConfig,
  // State management
  createEventQueue,
  markCompleted,
  resetForReconnect,
  resetReconnectAttempts,
  // Queue operations
  sendShutdownSentinels,
  // Utilities
  generateSubscriptionId,
  // Errors
  createNetworkError,
  // Stream utilities
  createAsyncIterator,
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

/**
 * Subscribe to the Tauri RPC backend.
 */
const subscribeToBackend = (
  request: SubscribeRequest,
  path: string,
  cleanup: () => void
): Effect.Effect<void, RpcEffectError> =>
  Effect.tryPromise({
    try: () => invoke("plugin:rpc|rpc_subscribe", { request }),
    catch: (error) => {
      cleanup();
      return createNetworkError(path, error);
    },
  });

/**
 * Unsubscribe from the Tauri RPC backend.
 */
const unsubscribeFromBackend = (id: string): Effect.Effect<void> =>
  Effect.tryPromise(() =>
    invoke("plugin:rpc|rpc_unsubscribe", { id: `sub_${id}` })
  ).pipe(Effect.catchAll(() => Effect.void));

// =============================================================================
// Connection Management
// =============================================================================

/**
 * Create connection effect for a subscription.
 */
const createConnectEffect = <T>(
  stateRef: Ref.Ref<TauriSubscriptionState>,
  path: string,
  input: unknown,
  eventQueue: Queue.Queue<QueueItem<T>>
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

    yield* subscribeToBackend(request, path, unlisten);
  });

/**
 * Create disconnect effect for a subscription.
 */
const createDisconnectEffect = <T>(
  stateRef: Ref.Ref<TauriSubscriptionState>,
  eventQueue: Queue.Queue<QueueItem<T>>
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

    yield* unsubscribeFromBackend(state.id);
  });

/**
 * Create reconnect effect for a subscription.
 */
const createReconnectEffect =
  <T>(
    stateRef: Ref.Ref<TauriSubscriptionState>,
    path: string,
    input: unknown,
    eventQueue: Queue.Queue<QueueItem<T>>
  ) =>
  (newId: string): Effect.Effect<void, RpcEffectError> =>
    Effect.gen(function* () {
      yield* resetForReconnect(stateRef, newId);
      yield* Ref.update(stateRef, (s) => ({ ...s, unlisten: null }));

      yield* createConnectEffect(stateRef, path, input, eventQueue);
      yield* resetReconnectAttempts(stateRef);
    });

// =============================================================================
// Event Iterator Factory
// =============================================================================

/**
 * Create an event iterator using Effect-idiomatic primitives.
 */
const createEventIteratorEffect = <T>(
  path: string,
  input: unknown = null,
  options: SubscriptionOptions = {}
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

    // Initial connection
    yield* createConnectEffect(stateRef, path, input, eventQueue);

    // Setup abort signal handler
    if (options.signal) {
      options.signal.addEventListener("abort", () => {
        Effect.runPromise(createDisconnectEffect(stateRef, eventQueue));
      });
    }

    const reconnectConfig: ReconnectConfig = {
      autoReconnect: options.autoReconnect ?? false,
      maxReconnects: options.maxReconnects ?? 5,
      reconnectDelay: options.reconnectDelay ?? 1000,
    };

    const disconnect = createDisconnectEffect(stateRef, eventQueue);
    const reconnect = createReconnectEffect(stateRef, path, input, eventQueue);

    // Create async iterator config
    const iteratorConfig: AsyncIteratorConfig<T, TauriSubscriptionState> = {
      stateRef,
      eventQueue,
      disconnect,
      reconnectConfig,
      path,
      reconnect,
    };

    // Build the EventIterator using the Effect-based async iterator
    const iterator: EventIterator<T> = {
      async return(): Promise<void> {
        await Effect.runPromise(disconnect);
      },

      [Symbol.asyncIterator](): AsyncIterator<T> {
        return createAsyncIterator(iteratorConfig);
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
  options: SubscriptionOptions = {}
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
  options: ConsumeOptions<T>
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
