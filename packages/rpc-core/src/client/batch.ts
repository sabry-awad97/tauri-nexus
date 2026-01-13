// =============================================================================
// @tauri-nexus/rpc-core - Type-Safe Batch Builder
// =============================================================================
// Fluent API for building and executing type-safe batch requests.

import type {
  SingleRequest,
  BatchCallOptions,
  BatchResponse,
} from "../core/types";
import type {
  ExtractCallablePaths,
  GetInputAtPath,
  GetOutputAtPath,
  TypedBatchResult,
} from "../core/inference";
import { executeBatch } from "./call";

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
// TypedBatchBuilder
// =============================================================================

/**
 * Type-safe batch builder that infers paths and inputs from a contract.
 *
 * @example
 * ```typescript
 * const response = await createTypedBatch<AppContract>()
 *   .add('health-check', 'health', undefined)
 *   .add('user-1', 'user.get', { id: 1 })
 *   .execute();
 *
 * const healthResult = response.getResult('health-check');
 * ```
 */
export class TypedBatchBuilder<
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
  ): TypedBatchBuilder<
    TContract,
    TOutputMap & Record<TId, GetOutputAtPath<TContract, TPath>>
  > {
    this.entries.push({ id, path, input });
    return this as unknown as TypedBatchBuilder<
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
  clear(): TypedBatchBuilder<TContract, Record<string, never>> {
    this.entries = [];
    return this as unknown as TypedBatchBuilder<
      TContract,
      Record<string, never>
    >;
  }

  /**
   * Execute the batch and return a typed response.
   */
  async execute(
    options?: BatchCallOptions,
  ): Promise<TypedBatchResponseWrapper<TOutputMap>> {
    const response = await executeBatch(this.getRequests(), options);
    return new TypedBatchResponseWrapper<TOutputMap>(response);
  }
}

// =============================================================================
// TypedBatchResponseWrapper
// =============================================================================

/**
 * Type-safe batch response with helper methods to get typed results.
 */
export class TypedBatchResponseWrapper<TOutputMap extends OutputTypeMap> {
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
   * Get a typed result by request ID.
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

// Backwards compatibility alias
export { TypedBatchResponseWrapper as TypedBatchResponse };
