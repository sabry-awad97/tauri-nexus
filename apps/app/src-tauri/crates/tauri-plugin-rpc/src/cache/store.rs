//! Cache store implementation

use lru::LruCache;
use serde::{Deserialize, Serialize};
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use super::config::CacheConfig;
use super::entry::{CacheEntry, generate_cache_key};
use super::error::{CacheError, CacheResult};
use super::metrics::CacheMetrics;
use super::pattern::pattern_matches;

#[cfg(test)]
mod test_constants {
    use std::time::Duration;

    /// Short TTL for expiration tests (50ms)
    pub const SHORT_TTL: Duration = Duration::from_millis(50);

    /// Long TTL for non-expiration tests (5 minutes)
    pub const LONG_TTL: Duration = Duration::from_secs(300);

    /// Wait time for cleanup operations in tests (100ms)
    pub const CLEANUP_WAIT: Duration = Duration::from_millis(100);
}

/// Thread-safe LRU cache with TTL support
pub struct Cache {
    pub(crate) config: CacheConfig,
    pub(crate) entries: Arc<RwLock<LruCache<String, CacheEntry>>>,
    pub(crate) metrics: Arc<CacheMetrics>,
}

impl Cache {
    /// Create a new cache with the given configuration
    pub fn new(config: CacheConfig) -> Self {
        let max_entries = NonZeroUsize::new(config.max_entries.max(1)).unwrap();
        Self {
            config,
            entries: Arc::new(RwLock::new(LruCache::new(max_entries))),
            metrics: Arc::new(CacheMetrics::new()),
        }
    }

    /// Get a cached value if it exists and hasn't expired (with error handling)
    ///
    /// Returns `Err(CacheError::CacheDisabled)` if caching is disabled.
    /// Returns `Ok(Some(value))` if the entry exists and is valid.
    /// Returns `Ok(None)` if the entry doesn't exist or has expired.
    #[tracing::instrument(skip(self, input), fields(enabled = %self.config.enabled))]
    pub async fn try_get(
        &self,
        path: &str,
        input: &serde_json::Value,
    ) -> CacheResult<Option<serde_json::Value>> {
        if !self.config.enabled {
            tracing::trace!("cache disabled");
            return Err(CacheError::CacheDisabled);
        }

        let key = generate_cache_key(path, input);
        tracing::trace!(key = %key, "generated cache key");

        let mut entries = self.entries.write().await;

        if let Some(entry) = entries.get(&key) {
            if entry.is_expired() {
                tracing::debug!("cache entry expired");
                entries.pop(&key);
                self.metrics.record_miss();
                return Ok(None);
            }
            tracing::debug!("cache hit");
            self.metrics.record_hit();
            return Ok(Some(entry.value.clone()));
        }

        tracing::debug!("cache miss");
        self.metrics.record_miss();
        Ok(None)
    }

    /// Get a cached value if it exists and hasn't expired
    ///
    /// This is a convenience wrapper around `try_get()` that returns `None`
    /// on any error. For explicit error handling, use `try_get()` instead.
    #[tracing::instrument(skip(self, input), fields(enabled = %self.config.enabled))]
    pub async fn get(&self, path: &str, input: &serde_json::Value) -> Option<serde_json::Value> {
        self.try_get(path, input).await.ok().flatten()
    }

    /// Set a cached value with the configured TTL for the procedure (with error handling)
    ///
    /// Returns `Err(CacheError::CacheDisabled)` if caching is disabled.
    /// Returns `Ok(())` if the value was successfully cached or if the path is excluded.
    #[tracing::instrument(skip(self, input, value), fields(enabled = %self.config.enabled))]
    pub async fn try_set(
        &self,
        path: &str,
        input: &serde_json::Value,
        value: serde_json::Value,
    ) -> CacheResult<()> {
        if !self.config.enabled {
            tracing::trace!("cache disabled");
            return Err(CacheError::CacheDisabled);
        }

        if !self.config.should_cache(path) {
            tracing::trace!("path excluded from caching");
            return Ok(());
        }

        // Lock contention optimization: Perform all computation before acquiring the write lock.
        // This minimizes the critical section and improves concurrent throughput.
        let key = generate_cache_key(path, input);
        let ttl = self.config.get_ttl(path);
        let entry = CacheEntry::new(value, ttl);

        // Acquire write lock only for the actual cache modification
        let mut entries = self.entries.write().await;

        // Check if we're at capacity and the key doesn't exist (will cause eviction)
        // Note: This check must be inside the lock to ensure atomicity
        let will_evict = entries.len() >= entries.cap().get() && !entries.contains(&key);

        entries.put(key, entry);

        if will_evict {
            tracing::debug!("LRU eviction occurred");
            self.metrics.record_eviction();
        }

        tracing::trace!(ttl_ms = %ttl.as_millis(), "cache entry stored");
        Ok(())
    }

