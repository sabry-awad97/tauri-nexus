//! Caching layer for RPC query procedures
//!
//! Provides configurable caching with TTL support and automatic invalidation.
//!
//! # Example
//!
//! ```rust,ignore
//! use tauri_plugin_rpc::cache::{Cache, CacheConfig};
//! use std::time::Duration;
//!
//! let config = CacheConfig::new()
//!     .with_default_ttl(Duration::from_secs(300))
//!     .with_max_entries(1000)
//!     .with_procedure_ttl("user.profile", Duration::from_secs(60));
//!
//! let cache = Cache::new(config);
//!
//! // Cache a value
//! cache.set("user.profile", &json!({"id": 1}), json!({"name": "Alice"})).await;
//!
//! // Get cached value
//! if let Some(value) = cache.get("user.profile", &json!({"id": 1})).await {
//!     println!("Cached: {}", value);
//! }
//!
//! // Invalidate on mutation
//! cache.invalidate_pattern("user.*").await;
//! ```
//!
//! # Tracing
//!
//! This module uses structured tracing for observability:
//!
//! - **Debug level**: Cache hits, misses, invalidations, and evictions
//! - **Trace level**: Detailed operations including key generation and TTL values
//!
//! All cache operations create spans with relevant context:
//! - `cache_get`: path, enabled status
//! - `cache_set`: path, enabled status, TTL
//! - `cache_invalidate`: path
//! - `cache_invalidate_pattern`: pattern, count
//! - `cache_invalidate_all`: count
//!
//! Cache keys are not logged to avoid exposing potentially sensitive input data.
//! Only procedure paths and operation outcomes are included in trace events.

mod error;

pub use error::{CacheError, CacheResult};

use crate::Context;
use crate::middleware::{MiddlewareFn, Next, Request, from_fn};
use lru::LruCache;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

// =============================================================================
// Constants
// =============================================================================

/// Default TTL for cache entries (5 minutes)
///
/// This value provides a reasonable balance between cache freshness and hit rate
/// for most use cases. Procedures with different requirements should use
/// `CacheConfig::with_procedure_ttl()` to override this default.
pub const DEFAULT_TTL_SECS: u64 = 300;

/// Default maximum number of cache entries
///
/// This limit prevents unbounded memory growth while allowing sufficient cache
/// capacity for typical applications. The LRU eviction policy ensures the most
/// recently used entries are retained when this limit is reached.
pub const DEFAULT_MAX_ENTRIES: usize = 1000;

#[cfg(test)]
mod test_constants {
    use std::time::Duration;

    /// Short TTL for expiration tests (50ms)
    pub const SHORT_TTL: Duration = Duration::from_millis(50);

    /// Long TTL for non-expiration tests (5 minutes)
    pub const LONG_TTL: Duration = Duration::from_secs(300);

    /// Wait time for cleanup operations in tests (100ms)
    pub const CLEANUP_WAIT: Duration = Duration::from_millis(100);

    /// Very short TTL for property-based expiration tests (5ms)
    pub const VERY_SHORT_TTL: Duration = Duration::from_millis(5);

    /// Buffer time for TTL expiration tests (15ms)
    pub const TTL_EXPIRATION_BUFFER: Duration = Duration::from_millis(15);
}

// =============================================================================
// Cache Metrics
// =============================================================================

use std::sync::atomic::{AtomicU64, Ordering};

/// Internal metrics tracking for cache operations
#[derive(Debug)]
struct CacheMetrics {
    /// Number of cache hits
    hits: AtomicU64,
    /// Number of cache misses
    misses: AtomicU64,
    /// Number of entries evicted due to LRU
    evictions: AtomicU64,
    /// Number of entries invalidated
    invalidations: AtomicU64,
}

impl CacheMetrics {
    fn new() -> Self {
        Self {
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            evictions: AtomicU64::new(0),
            invalidations: AtomicU64::new(0),
        }
    }

    fn record_hit(&self) {
        self.hits.fetch_add(1, Ordering::Relaxed);
    }

    fn record_miss(&self) {
        self.misses.fetch_add(1, Ordering::Relaxed);
    }

    fn record_eviction(&self) {
        self.evictions.fetch_add(1, Ordering::Relaxed);
    }

    fn record_invalidation(&self) {
        self.invalidations.fetch_add(1, Ordering::Relaxed);
    }

