//! Cache metrics tracking

use std::sync::atomic::{AtomicU64, Ordering};

/// Internal metrics tracking for cache operations
#[derive(Debug)]
pub struct CacheMetrics {
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
    pub fn new() -> Self {
        Self {
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            evictions: AtomicU64::new(0),
            invalidations: AtomicU64::new(0),
        }
    }

    pub fn record_hit(&self) {
        self.hits.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_miss(&self) {
        self.misses.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_eviction(&self) {
        self.evictions.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_invalidation(&self) {
        self.invalidations.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_invalidations(&self, count: u64) {
        self.invalidations.fetch_add(count, Ordering::Relaxed);
    }

    pub fn get_hits(&self) -> u64 {
        self.hits.load(Ordering::Relaxed)
    }

    pub fn get_misses(&self) -> u64 {
        self.misses.load(Ordering::Relaxed)
    }

    pub fn get_evictions(&self) -> u64 {
        self.evictions.load(Ordering::Relaxed)
    }

    pub fn get_invalidations(&self) -> u64 {
        self.invalidations.load(Ordering::Relaxed)
    }

    pub fn calculate_hit_ratio(&self) -> f64 {
        let hits = self.get_hits();
        let misses = self.get_misses();
        let total = hits + misses;

        if total == 0 {
            0.0
        } else {
            hits as f64 / total as f64
        }
    }

    pub fn reset(&self) {
        self.hits.store(0, Ordering::Relaxed);
        self.misses.store(0, Ordering::Relaxed);
        self.evictions.store(0, Ordering::Relaxed);
        self.invalidations.store(0, Ordering::Relaxed);
    }
}

impl Default for CacheMetrics {
    fn default() -> Self {
        Self::new()
    }
}
