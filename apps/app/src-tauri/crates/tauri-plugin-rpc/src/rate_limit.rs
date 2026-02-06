//! Rate limiting for RPC procedures
//!
//! Provides configurable rate limiting with multiple strategies:
//! - Fixed window: Simple counter reset at fixed intervals
//! - Sliding window: Weighted average of current and previous window
//! - Token bucket: Smooth rate limiting with burst capacity
//!
//! # Example
//!
//! ```rust,ignore
//! use tauri_plugin_rpc::rate_limit::{RateLimiter, RateLimitConfig, RateLimit, RateLimitStrategy};
//! use std::time::Duration;
//!
//! let config = RateLimitConfig::new()
//!     .with_default_limit(RateLimit::new(100, Duration::from_secs(60)))
//!     .with_procedure_limit("expensive.operation", RateLimit::new(10, Duration::from_secs(60)));
//!
//! let limiter = RateLimiter::new(config);
//!
//! // Check if request is allowed
//! match limiter.check("user.get", "client-123").await {
//!     Ok(()) => { /* proceed */ }
//!     Err(e) => { /* rate limited, e contains retry_after */ }
//! }
//! ```

use crate::middleware::{MiddlewareFn, Request, from_fn};
use crate::{Context, Next, RpcError};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

// =============================================================================
// Rate Limit Strategy
// =============================================================================

/// Strategy for rate limit calculation
#[derive(Debug, Clone, PartialEq, Default)]
#[non_exhaustive]
pub enum RateLimitStrategy {
    /// Fixed window: counter resets at fixed intervals
    FixedWindow,
    /// Sliding window: weighted average of current and previous window
    #[default]
    SlidingWindow,
    /// Token bucket: smooth rate limiting with configurable refill rate
    TokenBucket {
        /// Tokens added per second
        refill_rate: f64,
    },
}

// =============================================================================
// Rate Limit Configuration
// =============================================================================

/// Configuration for a single rate limit
#[derive(Debug, Clone)]
pub struct RateLimit {
    /// Maximum number of requests allowed
    pub requests: u32,
    /// Time window for the limit
    pub window: Duration,
    /// Strategy for rate calculation
    pub strategy: RateLimitStrategy,
}

impl RateLimit {
    /// Create a new rate limit with sliding window strategy
    pub fn new(requests: u32, window: Duration) -> Self {
        Self {
            requests,
            window,
            strategy: RateLimitStrategy::SlidingWindow,
        }
    }

    /// Create a rate limit with fixed window strategy
    pub fn fixed_window(requests: u32, window: Duration) -> Self {
        Self {
            requests,
            window,
            strategy: RateLimitStrategy::FixedWindow,
        }
    }

    /// Create a rate limit with sliding window strategy
    pub fn sliding_window(requests: u32, window: Duration) -> Self {
        Self {
            requests,
            window,
            strategy: RateLimitStrategy::SlidingWindow,
        }
    }

    /// Create a rate limit with token bucket strategy
    pub fn token_bucket(requests: u32, window: Duration, refill_rate: f64) -> Self {
        Self {
            requests,
            window,
            strategy: RateLimitStrategy::TokenBucket { refill_rate },
        }
    }

    /// Set the strategy
    #[must_use = "This method returns a new RateLimit and does not modify self"]
    pub fn with_strategy(mut self, strategy: RateLimitStrategy) -> Self {
        self.strategy = strategy;
        self
    }
}

impl Default for RateLimit {
    fn default() -> Self {
        // Default: 100 requests per minute
        Self::new(100, Duration::from_secs(60))
    }
}

/// Global rate limit configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Default limit applied to all procedures
    pub default_limit: Option<RateLimit>,
    /// Per-procedure limits (path -> limit)
    pub procedure_limits: HashMap<String, RateLimit>,
    /// Whether rate limiting is enabled
    pub enabled: bool,
}

