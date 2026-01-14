// =============================================================================
// @tauri-nexus/rpc-core - Batch Builder
// =============================================================================
// Fluent API for building and executing type-safe batch requests.

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
import {
  validatePath,
  createCallError,
  type RpcEffectError,
} from "@tauri-nexus/rpc-effect";
import { toRpcError, parseEffectError } from "../internal";

// =============================================================================
// Types
// =============================================================================

interface BatchEntry {
  id: string;
  path: string;
  input: unknown;
}

type OutputTypeMap = Record<string, unknown>;

// =============================================================================
// Batch Execution Effect
// =============================================================================

export const executeBatchEffect = <T = unknown>(
  requests: readonly SingleRequest[],
  options?: BatchCallOptions,
): Effect.Effect<BatchResponse<T>, RpcEffectError> =>
  Effect.gen(function* () {
    for (const req of requests) {
      yield* validatePath(req.path);
    }

    const normalizedRequests = requests.map((req) => ({
      ...req,
      input: req.input === undefined ? null : req.input,
    }));

    const batchRequest: BatchRequest = { requests: normalizedRequests };
    const timeoutMs = options?.timeout;

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
          Effect.sync(() => {
            const timeoutId = setTimeout(() => {}, timeoutMs);
            return timeoutId;
          }),
          () => executeInvoke,
          (timeoutId) => Effect.sync(() => clearTimeout(timeoutId)),
        ),
      );
    }

    return yield* executeInvoke;
  });

// =============================================================================
// EffectBatchBuilder
// =============================================================================

export class EffectBatchBuilder<
  TContract,
  TOutputMap extends OutputTypeMap = Record<string, never>,
> {
  private entries: BatchEntry[] = [];

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

  getRequests(): SingleRequest[] {
    return this.entries.map((e) => ({
      id: e.id,
      path: e.path,
      input: e.input,
    }));
  }

  size(): number {
    return this.entries.length;
  }

  clear(): EffectBatchBuilder<TContract, Record<string, never>> {
    this.entries = [];
    return this as unknown as EffectBatchBuilder<
      TContract,
      Record<string, never>
    >;
  }

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

  async execute(
    options?: BatchCallOptions,
  ): Promise<EffectBatchResponseWrapper<TOutputMap>> {
    try {
      return await Effect.runPromise(this.executeEffect(options));
    } catch (error) {
      const rpcError = toRpcError(
        parseEffectError(error, "batch", options?.timeout),
      );
      console.warn(
        `[RPC] Batch request failed: ${rpcError.code} - ${rpcError.message}`,
        rpcError.details,
      );
      throw rpcError;
    }
  }

  validateEffect(): Effect.Effect<void, RpcEffectError> {
    return Effect.gen(
      function* (this: EffectBatchBuilder<TContract, TOutputMap>) {
        for (const entry of this.entries) {
          yield* validatePath(entry.path);
        }
      }.bind(this),
    );
  }

  map<U>(fn: (entry: BatchEntry) => U): U[] {
    return this.entries.map(fn);
  }
}

// =============================================================================
// EffectBatchResponseWrapper
// =============================================================================

export class EffectBatchResponseWrapper<TOutputMap extends OutputTypeMap> {
  private readonly resultMap: Map<string, TypedBatchResult<unknown>>;
  private readonly orderedResults: TypedBatchResult<unknown>[];

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

  get results(): TypedBatchResult<unknown>[] {
    return this.orderedResults;
  }

  getResultEffect<TId extends keyof TOutputMap & string>(
    id: TId,
  ): Effect.Effect<TOutputMap[TId], RpcEffectError> {
    return Effect.gen(
      function* (this: EffectBatchResponseWrapper<TOutputMap>) {
        const result = this.resultMap.get(id);
        if (!result) {
          return yield* Effect.fail(
            createCallError("NOT_FOUND", `No result found for id: ${id}`),
          );
        }
        if (result.error) {
          return yield* Effect.fail(
            createCallError(
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

  isSuccess(id: string): boolean {
    const result = this.resultMap.get(id);
    return result ? !result.error : false;
  }

  isError(id: string): boolean {
    const result = this.resultMap.get(id);
    return result ? !!result.error : true;
  }

  getSuccessful(): TypedBatchResult<unknown>[] {
    return this.orderedResults.filter((r) => !r.error);
  }

  getFailed(): TypedBatchResult<unknown>[] {
    return this.orderedResults.filter((r) => r.error);
  }

  get successCount(): number {
    return this.orderedResults.filter((r) => !r.error).length;
  }

  get errorCount(): number {
    return this.orderedResults.filter((r) => r.error).length;
  }
}

// =============================================================================
// Factory Function
// =============================================================================

export function createEffectBatch<TContract>(): EffectBatchBuilder<
  TContract,
  Record<string, never>
> {
  return new EffectBatchBuilder<TContract, Record<string, never>>();
}