    /// Set a cached value with the configured TTL for the procedure
    ///
    /// This is a convenience wrapper around `try_set()` that silently ignores errors.
    /// For explicit error handling, use `try_set()` instead.
    #[tracing::instrument(skip(self, input, value), fields(enabled = %self.config.enabled))]
    pub async fn set(&self, path: &str, input: &serde_json::Value, value: serde_json::Value) {
        let _ = self.try_set(path, input, value).await;
    }

    /// Set a cached value with a custom TTL (with error handling)
    ///
    /// Returns `Err(CacheError::CacheDisabled)` if caching is disabled.
    /// Returns `Ok(())` if the value was successfully cached.
    pub async fn try_set_with_ttl(
        &self,
        path: &str,
        input: &serde_json::Value,
        value: serde_json::Value,
        ttl: Duration,
    ) -> CacheResult<()> {
        if !self.config.enabled {
            return Err(CacheError::CacheDisabled);
        }

        // Lock contention optimization: Perform all computation before acquiring the write lock
        let key = generate_cache_key(path, input);
        let entry = CacheEntry::new(value, ttl);

        // Acquire write lock only for the actual cache modification
        let mut entries = self.entries.write().await;

        // Check if we're at capacity and the key doesn't exist (will cause eviction)
        // Note: This check must be inside the lock to ensure atomicity
        let will_evict = entries.len() >= entries.cap().get() && !entries.contains(&key);

        entries.put(key, entry);

        if will_evict {
            self.metrics.record_eviction();
        }

        Ok(())
    }

    /// Set a cached value with a custom TTL
    ///
    /// This is a convenience wrapper around `try_set_with_ttl()` that silently ignores errors.
    /// For explicit error handling, use `try_set_with_ttl()` instead.
    pub async fn set_with_ttl(
        &self,
        path: &str,
        input: &serde_json::Value,
        value: serde_json::Value,
        ttl: Duration,
    ) {
        let _ = self.try_set_with_ttl(path, input, value, ttl).await;
    }

    /// Invalidate a specific cache entry
    #[tracing::instrument(skip(self, input))]
    pub async fn invalidate(&self, path: &str, input: &serde_json::Value) {
        let key = generate_cache_key(path, input);
        let mut entries = self.entries.write().await;
        if entries.pop(&key).is_some() {
            tracing::debug!("cache entry invalidated");
            self.metrics.record_invalidation();
        } else {
            tracing::trace!("cache entry not found for invalidation");
        }
    }

    /// Invalidate all entries matching a pattern
    #[tracing::instrument(skip(self))]
    pub async fn invalidate_pattern(&self, pattern: &str) {
        let mut entries = self.entries.write().await;

        // Collect keys to remove (can't modify while iterating)
        let keys_to_remove: Vec<String> = entries
            .iter()
            .filter_map(|(key, _)| {
                // Extract path from key (format: "path:input")
                let path = key.split(':').next()?;
                if pattern_matches(pattern, path) {
                    Some(key.clone())
                } else {
                    None
                }
            })
            .collect();

        let count = keys_to_remove.len() as u64;
        for key in keys_to_remove {
            entries.pop(&key);
        }

        if count > 0 {
            tracing::debug!(count = %count, "cache entries invalidated");
            self.metrics.record_invalidations(count);
        } else {
            tracing::trace!("no matching entries found for pattern");
        }
    }

    /// Invalidate all cache entries
    #[tracing::instrument(skip(self))]
    pub async fn invalidate_all(&self) {
        let mut entries = self.entries.write().await;
        let count = entries.len() as u64;
        entries.clear();

        if count > 0 {
            tracing::debug!(count = %count, "all cache entries invalidated");
            self.metrics.record_invalidations(count);
        } else {
            tracing::trace!("cache was already empty");
        }
    }