impl RateLimitConfig {
    /// Create a new empty configuration
    pub fn new() -> Self {
        Self {
            default_limit: None,
            procedure_limits: HashMap::new(),
            enabled: true,
        }
    }

    /// Set the default rate limit
    #[must_use = "This method returns a new RateLimitConfig and does not modify self"]
    pub fn with_default_limit(mut self, limit: RateLimit) -> Self {
        self.default_limit = Some(limit);
        self
    }

    /// Add a rate limit for a specific procedure
    #[must_use = "This method returns a new RateLimitConfig and does not modify self"]
    pub fn with_procedure_limit(mut self, path: impl Into<String>, limit: RateLimit) -> Self {
        self.procedure_limits.insert(path.into(), limit);
        self
    }

    /// Enable or disable rate limiting
    #[must_use = "This method returns a new RateLimitConfig and does not modify self"]
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Get the rate limit for a procedure path
    pub fn get_limit(&self, path: &str) -> Option<&RateLimit> {
        self.procedure_limits
            .get(path)
            .or(self.default_limit.as_ref())
    }
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Rate Limit State
// =============================================================================

/// State for fixed window rate limiting
#[derive(Debug, Clone)]
struct FixedWindowState {
    /// Number of requests in current window
    count: u32,
    /// Start of current window
    window_start: Instant,
}

impl FixedWindowState {
    fn new() -> Self {
        Self {
            count: 0,
            window_start: Instant::now(),
        }
    }
}

/// State for sliding window rate limiting
#[derive(Debug, Clone)]
struct SlidingWindowState {
    /// Requests in current window
    current_count: u32,
    /// Requests in previous window
    previous_count: u32,
    /// Start of current window
    window_start: Instant,
}

impl SlidingWindowState {
    fn new() -> Self {
        Self {
            current_count: 0,
            previous_count: 0,
            window_start: Instant::now(),
        }
    }
}

/// State for token bucket rate limiting
#[derive(Debug, Clone)]
struct TokenBucketState {
    /// Current number of tokens
    tokens: f64,
    /// Last time tokens were refilled
    last_refill: Instant,
}

impl TokenBucketState {
    fn new(max_tokens: u32) -> Self {
        Self {
            tokens: max_tokens as f64,
            last_refill: Instant::now(),
        }
    }
}

/// Combined state for any strategy
#[derive(Debug, Clone)]
enum RateLimitState {
    FixedWindow(FixedWindowState),
    SlidingWindow(SlidingWindowState),
    TokenBucket(TokenBucketState),
}

/// Key for rate limit state (procedure path + client identifier)
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct StateKey {
    path: String,
    client_id: String,
}

impl StateKey {
    fn new(path: impl Into<String>, client_id: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            client_id: client_id.into(),
        }
    }
}

// =============================================================================
// Rate Limiter
// =============================================================================

/// Thread-safe rate limiter
pub struct RateLimiter {
    config: RateLimitConfig,
    state: Arc<RwLock<HashMap<StateKey, RateLimitState>>>,
}

impl RateLimiter {
    /// Create a new rate limiter with the given configuration
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            state: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Check if a request is allowed
    ///
    /// Returns `Ok(())` if allowed, or `Err(RpcError)` with RATE_LIMITED code
    /// and retry_after in details if rate limited.
    pub async fn check(&self, path: &str, client_id: &str) -> Result<(), RpcError> {
        if !self.config.enabled {
            return Ok(());
        }

        let limit = match self.config.get_limit(path) {
            Some(l) => l,
            None => return Ok(()), // No limit configured
        };

        let key = StateKey::new(path, client_id);
        let mut state_map = self.state.write().await;

        let state = state_map
            .entry(key)
            .or_insert_with(|| Self::create_initial_state(limit));

        match &limit.strategy {
            RateLimitStrategy::FixedWindow => self.check_fixed_window(state, limit),
            RateLimitStrategy::SlidingWindow => self.check_sliding_window(state, limit),
            RateLimitStrategy::TokenBucket { refill_rate } => {
                self.check_token_bucket(state, limit, *refill_rate)
            }
        }
    }

