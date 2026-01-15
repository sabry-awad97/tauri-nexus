// =============================================================================
// RPC Call & Subscribe Operations
// =============================================================================

import { Effect, Stream, Layer, Schema } from "effect";
import type { RpcEffectError } from "../core/errors";
import type { InterceptorContext, EventIterator } from "../core/types";
import {
  createCallError,
  createTimeoutError,
  createCancelledError,
  isEffectRpcError,
} from "../core/error-utils";
import {
  RpcConfigService,
  RpcTransportService,
  RpcInterceptorService,
  RpcLoggerService,
  type RpcServices,
} from "../services";
import { validatePath } from "../validation";
import {
  RequestContext,
  TraceContext,
  createRequestContext,
  createTraceContext,
  generateTraceId,
  generateSpanId,
} from "../context";
import { createSchemaValidationError } from "../schema/error-schemas";

// Resilience types
import type { RpcCacheService } from "../cache/cache";
import type {
  BulkheadService,
  BulkheadFullError,
} from "../resilience/bulkhead";
import type {
  RateLimiterService,
  RateLimitExceededError,
} from "../resilience/rate-limiter";
import type {
  CircuitBreakerService,
  CircuitOpenError,
} from "../resilience/circuit-breaker";
import type { SpanContext } from "../context/tracing";

// =============================================================================
// Types
// =============================================================================

/** All resilience services that may be required */
export type ResilienceServices =
  | RpcCacheService
  | BulkheadService
  | RateLimiterService
  | CircuitBreakerService
  | SpanContext;

/** All resilience errors that may be returned */
export type ResilienceErrors =
  | BulkheadFullError
  | RateLimitExceededError
  | CircuitOpenError;

/** Schema configuration for input/output validation */
export interface SchemaConfig<TInput, TOutput> {
  readonly input: Schema.Schema<TInput, unknown, never>;
  readonly output: Schema.Schema<TOutput, unknown, never>;
  readonly skipInputValidation?: boolean;
  readonly skipOutputValidation?: boolean;
}

/** Resilience configuration */
export interface ResilienceConfig {
  readonly cache?: boolean;
  readonly circuitBreaker?: boolean;
  readonly rateLimit?: boolean;
  readonly bulkhead?: boolean;
  readonly metrics?: boolean;
}

/** Unified call options */
export interface CallOptions<TInput = unknown, TOutput = unknown> {
  readonly signal?: AbortSignal;
  readonly timeout?: number;
  readonly meta?: Record<string, unknown>;
  readonly tracing?: boolean;
  readonly traceId?: string;
  readonly parentSpanId?: string;
  readonly type?: "query" | "mutation" | "subscription";
  readonly sampled?: boolean;
  readonly baggage?: Map<string, string>;
  readonly schema?: SchemaConfig<TInput, TOutput>;
  readonly resilience?: ResilienceConfig;
}

/** Subscribe-specific options */
export interface SubscribeOptions<
  TInput = unknown,
  TEvent = unknown,
> extends Omit<CallOptions<TInput, TEvent>, "schema"> {
  readonly lastEventId?: string;
  readonly schema?: {
    readonly input: Schema.Schema<TInput, unknown, never>;
    readonly event: Schema.Schema<TEvent, unknown, never>;
    readonly skipInputValidation?: boolean;
    readonly skipEventValidation?: boolean;
  };
}

// =============================================================================
// Error Handling
// =============================================================================

export const defaultParseError = (
  error: unknown,
  path: string,
  timeoutMs?: number,
): RpcEffectError => {
  if (isEffectRpcError(error)) return error;

  if (error instanceof Error && error.name === "AbortError") {
    return timeoutMs !== undefined
      ? createTimeoutError(path, timeoutMs)
      : createCancelledError(path);
  }

  if (error instanceof Error) {
    return createCallError("UNKNOWN", error.message, undefined, error.stack);
  }

  return createCallError(
    "UNKNOWN",
    typeof error === "string" ? error : String(error),
  );
};

const getParseError = (transport: { parseError?: typeof defaultParseError }) =>
  transport.parseError ?? defaultParseError;

// =============================================================================
// Interceptor Execution
// =============================================================================