    fn record_invalidations(&self, count: u64) {
        self.invalidations.fetch_add(count, Ordering::Relaxed);
    }

    fn get_hits(&self) -> u64 {
        self.hits.load(Ordering::Relaxed)
    }

    fn get_misses(&self) -> u64 {
        self.misses.load(Ordering::Relaxed)
    }

    fn get_evictions(&self) -> u64 {
        self.evictions.load(Ordering::Relaxed)
    }

    fn get_invalidations(&self) -> u64 {
        self.invalidations.load(Ordering::Relaxed)
    }

    fn calculate_hit_ratio(&self) -> f64 {
        let hits = self.get_hits();
        let misses = self.get_misses();
        let total = hits + misses;

        if total == 0 {
            0.0
        } else {
            hits as f64 / total as f64
        }
    }

    fn reset(&self) {
        self.hits.store(0, Ordering::Relaxed);
        self.misses.store(0, Ordering::Relaxed);
        self.evictions.store(0, Ordering::Relaxed);
        self.invalidations.store(0, Ordering::Relaxed);
    }
}

// =============================================================================
// Cache Configuration
// =============================================================================

/// Configuration for the cache layer
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Default TTL for cached entries
    pub default_ttl: Duration,
    /// Per-procedure TTL overrides
    pub procedure_ttl: HashMap<String, Duration>,
    /// Maximum number of entries in the cache
    pub max_entries: usize,
    /// Whether caching is enabled
    pub enabled: bool,
    /// Patterns for procedures that should not be cached
    pub excluded_patterns: Vec<String>,
}

impl CacheConfig {
    /// Create a new cache configuration with defaults
    pub fn new() -> Self {
        Self {
            default_ttl: Duration::from_secs(DEFAULT_TTL_SECS),
            procedure_ttl: HashMap::new(),
            max_entries: DEFAULT_MAX_ENTRIES,
            enabled: true,
            excluded_patterns: Vec::new(),
        }
    }

    /// Set the default TTL for cached entries
    #[must_use = "This method returns a new CacheConfig and does not modify self"]
    pub fn with_default_ttl(mut self, ttl: Duration) -> Self {
        self.default_ttl = ttl;
        self
    }

    /// Set a TTL for a specific procedure
    #[must_use = "This method returns a new CacheConfig and does not modify self"]
    pub fn with_procedure_ttl(mut self, path: impl Into<String>, ttl: Duration) -> Self {
        self.procedure_ttl.insert(path.into(), ttl);
        self
    }

    /// Set the maximum number of entries
    #[must_use = "This method returns a new CacheConfig and does not modify self"]
    pub fn with_max_entries(mut self, max: usize) -> Self {
        self.max_entries = max;
        self
    }

    /// Enable or disable caching
    #[must_use = "This method returns a new CacheConfig and does not modify self"]
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Add a pattern for procedures that should not be cached
    #[must_use = "This method returns a new CacheConfig and does not modify self"]
    pub fn exclude_pattern(mut self, pattern: impl Into<String>) -> Self {
        self.excluded_patterns.push(pattern.into());
        self
    }

    /// Get the TTL for a specific procedure
    pub fn get_ttl(&self, path: &str) -> Duration {
        self.procedure_ttl
            .get(path)
            .copied()
            .unwrap_or(self.default_ttl)
    }