    /// Record a request (call after successful check if using two-phase)
    pub async fn record(&self, path: &str, client_id: &str) {
        if !self.config.enabled {
            return;
        }

        if self.config.get_limit(path).is_none() {
            return;
        }

        let key = StateKey::new(path, client_id);
        let mut state_map = self.state.write().await;

        if let Some(state) = state_map.get_mut(&key) {
            match state {
                RateLimitState::FixedWindow(s) => s.count += 1,
                RateLimitState::SlidingWindow(s) => s.current_count += 1,
                RateLimitState::TokenBucket(s) => s.tokens -= 1.0,
            }
        }
    }

    /// Check and record in one operation (most common use case)
    pub async fn check_and_record(&self, path: &str, client_id: &str) -> Result<(), RpcError> {
        self.check(path, client_id).await?;
        self.record(path, client_id).await;
        Ok(())
    }

    /// Get current usage for a path/client combination
    pub async fn get_usage(&self, path: &str, client_id: &str) -> Option<RateLimitUsage> {
        let limit = self.config.get_limit(path)?;
        let key = StateKey::new(path, client_id);
        let state_map = self.state.read().await;

        let state = state_map.get(&key)?;
        let (used, remaining) = match state {
            RateLimitState::FixedWindow(s) => {
                let elapsed = s.window_start.elapsed();
                if elapsed >= limit.window {
                    (0, limit.requests)
                } else {
                    (s.count, limit.requests.saturating_sub(s.count))
                }
            }
            RateLimitState::SlidingWindow(s) => {
                let elapsed = s.window_start.elapsed();
                let weight = 1.0 - (elapsed.as_secs_f64() / limit.window.as_secs_f64()).min(1.0);
                let weighted = (s.previous_count as f64 * weight) + s.current_count as f64;
                let used = weighted.ceil() as u32;
                (used, limit.requests.saturating_sub(used))
            }
            RateLimitState::TokenBucket(s) => {
                let remaining = s.tokens.max(0.0) as u32;
                (limit.requests.saturating_sub(remaining), remaining)
            }
        };

        Some(RateLimitUsage {
            limit: limit.requests,
            used,
            remaining,
            reset_at: limit.window,
        })
    }

    /// Clear all rate limit state (useful for testing)
    pub async fn clear(&self) {
        let mut state_map = self.state.write().await;
        state_map.clear();
    }

    /// Clear state for a specific client
    pub async fn clear_client(&self, client_id: &str) {
        let mut state_map = self.state.write().await;
        state_map.retain(|k, _| k.client_id != client_id);
    }

    // -------------------------------------------------------------------------
    // Strategy Implementations
    // -------------------------------------------------------------------------

    fn create_initial_state(limit: &RateLimit) -> RateLimitState {
        match &limit.strategy {
            RateLimitStrategy::FixedWindow => RateLimitState::FixedWindow(FixedWindowState::new()),
            RateLimitStrategy::SlidingWindow => {
                RateLimitState::SlidingWindow(SlidingWindowState::new())
            }
            RateLimitStrategy::TokenBucket { .. } => {
                RateLimitState::TokenBucket(TokenBucketState::new(limit.requests))
            }
        }
    }

    fn check_fixed_window(
        &self,
        state: &mut RateLimitState,
        limit: &RateLimit,
    ) -> Result<(), RpcError> {
        let RateLimitState::FixedWindow(s) = state else {
            return Ok(());
        };

        let elapsed = s.window_start.elapsed();

        // Reset window if expired
        if elapsed >= limit.window {
            s.count = 0;
            s.window_start = Instant::now();
        }

        // Check limit
        if s.count >= limit.requests {
            let retry_after = limit.window.saturating_sub(elapsed);
            return Err(Self::rate_limited_error(retry_after));
        }

        Ok(())
    }

