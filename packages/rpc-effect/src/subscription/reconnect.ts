// =============================================================================
// Subscription Reconnection Logic
// =============================================================================

import { Effect, Ref, Duration, Schedule } from "effect";
import type { RpcEffectError } from "../core/errors";
import { createCallError } from "../core/error-utils";
import type { SubscriptionState, ReconnectConfig } from "./types";
import { incrementReconnectAttempts } from "./state";

// =============================================================================
// Schedule-Based Reconnection
// =============================================================================

/**
 * Create a reconnection schedule with exponential backoff and jitter.
 * Uses Effect's Schedule for idiomatic retry behavior.
 */
export const createReconnectSchedule = (config: ReconnectConfig) =>
  Schedule.exponential(Duration.millis(config.reconnectDelay)).pipe(
    Schedule.jittered,
    Schedule.intersect(Schedule.recurs(config.maxReconnects)),
  );

/**
 * Calculate reconnection delay with exponential backoff and jitter.
 * @deprecated Use createReconnectSchedule for idiomatic Effect usage
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
  createCallError(
    "MAX_RECONNECTS_EXCEEDED",
    `Maximum reconnection attempts (${maxReconnects}) exceeded`,
    { attempts, maxReconnects, path },
  );

/**
 * Execute an effect with automatic reconnection using Schedule.
 * This is the idiomatic way to handle reconnection in Effect.
 */
export const withReconnection = <A, E extends RpcEffectError>(
  effect: Effect.Effect<A, E>,
  config: ReconnectConfig,
): Effect.Effect<A, E> => {
  if (!config.autoReconnect) return effect;

  return effect.pipe(Effect.retry(createReconnectSchedule(config)));
};
