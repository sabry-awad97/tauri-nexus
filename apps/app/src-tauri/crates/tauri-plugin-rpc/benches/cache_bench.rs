use criterion::{Criterion, criterion_group, criterion_main};
use serde_json::json;
use std::hint::black_box;
use tauri_plugin_rpc::cache::{Cache, CacheConfig};

fn bench_pattern_matching(c: &mut Criterion) {
    let mut group = c.benchmark_group("pattern_matching");

    // Benchmark exact match
    group.bench_function("exact_match", |b| {
        b.iter(|| {
            // This is internal, so we'll benchmark through cache operations
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let cache = Cache::new(CacheConfig::new());
                cache
                    .set("user.get", &json!({"id": 1}), json!({"name": "Alice"}))
                    .await;
                cache.invalidate_pattern(black_box("user.get")).await;
            });
        });
    });

    // Benchmark wildcard match
    group.bench_function("wildcard_match", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let cache = Cache::new(CacheConfig::new());
                // Add multiple entries
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

                // Invalidate with wildcard pattern
                cache.invalidate_pattern(black_box("user.*")).await;
            });
        });
    });

    // Benchmark global wildcard
    group.bench_function("global_wildcard", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let cache = Cache::new(CacheConfig::new());
                // Add multiple entries
                for i in 0..10 {
                    cache.set(&format!("path{}", i), &json!({}), json!(i)).await;
                }

                // Invalidate all
                cache.invalidate_pattern(black_box("*")).await;
            });
        });
    });

    group.finish();
}

fn bench_cache_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_operations");

    // Benchmark cache hit
    group.bench_function("cache_hit", |b| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let cache = Cache::new(CacheConfig::new());

        // Pre-populate cache
        rt.block_on(async {
            cache
                .set("user.get", &json!({"id": 1}), json!({"name": "Alice"}))
                .await;
        });

        b.iter(|| {
            rt.block_on(async {
                let result = cache
                    .get(black_box("user.get"), black_box(&json!({"id": 1})))
                    .await;
                black_box(result);
            });
        });
    });

    // Benchmark cache miss
    group.bench_function("cache_miss", |b| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let cache = Cache::new(CacheConfig::new());

        b.iter(|| {
            rt.block_on(async {
                let result = cache
                    .get(black_box("user.get"), black_box(&json!({"id": 999})))
                    .await;
                black_box(result);
            });
        });
    });

    // Benchmark cache set
    group.bench_function("cache_set", |b| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let cache = Cache::new(CacheConfig::new());
        let mut counter = 0;

        b.iter(|| {
            rt.block_on(async {
                cache
                    .set(
                        black_box("user.get"),
                        black_box(&json!({"id": counter})),
                        black_box(json!({"name": "Alice"})),
                    )
                    .await;
            });
            counter += 1;
        });
    });

    group.finish();
}

fn bench_concurrent_access(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_access");

    // Benchmark concurrent reads
    group.bench_function("concurrent_reads", |b| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let cache = Cache::new(CacheConfig::new());

        // Pre-populate cache
        rt.block_on(async {
            for i in 0..10 {
                cache.set(&format!("path{}", i), &json!({}), json!(i)).await;
            }
        });

        b.iter(|| {
            rt.block_on(async {
                let mut handles = vec![];
                for i in 0..10 {
                    let cache_clone = cache.clone();
                    let handle = tokio::spawn(async move {
                        cache_clone.get(&format!("path{}", i), &json!({})).await
                    });
                    handles.push(handle);
                }

                for handle in handles {
                    black_box(handle.await.unwrap());
                }
            });
        });
    });

    group.finish();
}

fn bench_middleware_optimization(c: &mut Criterion) {
    let mut group = c.benchmark_group("middleware_optimization");

    // Simulate the old approach: clone early
    group.bench_function("clone_early", |b| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let cache = Cache::new(CacheConfig::new());

        // Pre-populate cache for hits
        rt.block_on(async {
            cache
                .set("user.get", &json!({"id": 1}), json!({"name": "Alice"}))
                .await;
        });

        b.iter(|| {
            rt.block_on(async {
                // Old approach: clone immediately
                let path = black_box("user.get".to_string());
                let input = black_box(json!({"id": 1}));

                // Check cache (hit case - clones were unnecessary)
                let result = cache.get(&path, &input).await;
                black_box(result);
            });
        });
    });

    // Simulate the new approach: use references, clone only when needed
    group.bench_function("clone_deferred", |b| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let cache = Cache::new(CacheConfig::new());

        // Pre-populate cache for hits
        rt.block_on(async {
            cache
                .set("user.get", &json!({"id": 1}), json!({"name": "Alice"}))
                .await;
        });

        b.iter(|| {
            rt.block_on(async {
                // New approach: use references
                let path = black_box("user.get");
                let input = black_box(json!({"id": 1}));

                // Check cache (hit case - no clones needed)
                let result = cache.get(path, &input).await;
                black_box(result);

                // In real middleware, we'd only clone if we need to store
                // (which doesn't happen on cache hit)
            });
        });
    });

    // Benchmark cache miss scenario (where cloning is necessary)
    group.bench_function("clone_on_miss", |b| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let cache = Cache::new(CacheConfig::new());
        let mut counter = 0;

        b.iter(|| {
            rt.block_on(async {
                let path = black_box("user.get");
                let input = black_box(json!({"id": counter}));

                // Check cache (miss)
                let result = cache.get(path, &input).await;

                if result.is_none() {
                    // Clone only when we need to store
                    let path_owned = path.to_string();
                    let input_owned = input.clone();
                    cache
                        .set(&path_owned, &input_owned, json!({"name": "Alice"}))
                        .await;
                }
            });
            counter += 1;
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_pattern_matching,
    bench_cache_operations,
    bench_concurrent_access,
    bench_middleware_optimization
);
criterion_main!(benches);