    fn check_sliding_window(
        &self,
        state: &mut RateLimitState,
        limit: &RateLimit,
    ) -> Result<(), RpcError> {
        let RateLimitState::SlidingWindow(s) = state else {
            return Ok(());
        };

        let elapsed = s.window_start.elapsed();

        // Slide window if needed
        if elapsed >= limit.window {
            s.previous_count = s.current_count;
            s.current_count = 0;
            s.window_start = Instant::now();
        }

        // Calculate weighted count
        let weight =
            1.0 - (s.window_start.elapsed().as_secs_f64() / limit.window.as_secs_f64()).min(1.0);
        let weighted_count = (s.previous_count as f64 * weight) + s.current_count as f64;

        if weighted_count >= limit.requests as f64 {
            // Estimate retry time based on when enough requests will "slide out"
            let excess = weighted_count - limit.requests as f64 + 1.0;
            let retry_secs = (excess / limit.requests as f64) * limit.window.as_secs_f64();
            let retry_after = Duration::from_secs_f64(retry_secs.max(1.0));
            return Err(Self::rate_limited_error(retry_after));
        }

        Ok(())
    }

    fn check_token_bucket(
        &self,
        state: &mut RateLimitState,
        limit: &RateLimit,
        refill_rate: f64,
    ) -> Result<(), RpcError> {
        let RateLimitState::TokenBucket(s) = state else {
            return Ok(());
        };

        // Refill tokens based on elapsed time
        let elapsed = s.last_refill.elapsed();
        let refill = elapsed.as_secs_f64() * refill_rate;
        s.tokens = (s.tokens + refill).min(limit.requests as f64);
        s.last_refill = Instant::now();

        // Check if we have tokens
        if s.tokens < 1.0 {
            // Calculate time until next token
            let needed = 1.0 - s.tokens;
            let retry_secs = needed / refill_rate;
            let retry_after = Duration::from_secs_f64(retry_secs.max(0.1));
            return Err(Self::rate_limited_error(retry_after));
        }

        Ok(())
    }

    fn rate_limited_error(retry_after: Duration) -> RpcError {
        RpcError::rate_limited(format!(
            "Rate limit exceeded. Retry after {} seconds.",
            retry_after.as_secs()
        ))
        .with_details(serde_json::json!({
            "retry_after_ms": retry_after.as_millis(),
            "retry_after_secs": retry_after.as_secs()
        }))
    }
}

impl Clone for RateLimiter {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            state: self.state.clone(),
        }
    }
}

/// Rate limit usage information
#[derive(Debug, Clone)]
pub struct RateLimitUsage {
    /// Maximum requests allowed
    pub limit: u32,
    /// Requests used in current window
    pub used: u32,
    /// Remaining requests
    pub remaining: u32,
    /// Time until limit resets
    pub reset_at: Duration,
}

// =============================================================================
// Middleware
// =============================================================================

