// =============================================================================
// Cache - Effect Cache-based Request Caching
// =============================================================================

import { Effect, Context, Layer, Duration, Ref, HashMap } from "effect";
import { stableStringify } from "../utils/serialize";

// =============================================================================
// Types
// =============================================================================

/**
 * Cache key structure.
 */
export interface CacheKey {
  readonly path: string;
  readonly input: unknown;
}

/**
 * Cache configuration.
 */
export interface RpcCacheConfig {
  /** Maximum number of cached entries */
  readonly capacity: number;
  /** Time to live for cache entries */
  readonly timeToLive: Duration.Duration;
  /** Time to idle (unused entries expire) */
  readonly timeToIdle?: Duration.Duration;
}

// =============================================================================
// Default Configuration
// =============================================================================

const defaultConfig: RpcCacheConfig = {
  capacity: 1000,
  timeToLive: Duration.minutes(5),
};

// =============================================================================
// Cache Key Utilities
// =============================================================================

/**
 * Create a cache key from path and input.
 */
export const createCacheKey = (path: string, input: unknown): CacheKey => ({
  path,
  input,
});

/**
 * Serialize a cache key to a string.
 */
export const serializeCacheKey = (key: CacheKey): string =>
  `${key.path}:${stableStringify(key.input)}`;

// =============================================================================
// Simple In-Memory Cache
// =============================================================================

/**
 * Cache entry with expiration.
 */
interface CacheEntry<A> {
  readonly value: A;
  readonly expiresAt: number;
}

/**
 * Simple cache state.
 */
interface SimpleCacheState<A> {
  readonly entries: Map<string, CacheEntry<A>>;
}

// =============================================================================
// Cache Service
// =============================================================================

/**
 * RPC cache service interface.
 */
export interface RpcCacheServiceInterface {
  readonly cacheRef: Ref.Ref<SimpleCacheState<unknown>>;
  readonly invalidationRef: Ref.Ref<HashMap.HashMap<string, number>>;
  readonly config: RpcCacheConfig;
}

/**
 * RPC cache service.
 */
export class RpcCacheService extends Context.Tag("RpcCacheService")<
  RpcCacheService,
  RpcCacheServiceInterface
>() {}

// =============================================================================
// Cache Creation
// =============================================================================

/**
 * Create an RPC cache instance.
 */
export const createRpcCache = (
  config: Partial<RpcCacheConfig> = {},
): Effect.Effect<RpcCacheServiceInterface> =>
  Effect.gen(function* () {
    const fullConfig = { ...defaultConfig, ...config };

    const cacheRef = yield* Ref.make<SimpleCacheState<unknown>>({
      entries: new Map(),
    });

    const invalidationRef = yield* Ref.make(HashMap.empty<string, number>());

    return {
      cacheRef,
      invalidationRef,
      config: fullConfig,
    };
  });

/**
 * Create an RPC cache layer.
 */
export const createRpcCacheLayer = (
  config: Partial<RpcCacheConfig> = {},
): Layer.Layer<RpcCacheService> =>
  Layer.effect(RpcCacheService, createRpcCache(config));

// =============================================================================
// Cache Operations
// =============================================================================

/**
 * Get a value from cache.
 */
const getCached = <A>(
  cacheRef: Ref.Ref<SimpleCacheState<unknown>>,
  key: string,
): Effect.Effect<A | undefined> =>
  Effect.gen(function* () {
    const state = yield* Ref.get(cacheRef);
    const entry = state.entries.get(key);

    if (!entry) return undefined;

    if (entry.expiresAt < Date.now()) {
      // Entry expired, remove it
      yield* Ref.update(cacheRef, (s) => {
        const newEntries = new Map(s.entries);
        newEntries.delete(key);
        return { entries: newEntries };
      });
      return undefined;
    }

    return entry.value as A;
  });

/**
 * Set a value in cache.
 */
const setCached = <A>(
  cacheRef: Ref.Ref<SimpleCacheState<unknown>>,
  key: string,
  value: A,
  ttl: Duration.Duration,
): Effect.Effect<void> =>
  Ref.update(cacheRef, (state) => {
    const newEntries = new Map(state.entries);
    newEntries.set(key, {
      value,
      expiresAt: Date.now() + Duration.toMillis(ttl),
    });
    return { entries: newEntries };
  });

/**
 * Remove a value from cache.
 */
const removeCached = (
  cacheRef: Ref.Ref<SimpleCacheState<unknown>>,
  key: string,
): Effect.Effect<void> =>
  Ref.update(cacheRef, (state) => {
    const newEntries = new Map(state.entries);
    newEntries.delete(key);
    return { entries: newEntries };
  });

// =============================================================================
// Cache Combinators
// =============================================================================

/**
 * Wrap an effect with caching.
 */
export const withCache = <A, E, R>(
  path: string,
  input: unknown,
  effect: Effect.Effect<A, E, R>,
): Effect.Effect<A, E, R | RpcCacheService> =>
  Effect.gen(function* () {
    const { cacheRef, invalidationRef, config } = yield* RpcCacheService;
    const key = serializeCacheKey(createCacheKey(path, input));

    // Check if cache entry is invalidated
    const invalidations = yield* Ref.get(invalidationRef);
    const invalidatedAt = HashMap.get(invalidations, key);

    if (invalidatedAt._tag === "Some") {
      yield* removeCached(cacheRef, key);
      yield* Ref.update(invalidationRef, HashMap.remove(key));
    }

    // Check cache
    const cached = yield* getCached<A>(cacheRef, key);
    if (cached !== undefined) {
      return cached;
    }

    // Execute effect and cache result
    const result = yield* effect;
    yield* setCached(cacheRef, key, result, config.timeToLive);

    return result;
  });

/**
 * Wrap an effect with cache and automatic invalidation on mutation.
 */
export const withCacheInvalidation = <A, E, R>(
  _path: string,
  _input: unknown,
  effect: Effect.Effect<A, E, R>,
  invalidatePaths: readonly string[] = [],
): Effect.Effect<A, E, R | RpcCacheService> =>
  Effect.gen(function* () {
    const result = yield* effect;

    // Invalidate related cache entries
    if (invalidatePaths.length > 0) {
      yield* Effect.forEach(invalidatePaths, (p) => invalidateCacheByPath(p), {
        concurrency: "unbounded",
        discard: true,
      });
    }

    return result;
  });

/**
 * Invalidate a specific cache entry.
 */
export const invalidateCache = (
  path: string,
  input: unknown,
): Effect.Effect<void, never, RpcCacheService> =>
  Effect.gen(function* () {
    const { cacheRef } = yield* RpcCacheService;
    const key = serializeCacheKey(createCacheKey(path, input));
    yield* removeCached(cacheRef, key);
  });

/**
 * Invalidate all cache entries for a path.
 */
export const invalidateCacheByPath = (
  path: string,
): Effect.Effect<void, never, RpcCacheService> =>
  Effect.gen(function* () {
    const { cacheRef } = yield* RpcCacheService;

    yield* Ref.update(cacheRef, (state) => {
      const newEntries = new Map<string, CacheEntry<unknown>>();
      for (const [key, value] of state.entries) {
        if (!key.startsWith(`${path}:`)) {
          newEntries.set(key, value);
        }
      }
      return { entries: newEntries };
    });
  });
