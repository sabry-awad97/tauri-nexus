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

mod config;
mod entry;
mod error;
mod metrics;
mod middleware;
mod pattern;
mod store;

// Re-export public API with inline documentation
#[doc(inline)]
pub use config::{CacheConfig, DEFAULT_MAX_ENTRIES, DEFAULT_TTL_SECS};
#[doc(inline)]
pub use entry::{CacheEntry, generate_cache_key};
#[doc(inline)]
pub use error::{CacheError, CacheResult};
#[doc(inline)]
pub use middleware::{cache_middleware, invalidation_middleware};
#[doc(inline)]
pub use store::{Cache, CacheStats};

// Re-export pattern matching for advanced use cases
pub use pattern::pattern_matches;

// Property-Based Tests
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
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let config = CacheConfig::new().with_default_ttl(std::time::Duration::from_secs(300));
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
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let config = CacheConfig::new().with_default_ttl(std::time::Duration::from_millis(5));
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
                tokio::time::sleep(std::time::Duration::from_millis(15)).await;

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
