// =============================================================================
// Cache Strategies - Advanced caching patterns
// =============================================================================

import { Effect, Ref, Duration } from "effect";
import type { RpcEffectError } from "../core/errors";
import { RpcCacheService, createCacheKey, serializeCacheKey } from "./cache";

// =============================================================================
// Stale-While-Revalidate
// =============================================================================

/**
 * SWR configuration.
 */
export interface SWRConfig {
  /** Time after which data is considered stale */
  readonly staleAfter: Duration.Duration;
  /** Maximum age before data must be revalidated */
  readonly maxAge: Duration.Duration;
}

/**
 * SWR cache entry.
 */
interface SWREntry<A> {
  readonly data: A;
  readonly fetchedAt: number;
  readonly revalidating: boolean;
}

/**
 * Wrap an effect with stale-while-revalidate caching.
 * Returns stale data immediately while revalidating in the background.
 */
export const withStaleWhileRevalidate = <A, R>(
  path: string,
  input: unknown,
  effect: Effect.Effect<A, RpcEffectError, R>,
  config: SWRConfig,
): Effect.Effect<A, RpcEffectError, R> =>
  Effect.gen(function* () {
    // Create a local cache ref for SWR entries
    // In production, this would be part of the cache service
    const cacheRef = yield* Ref.make<Map<string, SWREntry<A>>>(new Map());
    const key = serializeCacheKey(createCacheKey(path, input));
    const now = Date.now();

    const cache = yield* Ref.get(cacheRef);
    const entry = cache.get(key);

    if (entry) {
      const age = now - entry.fetchedAt;
      const staleMs = Duration.toMillis(config.staleAfter);
      const maxAgeMs = Duration.toMillis(config.maxAge);

      // Data is fresh
      if (age < staleMs) {
        return entry.data;
      }

      // Data is stale but within max age - return stale and revalidate
      if (age < maxAgeMs && !entry.revalidating) {
        // Mark as revalidating
        yield* Ref.update(cacheRef, (c) => {
          const newCache = new Map(c);
          newCache.set(key, { ...entry, revalidating: true });
          return newCache;
        });

        // Revalidate in background
        yield* Effect.fork(
          effect.pipe(
            Effect.tap((data) =>
              Ref.update(cacheRef, (c) => {
                const newCache = new Map(c);
                newCache.set(key, {
                  data,
                  fetchedAt: Date.now(),
                  revalidating: false,
                });
                return newCache;
              }),
            ),
            Effect.catchAll(() =>
              Ref.update(cacheRef, (c) => {
                const newCache = new Map(c);
                const current = newCache.get(key);
                if (current) {
                  newCache.set(key, { ...current, revalidating: false });
                }
                return newCache;
              }),
            ),
          ),
        );

        return entry.data;
      }
    }

    // No cache or expired - fetch fresh data
    const data = yield* effect;

    yield* Ref.update(cacheRef, (c) => {
      const newCache = new Map(c);
      newCache.set(key, {
        data,
        fetchedAt: Date.now(),
        revalidating: false,
      });
      return newCache;
    });

    return data;
  });

// =============================================================================
// Cache Warming
// =============================================================================

/**
 * Cache warming configuration.
 */
export interface CacheWarmingConfig {
  /** Paths to warm */
  readonly paths: readonly {
    path: string;
    inputs: readonly unknown[];
  }[];
  /** Concurrency for warming requests */
  readonly concurrency: number;
  /** Whether to fail on any error */
  readonly failFast: boolean;
}

/**
 * Warm the cache with specified entries.
 */
export const warmCache = <A, R>(
  fetcher: (
    path: string,
    input: unknown,
  ) => Effect.Effect<A, RpcEffectError, R>,
  config: CacheWarmingConfig,
): Effect.Effect<
  { succeeded: number; failed: number },
  RpcEffectError,
  R | RpcCacheService
> =>
  Effect.gen(function* () {
    const requests = config.paths.flatMap(({ path, inputs }) =>
      inputs.map((input) => ({ path, input })),
    );

    let succeeded = 0;
    let failed = 0;

    if (config.failFast) {
      yield* Effect.forEach(
        requests,
        ({ path, input }) =>
          fetcher(path, input).pipe(
            Effect.tap(() =>
              Effect.sync(() => {
                succeeded++;
              }),
            ),
          ),
        { concurrency: config.concurrency },
      );
    } else {
      yield* Effect.forEach(
        requests,
        ({ path, input }) =>
          fetcher(path, input).pipe(
            Effect.tap(() =>
              Effect.sync(() => {
                succeeded++;
              }),
            ),
            Effect.catchAll(() =>
              Effect.sync(() => {
                failed++;
              }),
            ),
          ),
        { concurrency: config.concurrency },
      );
    }

    return { succeeded, failed };
  });