const executeWithInterceptors = <T>(
  ctx: InterceptorContext,
  operation: () => Promise<T>,
  parseError: (
    error: unknown,
    path: string,
    timeoutMs?: number,
  ) => RpcEffectError,
): Effect.Effect<T, RpcEffectError, RpcInterceptorService> =>
  Effect.gen(function* () {
    const { interceptors } = yield* RpcInterceptorService;

    let next = operation;
    for (let i = interceptors.length - 1; i >= 0; i--) {
      const interceptor = interceptors[i];
      const currentNext = next;
      next = () => interceptor.intercept(ctx, currentNext);
    }

    return yield* Effect.tryPromise({
      try: () => next(),
      catch: (error) => parseError(error, ctx.path),
    });
  });

// =============================================================================
// Core Implementation
// =============================================================================

const coreCall = <T>(
  path: string,
  input: unknown,
  options: CallOptions,
): Effect.Effect<T, RpcEffectError, RpcServices> =>
  Effect.gen(function* () {
    yield* validatePath(path);

    const config = yield* RpcConfigService;
    const transport = yield* RpcTransportService;
    const logger = yield* RpcLoggerService;

    const timeoutMs = options.timeout ?? config.defaultTimeout;
    const traceId = options.traceId ?? generateTraceId();
    const spanId = generateSpanId();
    const procedureType = options.type ?? "query";

    const ctx: InterceptorContext = {
      path,
      input,
      type: procedureType,
      meta: {
        ...options.meta,
        traceId,
        spanId,
        parentSpanId: options.parentSpanId,
      },
      signal: options.signal,
    };

    logger.debug(`Calling ${path}`, {
      input,
      timeout: timeoutMs,
      traceId,
      spanId,
    });
    const startTime = Date.now();

    const callEffect = executeWithInterceptors<T>(
      ctx,
      async () => {
        if (timeoutMs) {
          const controller = new AbortController();
          const timeoutId = setTimeout(() => controller.abort(), timeoutMs);
          try {
            const res = await transport.call<T>(path, input);
            clearTimeout(timeoutId);
            return res;
          } catch (error) {
            clearTimeout(timeoutId);
            throw error;
          }
        }
        return transport.call<T>(path, input);
      },
      getParseError(transport),
    );

    const result = options.tracing
      ? yield* callEffect.pipe(
          Effect.provide(
            Layer.merge(
              RequestContext.layer(
                createRequestContext(path, input, {
                  type: procedureType,
                  meta: options.meta,
                  traceId,
                  spanId,
                  parentSpanId: options.parentSpanId,
                  signal: options.signal,
                  timeout: timeoutMs,
                }),
              ),
              TraceContext.layer(
                createTraceContext({
                  traceId,
                  spanId,
                  parentSpanId: options.parentSpanId,
                  sampled: options.sampled,
                  baggage: options.baggage,
                }),
              ),
            ),
          ),
        )
      : yield* callEffect;

    logger.debug(`Completed ${path} in ${Date.now() - startTime}ms`, {
      traceId,
      spanId,
    });
    return result;
  });

const applyResilience = <T, E, R>(
  path: string,
  effect: Effect.Effect<T, E, R>,
  resilience: ResilienceConfig,
  tracing?: boolean,
): Effect.Effect<T, E | ResilienceErrors, R | ResilienceServices> =>
  Effect.gen(function* () {
    let current: Effect.Effect<
      T,
      E | ResilienceErrors,
      R | ResilienceServices
    > = effect as Effect.Effect<
      T,
      E | ResilienceErrors,
      R | ResilienceServices
    >;

    if (resilience.metrics) {
      const { withMetrics } = yield* Effect.promise(
        () => import("../metrics/metrics"),
      );
      current = withMetrics(path, "query", current) as typeof current;
    }

    if (resilience.cache) {
      const { withCache } = yield* Effect.promise(
        () => import("../cache/cache"),
      );
      current = withCache(path, undefined, current) as typeof current;
    }

    if (resilience.bulkhead) {
      const { withBulkhead } = yield* Effect.promise(
        () => import("../resilience/bulkhead"),
      );
      current = withBulkhead(path, current) as typeof current;
    }

    if (resilience.rateLimit) {
      const { withRateLimit } = yield* Effect.promise(
        () => import("../resilience/rate-limiter"),
      );
      current = withRateLimit(path, current) as typeof current;
    }

    if (resilience.circuitBreaker) {
      const { withCircuitBreaker } = yield* Effect.promise(
        () => import("../resilience/circuit-breaker"),
      );
      current = withCircuitBreaker(path, current) as typeof current;
    }

    if (tracing) {
      const { withSpan } = yield* Effect.promise(
        () => import("../context/tracing"),
      );
      current = withSpan(`rpc.call.${path}`, current) as typeof current;
    }

    return yield* current;
  }) as Effect.Effect<T, E | ResilienceErrors, R | ResilienceServices>;

