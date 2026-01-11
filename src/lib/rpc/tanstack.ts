// =============================================================================
// TanStack Query Integration for RPC Client
// =============================================================================
// Provides a utility wrapper around the RPC client that generates
// TanStack Query options automatically, similar to oRPC's approach.

import type { QueryKey } from "@tanstack/react-query";
import type { ContractRouter } from "./types";

// =============================================================================
// Types
// =============================================================================

export interface QueryOptionsResult<TOutput> {
  queryKey: QueryKey;
  queryFn: () => Promise<TOutput>;
  enabled?: boolean;
}

export interface MutationOptionsResult<TInput, TOutput> {
  mutationKey: QueryKey;
  mutationFn: (input: TInput) => Promise<TOutput>;
}

export interface InfiniteOptionsResult<TOutput, TPageParam> {
  queryKey: QueryKey;
  queryFn: (context: { pageParam: TPageParam }) => Promise<TOutput>;
  initialPageParam: TPageParam;
  getNextPageParam: (lastPage: TOutput) => TPageParam | undefined;
  getPreviousPageParam?: (firstPage: TOutput) => TPageParam | undefined;
  enabled?: boolean;
}

export interface KeyOptions<TInput = unknown> {
  input?: TInput;
  type?: "query" | "mutation" | "infinite";
}

// =============================================================================
// Type-Level Utils for Contract Traversal
// =============================================================================

type InferInput<T> = T extends { input: infer I } ? I : never;
type InferOutput<T> = T extends { output: infer O } ? O : never;
type InferProcedureType<T> = T extends { type: infer P } ? P : never;

/** Query procedure utils interface */
interface QueryProcedureUtils<TInput, TOutput> {
  queryOptions: TInput extends void
    ? (opts?: { enabled?: boolean }) => QueryOptionsResult<TOutput>
    : (opts: { input: TInput; enabled?: boolean }) => QueryOptionsResult<TOutput>;
  
  infiniteOptions: <TPageParam = unknown>(opts: {
    input: (pageParam: TPageParam) => TInput;
    initialPageParam: TPageParam;
    getNextPageParam: (lastPage: TOutput) => TPageParam | undefined;
    getPreviousPageParam?: (firstPage: TOutput) => TPageParam | undefined;
    enabled?: boolean;
  }) => {
    queryKey: QueryKey;
    queryFn: (context: { pageParam: TPageParam }) => Promise<TOutput>;
    initialPageParam: TPageParam;
    getNextPageParam: (lastPage: TOutput) => TPageParam | undefined;
    getPreviousPageParam?: (firstPage: TOutput) => TPageParam | undefined;
    enabled?: boolean;
  };

  infiniteKey: (opts?: { input?: TInput }) => QueryKey;
  
  queryKey: TInput extends void
    ? () => QueryKey
    : (opts: { input: TInput }) => QueryKey;
  key: (opts?: KeyOptions<TInput>) => QueryKey;
  call: TInput extends void ? () => Promise<TOutput> : (input: TInput) => Promise<TOutput>;
}

/** Mutation procedure utils interface */
interface MutationProcedureUtils<TInput, TOutput> {
  mutationOptions: () => MutationOptionsResult<TInput, TOutput>;
  mutationKey: () => QueryKey;
  key: () => QueryKey;
  call: TInput extends void ? () => Promise<TOutput> : (input: TInput) => Promise<TOutput>;
}

/** Convert a contract router to TanStack Query utils */
export type TanstackQueryUtils<TContract> = {
  [K in keyof TContract]: TContract[K] extends { type: "query" | "mutation" | "subscription" }
    ? InferProcedureType<TContract[K]> extends "query"
      ? QueryProcedureUtils<InferInput<TContract[K]>, InferOutput<TContract[K]>>
      : InferProcedureType<TContract[K]> extends "mutation"
        ? MutationProcedureUtils<InferInput<TContract[K]>, InferOutput<TContract[K]>>
        : never
    : TContract[K] extends object
      ? TanstackQueryUtils<TContract[K]> & { key: (opts?: KeyOptions) => QueryKey }
      : never;
} & {
  key: (opts?: KeyOptions) => QueryKey;
};

