//! Cache configuration

use std::collections::HashMap;
use std::time::Duration;

use super::pattern::pattern_matches;

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
