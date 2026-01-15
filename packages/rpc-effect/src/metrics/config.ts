// =============================================================================
// Metrics Configuration
// =============================================================================

import { MetricBoundaries } from "effect";

// =============================================================================
// Histogram Boundaries
// =============================================================================

/**
 * Default latency histogram boundaries in milliseconds.
 */
export const defaultLatencyBoundaries = MetricBoundaries.exponential({
  start: 1,
  factor: 2,
  count: 15,
});

/**
 * Create custom latency boundaries.
 */
export const createLatencyBoundaries = (
  boundaries: readonly number[],
): MetricBoundaries.MetricBoundaries =>
  MetricBoundaries.fromIterable(boundaries);

// =============================================================================
// Metric Naming
// =============================================================================

/**
 * Metric namespace configuration.
 */
export interface MetricNamespace {
  readonly prefix: string;
  readonly separator: string;
}

const defaultNamespace: MetricNamespace = {
  prefix: "rpc",
  separator: ".",
};

/**
 * Create a metric name with namespace.
 */
export const createMetricName = (
  name: string,
  namespace: MetricNamespace = defaultNamespace,
): string => `${namespace.prefix}${namespace.separator}${name}`;
