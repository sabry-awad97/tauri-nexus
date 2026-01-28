//! Router performance benchmarks
//!
//! These benchmarks measure the performance of key router operations:
//! - Simple procedure calls
//! - Procedures with middleware
//! - Compiled vs non-compiled router

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use serde::{Deserialize, Serialize};
use std::hint::black_box;
use tauri_plugin_rpc::{Context, Router, RpcResult};

#[derive(Clone, Default)]
struct BenchContext {
    #[allow(dead_code)]
    value: i32,
}

#[derive(Debug, Deserialize, Serialize)]
struct BenchInput {
    value: i32,
}

#[derive(Debug, Serialize)]
struct BenchOutput {
    result: i32,
}

async fn simple_handler(_ctx: Context<BenchContext>, input: BenchInput) -> RpcResult<BenchOutput> {
    Ok(BenchOutput {
        result: input.value * 2,
    })
}

fn bench_simple_call(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    let router = Router::new()
        .context(BenchContext::default())
        .procedure("bench.simple")
        .input::<BenchInput>()
        .query(simple_handler);

    let input = serde_json::json!({"value": 42});

    c.bench_function("simple_call", |b| {
        b.iter(|| {
            rt.block_on(async {
                black_box(router.call("bench.simple", input.clone()).await.unwrap())
            })
        });
    });
}

fn bench_call_with_middleware(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    let router = Router::new()
        .context(BenchContext::default())
        .procedure("bench.middleware")
        .use_middleware(|ctx, req, next| async move {
            // Simple passthrough middleware
            next(ctx, req).await
        })
        .input::<BenchInput>()
        .query(simple_handler);

    let input = serde_json::json!({"value": 42});

    c.bench_function("call_with_middleware", |b| {
        b.iter(|| {
            rt.block_on(async {
                black_box(
                    router
                        .call("bench.middleware", input.clone())
                        .await
                        .unwrap(),
                )
            })
        });
    });
}

fn bench_compiled_vs_uncompiled(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    // Create two separate routers - one to keep uncompiled, one to compile
    let uncompiled_router = Router::new()
        .context(BenchContext::default())
        .procedure("bench.test")
        .input::<BenchInput>()
        .query(simple_handler);

    let compiled_router = Router::new()
        .context(BenchContext::default())
        .procedure("bench.test")
        .input::<BenchInput>()
        .query(simple_handler)
        .compile();

    let input = serde_json::json!({"value": 42});

    let mut group = c.benchmark_group("compiled_vs_uncompiled");

    group.bench_function("uncompiled", |b| {
        b.iter(|| {
            rt.block_on(async {
                black_box(
                    uncompiled_router
                        .call("bench.test", input.clone())
                        .await
                        .unwrap(),
                )
            })
        });
    });

    group.bench_function("compiled", |b| {
        b.iter(|| {
            rt.block_on(async {
                black_box(
                    compiled_router
                        .call("bench.test", input.clone())
                        .await
                        .unwrap(),
                )
            })
        });
    });

    group.finish();
}

fn bench_multiple_middleware(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    let mut group = c.benchmark_group("multiple_middleware");

    for count in [0, 1, 3, 5].iter() {
        let mut chain = Router::new()
            .context(BenchContext::default())
            .procedure("bench.mw");

        for _ in 0..*count {
            chain = chain.use_middleware(|ctx, req, next| async move { next(ctx, req).await });
        }

        let router = chain.input::<BenchInput>().query(simple_handler);
        let input = serde_json::json!({"value": 42});

        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, _| {
            b.iter(|| {
                rt.block_on(async {
                    black_box(router.call("bench.mw", input.clone()).await.unwrap())
                })
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_simple_call,
    bench_call_with_middleware,
    bench_compiled_vs_uncompiled,
    bench_multiple_middleware
);
criterion_main!(benches);
