// =============================================================================
// Subscription Reconnection Logic
// =============================================================================

import { Effect, Ref, Duration } from "effect";
import type { RpcEffectError } from "../core/errors";
import { createCallError } from "../core/error-utils";
import type { SubscriptionState, ReconnectConfig } from "./types";
import { incrementReconnectAttempts } from "./state";

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
  createCallError(
    "MAX_RECONNECTS_EXCEEDED",
    `Maximum reconnection attempts (${maxReconnects}) exceeded`,
    { attempts, maxReconnects, path },
  );