    /// Invalidate multiple specific cache entries efficiently
    ///
    /// This method is more efficient than calling `invalidate()` multiple times
    /// because it acquires the write lock only once for all invalidations.
    ///
    /// # Performance
    ///
    /// - Time complexity: O(n) where n is the number of entries to invalidate
    /// - Lock acquisitions: 1 (regardless of batch size)
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let entries = vec![
    ///     ("user.profile".to_string(), json!({"id": 1})),
    ///     ("user.settings".to_string(), json!({"id": 1})),
    /// ];
    /// cache.invalidate_batch(&entries).await;
    /// ```
    #[tracing::instrument(skip(self, entries), fields(count = entries.len()))]
    pub async fn invalidate_batch(&self, entries: &[(String, serde_json::Value)]) {
        if entries.is_empty() {
            tracing::trace!("empty batch, nothing to invalidate");
            return;
        }

        // Generate all keys before acquiring the lock
        let keys: Vec<String> = entries
            .iter()
            .map(|(path, input)| generate_cache_key(path, input))
            .collect();

        // Single lock acquisition for all removals
        let mut cache = self.entries.write().await;
        let mut invalidated = 0u64;

        for key in keys {
            if cache.pop(&key).is_some() {
                invalidated += 1;
            }
        }

        if invalidated > 0 {
            tracing::debug!(invalidated = %invalidated, total = %entries.len(), "batch invalidation complete");
            self.metrics.record_invalidations(invalidated);
        } else {
            tracing::trace!("no entries found in batch");
        }
    }

    /// Get cache statistics
    pub async fn stats(&self) -> CacheStats {
        let entries = self.entries.read().await;
        let total = entries.len();
        let expired = entries.iter().filter(|(_, e)| e.is_expired()).count();

        CacheStats {
            total_entries: total,
            expired_entries: expired,
            max_entries: self.config.max_entries,
            hits: self.metrics.get_hits(),
            misses: self.metrics.get_misses(),
            hit_ratio: self.metrics.calculate_hit_ratio(),
            evictions: self.metrics.get_evictions(),
            invalidations: self.metrics.get_invalidations(),
        }
    }

    /// Remove expired entries and return the count of removed entries
    ///
    /// This method scans all cache entries and removes those that have expired.
    /// It returns the number of entries that were removed.
    ///
    /// # Performance
    ///
    /// - Time complexity: O(n) where n is the total number of cache entries
    /// - Acquires a write lock for the duration of the operation
    #[tracing::instrument(skip(self))]
    pub async fn cleanup_expired(&self) -> usize {
        let mut entries = self.entries.write().await;

        let expired_keys: Vec<String> = entries
            .iter()
            .filter_map(|(key, entry)| {
                if entry.is_expired() {
                    Some(key.clone())
                } else {
                    None
                }
            })
            .collect();

        let count = expired_keys.len();
        for key in expired_keys {
            entries.pop(&key);
        }

        if count > 0 {
            tracing::debug!(removed = %count, "expired entries cleaned up");
        } else {
            tracing::trace!("no expired entries to clean up");
        }

        count
    }

    /// Start a background task to periodically clean up expired entries
    ///
    /// Returns a `JoinHandle` that can be used to manage the cleanup task lifecycle.
    /// The task will run indefinitely until the handle is dropped or aborted.
    ///
    /// # Arguments
    ///
    /// * `interval` - Duration between cleanup runs
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use std::time::Duration;
    ///
    /// let cache = Cache::new(CacheConfig::new());
    ///
    /// // Start cleanup task that runs every 60 seconds
    /// let cleanup_handle = cache.start_cleanup_task(Duration::from_secs(60));
    ///
    /// // Later, stop the cleanup task
    /// cleanup_handle.abort();
    /// ```
    ///
    /// # Task Lifecycle
    ///
    /// - The task runs in the background using `tokio::spawn`
    /// - It will continue running until explicitly aborted
    /// - Dropping the handle does NOT stop the task
    /// - Use `JoinHandle::abort()` to stop the task
    pub fn start_cleanup_task(&self, interval: Duration) -> tokio::task::JoinHandle<()> {
        let cache = self.clone();

        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(interval);

            loop {
                interval_timer.tick().await;

                let removed = cache.cleanup_expired().await;

                if removed > 0 {
                    tracing::info!(
                        removed = %removed,
                        interval_ms = %interval.as_millis(),
                        "background cleanup completed"
                    );
                }
            }
        })
    }

    /// Check if a value is cached (without retrieving it)
    pub async fn contains(&self, path: &str, input: &serde_json::Value) -> bool {
        if !self.config.enabled {
            return false;
        }

        let key = generate_cache_key(path, input);
        let entries = self.entries.read().await;

        if let Some(entry) = entries.peek(&key) {
            return !entry.is_expired();
        }

        false
    }

    /// Reset all metrics counters to zero
    pub fn reset_metrics(&self) {
        self.metrics.reset();
    }

    /// Get the current cache hit ratio
    ///
    /// Returns a value between 0.0 and 1.0 representing the ratio of cache hits
    /// to total cache accesses (hits + misses). Returns 0.0 if no accesses have
    /// been made.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let cache = Cache::new(CacheConfig::new());
    ///
    /// // Make some cache accesses
    /// cache.set("key", &json!({}), json!("value")).await;
    /// cache.get("key", &json!({})).await; // hit
    /// cache.get("other", &json!({})).await; // miss
    ///
    /// let hit_ratio = cache.get_hit_ratio();
    /// assert_eq!(hit_ratio, 0.5); // 1 hit out of 2 accesses
    /// ```
    pub fn get_hit_ratio(&self) -> f64 {
        self.metrics.calculate_hit_ratio()
    }
}

