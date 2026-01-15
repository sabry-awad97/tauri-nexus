// =============================================================================
// Metrics - Effect Metric-based Observability
// =============================================================================

import { Effect, Metric, Context, Layer } from "effect";
import { defaultLatencyBoundaries, createMetricName } from "./config";

// =============================================================================
// Metric Tags
// =============================================================================

/**
 * Standard metric tags for RPC calls.
 */
export interface MetricTags {
  readonly path: string;
  readonly type: string;
  readonly status: "success" | "error";
  readonly errorCode?: string;
}

/**
 * Create metric tags from call context.
 */
export const createMetricTags = (
  path: string,
  type: string,
  status: "success" | "error",
  errorCode?: string,
): MetricTags => ({
  path,
  type,
  status,
  errorCode,
});

// =============================================================================
// Core Metrics
// =============================================================================

/**
 * Counter for total RPC calls.
 */
export const rpcCallCounter = Metric.counter(createMetricName("calls.total"), {
  description: "Total number of RPC calls",
  incremental: true,
});

/**
 * Counter for RPC errors.
 */
export const rpcErrorCounter = Metric.counter(
  createMetricName("errors.total"),
  {
    description: "Total number of RPC errors",
    incremental: true,
  },
);

/**
 * Histogram for RPC latency.
 */
export const rpcLatencyHistogram = Metric.histogram(
  createMetricName("latency.ms"),
  defaultLatencyBoundaries,
);

/**
 * Gauge for active RPC calls.
 */
export const rpcActiveCallsGauge = Metric.gauge(
  createMetricName("calls.active"),
);

/**
 * Counter for retry attempts.
 */
export const rpcRetryCounter = Metric.counter(
  createMetricName("retries.total"),
  {
    description: "Total number of retry attempts",
    incremental: true,
  },
);

/**
 * Counter for cache hits.
 */
export const rpcCacheHitCounter = Metric.counter(
  createMetricName("cache.hits"),
  {
    description: "Total number of cache hits",
    incremental: true,
  },
);

/**
 * Counter for cache misses.
 */
export const rpcCacheMissCounter = Metric.counter(
  createMetricName("cache.misses"),
  {
    description: "Total number of cache misses",
    incremental: true,
  },
);

// =============================================================================
// Metric Combinators
// =============================================================================

/**
 * Get error code from any error type.
 */
const getErrorCodeSafe = (error: unknown): string => {
  if (error && typeof error === "object" && "_tag" in error) {
    const tagged = error as { _tag: string; code?: string };
    if (tagged._tag === "RpcCallError" && tagged.code) {
      return tagged.code;
    }
    return tagged._tag;
  }
  return "UNKNOWN";
};

/**
 * Wrap an effect with full metrics tracking.
 */
export const withMetrics = <A, E, R>(
  path: string,
  type: string,
  effect: Effect.Effect<A, E, R>,
): Effect.Effect<A, E, R> => {
  // Create tagged metrics for this specific call
  const taggedCallCounter = rpcCallCounter.pipe(
    Metric.tagged("path", path),
    Metric.tagged("type", type),
  );

  const taggedLatencyHistogram = rpcLatencyHistogram.pipe(
    Metric.tagged("path", path),
  );

  return Effect.gen(function* () {
    const start = Date.now();

    // Track active calls
    yield* Effect.sync(() => rpcActiveCallsGauge.unsafeUpdate(1, []));

    const result = yield* effect.pipe(
      Effect.tap(() => {
        const duration = Date.now() - start;
        return Effect.all([
          Effect.sync(() => taggedLatencyHistogram.unsafeUpdate(duration, [])),
          Effect.sync(() =>
            taggedCallCounter
              .pipe(Metric.tagged("status", "success"))
              .unsafeUpdate(1, []),
          ),
        ]);
      }),
      Effect.tapError((error) => {
        const duration = Date.now() - start;
        return Effect.all([
          Effect.sync(() => taggedLatencyHistogram.unsafeUpdate(duration, [])),
          Effect.sync(() =>
            taggedCallCounter
              .pipe(
                Metric.tagged("status", "error"),
                Metric.tagged("error_code", getErrorCodeSafe(error)),
              )
              .unsafeUpdate(1, []),
          ),
        ]);
      }),
      Effect.ensuring(
        Effect.sync(() => rpcActiveCallsGauge.unsafeUpdate(-1, [])),
      ),
    );

    return result;
  }) as Effect.Effect<A, E, R>;
};