// =============================================================================
// Create TanStack Query Utils
// =============================================================================

export interface CreateTanstackQueryUtilsOptions {
  /** Base path for query keys (useful for avoiding conflicts) */
  path?: string[];
}

/**
 * Create TanStack Query utilities from an RPC client.
 * 
 * @example
 * ```typescript
 * const orpc = createTanstackQueryUtils(rpc);
 * 
 * // Query options
 * const query = useQuery(orpc.user.get.queryOptions({ input: { id: 1 } }));
 * 
 * // Infinite query options
 * const infinite = useInfiniteQuery(orpc.user.list.infiniteOptions({
 *   input: (pageParam) => ({ limit: 10, offset: pageParam }),
 *   initialPageParam: 0,
 *   getNextPageParam: (lastPage) => lastPage.nextOffset,
 * }));
 * 
 * // Mutation options
 * const mutation = useMutation(orpc.user.create.mutationOptions());
 * 
 * // Cache invalidation
 * queryClient.invalidateQueries({ queryKey: orpc.user.key() });
 * 
 * // Direct call
 * const user = await orpc.user.get.call({ id: 1 });
 * ```
 */
export function createTanstackQueryUtils<TContract extends ContractRouter>(
  client: unknown,
  options: CreateTanstackQueryUtilsOptions = {}
): TanstackQueryUtils<TContract> {
  const basePath = options.path ?? [];

  function createUtils(target: unknown, currentPath: string[]): unknown {
    return new Proxy(
      {},
      {
        get(_, prop: string) {
          if (prop === "key") {
            return (opts?: KeyOptions) => {
              if (opts?.input !== undefined) {
                return [...currentPath, opts.input];
              }
              return currentPath;
            };
          }

          const nextPath = [...currentPath, prop];
          const clientProp = (target as Record<string, unknown>)[prop];

          // If it's a function, it's a procedure
          if (typeof clientProp === "function") {
            const procedureFn = clientProp as (input?: unknown) => Promise<unknown>;

            return {
              // Query utils
              queryOptions: (opts?: { input?: unknown; enabled?: boolean }) => ({
                queryKey: opts?.input !== undefined ? [...nextPath, opts.input] : nextPath,
                queryFn: () =>
                  opts?.input !== undefined
                    ? procedureFn(opts.input)
                    : procedureFn(),
                enabled: opts?.enabled,
              }),
              queryKey: (opts?: { input?: unknown }) =>
                opts?.input !== undefined ? [...nextPath, opts.input] : nextPath,

              // Infinite query utils
              infiniteOptions: <TPageParam>(opts: {
                input: (pageParam: TPageParam) => unknown;
                initialPageParam: TPageParam;
                getNextPageParam: (lastPage: unknown) => TPageParam | undefined;
                getPreviousPageParam?: (firstPage: unknown) => TPageParam | undefined;
                enabled?: boolean;
              }) => ({
                queryKey: [...nextPath, "infinite"],
                queryFn: ({ pageParam }: { pageParam: TPageParam }) => 
                  procedureFn(opts.input(pageParam)),
                initialPageParam: opts.initialPageParam,
                getNextPageParam: opts.getNextPageParam,
                getPreviousPageParam: opts.getPreviousPageParam,
                enabled: opts.enabled,
              }),
              infiniteKey: (opts?: { input?: unknown }) =>
                opts?.input !== undefined 
                  ? [...nextPath, "infinite", opts.input] 
                  : [...nextPath, "infinite"],

              // Mutation utils
              mutationOptions: () => ({
                mutationKey: nextPath,
                mutationFn: (input: unknown) => procedureFn(input),
              }),
              mutationKey: () => nextPath,

              // Common utils
              key: (opts?: KeyOptions) => {
                if (opts?.input !== undefined) {
                  return [...nextPath, opts.input];
                }
                return nextPath;
              },
              call: procedureFn,
            };
          }

          // If it's an object, recurse
          if (typeof clientProp === "object" && clientProp !== null) {
            return createUtils(clientProp, nextPath);
          }

          return undefined;
        },
      }
    );
  }

  return createUtils(client, basePath) as TanstackQueryUtils<TContract>;
}
