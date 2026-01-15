// =============================================================================
// Timeout - Advanced timeout patterns with cleanup
// =============================================================================

import { Effect, Fiber, Duration, Option } from "effect";
import type { RpcEffectError } from "../core/errors";
import { createTimeoutError } from "../core/error-utils";

// =============================================================================
// Types
// =============================================================================

/**
 * Timeout configuration.
 */
export interface TimeoutConfig {
  /** Timeout duration in milliseconds */
  readonly timeoutMs: number;
  /** Optional cleanup function on timeout */
  readonly onTimeout?: () => Effect.Effect<void>;
}

/**
 * Hedging configuration for speculative execution.
 */
export interface HedgingConfig {
  /** Delay before starting hedge request (ms) */
  readonly hedgeDelay: number;
  /** Maximum number of hedge requests */
  readonly maxHedges: number;
  /** Total timeout for all attempts (ms) */
  readonly totalTimeout: number;
}

// =============================================================================
// Timeout with Cleanup
// =============================================================================

/**
 * Execute an effect with timeout and proper cleanup.
 * Uses Fiber-based timeout for clean interruption.
 */
export const withTimeoutAndCleanup = <A, R>(
  path: string,
  effect: Effect.Effect<A, RpcEffectError, R>,
  config: TimeoutConfig,
): Effect.Effect<A, RpcEffectError, R> =>
  Effect.gen(function* () {
    // Fork the effect
    const fiber = yield* Effect.fork(effect);

    // Race with timeout using timeoutFail for proper error handling
    const result = yield* Fiber.join(fiber).pipe(
      Effect.timeoutFail({
        duration: Duration.millis(config.timeoutMs),
        onTimeout: () => createTimeoutError(path, config.timeoutMs),
      }),
      Effect.catchTag("RpcTimeoutError", (error) =>
        Effect.gen(function* () {
          // Interrupt the fiber on timeout
          yield* Fiber.interrupt(fiber);

          // Run cleanup if provided
          if (config.onTimeout) {
            yield* config.onTimeout();
          }

          return yield* Effect.fail(error);
        }),
      ),
    );

    return result;
  });

// =============================================================================
// Hedging (Speculative Execution)
// =============================================================================

/**
 * Execute an effect with hedging (speculative execution).
 * Starts additional requests if the primary doesn't complete quickly.
 */
export const withHedging = <A, R>(
  path: string,
  effect: Effect.Effect<A, RpcEffectError, R>,
  config: HedgingConfig,
): Effect.Effect<A, RpcEffectError, R> =>
  Effect.gen(function* () {
    const fibers: Fiber.RuntimeFiber<A, RpcEffectError>[] = [];

    // Start primary request
    const primaryFiber = yield* Effect.fork(effect);
    fibers.push(primaryFiber);

    // Create hedge requests with delays
    const hedgeEffects = Array.from({ length: config.maxHedges }, (_, i) =>
      Effect.gen(function* () {
        // Wait for hedge delay
        yield* Effect.sleep(Duration.millis(config.hedgeDelay * (i + 1)));

        // Check if primary already completed
        const primaryStatus = yield* Fiber.poll(primaryFiber);
        if (Option.isSome(primaryStatus)) {
          // Primary completed, fail this hedge to let primary win
          return yield* Effect.fail(
            createTimeoutError(path, 0), // Signal that primary completed
          );
        }

        // Start hedge request
        const hedgeFiber = yield* Effect.fork(effect);
        fibers.push(hedgeFiber);

        return yield* Fiber.join(hedgeFiber);
      }),
    );

    // Race all requests with total timeout
    const result = yield* Effect.raceAll([
      Fiber.join(primaryFiber),
      ...hedgeEffects,
    ]).pipe(
      Effect.timeoutFail({
        duration: Duration.millis(config.totalTimeout),
        onTimeout: () => createTimeoutError(path, config.totalTimeout),
      }),
    );

    // Interrupt all remaining fibers
    yield* Effect.forEach(fibers, (fiber) => Fiber.interrupt(fiber), {
      concurrency: "unbounded",
      discard: true,
    });

    return result;
  });