    /// Check if a procedure should be cached
    pub fn should_cache(&self, path: &str) -> bool {
        if !self.enabled {
            return false;
        }

        for pattern in &self.excluded_patterns {
            if pattern_matches(pattern, path) {
                return false;
            }
        }

        true
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Cache Entry
// =============================================================================

/// A cached entry with expiration tracking
#[derive(Debug, Clone)]
pub struct CacheEntry {
    /// The cached value
    pub value: serde_json::Value,
    /// When the entry was created
    pub created_at: Instant,
    /// Time-to-live for this entry
    pub ttl: Duration,
}

impl CacheEntry {
    /// Create a new cache entry
    pub fn new(value: serde_json::Value, ttl: Duration) -> Self {
        Self {
            value,
            created_at: Instant::now(),
            ttl,
        }
    }

    /// Check if the entry has expired
    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() >= self.ttl
    }

    /// Get the remaining TTL
    pub fn remaining_ttl(&self) -> Duration {
        self.ttl.saturating_sub(self.created_at.elapsed())
    }
}

// =============================================================================
// Cache Key
// =============================================================================

/// Generate a deterministic cache key from path and input
pub fn generate_cache_key(path: &str, input: &serde_json::Value) -> String {
    // Normalize the input to ensure deterministic key generation
    let normalized_input = normalize_json(input);
    format!("{}:{}", path, normalized_input)
}

/// Normalize JSON for deterministic key generation
fn normalize_json(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "null".to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => {
            format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
        }
        serde_json::Value::Array(arr) => {
            let items: Vec<String> = arr.iter().map(normalize_json).collect();
            format!("[{}]", items.join(","))
        }
        serde_json::Value::Object(obj) => {
            // Sort keys for deterministic ordering
            let mut pairs: Vec<_> = obj.iter().collect();
            pairs.sort_by(|a, b| a.0.cmp(b.0));
            let items: Vec<String> = pairs
                .iter()
                .map(|(k, v)| format!("\"{}\":{}", k, normalize_json(v)))
                .collect();
            format!("{{{}}}", items.join(","))
        }
    }
}

// =============================================================================
// Cache
// =============================================================================

/// Thread-safe LRU cache with TTL support
pub struct Cache {
    config: CacheConfig,
    entries: Arc<RwLock<LruCache<String, CacheEntry>>>,
    metrics: Arc<CacheMetrics>,
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

