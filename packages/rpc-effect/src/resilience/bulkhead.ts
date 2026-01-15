// =============================================================================
// Bulkhead - Concurrency limiting pattern
// =============================================================================

import { Effect, Context, Layer, Data, Duration } from "effect";

// =============================================================================
// Types
// =============================================================================

/**
 * Bulkhead configuration.
 */
export interface BulkheadConfig {
  /** Maximum concurrent executions */
  readonly maxConcurrent: number;
  /** Maximum queue size for waiting requests */
  readonly maxQueue: number;
  /** Timeout for waiting in queue (ms) */
  readonly queueTimeout: number;
}

// =============================================================================
// Errors
// =============================================================================

/**
 * Error thrown when bulkhead is full.
 */
export class BulkheadFullError extends Data.TaggedError("BulkheadFullError")<{
  readonly path: string;
  readonly maxConcurrent: number;
  readonly maxQueue: number;
}> {
  get message(): string {
    return `Bulkhead full for ${this.path}. Max concurrent: ${this.maxConcurrent}, Max queue: ${this.maxQueue}`;
  }
}

// =============================================================================
// Default Configuration
// =============================================================================

const defaultConfig: BulkheadConfig = {
  maxConcurrent: 10,
  maxQueue: 100,
  queueTimeout: 30000,
};

// =============================================================================
// Bulkhead Service
// =============================================================================

/**
 * Bulkhead service interface.
 */
export interface BulkheadServiceInterface {
  readonly semaphore: Effect.Semaphore;
  readonly config: BulkheadConfig;
}

/**
 * Bulkhead service.
 */
export class BulkheadService extends Context.Tag("BulkheadService")<
  BulkheadService,
  BulkheadServiceInterface
>() {}

// =============================================================================
// Bulkhead Creation
// =============================================================================

/**
 * Create a bulkhead instance.
 */
export const createBulkhead = (
  config: Partial<BulkheadConfig> = {},
): Effect.Effect<BulkheadServiceInterface> =>
  Effect.gen(function* () {
    const fullConfig = { ...defaultConfig, ...config };
    const semaphore = yield* Effect.makeSemaphore(fullConfig.maxConcurrent);
    return { semaphore, config: fullConfig };
  });

/**
 * Create a bulkhead layer.
 */
export const createBulkheadLayer = (
  config: Partial<BulkheadConfig> = {},
): Layer.Layer<BulkheadService> =>
  Layer.effect(BulkheadService, createBulkhead(config));

// =============================================================================
// Bulkhead Combinator
// =============================================================================

/**
 * Wrap an effect with bulkhead protection.
 */
export const withBulkhead = <A, E, R>(
  path: string,
  effect: Effect.Effect<A, E, R>,
): Effect.Effect<A, E | BulkheadFullError, R | BulkheadService> =>
  Effect.gen(function* () {
    const { semaphore, config } = yield* BulkheadService;

    // Try to acquire permit with timeout
    const result = yield* semaphore
      .withPermits(1)(effect)
      .pipe(
        Effect.timeoutFail({
          duration: Duration.millis(config.queueTimeout),
          onTimeout: () =>
            new BulkheadFullError({
              path,
              maxConcurrent: config.maxConcurrent,
              maxQueue: config.maxQueue,
            }),
        }),
      );

    return result;
  });
