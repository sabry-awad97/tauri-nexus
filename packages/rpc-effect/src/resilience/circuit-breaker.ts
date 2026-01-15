// =============================================================================
// Circuit Breaker - Resilience pattern implementation
// =============================================================================

import { Effect, Context, Layer, Ref, Data } from "effect";
import type { RpcEffectError } from "../core/errors";
import { isRetryableError } from "../core/error-utils";

// =============================================================================
// Types
// =============================================================================

/**
 * Circuit breaker states.
 */
export type CircuitState = "closed" | "open" | "half-open";

/**
 * Circuit breaker configuration.
 */
export interface CircuitBreakerConfig {
  /** Number of failures before opening the circuit */
  readonly failureThreshold: number;
  /** Time to wait before transitioning from open to half-open (ms) */
  readonly resetTimeout: number;
  /** Number of successful calls in half-open to close the circuit */
  readonly successThreshold: number;
  /** Time window for counting failures (ms) */
  readonly failureWindow: number;
  /** Function to determine if an error should count as a failure */
  readonly isFailure: (error: RpcEffectError) => boolean;
}

/**
 * Circuit breaker internal state.
 */
export interface CircuitBreakerState {
  readonly state: CircuitState;
  readonly failures: number;
  readonly successes: number;
  readonly lastFailureTime: number;
  readonly lastStateChange: number;
}

// =============================================================================
// Errors
// =============================================================================

/**
 * Error thrown when circuit is open.
 */
export class CircuitOpenError extends Data.TaggedError("CircuitOpenError")<{
  readonly path: string;
  readonly remainingTime: number;
}> {
  get message(): string {
    return `Circuit breaker is open for ${this.path}. Retry after ${this.remainingTime}ms`;
  }
}

// =============================================================================
// Default Configuration
// =============================================================================

const defaultConfig: CircuitBreakerConfig = {
  failureThreshold: 5,
  resetTimeout: 30000,
  successThreshold: 3,
  failureWindow: 60000,
  isFailure: isRetryableError,
};

// =============================================================================
// Circuit Breaker Service
// =============================================================================

/**
 * Circuit breaker service interface.
 */
export interface CircuitBreakerServiceInterface {
  readonly stateRef: Ref.Ref<CircuitBreakerState>;
  readonly config: CircuitBreakerConfig;
}

/**
 * Circuit breaker service.
 */
export class CircuitBreakerService extends Context.Tag("CircuitBreakerService")<
  CircuitBreakerService,
  CircuitBreakerServiceInterface
>() {}

// =============================================================================
// State Management
// =============================================================================

const initialState: CircuitBreakerState = {
  state: "closed",
  failures: 0,
  successes: 0,
  lastFailureTime: 0,
  lastStateChange: Date.now(),
};

/**
 * Create a circuit breaker instance.
 */
export const createCircuitBreaker = (
  config: Partial<CircuitBreakerConfig> = {},
): Effect.Effect<CircuitBreakerServiceInterface> =>
  Effect.gen(function* () {
    const fullConfig = { ...defaultConfig, ...config };
    const stateRef = yield* Ref.make(initialState);
    return { stateRef, config: fullConfig };
  });

/**
 * Create a circuit breaker layer.
 */
export const createCircuitBreakerLayer = (
  config: Partial<CircuitBreakerConfig> = {},
): Layer.Layer<CircuitBreakerService> =>
  Layer.effect(CircuitBreakerService, createCircuitBreaker(config));

// =============================================================================
// State Transitions
// =============================================================================

const recordFailure = (
  stateRef: Ref.Ref<CircuitBreakerState>,
  config: CircuitBreakerConfig,
): Effect.Effect<void> =>
  Ref.update(stateRef, (state) => {
    const now = Date.now();

    // Reset failures if outside the failure window
    const failures =
      now - state.lastFailureTime > config.failureWindow
        ? 1
        : state.failures + 1;

    // Transition to open if threshold exceeded
    if (failures >= config.failureThreshold && state.state === "closed") {
      return {
        ...state,
        state: "open" as const,
        failures,
        successes: 0,
        lastFailureTime: now,
        lastStateChange: now,
      };
    }

    // In half-open, any failure reopens the circuit
    if (state.state === "half-open") {
      return {
        ...state,
        state: "open" as const,
        failures: 1,
        successes: 0,
        lastFailureTime: now,
        lastStateChange: now,
      };
    }

    return {
      ...state,
      failures,
      lastFailureTime: now,
    };
  });

