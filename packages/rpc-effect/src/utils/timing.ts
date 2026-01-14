// =============================================================================
// Timing Utilities
// =============================================================================

import { Effect, Duration } from "effect";

export const sleep = (ms: number): Effect.Effect<void> =>
  Effect.sleep(Duration.millis(ms));

export const calculateBackoff = (
  attempt: number,
  baseDelay: number = 1000,
  maxDelay: number = 30000,
  jitter: boolean = true,
): Effect.Effect<number> =>
  Effect.sync(() => {
    const exponentialDelay = baseDelay * Math.pow(2, attempt);
    const cappedDelay = Math.min(exponentialDelay, maxDelay);

    if (jitter) {
      return cappedDelay * (0.5 + Math.random() * 0.5);
    }

    return cappedDelay;
  });
