// =============================================================================
// Request Context - Effect-native request-scoped context
// =============================================================================

import { Context, Effect, Layer, Ref } from "effect";
import type { ProcedureType } from "../core/types";

// =============================================================================
// Request Context
// =============================================================================

/**
 * Request context data structure.
 */
export interface RequestContextData {
  readonly path: string;
  readonly input: unknown;
  readonly type: ProcedureType;
  readonly meta: Record<string, unknown>;
  readonly traceId: string;
  readonly spanId: string;
  readonly parentSpanId?: string;
  readonly startTime: number;
  readonly signal?: AbortSignal;
  readonly timeout?: number;
}

/**
 * Request context service - provides request-scoped data.
 */
export class RequestContext extends Context.Tag("RequestContext")<
  RequestContext,
  RequestContextData
>() {
  static layer(data: RequestContextData) {
    return Layer.succeed(RequestContext, data);
  }
}

/**
 * Create request context data with defaults.
 */
export const createRequestContext = (
  path: string,
  input: unknown,
  options: {
    type?: ProcedureType;
    meta?: Record<string, unknown>;
    traceId?: string;
    spanId?: string;
    parentSpanId?: string;
    signal?: AbortSignal;
    timeout?: number;
  } = {},
): RequestContextData => ({
  path,
  input,
  type: options.type ?? "query",
  meta: options.meta ?? {},
  traceId: options.traceId ?? generateTraceId(),
  spanId: options.spanId ?? generateSpanId(),
  parentSpanId: options.parentSpanId,
  startTime: Date.now(),
  signal: options.signal,
  timeout: options.timeout,
});

/**
 * Run an effect with request context.
 */
export const withRequestContext = <A, E, R>(
  effect: Effect.Effect<A, E, R | RequestContext>,
  ctx: RequestContextData,
): Effect.Effect<A, E, Exclude<R, RequestContext>> =>
  effect.pipe(Effect.provide(RequestContext.layer(ctx)));

/**
 * Get the current request path.
 */
export const getRequestPath = Effect.map(RequestContext, (ctx) => ctx.path);

/**
 * Get the current request input.
 */
export const getRequestInput = Effect.map(RequestContext, (ctx) => ctx.input);

/**
 * Get the current request metadata.
 */
export const getRequestMeta = Effect.map(RequestContext, (ctx) => ctx.meta);

/**
 * Get the current trace ID.
 */
export const getTraceId = Effect.map(RequestContext, (ctx) => ctx.traceId);

// =============================================================================
// Response Context
// =============================================================================

/**
 * Response context data structure.
 */
export interface ResponseContextData {
  readonly data: unknown;
  readonly meta: Record<string, unknown>;
  readonly durationMs: number;
  readonly statusCode?: number;
}

/**
 * Response context service - provides response-scoped data.
 */
export class ResponseContext extends Context.Tag("ResponseContext")<
  ResponseContext,
  ResponseContextData
>() {
  static layer(data: ResponseContextData) {
    return Layer.succeed(ResponseContext, data);
  }
}

/**
 * Create response context data.
 */
export const createResponseContext = (
  data: unknown,
  durationMs: number,
  options: {
    meta?: Record<string, unknown>;
    statusCode?: number;
  } = {},
): ResponseContextData => ({
  data,
  meta: options.meta ?? {},
  durationMs,
  statusCode: options.statusCode,
});

/**
 * Run an effect with response context.
 */
export const withResponseContext = <A, E, R>(
  effect: Effect.Effect<A, E, R | ResponseContext>,
  ctx: ResponseContextData,
): Effect.Effect<A, E, Exclude<R, ResponseContext>> =>
  effect.pipe(Effect.provide(ResponseContext.layer(ctx)));

// =============================================================================
// Timing Context
// =============================================================================

/**
 * Timing context data structure.
 */
export interface TimingContextData {
  readonly startTime: number;
  readonly checkpoints: Map<string, number>;
}

/**
 * Timing context service - provides timing measurements.
 */
export class TimingContext extends Context.Tag("TimingContext")<
  TimingContext,
  Ref.Ref<TimingContextData>
>() {}

/**
 * Create timing context.
 */
export const createTimingContext = Effect.gen(function* () {
  const ref = yield* Ref.make<TimingContextData>({
    startTime: Date.now(),
    checkpoints: new Map(),
  });
  return ref;
});

/**
 * Run an effect with timing context.
 */