    /// Remove expired entries
    pub async fn cleanup_expired(&self) {
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

        for key in expired_keys {
            entries.pop(&key);
        }
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

// =============================================================================
// Pattern Matching
// =============================================================================

/// Check if a pattern matches a path
/// Supports:
/// - Exact match: "user.get" matches "user.get"
/// - Wildcard suffix: "user.*" matches "user.get", "user.create", etc.
/// - Global wildcard: "*" matches everything
fn pattern_matches(pattern: &str, path: &str) -> bool {
    if pattern == "*" {
        return true;
    }

    if let Some(prefix) = pattern.strip_suffix(".*") {
        // Exact match: "user.*" matches "user"
        if path == prefix {
            return true;
        }

        // Prefix match: "user.*" matches "user.get", "user.profile", etc.
        // Avoid allocation by checking length and using byte-level comparison
        if path.len() > prefix.len() + 1
            && path.starts_with(prefix)
            && path.as_bytes()[prefix.len()] == b'.'
        {
            return true;
        }

        return false;
    }

    pattern == path
}

// =============================================================================
// Cache Middleware
// =============================================================================

/// Create a caching middleware for query procedures
///
/// This middleware caches successful query responses and returns cached
/// values when available. It does NOT cache mutations or subscriptions.
///
/// # Example
///
/// ```rust,ignore
/// use tauri_plugin_rpc::cache::{cache_middleware, Cache, CacheConfig};
/// use std::time::Duration;
///
/// let config = CacheConfig::new()
///     .with_default_ttl(Duration::from_secs(300));
/// let cache = Cache::new(config);
///
/// let router = Router::new()
///     .middleware_fn(cache_middleware(cache))
///     .query("user.profile", get_profile_handler);
/// ```
pub fn cache_middleware<Ctx>(cache: Cache) -> MiddlewareFn<Ctx>
where
    Ctx: Clone + Send + Sync + 'static,
{
    from_fn(move |ctx: Context<Ctx>, req: Request, next: Next<Ctx>| {
        let cache = cache.clone();
        let path = req.path.clone();
        let input = req.input.clone();

        async move {
            // Only cache queries (check procedure type if available)
            // For now, we cache all procedures - mutations should use invalidation
            if !cache.config.should_cache(&path) {
                tracing::trace!(path = %path, "Cache bypass: path excluded");
                return next(ctx, req).await;
            }

            // Check cache first
            if let Some(cached) = cache.get(&path, &input).await {
                tracing::debug!(
                    path = %path,
                    "Cache hit"
                );
                return Ok(cached);
            }

            tracing::debug!(path = %path, "Cache miss");

            // Execute handler
            let result = next(ctx, req).await?;

            // Cache successful result
            let ttl = cache.config.get_ttl(&path);
            cache.set(&path, &input, result.clone()).await;

            tracing::trace!(
                path = %path,
                ttl_ms = %ttl.as_millis(),
                "Cache entry stored"
            );

            Ok(result)
        }
    })
}

/// Create a cache invalidation middleware for mutation procedures
///
/// This middleware invalidates cache entries after successful mutations.
/// Configure patterns to invalidate related cache entries.
///
/// # Example
///
/// ```rust,ignore
/// use tauri_plugin_rpc::cache::{invalidation_middleware, Cache, CacheConfig};
///
/// let cache = Cache::new(CacheConfig::new());
///
/// // Invalidate user.* cache entries after user mutations
/// let invalidation_rules = vec![
///     ("user.update", vec!["user.*"]),
///     ("user.delete", vec!["user.*"]),
/// ];
///
/// let router = Router::new()
///     .middleware_fn(invalidation_middleware(cache, invalidation_rules))
///     .mutation("user.update", update_user_handler);
/// ```
pub fn invalidation_middleware<Ctx>(
    cache: Cache,
    rules: Vec<(impl Into<String>, Vec<impl Into<String>>)>,
) -> MiddlewareFn<Ctx>
where
    Ctx: Clone + Send + Sync + 'static,
{
    let rules: HashMap<String, Vec<String>> = rules
        .into_iter()
        .map(|(path, patterns)| {
            (
                path.into(),
                patterns.into_iter().map(|p| p.into()).collect(),
            )
        })
        .collect();
    let rules = Arc::new(rules);

    from_fn(move |ctx: Context<Ctx>, req: Request, next: Next<Ctx>| {
        let cache = cache.clone();
        let rules = Arc::clone(&rules);
        let path = req.path.clone();

        async move {
            // Execute handler first
            let result = next(ctx, req).await?;

            // Invalidate cache entries based on rules
            if let Some(patterns) = rules.get(&path) {
                let pattern_count = patterns.len();
                for pattern in patterns {
                    tracing::debug!(
                        path = %path,
                        pattern = %pattern,
                        "Invalidating cache entries"
                    );
                    cache.invalidate_pattern(pattern).await;
                }
                tracing::trace!(
                    path = %path,
                    patterns_invalidated = %pattern_count,
                    "Cache invalidation complete"
                );
            }

            Ok(result)
        }
    })
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::time::Duration;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_cache_config_defaults() {
        let config = CacheConfig::new();
        assert_eq!(config.default_ttl, Duration::from_secs(DEFAULT_TTL_SECS));
        assert_eq!(config.max_entries, DEFAULT_MAX_ENTRIES);
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

        cache.cleanup_expired().await;

        let stats_after = cache.stats().await;
        assert_eq!(stats_after.total_entries, 0);
    }

    #[test]
    fn test_cache_key_determinism() {
        let input1 = json!({"b": 2, "a": 1});
        let input2 = json!({"a": 1, "b": 2});

        let key1 = generate_cache_key("test", &input1);
        let key2 = generate_cache_key("test", &input2);

        // Keys should be the same regardless of object key order
        assert_eq!(key1, key2);
    }

    #[test]
    fn test_cache_key_different_inputs() {
        let input1 = json!({"id": 1});
        let input2 = json!({"id": 2});

        let key1 = generate_cache_key("test", &input1);
        let key2 = generate_cache_key("test", &input2);

        assert_ne!(key1, key2);
    }

    #[test]
    fn test_cache_key_different_paths() {
        let input = json!({"id": 1});

        let key1 = generate_cache_key("user.get", &input);
        let key2 = generate_cache_key("post.get", &input);

        assert_ne!(key1, key2);
    }

    #[test]
    fn test_pattern_matches_exact() {
        assert!(pattern_matches("user.get", "user.get"));
        assert!(!pattern_matches("user.get", "user.create"));
    }

    #[test]
    fn test_pattern_matches_wildcard() {
        assert!(pattern_matches("user.*", "user.get"));
        assert!(pattern_matches("user.*", "user.create"));
        assert!(pattern_matches("user.*", "user"));
        assert!(!pattern_matches("user.*", "post.get"));
    }

    #[test]
    fn test_pattern_matches_global() {
        assert!(pattern_matches("*", "anything"));
        assert!(pattern_matches("*", "user.get"));
    }

    #[test]
    fn test_normalize_json_primitives() {
        assert_eq!(normalize_json(&json!(null)), "null");
        assert_eq!(normalize_json(&json!(true)), "true");
        assert_eq!(normalize_json(&json!(false)), "false");
        assert_eq!(normalize_json(&json!(42)), "42");
        assert_eq!(normalize_json(&json!(3.1)), "3.1");
        assert_eq!(normalize_json(&json!("hello")), "\"hello\"");
    }

    #[test]
    fn test_normalize_json_array() {
        assert_eq!(normalize_json(&json!([1, 2, 3])), "[1,2,3]");
        assert_eq!(normalize_json(&json!(["a", "b"])), "[\"a\",\"b\"]");
    }

    #[test]
    fn test_normalize_json_object_sorted() {
        let obj = json!({"z": 1, "a": 2, "m": 3});
        let normalized = normalize_json(&obj);
        assert_eq!(normalized, "{\"a\":2,\"m\":3,\"z\":1}");
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
}

// =============================================================================
// Property-Based Tests
// =============================================================================

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;
    use serde_json::json;

    // Strategy for generating procedure paths
    fn path_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("user.get".to_string()),
            Just("user.profile".to_string()),
            Just("post.get".to_string()),
            Just("post.list".to_string()),
            Just("admin.users".to_string()),
        ]
    }