impl Clone for Cache {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            entries: self.entries.clone(),
            metrics: self.metrics.clone(),
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    /// Total number of entries in the cache
    pub total_entries: usize,
    /// Number of expired entries (not yet cleaned up)
    pub expired_entries: usize,
    /// Maximum number of entries allowed
    pub max_entries: usize,
    /// Number of cache hits (since last reset)
    pub hits: u64,
    /// Number of cache misses (since last reset)
    pub misses: u64,
    /// Hit ratio (0.0 to 1.0)
    pub hit_ratio: f64,
    /// Total evictions due to LRU
    pub evictions: u64,
    /// Total invalidations
    pub invalidations: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::time::Duration;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_cache_config_defaults() {
        let config = CacheConfig::new();
        assert_eq!(
            config.default_ttl,
            Duration::from_secs(super::super::config::DEFAULT_TTL_SECS)
        );
        assert_eq!(
            config.max_entries,
            super::super::config::DEFAULT_MAX_ENTRIES
        );
        assert!(config.enabled);
    }

    #[tokio::test]
    async fn test_cache_config_builder() {
        let config = CacheConfig::new()
            .with_default_ttl(Duration::from_secs(60))
            .with_max_entries(500)
            .with_procedure_ttl("user.profile", Duration::from_secs(30))
            .with_enabled(false)
            .exclude_pattern("admin.*");

        assert_eq!(config.default_ttl, Duration::from_secs(60));
        assert_eq!(config.max_entries, 500);
        assert!(!config.enabled);
        assert_eq!(config.get_ttl("user.profile"), Duration::from_secs(30));
        assert_eq!(config.get_ttl("other"), Duration::from_secs(60));
    }

    #[tokio::test]
    async fn test_cache_set_and_get() {
        let cache = Cache::new(CacheConfig::new());
        let input = json!({"id": 1});
        let value = json!({"name": "Alice"});

        cache.set("user.get", &input, value.clone()).await;

        let cached = cache.get("user.get", &input).await;
        assert_eq!(cached, Some(value));
    }

    #[tokio::test]
    async fn test_cache_miss() {
        let cache = Cache::new(CacheConfig::new());
        let input = json!({"id": 1});

        let cached = cache.get("user.get", &input).await;
        assert_eq!(cached, None);
    }

    #[tokio::test]
    async fn test_cache_expiration() {
        use test_constants::*;

        let config = CacheConfig::new().with_default_ttl(SHORT_TTL);
        let cache = Cache::new(config);
        let input = json!({"id": 1});
        let value = json!({"name": "Alice"});

        cache.set("user.get", &input, value.clone()).await;

        // Should be cached
        assert!(cache.get("user.get", &input).await.is_some());

        // Wait for expiration
        sleep(CLEANUP_WAIT).await;

        // Should be expired
        assert!(cache.get("user.get", &input).await.is_none());
    }

    #[tokio::test]
    async fn test_cache_invalidate_specific() {
        let cache = Cache::new(CacheConfig::new());
        let input1 = json!({"id": 1});
        let input2 = json!({"id": 2});

        cache
            .set("user.get", &input1, json!({"name": "Alice"}))
            .await;
        cache.set("user.get", &input2, json!({"name": "Bob"})).await;

        // Invalidate only input1
        cache.invalidate("user.get", &input1).await;

        assert!(cache.get("user.get", &input1).await.is_none());
        assert!(cache.get("user.get", &input2).await.is_some());
    }

    #[tokio::test]
    async fn test_cache_invalidate_pattern() {
        let cache = Cache::new(CacheConfig::new());

        cache
            .set("user.get", &json!({"id": 1}), json!({"name": "Alice"}))
            .await;
        cache
            .set("user.profile", &json!({"id": 1}), json!({"bio": "Hello"}))
            .await;
        cache
            .set("post.get", &json!({"id": 1}), json!({"title": "Test"}))
            .await;

        // Invalidate all user.* entries
        cache.invalidate_pattern("user.*").await;

        assert!(cache.get("user.get", &json!({"id": 1})).await.is_none());
        assert!(cache.get("user.profile", &json!({"id": 1})).await.is_none());
        assert!(cache.get("post.get", &json!({"id": 1})).await.is_some());
    }

