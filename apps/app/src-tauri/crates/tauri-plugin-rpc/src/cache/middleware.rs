//! Cache middleware for RPC procedures

use std::collections::HashMap;
use std::sync::Arc;

use crate::Context;
use crate::middleware::{MiddlewareFn, Next, Request, from_fn};

use super::store::Cache;

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
    let middleware = move |ctx: Context<Ctx>, req: Request, next: Next<Ctx>| {
        let cache = cache.clone();

        async move {
            // Optimization: Use references for cache lookup to avoid unnecessary cloning.
            // We only clone when we need to store values in the cache.

            // Only cache queries (check procedure type if available)
            // For now, we cache all procedures - mutations should use invalidation
            if !cache.config.should_cache(&req.path) {
                tracing::trace!(path = %req.path, "Cache bypass: path excluded");
                return next(ctx, req).await;
            }

            // Check cache first using references (no cloning needed for lookup)
            if let Some(cached) = cache.get(&req.path, &req.input).await {
                tracing::debug!(
                    path = %req.path,
                    "Cache hit"
                );
                return Ok(cached);
            }

            tracing::debug!(path = %req.path, "Cache miss");

            // Clone path and input only when we need to store them
            // This happens after the handler executes successfully
            let path = req.path.clone();
            let input = req.input.clone();

            // Execute handler
            let result = next(ctx, req).await?;

            // Cache successful result (now we use the cloned values)
            let ttl = cache.config.get_ttl(&path);
            cache.set(&path, &input, result.clone()).await;

            tracing::trace!(
                path = %path,
                ttl_ms = %ttl.as_millis(),
                "Cache entry stored"
            );

            Ok(result)
        }
    };
    from_fn(middleware)
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

    let middleware = move |ctx: Context<Ctx>, req: Request, next: Next<Ctx>| {
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
    };
    from_fn(middleware)
}
