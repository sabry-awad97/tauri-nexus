// =============================================================================
// @tauri-nexus/rpc-core - Effect-Based Batch Builder
// =============================================================================
// Fluent API for building and executing type-safe batch requests using Effect.

import { Effect, pipe } from "effect";
import { invoke } from "@tauri-apps/api/core";
import type {
  SingleRequest,
  BatchCallOptions,
  BatchResponse,
  BatchRequest,
} from "../core/types";
import type {
  ExtractCallablePaths,
  GetInputAtPath,
  GetOutputAtPath,
  TypedBatchResult,
} from "../core/inference";
import { validatePathEffect } from "../core/effect-validation";
import {
  makeCallError,
  parseEffectError,
  toPublicError,
} from "../internal/effect-errors";
import type { RpcEffectError } from "../internal/effect-types";

// =============================================================================
// Types
// =============================================================================

/** Internal type to track batch entries with their output types */
interface BatchEntry {
  id: string;
  path: string;
  input: unknown;
}

/** Type map for tracking request IDs to their output types */
type OutputTypeMap = Record<string, unknown>;

// =============================================================================
// Effect-Based Batch Execution
// =============================================================================

/**
 * Execute batch requests using Effect.
 */
export const executeBatchEffect = <T = unknown>(
  requests: readonly SingleRequest[],
  options?: BatchCallOptions,
): Effect.Effect<BatchResponse<T>, RpcEffectError> =>
  Effect.gen(function* () {
    // Validate all paths
    for (const req of requests) {
      yield* validatePathEffect(req.path);
    }

    const normalizedRequests = requests.map((req) => ({
      ...req,
      input: req.input === undefined ? null : req.input,
    }));

    const batchRequest: BatchRequest = { requests: normalizedRequests };
    const timeoutMs = options?.timeout;

    // Execute with optional timeout
    const executeInvoke = Effect.tryPromise({
      try: () =>
        invoke<BatchResponse<T>>("plugin:rpc|rpc_call_batch", {
          batch: batchRequest,
        }),
      catch: (error) => parseEffectError(error, "batch", timeoutMs),
    });

    if (timeoutMs) {
      return yield* pipe(
        Effect.acquireUseRelease(
          // Acquire: set up timeout
          Effect.sync(() => {
            const controller = new AbortController();
            const timeoutId = setTimeout(() => controller.abort(), timeoutMs);
            return { timeoutId, controller };
          }),
          // Use: execute the invoke
          () => executeInvoke,
          // Release: clear timeout
          ({ timeoutId }) => Effect.sync(() => clearTimeout(timeoutId)),
        ),
      );
    }

    return yield* executeInvoke;
  });

// =============================================================================
// EffectBatchBuilder
// =============================================================================

/**
 * Type-safe batch builder that uses Effect for execution.
 *
 * @example
 * ```typescript
 * // Effect-based execution
 * const effect = createEffectBatch<AppContract>()
 *   .add('health-check', 'health', undefined)
 *   .add('user-1', 'user.get', { id: 1 })
 *   .executeEffect();
 *
 * const response = await Effect.runPromise(effect);
 *
 * // Or use Promise-based execution
 * const response = await createEffectBatch<AppContract>()
 *   .add('health-check', 'health', undefined)
 *   .execute();
 * ```
 */
export class EffectBatchBuilder<
  TContract,
  TOutputMap extends OutputTypeMap = Record<string, never>,
> {
  private entries: BatchEntry[] = [];

  /**
   * Add a type-safe request to the batch.
   */
  add<TId extends string, TPath extends ExtractCallablePaths<TContract>>(
    id: TId,
    path: TPath,
    input: GetInputAtPath<TContract, TPath>,
  ): EffectBatchBuilder<
    TContract,
    TOutputMap & Record<TId, GetOutputAtPath<TContract, TPath>>
  > {
    this.entries.push({ id, path, input });
    return this as unknown as EffectBatchBuilder<
      TContract,
      TOutputMap & Record<TId, GetOutputAtPath<TContract, TPath>>
    >;
  }

  /**
   * Get the current requests in the batch.
   */
  getRequests(): SingleRequest[] {
    return this.entries.map((e) => ({
      id: e.id,
      path: e.path,
      input: e.input,
    }));
  }

  /**
   * Get the number of requests in the batch.
   */
  size(): number {
    return this.entries.length;
  }

  /**
   * Clear all requests from the batch.
   */
  clear(): EffectBatchBuilder<TContract, Record<string, never>> {
    this.entries = [];
    return this as unknown as EffectBatchBuilder<
      TContract,
      Record<string, never>
    >;
  }

  /**
   * Execute the batch using Effect.
   * Returns an Effect that can be composed with other Effects.
   */
  executeEffect(
    options?: BatchCallOptions,
  ): Effect.Effect<EffectBatchResponseWrapper<TOutputMap>, RpcEffectError> {
    return pipe(
      executeBatchEffect(this.getRequests(), options),
      Effect.map(
        (response) => new EffectBatchResponseWrapper<TOutputMap>(response),
      ),
    );
  }

  /**
   * Execute the batch and return a Promise (convenience method).
   */
  async execute(
    options?: BatchCallOptions,
  ): Promise<EffectBatchResponseWrapper<TOutputMap>> {
    try {
      return await Effect.runPromise(this.executeEffect(options));
    } catch (error) {
      const rpcError = toPublicError(
        parseEffectError(error, "batch", options?.timeout),
      );
      console.warn(
        `[RPC] Batch request failed: ${rpcError.code} - ${rpcError.message}`,
        rpcError.details,
      );
      throw rpcError;
    }
  }

  /**
   * Validate all paths in the batch without executing.
   */
  validateEffect(): Effect.Effect<void, RpcEffectError> {
    return Effect.gen(
      function* (this: EffectBatchBuilder<TContract, TOutputMap>) {
        for (const entry of this.entries) {
          yield* validatePathEffect(entry.path);
        }
      }.bind(this),
    );
  }

  /**
   * Map over the batch entries.
   */
  map<U>(fn: (entry: BatchEntry) => U): U[] {
    return this.entries.map(fn);
  }
}