    // Strategy for generating simple JSON values
    fn json_value_strategy() -> impl Strategy<Value = serde_json::Value> {
        prop_oneof![
            Just(json!(null)),
            any::<bool>().prop_map(|b| json!(b)),
            any::<i32>().prop_map(|n| json!(n)),
            "[a-z]{1,10}".prop_map(|s| json!(s)),
        ]
    }

    // Strategy for generating JSON objects
    fn json_object_strategy() -> impl Strategy<Value = serde_json::Value> {
        prop::collection::hash_map("[a-z]{1,5}", json_value_strategy(), 0..5).prop_map(|map| {
            let obj: serde_json::Map<String, serde_json::Value> = map.into_iter().collect();
            serde_json::Value::Object(obj)
        })
    }

    proptest! {
        /// Property 13: Cache Key Determinism
        /// For any procedure path and input, the generated cache key SHALL be deterministic
        /// (same path + input = same key).
        #[test]
        fn prop_cache_key_determinism(
            path in path_strategy(),
            input in json_object_strategy(),
        ) {
            let key1 = generate_cache_key(&path, &input);
            let key2 = generate_cache_key(&path, &input);

            prop_assert_eq!(key1, key2, "Cache keys should be deterministic");
        }

        /// Property: Different inputs produce different keys
        #[test]
        fn prop_different_inputs_different_keys(
            path in path_strategy(),
            input1 in json_object_strategy(),
            input2 in json_object_strategy(),
        ) {
            let key1 = generate_cache_key(&path, &input1);
            let key2 = generate_cache_key(&path, &input2);

            // If inputs are different, keys should be different
            if input1 != input2 {
                prop_assert_ne!(key1, key2, "Different inputs should produce different keys");
            }
        }

        /// Property: Object key order doesn't affect cache key
        #[test]
        fn prop_object_key_order_independent(
            a_val in any::<i32>(),
            b_val in any::<i32>(),
        ) {
            let input1 = json!({"a": a_val, "b": b_val});
            let input2 = json!({"b": b_val, "a": a_val});

            let key1 = generate_cache_key("test", &input1);
            let key2 = generate_cache_key("test", &input2);

            prop_assert_eq!(key1, key2, "Object key order should not affect cache key");
        }

        /// Property 14: Cache Hit Behavior
        /// For any cached value, getting it before TTL expiration SHALL return the cached value.
        #[test]
        fn prop_cache_hit_returns_value(
            path in path_strategy(),
            input in json_object_strategy(),
            value in json_value_strategy(),
        ) {
            use test_constants::*;

            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let config = CacheConfig::new().with_default_ttl(LONG_TTL);
                let cache = Cache::new(config);

                // Set value
                cache.set(&path, &input, value.clone()).await;

                // Get should return the same value
                let cached = cache.get(&path, &input).await;
                prop_assert_eq!(cached, Some(value), "Cache hit should return cached value");

                Ok(())
            })?;
        }

        /// Property: Cache miss returns None
        #[test]
        fn prop_cache_miss_returns_none(
            path in path_strategy(),
            input in json_object_strategy(),
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let cache = Cache::new(CacheConfig::new());

                // Get without set should return None
                let cached = cache.get(&path, &input).await;
                prop_assert_eq!(cached, None, "Cache miss should return None");

                Ok(())
            })?;
        }

        /// Property: Invalidation removes cached value
        #[test]
        fn prop_invalidation_removes_value(
            path in path_strategy(),
            input in json_object_strategy(),
            value in json_value_strategy(),
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let cache = Cache::new(CacheConfig::new());

                // Set and verify
                cache.set(&path, &input, value).await;
                prop_assert!(cache.get(&path, &input).await.is_some());

                // Invalidate
                cache.invalidate(&path, &input).await;

                // Should be gone
                prop_assert!(cache.get(&path, &input).await.is_none());

                Ok(())
            })?;
        }

        /// Property: Pattern invalidation removes matching entries
        #[test]
        fn prop_pattern_invalidation(
            value in json_value_strategy(),
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let cache = Cache::new(CacheConfig::new());
                let input = json!({});

                // Set entries in different namespaces
                cache.set("user.get", &input, value.clone()).await;
                cache.set("user.profile", &input, value.clone()).await;
                cache.set("post.get", &input, value.clone()).await;

                // Invalidate user.*
                cache.invalidate_pattern("user.*").await;

                // User entries should be gone
                prop_assert!(cache.get("user.get", &input).await.is_none());
                prop_assert!(cache.get("user.profile", &input).await.is_none());

                // Post entry should remain
                prop_assert!(cache.get("post.get", &input).await.is_some());

                Ok(())
            })?;
        }

        /// **Property 4: Cache TTL Expiration**
        /// *For any* cached value with TTL T, the value SHALL be retrievable before time T
        /// and SHALL NOT be retrievable after time T has elapsed.
        /// **Feature: tauri-rpc-production-audit, Property 4: Cache TTL Expiration**
        /// **Validates: Requirements 4.6**
        #[test]
        fn prop_cache_ttl_expiration(
            path in path_strategy(),
            input in json_object_strategy(),
            value in json_value_strategy(),
        ) {
            use test_constants::*;

            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let config = CacheConfig::new().with_default_ttl(VERY_SHORT_TTL);
                let cache = Cache::new(config);

                // Set value with TTL
                cache.set(&path, &input, value.clone()).await;

                // Value should be retrievable immediately (before TTL)
                let cached_before = cache.get(&path, &input).await;
                prop_assert_eq!(
                    cached_before,
                    Some(value.clone()),
                    "Value should be retrievable before TTL expires"
                );

                // Wait for TTL to expire (add small buffer for timing)
                tokio::time::sleep(TTL_EXPIRATION_BUFFER).await;

                // Value should NOT be retrievable after TTL
                let cached_after = cache.get(&path, &input).await;
                prop_assert_eq!(
                    cached_after,
                    None,
                    "Value should NOT be retrievable after TTL expires"
                );

                Ok(())
            })?;
        }

        /// Property: LRU eviction removes least recently used entries
        /// **Feature: tauri-rpc-production-audit, Property 12: LRU Cache Eviction**
        /// **Validates: Requirements 8.3**
        #[test]
        fn prop_lru_cache_eviction(
            capacity in 2usize..10,
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let config = CacheConfig::new().with_max_entries(capacity);
                let cache = Cache::new(config);
                let input = json!({});

                // Fill cache to capacity
                for i in 0..capacity {
                    let path = format!("path{}", i);
                    cache.set(&path, &input, json!(i)).await;
                }

                // All entries should be present
                for i in 0..capacity {
                    let path = format!("path{}", i);
                    prop_assert!(
                        cache.get(&path, &input).await.is_some(),
                        "Entry {} should be present before eviction",
                        i
                    );
                }

                // Add one more entry (should evict LRU - path0)
                cache.set("new_path", &input, json!("new")).await;

                // First entry (path0) should be evicted
                prop_assert!(
                    cache.get("path0", &input).await.is_none(),
                    "LRU entry (path0) should be evicted"
                );

                // New entry should be present
                prop_assert!(
                    cache.get("new_path", &input).await.is_some(),
                    "New entry should be present"
                );

                // Other entries should still be present
                for i in 1..capacity {
                    let path = format!("path{}", i);
                    prop_assert!(
                        cache.get(&path, &input).await.is_some(),
                        "Entry {} should still be present after eviction",
                        i
                    );
                }

                Ok(())
            })?;
        }
    }
}
