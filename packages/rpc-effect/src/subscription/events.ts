// =============================================================================
// Subscription Event Processing
// =============================================================================

import { Effect, Ref, Queue } from "effect";
import type { Event } from "../core/types";
import type { RpcEffectError } from "../core/errors";
import { createCallError } from "../core/error-utils";
import type {
  SubscriptionState,
  SubscriptionError,
  ReconnectConfig,
  QueueItem,
} from "./types";
import { markCompleted, updateLastEventId } from "./state";
import { shouldReconnect, maxReconnectsExceededError } from "./reconnect";

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
      error: createCallError(error.code, error.message, error.details),
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
  if (
    error &&
    typeof error === "object" &&
    "code" in error &&
    "message" in error
  ) {
    const e = error as { code: string; message: string; details?: unknown };
    return { code: e.code, message: e.message, details: e.details };
  }

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
