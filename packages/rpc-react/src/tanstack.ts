// =============================================================================
// TanStack Query Integration for RPC Client
// =============================================================================
// Provides a utility wrapper around the RPC client that generates
// TanStack Query options automatically, similar to oRPC's approach.

import type { QueryKey } from "@tanstack/react-query";

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
    : (opts: {
        input: TInput;
        enabled?: boolean;
      }) => QueryOptionsResult<TOutput>;

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
  call: TInput extends void
    ? () => Promise<TOutput>
    : (input: TInput) => Promise<TOutput>;
}

/** Mutation procedure utils interface */
interface MutationProcedureUtils<TInput, TOutput> {
  mutationOptions: () => MutationOptionsResult<TInput, TOutput>;
  mutationKey: () => QueryKey;
  key: () => QueryKey;
  call: TInput extends void
    ? () => Promise<TOutput>
    : (input: TInput) => Promise<TOutput>;
}

/** Convert a contract router to TanStack Query utils */
export type TanstackQueryUtils<TContract> = {
  [K in keyof TContract]: TContract[K] extends {
    type: "query" | "mutation" | "subscription";
  }
    ? InferProcedureType<TContract[K]> extends "query"
      ? QueryProcedureUtils<InferInput<TContract[K]>, InferOutput<TContract[K]>>
      : InferProcedureType<TContract[K]> extends "mutation"
        ? MutationProcedureUtils<
            InferInput<TContract[K]>,
            InferOutput<TContract[K]>
          >
        : never
    : TContract[K] extends object
      ? TanstackQueryUtils<TContract[K]> & {
          key: (opts?: KeyOptions) => QueryKey;
        }
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
export function createTanstackQueryUtils<TContract extends object>(
  client: unknown,
  options: CreateTanstackQueryUtilsOptions = {},
): TanstackQueryUtils<TContract> {
  const basePath = options.path ?? [];

  function createUtils(target: unknown, currentPath: string[]): unknown {
    // For proxy-based clients, we can't distinguish between namespaces and procedures
    // by checking typeof, because the proxy returns callable proxies for everything.
    // Instead, we create a proxy that provides both namespace traversal AND procedure utils.
    // The procedure utils (queryOptions, mutationOptions, etc.) are available at every level,
    // and they use the current path to call the underlying client.

    return new Proxy(
      {},
      {
        get(_, prop: string) {
          // Handle special keys
          if (prop === "key") {
            return (opts?: KeyOptions) => {
              if (opts?.input !== undefined) {
                return [...currentPath, opts.input];
              }
              return currentPath;
            };
          }

          // Procedure utils - these are available at every level
          if (prop === "queryOptions") {
            return (opts?: { input?: unknown; enabled?: boolean }) => ({
              queryKey:
                opts?.input !== undefined
                  ? [...currentPath, opts.input]
                  : currentPath,
              queryFn: () => {
                const fn = getClientFn(target, currentPath);
                return opts?.input !== undefined ? fn(opts.input) : fn();
              },
              enabled: opts?.enabled,
            });
          }

          if (prop === "queryKey") {
            return (opts?: { input?: unknown }) =>
              opts?.input !== undefined
                ? [...currentPath, opts.input]
                : currentPath;
          }

          if (prop === "infiniteOptions") {
            return <TPageParam>(opts: {
              input: (pageParam: TPageParam) => unknown;
              initialPageParam: TPageParam;
              getNextPageParam: (lastPage: unknown) => TPageParam | undefined;
              getPreviousPageParam?: (
                firstPage: unknown,
              ) => TPageParam | undefined;
              enabled?: boolean;
            }) => ({
              queryKey: [...currentPath, "infinite"],
              queryFn: ({ pageParam }: { pageParam: TPageParam }) => {
                const fn = getClientFn(target, currentPath);
                return fn(opts.input(pageParam));
              },
              initialPageParam: opts.initialPageParam,
              getNextPageParam: opts.getNextPageParam,
              getPreviousPageParam: opts.getPreviousPageParam,
              enabled: opts.enabled,
            });
          }

          if (prop === "infiniteKey") {
            return (opts?: { input?: unknown }) =>
              opts?.input !== undefined
                ? [...currentPath, "infinite", opts.input]
                : [...currentPath, "infinite"];
          }

          if (prop === "mutationOptions") {
            return () => ({
              mutationKey: currentPath,
              mutationFn: (input: unknown) => {
                const fn = getClientFn(target, currentPath);
                return fn(input);
              },
            });
          }

          if (prop === "mutationKey") {
            return () => currentPath;
          }

          if (prop === "call") {
            return (input?: unknown) => {
              const fn = getClientFn(target, currentPath);
              return input !== undefined ? fn(input) : fn();
            };
          }

          // For any other property, recurse into the namespace
          const nextPath = [...currentPath, prop];
          const clientProp = (target as Record<string, unknown>)[prop];

          // Skip undefined/null
          if (clientProp === null || clientProp === undefined) {
            return undefined;
          }

          return createUtils(clientProp, nextPath);
        },
      },
    );
  }

  /**
   * Get the callable function from the client at the current path.
   * For proxy-based clients, the target itself is callable.
   * For plain object clients, we need to traverse to find the function.
   */
  function getClientFn(
    target: unknown,
    path: string[],
  ): (input?: unknown) => Promise<unknown> {
    // If path is empty, target itself should be callable (for proxy clients at root)
    if (path.length === 0) {
      if (typeof target === "function") {
        return target as (input?: unknown) => Promise<unknown>;
      }
      throw new Error("Cannot call root of client");
    }

    // For proxy-based clients, the target is already the callable for this path
    if (typeof target === "function") {
      return target as (input?: unknown) => Promise<unknown>;
    }

    // For plain object clients, traverse the path
    let current: unknown = target;
    for (const segment of path) {
      if (current === null || current === undefined) {
        throw new Error(`Cannot find procedure at path: ${path.join(".")}`);
      }
      current = (current as Record<string, unknown>)[segment];
    }

    if (typeof current !== "function") {
      throw new Error(`Path ${path.join(".")} is not a callable procedure`);
    }

    return current as (input?: unknown) => Promise<unknown>;
  }

  return createUtils(client, basePath) as TanstackQueryUtils<TContract>;
}