    #[tokio::test]
    async fn test_cache_invalidate_all() {
        let cache = Cache::new(CacheConfig::new());

        cache
            .set("user.get", &json!({"id": 1}), json!({"name": "Alice"}))
            .await;
        cache
            .set("post.get", &json!({"id": 1}), json!({"title": "Test"}))
            .await;

        cache.invalidate_all().await;

        assert!(cache.get("user.get", &json!({"id": 1})).await.is_none());
        assert!(cache.get("post.get", &json!({"id": 1})).await.is_none());
    }

    #[tokio::test]
    async fn test_cache_disabled() {
        let config = CacheConfig::new().with_enabled(false);
        let cache = Cache::new(config);
        let input = json!({"id": 1});
        let value = json!({"name": "Alice"});

        cache.set("user.get", &input, value).await;

        // Should not be cached when disabled
        assert!(cache.get("user.get", &input).await.is_none());
    }

    #[tokio::test]
    async fn test_cache_excluded_pattern() {
        let config = CacheConfig::new().exclude_pattern("admin.*");
        let cache = Cache::new(config);

        cache
            .set("admin.users", &json!({}), json!({"users": []}))
            .await;
        cache
            .set("user.get", &json!({"id": 1}), json!({"name": "Alice"}))
            .await;

        // Admin should not be cached
        assert!(cache.get("admin.users", &json!({})).await.is_none());
        // User should be cached
        assert!(cache.get("user.get", &json!({"id": 1})).await.is_some());
    }

    #[tokio::test]
    async fn test_cache_lru_eviction() {
        let config = CacheConfig::new().with_max_entries(2);
        let cache = Cache::new(config);

        cache.set("a", &json!({}), json!(1)).await;
        cache.set("b", &json!({}), json!(2)).await;
        cache.set("c", &json!({}), json!(3)).await;

        // "a" should be evicted (LRU)
        assert!(cache.get("a", &json!({})).await.is_none());
        assert!(cache.get("b", &json!({})).await.is_some());
        assert!(cache.get("c", &json!({})).await.is_some());
    }

    #[tokio::test]
    async fn test_cache_stats() {
        let cache = Cache::new(CacheConfig::new());

        cache.set("a", &json!({}), json!(1)).await;
        cache.set("b", &json!({}), json!(2)).await;

        let stats = cache.stats().await;
        assert_eq!(stats.total_entries, 2);
        assert_eq!(stats.max_entries, 1000);
    }

    #[tokio::test]
    async fn test_cache_contains() {
        let cache = Cache::new(CacheConfig::new());
        let input = json!({"id": 1});

        assert!(!cache.contains("user.get", &input).await);

        cache
            .set("user.get", &input, json!({"name": "Alice"}))
            .await;

        assert!(cache.contains("user.get", &input).await);
    }

    #[tokio::test]
    async fn test_cache_cleanup_expired() {
        use test_constants::*;

        let config = CacheConfig::new().with_default_ttl(SHORT_TTL);
        let cache = Cache::new(config);

        cache.set("a", &json!({}), json!(1)).await;
        cache.set("b", &json!({}), json!(2)).await;

        // Wait for expiration
        sleep(CLEANUP_WAIT).await;

        let stats_before = cache.stats().await;
        assert_eq!(stats_before.total_entries, 2);
        assert_eq!(stats_before.expired_entries, 2);

        let removed = cache.cleanup_expired().await;
        assert_eq!(removed, 2);

        let stats_after = cache.stats().await;
        assert_eq!(stats_after.total_entries, 0);
    }

    #[tokio::test]
    async fn test_metrics_hit_and_miss() {
        let cache = Cache::new(CacheConfig::new());
        let input = json!({"id": 1});

        // Initial stats should be zero
        let stats = cache.stats().await;
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);
        assert_eq!(stats.hit_ratio, 0.0);