// =============================================================================
// EffectBatchResponseWrapper
// =============================================================================

/**
 * Type-safe batch response with Effect-based helper methods.
 */
export class EffectBatchResponseWrapper<TOutputMap extends OutputTypeMap> {
  private resultMap: Map<string, TypedBatchResult<unknown>>;
  private orderedResults: TypedBatchResult<unknown>[];

  constructor(response: BatchResponse<unknown>) {
    this.orderedResults = response.results as TypedBatchResult<unknown>[];
    this.resultMap = new Map();
    for (const result of response.results) {
      this.resultMap.set(result.id, result);
      if (result.error) {
        console.warn(
          `[RPC] Batch request '${result.id}' failed: ${result.error.code} - ${result.error.message}`,
        );
      }
    }
  }

  /**
   * Get all results in order.
   */
  get results(): TypedBatchResult<unknown>[] {
    return this.orderedResults;
  }

  /**
   * Get a typed result by request ID using Effect.
   */
  getResultEffect<TId extends keyof TOutputMap & string>(
    id: TId,
  ): Effect.Effect<TOutputMap[TId], RpcEffectError> {
    return Effect.gen(
      function* (this: EffectBatchResponseWrapper<TOutputMap>) {
        const result = this.resultMap.get(id);
        if (!result) {
          return yield* Effect.fail(
            makeCallError("NOT_FOUND", `No result found for id: ${id}`),
          );
        }
        if (result.error) {
          return yield* Effect.fail(
            makeCallError(
              result.error.code,
              result.error.message,
              result.error.details,
            ),
          );
        }
        return result.data as TOutputMap[TId];
      }.bind(this),
    );
  }

  /**
   * Get a typed result by request ID (synchronous).
   */
  getResult<TId extends keyof TOutputMap & string>(
    id: TId,
  ): TypedBatchResult<TOutputMap[TId]> {
    const result = this.resultMap.get(id);
    if (!result) {
      return {
        id,
        error: { code: "NOT_FOUND", message: `No result found for id: ${id}` },
      };
    }
    return result as TypedBatchResult<TOutputMap[TId]>;
  }

  /**
   * Check if a specific request succeeded.
   */
  isSuccess(id: string): boolean {
    const result = this.resultMap.get(id);
    return result ? !result.error : false;
  }

  /**
   * Check if a specific request failed.
   */
  isError(id: string): boolean {
    const result = this.resultMap.get(id);
    return result ? !!result.error : true;
  }

  /**
   * Get all successful results.
   */
  getSuccessful(): TypedBatchResult<unknown>[] {
    return this.orderedResults.filter((r) => !r.error);
  }

  /**
   * Get all failed results.
   */
  getFailed(): TypedBatchResult<unknown>[] {
    return this.orderedResults.filter((r) => r.error);
  }

  /**
   * Get all successful results using Effect.
   */
  getSuccessfulEffect(): Effect.Effect<TypedBatchResult<unknown>[]> {
    return Effect.succeed(this.getSuccessful());
  }

  /**
   * Get all failed results using Effect.
   */
  getFailedEffect(): Effect.Effect<TypedBatchResult<unknown>[]> {
    return Effect.succeed(this.getFailed());
  }

  /**
   * Process all results with Effect, failing on first error.
   */
  processAllEffect<U>(
    fn: (data: unknown, id: string) => U,
  ): Effect.Effect<U[], RpcEffectError> {
    return Effect.gen(
      function* (this: EffectBatchResponseWrapper<TOutputMap>) {
        const results: U[] = [];
        for (const result of this.orderedResults) {
          if (result.error) {
            return yield* Effect.fail(
              makeCallError(
                result.error.code,
                result.error.message,
                result.error.details,
              ),
            );
          }
          results.push(fn(result.data, result.id));
        }
        return results;
      }.bind(this),
    );
  }

  /**
   * Process all results, collecting errors instead of failing fast.
   */
  processAllCollectingErrorsEffect<U>(
    fn: (data: unknown, id: string) => U,
  ): Effect.Effect<{ successes: U[]; errors: TypedBatchResult<unknown>[] }> {
    return Effect.sync(() => {
      const successes: U[] = [];
      const errors: TypedBatchResult<unknown>[] = [];

      for (const result of this.orderedResults) {
        if (result.error) {
          errors.push(result);
        } else {
          successes.push(fn(result.data, result.id));
        }
      }

      return { successes, errors };
    });
  }

  /**
   * Get the count of successful requests.
   */
  get successCount(): number {
    return this.orderedResults.filter((r) => !r.error).length;
  }

  /**
   * Get the count of failed requests.
   */
  get errorCount(): number {
    return this.orderedResults.filter((r) => r.error).length;
  }
}

// =============================================================================
// Factory Function
// =============================================================================

/**
 * Create a new Effect-based batch builder.
 */
export function createEffectBatch<TContract>(): EffectBatchBuilder<
  TContract,
  Record<string, never>
> {
  return new EffectBatchBuilder<TContract, Record<string, never>>();
}
