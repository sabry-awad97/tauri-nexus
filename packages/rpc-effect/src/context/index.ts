// =============================================================================
// Context Module - Effect-native Request/Response Context
// =============================================================================
// Provides request-scoped context using Effect's Context system.

export {
  // Request context
  RequestContext,
  type RequestContextData,
  createRequestContext,
  withRequestContext,
  getRequestPath,
  getRequestInput,
  getRequestMeta,
  getTraceId,
  // Response context
  ResponseContext,
  type ResponseContextData,
  createResponseContext,
  withResponseContext,
  // Timing context
  TimingContext,
  type TimingContextData,
  createTimingContext,
  withTiming,
  measureDuration,
  // Trace context
  TraceContext,
  type TraceContextData,
  createTraceContext,
  withTracing,
  generateTraceId,
  generateSpanId,
  // Combined context layer
  createFullRequestContext,
  type FullRequestContext,
} from "./request-context";

export {
  // Span management
  SpanContext,
  type SpanData,
  createSpan,
  withSpan,
  addSpanAttribute,
  addSpanEvent,
  endSpan,
  getCurrentSpan,
  // Distributed tracing
  type TraceHeaders,
  extractTraceHeaders,
  injectTraceHeaders,
} from "./tracing";