/**
 * Track latency of an effect.
 */
export const withLatencyTracking =
  (path: string) =>
  <A, E, R>(effect: Effect.Effect<A, E, R>): Effect.Effect<A, E, R> => {
    const taggedHistogram = rpcLatencyHistogram.pipe(
      Metric.tagged("path", path),
    );

    return Effect.gen(function* () {
      const start = Date.now();
      const result = yield* effect;
      const duration = Date.now() - start;

      yield* Effect.sync(() => taggedHistogram.unsafeUpdate(duration, []));

      return result;
    }) as Effect.Effect<A, E, R>;
  };

/**
 * Count errors from an effect.
 */
export const withErrorCounting =
  (path: string, type: string) =>
  <A, E, R>(effect: Effect.Effect<A, E, R>): Effect.Effect<A, E, R> =>
    effect.pipe(
      Effect.tapError((error) =>
        Effect.sync(() =>
          rpcErrorCounter
            .pipe(
              Metric.tagged("path", path),
              Metric.tagged("type", type),
              Metric.tagged("error_code", getErrorCodeSafe(error)),
            )
            .unsafeUpdate(1, []),
        ),
      ),
    );

/**
 * Track active calls using a gauge.
 */
export const withActiveCallTracking = <A, E, R>(
  effect: Effect.Effect<A, E, R>,
): Effect.Effect<A, E, R> =>
  Effect.acquireUseRelease(
    Effect.sync(() => rpcActiveCallsGauge.unsafeUpdate(1, [])),
    () => effect,
    () => Effect.sync(() => rpcActiveCallsGauge.unsafeUpdate(-1, [])),
  );

// =============================================================================
// Metrics Service
// =============================================================================

/**
 * Metrics configuration.
 */
export interface MetricsConfig {
  readonly enabled: boolean;
  readonly prefix: string;
  readonly defaultTags: Record<string, string>;
}

const defaultMetricsConfig: MetricsConfig = {
  enabled: true,
  prefix: "rpc",
  defaultTags: {},
};

/**
 * Metrics service for configuration and access.
 */
export class MetricsService extends Context.Tag("MetricsService")<
  MetricsService,
  MetricsConfig
>() {
  static Default = Layer.succeed(MetricsService, defaultMetricsConfig);

  static layer(config: Partial<MetricsConfig> = {}) {
    return Layer.succeed(MetricsService, {
      ...defaultMetricsConfig,
      ...config,
    });
  }
}

/**
 * Create a metrics layer with configuration.
 */
export const createMetricsLayer = (
  config: Partial<MetricsConfig> = {},
): Layer.Layer<MetricsService> => MetricsService.layer(config);

// =============================================================================
// Metric Snapshots
// =============================================================================

/**
 * Metric snapshot for reporting.
 */
export interface MetricSnapshot {
  readonly totalCalls: number;
  readonly totalErrors: number;
  readonly activeCalls: number;
  readonly totalRetries: number;
  readonly cacheHits: number;
  readonly cacheMisses: number;
  readonly timestamp: number;
}

/**
 * Get a snapshot of current metrics.
 * Note: This is a simplified implementation. In production, you'd use
 * Effect's MetricClient for proper metric collection.
 */
export const getMetricSnapshot = Effect.sync(() => {
  // In a real implementation, you would collect from MetricClient
  // This is a placeholder that returns zeros
  const snapshot: MetricSnapshot = {
    totalCalls: 0,
    totalErrors: 0,
    activeCalls: 0,
    totalRetries: 0,
    cacheHits: 0,
    cacheMisses: 0,
    timestamp: Date.now(),
  };
  return snapshot;
});