        // Cache miss
        cache.get("user.get", &input).await;
        let stats = cache.stats().await;
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.hit_ratio, 0.0);

        // Set value
        cache
            .set("user.get", &input, json!({"name": "Alice"}))
            .await;

        // Cache hit
        cache.get("user.get", &input).await;
        let stats = cache.stats().await;
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.hit_ratio, 0.5);

        // Another hit
        cache.get("user.get", &input).await;
        let stats = cache.stats().await;
        assert_eq!(stats.hits, 2);
        assert_eq!(stats.misses, 1);
        assert!((stats.hit_ratio - 0.666666).abs() < 0.001);
    }

    #[tokio::test]
    async fn test_metrics_evictions() {
        let config = CacheConfig::new().with_max_entries(2);
        let cache = Cache::new(config);

        // Fill cache
        cache.set("a", &json!({}), json!(1)).await;
        cache.set("b", &json!({}), json!(2)).await;

        let stats = cache.stats().await;
        assert_eq!(stats.evictions, 0);

        // This should evict "a"
        cache.set("c", &json!({}), json!(3)).await;

        let stats = cache.stats().await;
        assert_eq!(stats.evictions, 1);
    }

    #[tokio::test]
    async fn test_metrics_invalidations() {
        let cache = Cache::new(CacheConfig::new());

        cache
            .set("user.get", &json!({"id": 1}), json!({"name": "Alice"}))
            .await;
        cache
            .set("user.profile", &json!({"id": 1}), json!({"bio": "Hello"}))
            .await;
        cache
            .set("post.get", &json!({"id": 1}), json!({"title": "Test"}))
            .await;

        let stats = cache.stats().await;
        assert_eq!(stats.invalidations, 0);

        // Invalidate one entry
        cache.invalidate("user.get", &json!({"id": 1})).await;
        let stats = cache.stats().await;
        assert_eq!(stats.invalidations, 1);

        // Invalidate pattern (should invalidate 1 more: user.profile)
        cache.invalidate_pattern("user.*").await;
        let stats = cache.stats().await;
        assert_eq!(stats.invalidations, 2);

        // Invalidate all (should invalidate 1 more: post.get)
        cache.invalidate_all().await;
        let stats = cache.stats().await;
        assert_eq!(stats.invalidations, 3);
    }

    #[tokio::test]
    async fn test_metrics_reset() {
        let cache = Cache::new(CacheConfig::new());
        let input = json!({"id": 1});

        // Generate some metrics
        cache
            .set("user.get", &input, json!({"name": "Alice"}))
            .await;
        cache.get("user.get", &input).await; // hit
        cache.get("other", &input).await; // miss

        let stats = cache.stats().await;
        assert!(stats.hits > 0 || stats.misses > 0);

        // Reset metrics
        cache.reset_metrics();

        let stats = cache.stats().await;
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);
        assert_eq!(stats.evictions, 0);
        assert_eq!(stats.invalidations, 0);
        assert_eq!(stats.hit_ratio, 0.0);
    }

    #[tokio::test]
    async fn test_metrics_expired_entry_counts_as_miss() {
        use test_constants::*;

        let config = CacheConfig::new().with_default_ttl(SHORT_TTL);
        let cache = Cache::new(config);
        let input = json!({"id": 1});

        cache
            .set("user.get", &input, json!({"name": "Alice"}))
            .await;

        // Wait for expiration
        tokio::time::sleep(CLEANUP_WAIT).await;

        // Accessing expired entry should count as miss
        cache.get("user.get", &input).await;

        let stats = cache.stats().await;
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 1);
    }

    #[tokio::test]
    async fn test_get_hit_ratio_convenience_method() {
        let cache = Cache::new(CacheConfig::new());
        let input = json!({"id": 1});

        // Initial hit ratio should be 0.0
        assert_eq!(cache.get_hit_ratio(), 0.0);

        // Cache miss
        cache.get("user.get", &input).await;
        assert_eq!(cache.get_hit_ratio(), 0.0);

        // Set value
        cache
            .set("user.get", &input, json!({"name": "Alice"}))
            .await;

        // Cache hit
        cache.get("user.get", &input).await;
        assert_eq!(cache.get_hit_ratio(), 0.5);

        // Another hit
        cache.get("user.get", &input).await;
        assert!((cache.get_hit_ratio() - 0.666666).abs() < 0.001);
    }

    #[tokio::test]
    async fn test_try_get_with_disabled_cache() {
        let config = CacheConfig::new().with_enabled(false);
        let cache = Cache::new(config);
        let input = json!({"id": 1});

        let result = cache.try_get("user.get", &input).await;
        assert!(matches!(result, Err(CacheError::CacheDisabled)));
    }

    #[tokio::test]
    async fn test_try_get_success() {
        let cache = Cache::new(CacheConfig::new());
        let input = json!({"id": 1});
        let value = json!({"name": "Alice"});

        // Set value
        cache.set("user.get", &input, value.clone()).await;

        // Try get should return Ok(Some(value))
        let result = cache.try_get("user.get", &input).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some(value));
    }

    #[tokio::test]
    async fn test_try_get_miss() {
        let cache = Cache::new(CacheConfig::new());
        let input = json!({"id": 1});

        // Try get on non-existent entry should return Ok(None)
        let result = cache.try_get("user.get", &input).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);
    }

    #[tokio::test]
    async fn test_try_set_with_disabled_cache() {
        let config = CacheConfig::new().with_enabled(false);
        let cache = Cache::new(config);
        let input = json!({"id": 1});
        let value = json!({"name": "Alice"});

        let result = cache.try_set("user.get", &input, value).await;
        assert!(matches!(result, Err(CacheError::CacheDisabled)));
    }

    #[tokio::test]
    async fn test_try_set_success() {
        let cache = Cache::new(CacheConfig::new());
        let input = json!({"id": 1});
        let value = json!({"name": "Alice"});

        let result = cache.try_set("user.get", &input, value.clone()).await;
        assert!(result.is_ok());

        // Verify it was actually cached
        let cached = cache.get("user.get", &input).await;
        assert_eq!(cached, Some(value));
    }

    #[tokio::test]
    async fn test_try_set_with_excluded_path() {
        let config = CacheConfig::new().exclude_pattern("admin.*");
        let cache = Cache::new(config);
        let input = json!({});
        let value = json!({"users": []});

        // Should return Ok(()) but not actually cache
        let result = cache.try_set("admin.users", &input, value).await;
        assert!(result.is_ok());

        // Verify it was not cached
        let cached = cache.get("admin.users", &input).await;
        assert_eq!(cached, None);
    }

    #[tokio::test]
    async fn test_try_set_with_ttl_disabled_cache() {
        use test_constants::*;

        let config = CacheConfig::new().with_enabled(false);
        let cache = Cache::new(config);
        let input = json!({"id": 1});
        let value = json!({"name": "Alice"});

        let result = cache
            .try_set_with_ttl("user.get", &input, value, LONG_TTL)
            .await;
        assert!(matches!(result, Err(CacheError::CacheDisabled)));
    }

    #[tokio::test]
    async fn test_get_wrapper_ignores_errors() {
        let config = CacheConfig::new().with_enabled(false);
        let cache = Cache::new(config);
        let input = json!({"id": 1});

        // get() should return None instead of propagating error
        let result = cache.get("user.get", &input).await;
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_set_wrapper_ignores_errors() {
        let config = CacheConfig::new().with_enabled(false);
        let cache = Cache::new(config);
        let input = json!({"id": 1});
        let value = json!({"name": "Alice"});

        // set() should not panic even with disabled cache
        cache.set("user.get", &input, value).await;
        // Test passes if we reach here without panicking
    }

    #[tokio::test]
    async fn test_invalidate_batch_empty() {
        let cache = Cache::new(CacheConfig::new());
        let entries = vec![];

        // Should handle empty batch gracefully
        cache.invalidate_batch(&entries).await;

        let stats = cache.stats().await;
        assert_eq!(stats.invalidations, 0);
    }

    #[tokio::test]
    async fn test_invalidate_batch_single() {
        let cache = Cache::new(CacheConfig::new());
        let input = json!({"id": 1});

        cache
            .set("user.get", &input, json!({"name": "Alice"}))
            .await;

        let entries = vec![("user.get".to_string(), input)];
        cache.invalidate_batch(&entries).await;

        let stats = cache.stats().await;
        assert_eq!(stats.invalidations, 1);
        assert!(cache.get("user.get", &json!({"id": 1})).await.is_none());
    }

    #[tokio::test]
    async fn test_invalidate_batch_multiple() {
        let cache = Cache::new(CacheConfig::new());

        // Set multiple entries
        cache
            .set("user.get", &json!({"id": 1}), json!({"name": "Alice"}))
            .await;
        cache
            .set("user.profile", &json!({"id": 1}), json!({"bio": "Hello"}))
            .await;
        cache
            .set("user.settings", &json!({"id": 1}), json!({"theme": "dark"}))
            .await;
        cache
            .set("post.get", &json!({"id": 1}), json!({"title": "Test"}))
            .await;

        // Batch invalidate user entries
        let entries = vec![
            ("user.get".to_string(), json!({"id": 1})),
            ("user.profile".to_string(), json!({"id": 1})),
            ("user.settings".to_string(), json!({"id": 1})),
        ];
        cache.invalidate_batch(&entries).await;

        let stats = cache.stats().await;
        assert_eq!(stats.invalidations, 3);

        // User entries should be gone
        assert!(cache.get("user.get", &json!({"id": 1})).await.is_none());
        assert!(cache.get("user.profile", &json!({"id": 1})).await.is_none());
        assert!(
            cache
                .get("user.settings", &json!({"id": 1}))
                .await
                .is_none()
        );

        // Post entry should remain
        assert!(cache.get("post.get", &json!({"id": 1})).await.is_some());
    }

    #[tokio::test]
    async fn test_invalidate_batch_nonexistent() {
        let cache = Cache::new(CacheConfig::new());

        // Try to invalidate entries that don't exist
        let entries = vec![
            ("user.get".to_string(), json!({"id": 999})),
            ("user.profile".to_string(), json!({"id": 999})),
        ];
        cache.invalidate_batch(&entries).await;

        let stats = cache.stats().await;
        assert_eq!(stats.invalidations, 0);
    }

    #[tokio::test]
    async fn test_invalidate_batch_mixed() {
        let cache = Cache::new(CacheConfig::new());

        // Set one entry
        cache
            .set("user.get", &json!({"id": 1}), json!({"name": "Alice"}))
            .await;

        // Batch with one existing and one non-existing entry
        let entries = vec![
            ("user.get".to_string(), json!({"id": 1})),
            ("user.profile".to_string(), json!({"id": 999})),
        ];
        cache.invalidate_batch(&entries).await;

        let stats = cache.stats().await;
        assert_eq!(stats.invalidations, 1);
    }

    #[tokio::test]
    async fn test_invalidate_batch_performance() {
        let cache = Cache::new(CacheConfig::new());

        // Set many entries
        for i in 0..100 {
            cache
                .set(&format!("item.{}", i), &json!({}), json!(i))
                .await;
        }

        // Batch invalidate all at once
        let entries: Vec<_> = (0..100)
            .map(|i| (format!("item.{}", i), json!({})))
            .collect();

        cache.invalidate_batch(&entries).await;

        let stats = cache.stats().await;
        assert_eq!(stats.invalidations, 100);
        assert_eq!(stats.total_entries, 0);
    }

    #[tokio::test]
    async fn test_cleanup_expired_returns_count() {
        use test_constants::*;

        let config = CacheConfig::new().with_default_ttl(SHORT_TTL);
        let cache = Cache::new(config);

        // Set entries
        cache.set("a", &json!({}), json!(1)).await;
        cache.set("b", &json!({}), json!(2)).await;
        cache.set("c", &json!({}), json!(3)).await;

        // Wait for expiration
        tokio::time::sleep(CLEANUP_WAIT).await;

        // Cleanup should return count of removed entries
        let removed = cache.cleanup_expired().await;
        assert_eq!(removed, 3);

        // Second cleanup should return 0
        let removed = cache.cleanup_expired().await;
        assert_eq!(removed, 0);
    }

    #[tokio::test]
    async fn test_start_cleanup_task() {
        use test_constants::*;

        let config = CacheConfig::new().with_default_ttl(SHORT_TTL);
        let cache = Cache::new(config);

        // Start cleanup task with short interval
        let cleanup_handle = cache.start_cleanup_task(Duration::from_millis(50));

        // Set entries
        cache.set("a", &json!({}), json!(1)).await;
        cache.set("b", &json!({}), json!(2)).await;

        // Wait for entries to expire
        tokio::time::sleep(CLEANUP_WAIT).await;

        // Wait for cleanup task to run
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Entries should be cleaned up by background task
        let stats = cache.stats().await;
        assert_eq!(stats.total_entries, 0);

        // Stop the cleanup task
        cleanup_handle.abort();
    }

    #[tokio::test]
    async fn test_cleanup_task_lifecycle() {
        let cache = Cache::new(CacheConfig::new());

        // Start cleanup task
        let cleanup_handle = cache.start_cleanup_task(Duration::from_millis(50));

        // Task should be running
        assert!(!cleanup_handle.is_finished());

        // Abort the task
        cleanup_handle.abort();

        // Wait a bit for abort to complete
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Task should be finished after abort
        assert!(cleanup_handle.is_finished());
    }

    #[tokio::test]
    async fn test_cleanup_task_multiple_runs() {
        use test_constants::*;

        let config = CacheConfig::new().with_default_ttl(SHORT_TTL);
        let cache = Cache::new(config);

        // Start cleanup task
        let cleanup_handle = cache.start_cleanup_task(Duration::from_millis(50));

        // Add entries in multiple batches
        for batch in 0..3 {
            cache
                .set(&format!("item_{}", batch), &json!({}), json!(batch))
                .await;
            tokio::time::sleep(CLEANUP_WAIT).await;
            // Wait for cleanup to run
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        // All expired entries should have been cleaned up
        let stats = cache.stats().await;
        assert_eq!(stats.total_entries, 0);

        cleanup_handle.abort();
    }
}