const recordSuccess = (
  stateRef: Ref.Ref<CircuitBreakerState>,
  config: CircuitBreakerConfig,
): Effect.Effect<void> =>
  Ref.update(stateRef, (state) => {
    const now = Date.now();

    // In half-open, count successes toward closing
    if (state.state === "half-open") {
      const successes = state.successes + 1;
      if (successes >= config.successThreshold) {
        return {
          ...state,
          state: "closed" as const,
          failures: 0,
          successes: 0,
          lastStateChange: now,
        };
      }
      return { ...state, successes };
    }

    // In closed state, reset failure count on success
    return {
      ...state,
      failures: Math.max(0, state.failures - 1),
    };
  });

const tryTransitionToHalfOpen = (
  stateRef: Ref.Ref<CircuitBreakerState>,
  config: CircuitBreakerConfig,
): Effect.Effect<boolean> =>
  Ref.modify(stateRef, (state) => {
    const now = Date.now();

    if (
      state.state === "open" &&
      now - state.lastStateChange >= config.resetTimeout
    ) {
      return [
        true,
        {
          ...state,
          state: "half-open" as const,
          successes: 0,
          lastStateChange: now,
        },
      ];
    }

    return [false, state];
  });

// =============================================================================
// Circuit Breaker Combinator
// =============================================================================

/**
 * Check if an error should count as a circuit breaker failure.
 */
const isCircuitBreakerFailure = (
  error: unknown,
  config: CircuitBreakerConfig,
): boolean => {
  // Check if it's an RpcEffectError
  if (error && typeof error === "object" && "_tag" in error) {
    const tagged = error as { _tag: string };
    // Use the config's isFailure function if the error looks like RpcEffectError
    if (
      tagged._tag === "RpcCallError" ||
      tagged._tag === "RpcTimeoutError" ||
      tagged._tag === "RpcNetworkError"
    ) {
      return config.isFailure(error as RpcEffectError);
    }
  }
  // For other errors, consider them failures
  return true;
};

/**
 * Wrap an effect with circuit breaker protection.
 */
export const withCircuitBreaker = <A, E, R>(
  path: string,
  effect: Effect.Effect<A, E, R>,
): Effect.Effect<A, E | CircuitOpenError, R | CircuitBreakerService> =>
  Effect.gen(function* () {
    const { stateRef, config } = yield* CircuitBreakerService;
    const state = yield* Ref.get(stateRef);

    // Check if circuit is open
    if (state.state === "open") {
      const elapsed = Date.now() - state.lastStateChange;
      const remaining = config.resetTimeout - elapsed;

      if (remaining > 0) {
        return yield* Effect.fail(
          new CircuitOpenError({ path, remainingTime: remaining }),
        );
      }

      // Try to transition to half-open
      yield* tryTransitionToHalfOpen(stateRef, config);
    }

    // Execute the effect
    return yield* effect.pipe(
      Effect.tap(() => recordSuccess(stateRef, config)),
      Effect.tapError((error) => {
        if (isCircuitBreakerFailure(error, config)) {
          return recordFailure(stateRef, config);
        }
        return Effect.void;
      }),
    );
  });

// =============================================================================
// Utility Functions
// =============================================================================

/**
 * Get the current circuit state.
 */
export const getCircuitState: Effect.Effect<
  CircuitBreakerState,
  never,
  CircuitBreakerService
> = Effect.gen(function* () {
  const { stateRef } = yield* CircuitBreakerService;
  return yield* Ref.get(stateRef);
});

/**
 * Manually reset the circuit breaker.
 */
export const resetCircuit: Effect.Effect<void, never, CircuitBreakerService> =
  Effect.gen(function* () {
    const { stateRef } = yield* CircuitBreakerService;
    yield* Ref.set(stateRef, initialState);
  });