export const withTiming = <A, E, R>(
  effect: Effect.Effect<A, E, R | TimingContext>,
): Effect.Effect<A, E, Exclude<R, TimingContext>> =>
  Effect.gen(function* () {
    const timingRef = yield* createTimingContext;
    return yield* effect.pipe(
      Effect.provide(Layer.succeed(TimingContext, timingRef)),
    );
  });

/**
 * Measure duration of an effect.
 */
export const measureDuration = <A, E, R>(
  name: string,
  effect: Effect.Effect<A, E, R>,
): Effect.Effect<A, E, R | TimingContext> =>
  Effect.gen(function* () {
    const timingRef = yield* TimingContext;
    const start = Date.now();

    const result = yield* effect;

    yield* Ref.update(timingRef, (state) => {
      const newCheckpoints = new Map(state.checkpoints);
      newCheckpoints.set(name, Date.now() - start);
      return { ...state, checkpoints: newCheckpoints };
    });

    return result;
  });

// =============================================================================
// Trace Context
// =============================================================================

/**
 * Trace context data structure for distributed tracing.
 */
export interface TraceContextData {
  readonly traceId: string;
  readonly spanId: string;
  readonly parentSpanId?: string;
  readonly sampled: boolean;
  readonly baggage: Map<string, string>;
}

/**
 * Trace context service - provides distributed tracing context.
 */
export class TraceContext extends Context.Tag("TraceContext")<
  TraceContext,
  TraceContextData
>() {
  static layer(data: TraceContextData) {
    return Layer.succeed(TraceContext, data);
  }
}

/**
 * Create trace context data.
 */
export const createTraceContext = (
  options: {
    traceId?: string;
    spanId?: string;
    parentSpanId?: string;
    sampled?: boolean;
    baggage?: Map<string, string>;
  } = {},
): TraceContextData => ({
  traceId: options.traceId ?? generateTraceId(),
  spanId: options.spanId ?? generateSpanId(),
  parentSpanId: options.parentSpanId,
  sampled: options.sampled ?? true,
  baggage: options.baggage ?? new Map(),
});

/**
 * Run an effect with trace context.
 */
export const withTracing = <A, E, R>(
  effect: Effect.Effect<A, E, R | TraceContext>,
  ctx?: TraceContextData,
): Effect.Effect<A, E, Exclude<R, TraceContext>> =>
  effect.pipe(Effect.provide(TraceContext.layer(ctx ?? createTraceContext())));

// =============================================================================
// ID Generation
// =============================================================================

/**
 * Generate a trace ID (128-bit hex string).
 */
export const generateTraceId = (): string => {
  if (typeof crypto !== "undefined" && crypto.randomUUID) {
    return crypto.randomUUID().replace(/-/g, "");
  }
  return "xxxxxxxxxxxxxxxxxxxxxxxxxxxx".replace(/x/g, () =>
    Math.floor(Math.random() * 16).toString(16),
  );
};

/**
 * Generate a span ID (64-bit hex string).
 */
export const generateSpanId = (): string => {
  if (typeof crypto !== "undefined" && crypto.getRandomValues) {
    const bytes = new Uint8Array(8);
    crypto.getRandomValues(bytes);
    return Array.from(bytes)
      .map((b) => b.toString(16).padStart(2, "0"))
      .join("");
  }
  return "xxxxxxxxxxxxxxxx".replace(/x/g, () =>
    Math.floor(Math.random() * 16).toString(16),
  );
};

// =============================================================================
// Combined Context
// =============================================================================

/**
 * Full request context combining all context types.
 */
export interface FullRequestContext {
  readonly request: RequestContextData;
  readonly trace: TraceContextData;
}

/**
 * Create a full request context layer.
 */
export const createFullRequestContext = (
  path: string,
  input: unknown,
  options: {
    type?: ProcedureType;
    meta?: Record<string, unknown>;
    traceId?: string;
    spanId?: string;
    parentSpanId?: string;
    signal?: AbortSignal;
    timeout?: number;
    sampled?: boolean;
  } = {},
): Layer.Layer<RequestContext | TraceContext> => {
  const traceId = options.traceId ?? generateTraceId();
  const spanId = options.spanId ?? generateSpanId();

  const requestCtx = createRequestContext(path, input, {
    ...options,
    traceId,
    spanId,
  });

  const traceCtx = createTraceContext({
    traceId,
    spanId,
    parentSpanId: options.parentSpanId,
    sampled: options.sampled,
  });

  return Layer.merge(
    RequestContext.layer(requestCtx),
    TraceContext.layer(traceCtx),
  );
};