// =============================================================================
// Call Function
// =============================================================================

/**
 * Make an RPC call.
 *
 * @example
 * ```ts
 * // Basic call
 * const user = yield* call<User>("users.get", { id: 1 });
 *
 * // With timeout and tracing
 * const user = yield* call<User>("users.get", { id: 1 }, {
 *   timeout: 5000,
 *   tracing: true,
 * });
 *
 * // With schema validation
 * const user = yield* call("users.get", { id: 1 }, {
 *   schema: {
 *     input: Schema.Struct({ id: Schema.Number }),
 *     output: Schema.Struct({ id: Schema.Number, name: Schema.String }),
 *   },
 * });
 *
 * // With resilience
 * const user = yield* call<User>("users.get", { id: 1 }, {
 *   resilience: { cache: true, circuitBreaker: true, metrics: true },
 * });
 * ```
 */
export function call<TInput, TOutput>(
  path: string,
  input: TInput,
  options: CallOptions<TInput, TOutput> & {
    schema: SchemaConfig<TInput, TOutput>;
    resilience: ResilienceConfig;
  },
): Effect.Effect<
  TOutput,
  RpcEffectError | ResilienceErrors,
  RpcServices | ResilienceServices
>;

export function call<TInput, TOutput>(
  path: string,
  input: TInput,
  options: CallOptions<TInput, TOutput> & {
    schema: SchemaConfig<TInput, TOutput>;
  },
): Effect.Effect<TOutput, RpcEffectError, RpcServices>;

export function call<T>(
  path: string,
  input: unknown,
  options: CallOptions & { resilience: ResilienceConfig },
): Effect.Effect<
  T,
  RpcEffectError | ResilienceErrors,
  RpcServices | ResilienceServices
>;

export function call<T>(
  path: string,
  input: unknown,
  options?: CallOptions,
): Effect.Effect<T, RpcEffectError, RpcServices>;

export function call<TInput = unknown, TOutput = unknown>(
  path: string,
  input: TInput,
  options: CallOptions<TInput, TOutput> = {},
): Effect.Effect<
  TOutput,
  RpcEffectError | ResilienceErrors,
  RpcServices | ResilienceServices
> {
  return Effect.gen(function* () {
    const { schema, resilience, ...baseOptions } = options;

    let validatedInput: unknown = input;
    if (schema && !schema.skipInputValidation) {
      validatedInput = yield* Schema.decodeUnknown(schema.input)(input).pipe(
        Effect.mapError((error) => createSchemaValidationError(path, error)),
      );
    }

    let result: unknown;
    if (resilience) {
      result = yield* applyResilience(
        path,
        coreCall<unknown>(path, validatedInput, baseOptions),
        resilience,
        baseOptions.tracing,
      );
    } else {
      result = yield* coreCall<unknown>(path, validatedInput, baseOptions);
    }

    if (schema && !schema.skipOutputValidation) {
      return yield* Schema.decodeUnknown(schema.output)(result).pipe(
        Effect.mapError((error) => createSchemaValidationError(path, error)),
      );
    }

    return result as TOutput;
  }) as Effect.Effect<
    TOutput,
    RpcEffectError | ResilienceErrors,
    RpcServices | ResilienceServices
  >;
}

// =============================================================================
// Subscribe Function
// =============================================================================

