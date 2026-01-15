// =============================================================================
// Metrics Module - Effect Metric-based Observability
// =============================================================================
// Provides built-in observability using Effect's Metric system.

export {
  // Core metrics
  rpcCallCounter,
  rpcErrorCounter,
  rpcLatencyHistogram,
  rpcActiveCallsGauge,
  rpcRetryCounter,
  rpcCacheHitCounter,
  rpcCacheMissCounter,
  // Metric combinators
  withMetrics,
  withLatencyTracking,
  withErrorCounting,
  withActiveCallTracking,
  // Metric tags
  type MetricTags,
  createMetricTags,
  // Metric service
  MetricsService,
  type MetricsConfig,
  createMetricsLayer,
  // Metric snapshots
  getMetricSnapshot,
  type MetricSnapshot,
} from "./metrics";

export {
  // Histogram boundaries
  defaultLatencyBoundaries,
  createLatencyBoundaries,
  // Metric naming
  createMetricName,
  type MetricNamespace,
} from "./config";
