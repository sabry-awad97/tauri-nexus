// =============================================================================
// Cache Module - Effect Cache-based Request Caching
// =============================================================================
// Provides request caching using Effect's Cache system.

export {
  // Cache service
  RpcCacheService,
  type RpcCacheConfig,
  type CacheKey,
  createRpcCache,
  createRpcCacheLayer,
  // Cache combinators
  withCache,
  withCacheInvalidation,
  invalidateCache,
  invalidateCacheByPath,
  // Cache key utilities
  createCacheKey,
  serializeCacheKey,
} from "./cache";

export {
  // Stale-while-revalidate
  withStaleWhileRevalidate,
  type SWRConfig,
  // Cache warming
  warmCache,
  type CacheWarmingConfig,
} from "./strategies";