/**
 * Subscribe to an RPC stream.
 *
 * @example
 * ```ts
 * // Basic subscribe
 * const iterator = yield* subscribe<Event>("events.stream", { topic: "updates" });
 *
 * // With schema validation
 * const iterator = yield* subscribe("events.stream", { topic: "updates" }, {
 *   schema: {
 *     input: Schema.Struct({ topic: Schema.String }),
 *     event: Schema.Struct({ id: Schema.String, data: Schema.Unknown }),
 *   },
 * });
 *
 * // As Effect Stream
 * const stream = yield* subscribe<Event>("events.stream", { topic: "updates" }, {
 *   asStream: true,
 * });
 * ```
 */
export function subscribe<TInput, TEvent>(
  path: string,
  input: TInput,
  options: SubscribeOptions<TInput, TEvent> & {
    schema: NonNullable<SubscribeOptions<TInput, TEvent>["schema"]>;
  },
): Effect.Effect<EventIterator<TEvent>, RpcEffectError, RpcServices>;

export function subscribe<T>(
  path: string,
  input: unknown,
  options?: SubscribeOptions,
): Effect.Effect<EventIterator<T>, RpcEffectError, RpcServices>;

export function subscribe<TInput = unknown, TEvent = unknown>(
  path: string,
  input: TInput,
  options: SubscribeOptions<TInput, TEvent> = {},
): Effect.Effect<EventIterator<TEvent>, RpcEffectError, RpcServices> {
  return Effect.gen(function* () {
    yield* validatePath(path);

    const { schema, ...baseOptions } = options;
    const transport = yield* RpcTransportService;
    const logger = yield* RpcLoggerService;

    // Validate input
    let validatedInput: unknown = input;
    if (schema && !schema.skipInputValidation) {
      validatedInput = yield* Schema.decodeUnknown(schema.input)(input).pipe(
        Effect.mapError((error) => createSchemaValidationError(path, error)),
      );
    }

    logger.debug(`Subscribing to ${path}`, { input: validatedInput });

    const rawIterator = yield* Effect.tryPromise({
      try: () =>
        transport.subscribe<unknown>(path, validatedInput, {
          lastEventId: baseOptions.lastEventId,
          signal: baseOptions.signal,
        }),
      catch: (error) => getParseError(transport)(error, path),
    });

    // Return raw iterator if no event validation
    if (!schema || schema.skipEventValidation) {
      return rawIterator as EventIterator<TEvent>;
    }

    // Create validating iterator
    const eventSchema = schema.event;
    const validatingIterator: EventIterator<TEvent> = {
      [Symbol.asyncIterator]() {
        const inner = rawIterator[Symbol.asyncIterator]();
        return {
          async next() {
            const result = await inner.next();
            if (result.done) {
              return { done: true as const, value: undefined };
            }
            const decoded = (Schema.decodeUnknownSync as any)(eventSchema)(
              result.value,
            ) as TEvent;
            return { done: false as const, value: decoded };
          },
          async return() {
            await inner.return?.();
            return { done: true as const, value: undefined };
          },
        };
      },
      return: async () => {
        await rawIterator.return?.();
      },
    };

    return validatingIterator;
  }) as Effect.Effect<EventIterator<TEvent>, RpcEffectError, RpcServices>;
}

// =============================================================================
// Stream Utilities
// =============================================================================

/**
 * Subscribe and return as Effect Stream.
 */
export const subscribeStream = <T>(
  path: string,
  input: unknown,
  options: SubscribeOptions = {},
): Effect.Effect<
  Stream.Stream<T, RpcEffectError>,
  RpcEffectError,
  RpcServices
> =>
  Effect.gen(function* () {
    const iterator = yield* subscribe<T>(path, input, options);
    return Stream.fromAsyncIterable(iterator, (error) =>
      createCancelledError(
        path,
        error instanceof Error ? error.message : String(error),
      ),
    );
  });

/**
 * Subscribe and collect all events into an array.
 */
export const subscribeCollect = <T>(
  path: string,
  input: unknown,
  options: SubscribeOptions = {},
): Effect.Effect<T[], RpcEffectError, RpcServices> =>
  Effect.gen(function* () {
    const stream = yield* subscribeStream<T>(path, input, options);
    const chunk = yield* Stream.runCollect(stream);
    return [...chunk];
  });

