// =============================================================================
// Batch Operations
// =============================================================================

import { Effect, Either } from "effect";
import type { RpcEffectError } from "../core/errors";
import {
  RpcTransportService,
  RpcLoggerService,
  type RpcServices,
} from "../services";
import { validatePath } from "../validation";
import { defaultParseError, call } from "./call";

// =============================================================================
// Types
// =============================================================================

export interface BatchRequestItem {
  readonly id: string;
  readonly path: string;
  readonly input: unknown;
}

export interface BatchResultItem<T = unknown> {
  readonly id: string;
  readonly data?: T;
  readonly error?: { code: string; message: string; details?: unknown };
}

export interface BatchRequest {
  readonly requests: readonly BatchRequestItem[];
}

export interface BatchResponse<T = unknown> {
  readonly results: readonly BatchResultItem<T>[];
}

// =============================================================================
// Batch Operations
// =============================================================================

/**
 * Validate batch requests (paths only).
 */
export const validateBatchRequests = (
  requests: readonly BatchRequestItem[],
): Effect.Effect<readonly BatchRequestItem[], RpcEffectError> =>
  Effect.gen(function* () {
    for (const req of requests) {
      yield* validatePath(req.path);
    }
    return requests;
  });

/**
 * Execute a batch of RPC calls using the transport's batch method.
 */
export const batchCall = <T = unknown>(
  requests: readonly BatchRequestItem[],
): Effect.Effect<BatchResponse<T>, RpcEffectError, RpcServices> =>
  Effect.gen(function* () {
    const transport = yield* RpcTransportService;
    const logger = yield* RpcLoggerService;

    for (const req of requests) {
      yield* validatePath(req.path);
    }

    logger.debug(`Executing batch with ${requests.length} requests`);

    const parseError = transport.parseError ?? defaultParseError;

    const response = yield* Effect.tryPromise({
      try: () => transport.callBatch<T>(requests),
      catch: (error) => parseError(error, "batch"),
    });

    return response as BatchResponse<T>;
  });

// =============================================================================
// Parallel Batch Operations with Effect.all
// =============================================================================

/**
 * Execute batch requests in parallel using Effect.all with concurrency control.
 * This is more idiomatic than sequential execution when requests are independent.
 *
 * @param requests - Array of batch request items
 * @param concurrency - Maximum number of concurrent requests (default: 5)
 * @returns Array of results with Either for success/failure per request
 */
export const batchCallParallel = <T = unknown>(
  requests: readonly BatchRequestItem[],
  concurrency: number = 5,
): Effect.Effect<
  readonly Either.Either<T, RpcEffectError>[],
  never,
  RpcServices
> =>
  Effect.all(
    requests.map((req) =>
      call<T>(req.path, req.input).pipe(
        Effect.either,
        Effect.map((result) =>
          Either.isRight(result)
            ? Either.right(result.right)
            : Either.left(result.left),
        ),
      ),
    ),
    { concurrency },
  );

/**
 * Execute batch requests in parallel and collect successful results.
 * Failed requests are logged but don't fail the entire batch.
 *
 * @param requests - Array of batch request items
 * @param concurrency - Maximum number of concurrent requests (default: 5)
 * @returns BatchResponse with results for each request
 */
export const batchCallParallelCollect = <T = unknown>(
  requests: readonly BatchRequestItem[],
  concurrency: number = 5,
): Effect.Effect<BatchResponse<T>, never, RpcServices> =>
  Effect.gen(function* () {
    const logger = yield* RpcLoggerService;
    logger.debug(
      `Executing parallel batch with ${requests.length} requests (concurrency: ${concurrency})`,
    );

    const results = yield* batchCallParallel<T>(requests, concurrency);

    const batchResults: BatchResultItem<T>[] = requests.map((req, index) => {
      const result = results[index];
      if (Either.isRight(result)) {
        return { id: req.id, data: result.right };
      } else {
        const error = result.left;
        return {
          id: req.id,
          error: {
            code: error._tag === "RpcCallError" ? error.code : error._tag,
            message: error.message,
            details: "details" in error ? error.details : undefined,
          },
        };
      }
    });

    return { results: batchResults };
  });

/**
 * Execute batch requests in parallel, failing fast on first error.
 * Use this when all requests must succeed.
 *
 * @param requests - Array of batch request items
 * @param concurrency - Maximum number of concurrent requests (default: 5)
 * @returns Array of successful results, or fails with first error
 */
export const batchCallParallelFailFast = <T = unknown>(
  requests: readonly BatchRequestItem[],
  concurrency: number = 5,
): Effect.Effect<readonly T[], RpcEffectError, RpcServices> =>
  Effect.all(
    requests.map((req) => call<T>(req.path, req.input)),
    { concurrency },
  );

/**
 * Execute batch requests sequentially (one at a time).
 * Useful when order matters or to avoid overwhelming the server.
 */
export const batchCallSequential = <T = unknown>(
  requests: readonly BatchRequestItem[],
): Effect.Effect<BatchResponse<T>, never, RpcServices> =>
  batchCallParallelCollect<T>(requests, 1);