/// Create a rate limiting middleware
///
/// # Example
///
/// ```rust,ignore
/// use tauri_plugin_rpc::rate_limit::{rate_limit_middleware, RateLimiter, RateLimitConfig, RateLimit};
/// use std::time::Duration;
///
/// let config = RateLimitConfig::new()
///     .with_default_limit(RateLimit::new(100, Duration::from_secs(60)));
/// let limiter = RateLimiter::new(config);
///
/// let router = Router::new()
///     .middleware(rate_limit_middleware(limiter, |_req| "default-client".to_string()))
///     .query("test", handler);
/// ```
pub fn rate_limit_middleware<Ctx, F>(limiter: RateLimiter, client_id_fn: F) -> MiddlewareFn<Ctx>
where
    Ctx: Clone + Send + Sync + 'static,
    F: Fn(&Request) -> String + Clone + Send + Sync + 'static,
{
    let middleware = move |ctx: Context<Ctx>, req: Request, next: Next<Ctx>| {
        let limiter = limiter.clone();
        let client_id_fn = client_id_fn.clone();
        let path = req.path.clone();
        let client_id = client_id_fn(&req);

        async move {
            // Check rate limit
            match limiter.check_and_record(&path, &client_id).await {
                Ok(()) => {
                    // Get usage info for logging
                    if let Some(usage) = limiter.get_usage(&path, &client_id).await {
                        tracing::trace!(
                            path = %path,
                            client_id = %client_id,
                            remaining = %usage.remaining,
                            limit = %usage.limit,
                            "Rate limit check passed"
                        );
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        path = %path,
                        client_id = %client_id,
                        error_code = %e.code,
                        "Rate limit exceeded"
                    );
                    return Err(e);
                }
            }

            // Proceed with request
            next(ctx, req).await
        }
    };
    from_fn(middleware)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_fixed_window_allows_within_limit() {
        let config = RateLimitConfig::new()
            .with_default_limit(RateLimit::fixed_window(5, Duration::from_secs(60)));
        let limiter = RateLimiter::new(config);

        for _ in 0..5 {
            assert!(limiter.check_and_record("test", "client1").await.is_ok());
        }
    }

    #[tokio::test]
    async fn test_fixed_window_blocks_over_limit() {
        let config = RateLimitConfig::new()
            .with_default_limit(RateLimit::fixed_window(3, Duration::from_secs(60)));
        let limiter = RateLimiter::new(config);

        for _ in 0..3 {
            assert!(limiter.check_and_record("test", "client1").await.is_ok());
        }

        let result = limiter.check_and_record("test", "client1").await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code, crate::RpcErrorCode::RateLimited);
    }

    #[tokio::test]
    async fn test_fixed_window_resets_after_window() {
        let config = RateLimitConfig::new()
            .with_default_limit(RateLimit::fixed_window(2, Duration::from_millis(100)));
        let limiter = RateLimiter::new(config);

        // Use up limit
        assert!(limiter.check_and_record("test", "client1").await.is_ok());
        assert!(limiter.check_and_record("test", "client1").await.is_ok());
        assert!(limiter.check_and_record("test", "client1").await.is_err());

        // Wait for window to reset
        sleep(Duration::from_millis(150)).await;

        // Should be allowed again
        assert!(limiter.check_and_record("test", "client1").await.is_ok());
    }

    #[tokio::test]
    async fn test_sliding_window_allows_within_limit() {
        let config = RateLimitConfig::new()
            .with_default_limit(RateLimit::sliding_window(5, Duration::from_secs(60)));
        let limiter = RateLimiter::new(config);

        for _ in 0..5 {
            assert!(limiter.check_and_record("test", "client1").await.is_ok());
        }
    }

    #[tokio::test]
    async fn test_sliding_window_blocks_over_limit() {
        let config = RateLimitConfig::new()
            .with_default_limit(RateLimit::sliding_window(3, Duration::from_secs(60)));
        let limiter = RateLimiter::new(config);

        for _ in 0..3 {
            assert!(limiter.check_and_record("test", "client1").await.is_ok());
        }

        let result = limiter.check_and_record("test", "client1").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_token_bucket_allows_burst() {
        let config = RateLimitConfig::new().with_default_limit(RateLimit::token_bucket(
            5,
            Duration::from_secs(60),
            1.0,
        ));
        let limiter = RateLimiter::new(config);

        // Should allow burst up to bucket size
        for _ in 0..5 {
            assert!(limiter.check_and_record("test", "client1").await.is_ok());
        }
    }

    #[tokio::test]
    async fn test_token_bucket_blocks_when_empty() {
        let config = RateLimitConfig::new().with_default_limit(RateLimit::token_bucket(
            2,
            Duration::from_secs(60),
            1.0,
        ));
        let limiter = RateLimiter::new(config);

        assert!(limiter.check_and_record("test", "client1").await.is_ok());
        assert!(limiter.check_and_record("test", "client1").await.is_ok());

        let result = limiter.check_and_record("test", "client1").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_token_bucket_refills() {
        let config = RateLimitConfig::new().with_default_limit(RateLimit::token_bucket(
            2,
            Duration::from_secs(60),
            20.0,
        )); // 20 tokens/sec
        let limiter = RateLimiter::new(config);

        // Use all tokens
        assert!(limiter.check_and_record("test", "client1").await.is_ok());
        assert!(limiter.check_and_record("test", "client1").await.is_ok());
        assert!(limiter.check_and_record("test", "client1").await.is_err());

        // Wait for refill (at 20/sec, 1 token in 50ms)
        sleep(Duration::from_millis(100)).await;

        // Should have at least 1 token now
        assert!(limiter.check_and_record("test", "client1").await.is_ok());
    }

    #[tokio::test]
    async fn test_per_procedure_limits() {
        let config = RateLimitConfig::new()
            .with_default_limit(RateLimit::fixed_window(10, Duration::from_secs(60)))
            .with_procedure_limit(
                "expensive",
                RateLimit::fixed_window(2, Duration::from_secs(60)),
            );
        let limiter = RateLimiter::new(config);

        // Expensive procedure has lower limit
        assert!(
            limiter
                .check_and_record("expensive", "client1")
                .await
                .is_ok()
        );
        assert!(
            limiter
                .check_and_record("expensive", "client1")
                .await
                .is_ok()
        );
        assert!(
            limiter
                .check_and_record("expensive", "client1")
                .await
                .is_err()
        );

        // Regular procedure still has higher limit
        for _ in 0..10 {
            assert!(limiter.check_and_record("regular", "client1").await.is_ok());
        }
    }

    #[tokio::test]
    async fn test_per_client_isolation() {
        let config = RateLimitConfig::new()
            .with_default_limit(RateLimit::fixed_window(2, Duration::from_secs(60)));
        let limiter = RateLimiter::new(config);

        // Client 1 uses their limit
        assert!(limiter.check_and_record("test", "client1").await.is_ok());
        assert!(limiter.check_and_record("test", "client1").await.is_ok());
        assert!(limiter.check_and_record("test", "client1").await.is_err());

        // Client 2 has their own limit
        assert!(limiter.check_and_record("test", "client2").await.is_ok());
        assert!(limiter.check_and_record("test", "client2").await.is_ok());
        assert!(limiter.check_and_record("test", "client2").await.is_err());
    }

    #[tokio::test]
    async fn test_disabled_rate_limiting() {
        let config = RateLimitConfig::new()
            .with_default_limit(RateLimit::fixed_window(1, Duration::from_secs(60)))
            .with_enabled(false);
        let limiter = RateLimiter::new(config);

        // Should allow unlimited when disabled
        for _ in 0..100 {
            assert!(limiter.check_and_record("test", "client1").await.is_ok());
        }
    }

    #[tokio::test]
    async fn test_no_limit_configured() {
        let config = RateLimitConfig::new(); // No default limit
        let limiter = RateLimiter::new(config);

        // Should allow unlimited when no limit configured
        for _ in 0..100 {
            assert!(limiter.check_and_record("test", "client1").await.is_ok());
        }
    }

    #[tokio::test]
    async fn test_rate_limited_error_has_retry_after() {
        let config = RateLimitConfig::new()
            .with_default_limit(RateLimit::fixed_window(1, Duration::from_secs(60)));
        let limiter = RateLimiter::new(config);

        assert!(limiter.check_and_record("test", "client1").await.is_ok());

        let result = limiter.check_and_record("test", "client1").await;
        let err = result.unwrap_err();

        assert_eq!(err.code, crate::RpcErrorCode::RateLimited);
        assert!(err.details.is_some());

        let details = err.details.unwrap();
        assert!(details.get("retry_after_ms").is_some());
        assert!(details.get("retry_after_secs").is_some());
    }

    #[tokio::test]
    async fn test_get_usage() {
        let config = RateLimitConfig::new()
            .with_default_limit(RateLimit::fixed_window(5, Duration::from_secs(60)));
        let limiter = RateLimiter::new(config);

        // Initial state - no usage yet
        assert!(limiter.get_usage("test", "client1").await.is_none());

        // After some requests
        limiter.check_and_record("test", "client1").await.unwrap();
        limiter.check_and_record("test", "client1").await.unwrap();

        let usage = limiter.get_usage("test", "client1").await.unwrap();
        assert_eq!(usage.limit, 5);
        assert_eq!(usage.used, 2);
        assert_eq!(usage.remaining, 3);
    }

    #[tokio::test]
    async fn test_clear_client() {
        let config = RateLimitConfig::new()
            .with_default_limit(RateLimit::fixed_window(2, Duration::from_secs(60)));
        let limiter = RateLimiter::new(config);

        // Use up limit
        assert!(limiter.check_and_record("test", "client1").await.is_ok());
        assert!(limiter.check_and_record("test", "client1").await.is_ok());
        assert!(limiter.check_and_record("test", "client1").await.is_err());

        // Clear client state
        limiter.clear_client("client1").await;

        // Should be allowed again
        assert!(limiter.check_and_record("test", "client1").await.is_ok());
    }
}

// =============================================================================
// Property-Based Tests
// =============================================================================

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;
    use std::time::Duration;

    // Strategy for generating rate limit configurations
    fn rate_limit_strategy() -> impl Strategy<Value = RateLimit> {
        (1u32..100, 100u64..10000, 0u8..3).prop_map(|(requests, window_ms, strategy_idx)| {
            let window = Duration::from_millis(window_ms);
            match strategy_idx {
                0 => RateLimit::fixed_window(requests, window),
                1 => RateLimit::sliding_window(requests, window),
                _ => RateLimit::token_bucket(
                    requests,
                    window,
                    requests as f64 / window.as_secs_f64(),
                ),
            }
        })
    }

    // Strategy for generating client IDs
    fn client_id_strategy() -> impl Strategy<Value = String> {
        "[a-z]{1,10}".prop_map(|s| s)
    }

    // Strategy for generating procedure paths
    fn path_strategy() -> impl Strategy<Value = String> {
        "[a-z]{1,5}(\\.[a-z]{1,5}){0,2}".prop_map(|s| s)
    }

    proptest! {
        /// Property 5: Rate Limit Strategy Correctness
        /// For any rate limit configuration, requests within the limit should be allowed
        /// and requests exceeding the limit should be blocked.
        #[test]
        fn prop_rate_limit_allows_within_limit(
            limit in rate_limit_strategy(),
            client_id in client_id_strategy(),
            path in path_strategy(),
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let config = RateLimitConfig::new()
                    .with_procedure_limit(&path, limit.clone());
                let limiter = RateLimiter::new(config);

                // All requests within limit should succeed
                let requests_to_make = limit.requests.min(50); // Cap for test speed
                for _ in 0..requests_to_make {
                    let result = limiter.check_and_record(&path, &client_id).await;
                    prop_assert!(result.is_ok(), "Request within limit should succeed");
                }

                Ok(())
            })?;
        }

        /// Property 6: Rate Limit Enforcement
        /// For any rate limit configuration, exceeding the limit should return RATE_LIMITED error
        /// with retry_after information.
        #[test]
        fn prop_rate_limit_blocks_over_limit(
            requests in 1u32..20,
            window_ms in 1000u64..5000,
            client_id in client_id_strategy(),
            path in path_strategy(),
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let limit = RateLimit::fixed_window(requests, Duration::from_millis(window_ms));
                let config = RateLimitConfig::new()
                    .with_procedure_limit(&path, limit);
                let limiter = RateLimiter::new(config);

                // Use up all allowed requests
                for _ in 0..requests {
                    let _ = limiter.check_and_record(&path, &client_id).await;
                }

                // Next request should be rate limited
                let result = limiter.check_and_record(&path, &client_id).await;
                prop_assert!(result.is_err(), "Request over limit should fail");

                let err = result.unwrap_err();
                prop_assert_eq!(err.code, crate::RpcErrorCode::RateLimited);
                prop_assert!(err.details.is_some(), "Error should have retry_after details");

                let details = err.details.unwrap();
                prop_assert!(details.get("retry_after_ms").is_some());
                prop_assert!(details.get("retry_after_secs").is_some());

                Ok(())
            })?;
        }

        /// Property: Per-client isolation
        /// Different clients should have independent rate limits.
        #[test]
        fn prop_per_client_isolation(
            requests in 1u32..10,
            window_ms in 1000u64..5000,
            path in path_strategy(),
            num_clients in 2usize..5,
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let limit = RateLimit::fixed_window(requests, Duration::from_millis(window_ms));
                let config = RateLimitConfig::new()
                    .with_procedure_limit(&path, limit);
                let limiter = RateLimiter::new(config);

                // Each client should be able to make `requests` calls
                for client_idx in 0..num_clients {
                    let client_id = format!("client{}", client_idx);
                    for _ in 0..requests {
                        let result = limiter.check_and_record(&path, &client_id).await;
                        prop_assert!(result.is_ok(), "Each client should have independent limit");
                    }
                }

                Ok(())
            })?;
        }

        /// Property: Per-procedure limits
        /// Different procedures should have independent rate limits.
        #[test]
        fn prop_per_procedure_limits(
            requests in 1u32..10,
            window_ms in 1000u64..5000,
            client_id in client_id_strategy(),
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let limit = RateLimit::fixed_window(requests, Duration::from_millis(window_ms));
                let config = RateLimitConfig::new()
                    .with_procedure_limit("path1", limit.clone())
                    .with_procedure_limit("path2", limit);
                let limiter = RateLimiter::new(config);

                // Use up limit on path1
                for _ in 0..requests {
                    let _ = limiter.check_and_record("path1", &client_id).await;
                }

                // path2 should still be available
                for _ in 0..requests {
                    let result = limiter.check_and_record("path2", &client_id).await;
                    prop_assert!(result.is_ok(), "Different paths should have independent limits");
                }

                Ok(())
            })?;
        }

        /// Property: Disabled rate limiting allows all requests
        #[test]
        fn prop_disabled_allows_all(
            requests in 1u32..100,
            client_id in client_id_strategy(),
            path in path_strategy(),
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let config = RateLimitConfig::new()
                    .with_default_limit(RateLimit::fixed_window(1, Duration::from_secs(60)))
                    .with_enabled(false);
                let limiter = RateLimiter::new(config);

                // All requests should succeed when disabled
                for _ in 0..requests {
                    let result = limiter.check_and_record(&path, &client_id).await;
                    prop_assert!(result.is_ok(), "Disabled limiter should allow all requests");
                }

                Ok(())
            })?;
        }

        /// Property: Token bucket refills correctly
        /// After waiting, tokens should be refilled proportionally.
        #[test]
        fn prop_token_bucket_refills(
            bucket_size in 2u32..5,
            refill_rate in 50.0f64..200.0,
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let limit = RateLimit::token_bucket(bucket_size, Duration::from_secs(60), refill_rate);
                let config = RateLimitConfig::new()
                    .with_procedure_limit("test", limit);
                let limiter = RateLimiter::new(config);

                // Use all tokens
                for _ in 0..bucket_size {
                    let _ = limiter.check_and_record("test", "client").await;
                }

                // Should be rate limited
                let result = limiter.check_and_record("test", "client").await;
                prop_assert!(result.is_err(), "Should be rate limited after using all tokens");

                // Wait for at least one token to refill (with high refill rate, this is fast)
                let wait_time = Duration::from_secs_f64(1.5 / refill_rate);
                tokio::time::sleep(wait_time).await;

                // Should have at least one token now
                let result = limiter.check_and_record("test", "client").await;
                prop_assert!(result.is_ok(), "Should have refilled at least one token");

                Ok(())
            })?;
        }
    }
}