/**
 * Subscribe and process each event with a callback.
 */
export const subscribeForEach = <T>(
  path: string,
  input: unknown,
  onEvent: (event: T) => Effect.Effect<void>,
  options: SubscribeOptions = {},
): Effect.Effect<void, RpcEffectError, RpcServices> =>
  Effect.gen(function* () {
    const stream = yield* subscribeStream<T>(path, input, options);
    yield* Stream.runForEach(stream, onEvent);
  });

// =============================================================================
// Factory Functions
// =============================================================================

/**
 * Create a typed procedure caller.
 *
 * @example
 * ```ts
 * const getUser = createCall(
 *   "users.get",
 *   Schema.Struct({ id: Schema.Number }),
 *   Schema.Struct({ id: Schema.Number, name: Schema.String })
 * );
 *
 * const user = yield* getUser({ id: 1 });
 * ```
 */
export const createCall = <
  TInput,
  TOutput,
  TInputSchema extends Schema.Schema<TInput, unknown, never>,
  TOutputSchema extends Schema.Schema<TOutput, unknown, never>,
>(
  path: string,
  inputSchema: TInputSchema,
  outputSchema: TOutputSchema,
  defaultOptions: Omit<CallOptions<TInput, TOutput>, "schema"> = {},
) => {
  return (
    input: TInput,
    options: Omit<CallOptions<TInput, TOutput>, "schema"> = {},
  ): Effect.Effect<TOutput, RpcEffectError, RpcServices> =>
    call(path, input, {
      ...defaultOptions,
      ...options,
      schema: { input: inputSchema, output: outputSchema },
    });
};

/**
 * Create a typed procedure caller with resilience.
 *
 * @example
 * ```ts
 * const getUser = createResilientCall(
 *   "users.get",
 *   Schema.Struct({ id: Schema.Number }),
 *   Schema.Struct({ id: Schema.Number, name: Schema.String }),
 *   { cache: true, circuitBreaker: true }
 * );
 *
 * const user = yield* getUser({ id: 1 });
 * ```
 */
export const createResilientCall = <
  TInput,
  TOutput,
  TInputSchema extends Schema.Schema<TInput, unknown, never>,
  TOutputSchema extends Schema.Schema<TOutput, unknown, never>,
>(
  path: string,
  inputSchema: TInputSchema,
  outputSchema: TOutputSchema,
  resilience: ResilienceConfig,
  defaultOptions: Omit<
    CallOptions<TInput, TOutput>,
    "schema" | "resilience"
  > = {},
) => {
  return (
    input: TInput,
    options: Omit<CallOptions<TInput, TOutput>, "schema" | "resilience"> = {},
  ): Effect.Effect<
    TOutput,
    RpcEffectError | ResilienceErrors,
    RpcServices | ResilienceServices
  > =>
    call(path, input, {
      ...defaultOptions,
      ...options,
      schema: { input: inputSchema, output: outputSchema },
      resilience,
    });
};

/**
 * Create a typed subscription.
 *
 * @example
 * ```ts
 * const streamEvents = createSubscribe(
 *   "events.stream",
 *   Schema.Struct({ topic: Schema.String }),
 *   Schema.Struct({ id: Schema.String, data: Schema.Unknown })
 * );
 *
 * const iterator = yield* streamEvents({ topic: "updates" });
 * ```
 */
export const createSubscribe = <
  TInput,
  TEvent,
  TInputSchema extends Schema.Schema<TInput, unknown, never>,
  TEventSchema extends Schema.Schema<TEvent, unknown, never>,
>(
  path: string,
  inputSchema: TInputSchema,
  eventSchema: TEventSchema,
  defaultOptions: Omit<SubscribeOptions<TInput, TEvent>, "schema"> = {},
) => {
  return (
    input: TInput,
    options: Omit<SubscribeOptions<TInput, TEvent>, "schema"> = {},
  ): Effect.Effect<EventIterator<TEvent>, RpcEffectError, RpcServices> =>
    subscribe(path, input, {
      ...defaultOptions,
      ...options,
      schema: { input: inputSchema, event: eventSchema },
    });
};
