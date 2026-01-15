// =============================================================================
// Tracing - Distributed tracing support
// =============================================================================

import { Context, Effect, Layer, Ref } from "effect";
import { TraceContext, generateSpanId } from "./request-context";

// =============================================================================
// Span Data
// =============================================================================

/**
 * Span data structure for tracing.
 */
export interface SpanData {
  readonly spanId: string;
  readonly parentSpanId?: string;
  readonly name: string;
  readonly startTime: number;
  readonly endTime?: number;
  readonly attributes: Map<string, unknown>;
  readonly events: Array<{
    name: string;
    timestamp: number;
    attributes?: Record<string, unknown>;
  }>;
  readonly status: "unset" | "ok" | "error";
  readonly statusMessage?: string;
}

/**
 * Span context service - provides current span data.
 */
export class SpanContext extends Context.Tag("SpanContext")<
  SpanContext,
  Ref.Ref<SpanData>
>() {}

// =============================================================================
// Span Operations
// =============================================================================

/**
 * Create a new span.
 */
export const createSpan = (
  name: string,
  parentSpanId?: string,
): Effect.Effect<Ref.Ref<SpanData>> =>
  Ref.make<SpanData>({
    spanId: generateSpanId(),
    parentSpanId,
    name,
    startTime: Date.now(),
    attributes: new Map(),
    events: [],
    status: "unset",
  });

/**
 * Run an effect within a span.
 */
export const withSpan = <A, E, R>(
  name: string,
  effect: Effect.Effect<A, E, R>,
): Effect.Effect<A, E, R | TraceContext> =>
  Effect.gen(function* () {
    const traceCtx = yield* TraceContext;
    const spanRef = yield* createSpan(name, traceCtx.spanId);

    const result = yield* effect.pipe(
      Effect.provide(Layer.succeed(SpanContext, spanRef)),
      Effect.tapError(() =>
        Ref.update(spanRef, (span) => ({
          ...span,
          status: "error" as const,
          endTime: Date.now(),
        })),
      ),
      Effect.tap(() =>
        Ref.update(spanRef, (span) => ({
          ...span,
          status: "ok" as const,
          endTime: Date.now(),
        })),
      ),
    );

    return result;
  });

/**
 * Add an attribute to the current span.
 */
export const addSpanAttribute = (
  key: string,
  value: unknown,
): Effect.Effect<void, never, SpanContext> =>
  Effect.gen(function* () {
    const spanRef = yield* SpanContext;
    yield* Ref.update(spanRef, (span) => {
      const newAttributes = new Map(span.attributes);
      newAttributes.set(key, value);
      return { ...span, attributes: newAttributes };
    });
  });

/**
 * Add an event to the current span.
 */
export const addSpanEvent = (
  name: string,
  attributes?: Record<string, unknown>,
): Effect.Effect<void, never, SpanContext> =>
  Effect.gen(function* () {
    const spanRef = yield* SpanContext;
    yield* Ref.update(spanRef, (span) => ({
      ...span,
      events: [...span.events, { name, timestamp: Date.now(), attributes }],
    }));
  });

/**
 * End the current span.
 */
export const endSpan = (
  status: "ok" | "error" = "ok",
  message?: string,
): Effect.Effect<void, never, SpanContext> =>
  Effect.gen(function* () {
    const spanRef = yield* SpanContext;
    yield* Ref.update(spanRef, (span) => ({
      ...span,
      status,
      statusMessage: message,
      endTime: Date.now(),
    }));
  });

/**
 * Get the current span data.
 */
export const getCurrentSpan: Effect.Effect<SpanData, never, SpanContext> =
  Effect.gen(function* () {
    const spanRef = yield* SpanContext;
    return yield* Ref.get(spanRef);
  });

// =============================================================================
// Trace Headers (W3C Trace Context)
// =============================================================================

/**
 * Trace headers for distributed tracing propagation.
 */
export interface TraceHeaders {
  readonly traceparent?: string;
  readonly tracestate?: string;
  baggage?: string;
}

/**
 * Extract trace context from headers (W3C Trace Context format).
 */
export const extractTraceHeaders = (
  headers: Record<string, string | undefined>,
): {
  traceId?: string;
  spanId?: string;
  sampled?: boolean;
  baggage?: Map<string, string>;
} => {
  const result: {
    traceId?: string;
    spanId?: string;
    sampled?: boolean;
    baggage?: Map<string, string>;
  } = {};

  // Parse traceparent header: version-traceId-spanId-flags
  const traceparent = headers["traceparent"];
  if (traceparent) {
    const parts = traceparent.split("-");
    if (parts.length === 4) {
      result.traceId = parts[1];
      result.spanId = parts[2];
      result.sampled = (parseInt(parts[3], 16) & 0x01) === 0x01;
    }
  }

  // Parse baggage header
  const baggage = headers["baggage"];
  if (baggage) {
    result.baggage = new Map();
    for (const item of baggage.split(",")) {
      const [key, value] = item.split("=").map((s) => s.trim());
      if (key && value) {
        result.baggage.set(decodeURIComponent(key), decodeURIComponent(value));
      }
    }
  }

  return result;
};

/**
 * Inject trace context into headers (W3C Trace Context format).
 */
export const injectTraceHeaders = (
  traceId: string,
  spanId: string,
  sampled: boolean = true,
  baggage?: Map<string, string>,
): TraceHeaders => {
  const headers: TraceHeaders = {
    traceparent: `00-${traceId}-${spanId}-${sampled ? "01" : "00"}`,
  };

  if (baggage && baggage.size > 0) {
    const baggageItems: string[] = [];
    for (const [key, value] of baggage) {
      baggageItems.push(
        `${encodeURIComponent(key)}=${encodeURIComponent(value)}`,
      );
    }
    headers.baggage = baggageItems.join(",");
  }

  return headers;
};
