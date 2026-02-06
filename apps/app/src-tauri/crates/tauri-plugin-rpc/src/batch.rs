//! Batch request processing for RPC operations
//!
//! This module provides types and utilities for processing multiple RPC calls
//! in a single request, reducing IPC overhead.
//!
//! # Features
//!
//! - **Streaming execution**: Uses `FuturesUnordered` for efficient parallel processing
//! - **Fail-fast validation**: Validates batch before executing any requests
//! - **Metrics tracking**: Collects success/error counts and execution duration
//! - **Error context preservation**: Maintains full error information in results
//!
//! # Example
//!
//! ```rust,ignore
//! use tauri_plugin_rpc::batch::{BatchRequest, SingleRequest, BatchConfig, execute_batch};
//!
//! let batch = BatchRequest {
//!     requests: vec![
//!         SingleRequest { id: "1".into(), path: "user.get".into(), input: json!({"id": 1}) },
//!         SingleRequest { id: "2".into(), path: "user.list".into(), input: json!(null) },
//!     ],
//! };
//!
//! let (response, metrics) = execute_batch(batch, router, &config).await?;
//! println!("Processed {} requests in {}ms", metrics.total_requests, metrics.duration_ms);
//! ```

use crate::{RpcConfig, RpcError, plugin::DynRouter};
use futures::stream::{FuturesUnordered, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, trace, warn};

// =============================================================================
// Batch Configuration
// =============================================================================

/// Configuration for batch request processing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchConfig {
    /// Maximum number of requests allowed in a single batch.
    /// Requests exceeding this limit will be rejected.
    pub max_batch_size: usize,
    /// Whether to execute requests in parallel.
    /// When true, uses `futures::join_all` for concurrent execution.
    /// When false, executes requests sequentially.
    pub parallel_execution: bool,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            max_batch_size: 100,
            parallel_execution: true,
        }
    }
}

impl BatchConfig {
    /// Create a new batch configuration with default values.
    pub fn new() -> Self {
        trace!("Creating new BatchConfig with defaults");
        Self::default()
    }

    /// Set the maximum batch size.
    #[must_use = "This method returns a new BatchConfig and does not modify self"]
    pub fn with_max_batch_size(mut self, size: usize) -> Self {
        trace!(max_batch_size = size, "Setting batch max size");
        self.max_batch_size = size;
        self
    }

    /// Set whether to execute requests in parallel.
    #[must_use = "This method returns a new BatchConfig and does not modify self"]
    pub fn with_parallel_execution(mut self, parallel: bool) -> Self {
        trace!(
            parallel_execution = parallel,
            "Setting batch parallel execution"
        );
        self.parallel_execution = parallel;
        self
    }

    /// Validate the batch configuration.
    pub fn validate(&self) -> Result<(), String> {
        if self.max_batch_size == 0 {
            warn!("BatchConfig validation failed: max_batch_size must be greater than 0");
            return Err("max_batch_size must be greater than 0".to_string());
        }
        trace!(
            max_batch_size = self.max_batch_size,
            parallel = self.parallel_execution,
            "BatchConfig validated"
        );
        Ok(())
    }
}

// =============================================================================
// Batch Request Types
// =============================================================================

/// A single request within a batch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SingleRequest {
    /// Unique identifier for this request within the batch.
    /// Used to correlate results with requests.
    pub id: String,
    /// The procedure path to call (e.g., "user.get", "post.create").
    pub path: String,
    /// Input data for the procedure.
    /// Defaults to null if not provided.
    #[serde(default = "default_input")]
    pub input: serde_json::Value,
}

/// Default input value for requests without input.
fn default_input() -> serde_json::Value {
    serde_json::Value::Null
}

/// A batch of RPC requests to be processed together.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchRequest {
    /// The list of requests to process.
    pub requests: Vec<SingleRequest>,
}

impl BatchRequest {
    /// Create a new empty batch request.
    pub fn new() -> Self {
        trace!("Creating new empty BatchRequest");
        Self {
            requests: Vec::new(),
        }
    }

    /// Add a request to the batch.
    pub fn add(
        mut self,
        id: impl Into<String>,
        path: impl Into<String>,
        input: serde_json::Value,
    ) -> Self {
        let id = id.into();
        let path = path.into();
        trace!(request_id = %id, path = %path, "Adding request to batch");
        self.requests.push(SingleRequest { id, path, input });
        self
    }

    /// Get the number of requests in the batch.
    pub fn len(&self) -> usize {
        self.requests.len()
    }

    /// Check if the batch is empty.
    pub fn is_empty(&self) -> bool {
        self.requests.is_empty()
    }

    /// Validate the batch against configuration limits.
    pub fn validate(&self, config: &BatchConfig) -> Result<(), RpcError> {
        if self.requests.is_empty() {
            warn!("Batch validation failed: batch request cannot be empty");
            return Err(RpcError::bad_request("Batch request cannot be empty"));
        }
        if self.requests.len() > config.max_batch_size {
            warn!(
                batch_size = self.requests.len(),
                max_size = config.max_batch_size,
                "Batch validation failed: size exceeds maximum"
            );
            return Err(RpcError::bad_request(format!(
                "Batch size {} exceeds maximum allowed size {}",
                self.requests.len(),
                config.max_batch_size
            )));
        }
        debug!(
            batch_size = self.requests.len(),
            max_size = config.max_batch_size,
            "Batch request validated"
        );
        Ok(())
    }
}

impl Default for BatchRequest {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Batch Response Types
// =============================================================================

/// Result of a single request within a batch.
/// Contains either the successful result or an error.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchResult {
    /// The ID of the request this result corresponds to.
    pub id: String,
    /// The result of the request - either success data or error.
    #[serde(flatten)]
    pub result: BatchResultData,
}

/// The data portion of a batch result.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BatchResultData {
    /// Successful result with data.
    Success {
        /// The result data from the procedure.
        data: serde_json::Value,
    },
    /// Error result.
    Error {
        /// The error that occurred.
        error: RpcError,
    },
}

impl BatchResult {
    /// Create a successful batch result.
    pub fn success(id: impl Into<String>, data: serde_json::Value) -> Self {
        let id = id.into();
        trace!(request_id = %id, "Batch result: success");
        Self {
            id,
            result: BatchResultData::Success { data },
        }
    }

    /// Create an error batch result.
    pub fn error(id: impl Into<String>, error: RpcError) -> Self {
        let id = id.into();
        debug!(
            request_id = %id,
            error_code = %error.code,
            error_message = %error.message,
            "Batch result: error"
        );
        Self {
            id,
            result: BatchResultData::Error { error },
        }
    }

    /// Check if this result is successful.
    pub fn is_success(&self) -> bool {
        matches!(self.result, BatchResultData::Success { .. })
    }

    /// Check if this result is an error.
    pub fn is_error(&self) -> bool {
        matches!(self.result, BatchResultData::Error { .. })
    }

    /// Get the data if successful.
    pub fn data(&self) -> Option<&serde_json::Value> {
        match &self.result {
            BatchResultData::Success { data } => Some(data),
            BatchResultData::Error { .. } => None,
        }
    }

    /// Get the error if failed.
    pub fn get_error(&self) -> Option<&RpcError> {
        match &self.result {
            BatchResultData::Success { .. } => None,
            BatchResultData::Error { error } => Some(error),
        }
    }
}

/// Response containing results for all requests in a batch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchResponse {
    /// Results for each request, in the same order as the input requests.
    pub results: Vec<BatchResult>,
}

impl BatchResponse {
    /// Create a new batch response with the given results.
    pub fn new(results: Vec<BatchResult>) -> Self {
        let success_count = results.iter().filter(|r| r.is_success()).count();
        let error_count = results.iter().filter(|r| r.is_error()).count();
        debug!(
            total = results.len(),
            success = success_count,
            errors = error_count,
            "Created batch response"
        );
        Self { results }
    }

    /// Get the number of results.
    pub fn len(&self) -> usize {
        self.results.len()
    }

    /// Check if the response is empty.
    pub fn is_empty(&self) -> bool {
        self.results.is_empty()
    }

    /// Count successful results.
    pub fn success_count(&self) -> usize {
        self.results.iter().filter(|r| r.is_success()).count()
    }

    /// Count error results.
    pub fn error_count(&self) -> usize {
        self.results.iter().filter(|r| r.is_error()).count()
    }

    /// Check if all results are successful.
    pub fn all_success(&self) -> bool {
        self.results.iter().all(|r| r.is_success())
    }

    /// Check if any result is an error.
    pub fn has_errors(&self) -> bool {
        self.results.iter().any(|r| r.is_error())
    }

    /// Get result by request ID.
    pub fn get(&self, id: &str) -> Option<&BatchResult> {
        self.results.iter().find(|r| r.id == id)
    }
}

// =============================================================================
// Batch Execution
// =============================================================================

/// Metrics collected during batch execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchMetrics {
    /// Total number of requests in the batch
    pub total_requests: usize,
    /// Number of successful requests
    pub success_count: usize,
    /// Number of failed requests
    pub error_count: usize,
    /// Total execution duration in milliseconds
    pub duration_ms: u64,
}

impl BatchMetrics {
    /// Create new metrics with default values.
    pub fn new(total_requests: usize) -> Self {
        Self {
            total_requests,
            success_count: 0,
            error_count: 0,
            duration_ms: 0,
        }
    }
}

/// Execute a batch of RPC requests with streaming and metrics collection.
///
/// This function:
/// 1. Validates the batch size before execution (fail-fast)
/// 2. Executes requests in parallel using `FuturesUnordered` for streaming
/// 3. Tracks metrics (success/error counts, duration)
/// 4. Preserves error context in results
///
/// # Arguments
///
/// * `batch` - The batch request to execute
/// * `router` - The RPC router for executing procedures
/// * `rpc_config` - RPC configuration for validation
///
/// # Returns
///
/// A tuple of `(BatchResponse, BatchMetrics)` containing the results and metrics.
///
/// # Errors
///
/// Returns an error if batch validation fails (size limits, empty batch, etc.).
///
/// # Examples
///
/// ```rust,ignore
/// let (response, metrics) = execute_batch(batch, router, &config).await?;
/// assert_eq!(metrics.total_requests, batch.len());
/// ```
pub async fn execute_batch(
    batch: BatchRequest,
    router: Arc<dyn DynRouter>,
    rpc_config: &RpcConfig,
) -> Result<(BatchResponse, BatchMetrics), RpcError> {
    let start = std::time::Instant::now();
    let batch_config = &rpc_config.batch_config;

    // Fail-fast validation: Check batch size before individual request validation
    if let Err(e) = batch.validate(batch_config) {
        warn!(
            batch_size = batch.len(),
            max_size = batch_config.max_batch_size,
            "Batch validation failed"
        );
        return Err(e);
    }

    debug!(
        batch_size = batch.len(),
        parallel = batch_config.parallel_execution,
        "Executing batch request"
    );

    let total_requests = batch.len();
    let mut metrics = BatchMetrics::new(total_requests);

    // Execute batch using streaming for better performance
    let results = if batch_config.parallel_execution {
        execute_parallel(&batch, router).await
    } else {
        execute_sequential(&batch, router).await
    };

    // Update metrics
    for result in &results {
        if result.is_success() {
            metrics.success_count += 1;
        } else {
            metrics.error_count += 1;
        }
    }

    metrics.duration_ms = start.elapsed().as_millis() as u64;

    debug!(
        total = metrics.total_requests,
        success = metrics.success_count,
        errors = metrics.error_count,
        duration_ms = metrics.duration_ms,
        "Batch execution completed"
    );

    let response = BatchResponse::new(results);
    Ok((response, metrics))
}

/// Execute batch requests in parallel using FuturesUnordered for streaming.
async fn execute_parallel(batch: &BatchRequest, router: Arc<dyn DynRouter>) -> Vec<BatchResult> {
    let mut futures = FuturesUnordered::new();

    for req in &batch.requests {
        let id = req.id.clone();
        let path = req.path.clone();
        let input = req.input.clone();
        let router = router.clone();

        futures.push(async move {
            match router.call(&path, input).await {
                Ok(data) => {
                    debug!(request_id = %id, path = %path, "Batch request succeeded");
                    BatchResult::success(id, data)
                }
                Err(error) => {
                    warn!(
                        request_id = %id,
                        path = %path,
                        error_code = %error.code,
                        error_message = %error.message,
                        "Batch request failed"
                    );
                    BatchResult::error(id, error)
                }
            }
        });
    }

    let mut results = Vec::with_capacity(batch.len());
    while let Some(result) = futures.next().await {
        results.push(result);
    }

    // Preserve original order by sorting by request ID
    let original_order: std::collections::HashMap<_, _> = batch
        .requests
        .iter()
        .enumerate()
        .map(|(i, req)| (req.id.clone(), i))
        .collect();

    results.sort_by_key(|r| original_order.get(&r.id).copied().unwrap_or(usize::MAX));
    results
}

/// Execute batch requests sequentially.
async fn execute_sequential(batch: &BatchRequest, router: Arc<dyn DynRouter>) -> Vec<BatchResult> {
    let mut results = Vec::with_capacity(batch.len());

    for req in &batch.requests {
        let result = match router.call(&req.path, req.input.clone()).await {
            Ok(data) => {
                debug!(request_id = %req.id, path = %req.path, "Batch request succeeded");
                BatchResult::success(&req.id, data)
            }
            Err(error) => {
                warn!(
                    request_id = %req.id,
                    path = %req.path,
                    error_code = %error.code,
                    error_message = %error.message,
                    "Batch request failed"
                );
                BatchResult::error(&req.id, error)
            }
        };
        results.push(result);
    }

    results
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod execution_tests {
    use super::*;
    use crate::{Context, EmptyContext, Router, RpcResult};
    use proptest::prelude::*;
    use serde_json::json;

    // Mock handler for testing
    async fn mock_handler(
        _ctx: Context<EmptyContext>,
        input: serde_json::Value,
    ) -> RpcResult<serde_json::Value> {
        if input.get("fail").and_then(|v| v.as_bool()).unwrap_or(false) {
            Err(RpcError::internal("Mock error"))
        } else {
            Ok(json!({"result": "success"}))
        }
    }

    fn create_test_router() -> Router<EmptyContext> {
        Router::new()
            .context(EmptyContext)
            .query("test.success", mock_handler)
            .query("test.fail", mock_handler)
    }

    // Property 17: Batch streaming execution
    proptest! {
        #[test]
        fn prop_batch_streaming_execution(
            count in 1usize..20usize
        ) {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                let router = create_test_router().compile();
                let config = RpcConfig::default();

                let mut batch = BatchRequest::new();
                for i in 0..count {
                    batch = batch.add(format!("req_{}", i), "test.success", json!({}));
                }

                let result = execute_batch(batch, Arc::new(router), &config).await;
                assert!(result.is_ok());

                let (response, metrics) = result.unwrap();
                assert_eq!(metrics.total_requests, count);
                assert_eq!(response.len(), count);
            });
        }
    }

    // Property 18: Batch validation fail-fast
    #[test]
    fn test_batch_validation_fail_fast() {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let router = create_test_router().compile();
            let config = RpcConfig::default()
                .with_batch_config(BatchConfig::default().with_max_batch_size(2));

            // Create oversized batch
            let batch = BatchRequest::new()
                .add("1", "test.success", json!({}))
                .add("2", "test.success", json!({}))
                .add("3", "test.success", json!({}));

            let result = execute_batch(batch, Arc::new(router), &config).await;
            assert!(result.is_err());

            let err = result.unwrap_err();
            assert_eq!(err.code.as_str(), "BAD_REQUEST");
            assert!(err.message.contains("exceeds maximum"));
        });
    }

    // Property 19: Batch metrics accuracy
    proptest! {
        #[test]
        fn prop_batch_metrics_accuracy(
            success_count in 1usize..10usize,
            error_count in 0usize..10usize
        ) {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                let router = create_test_router().compile();
                let config = RpcConfig::default();

                let mut batch = BatchRequest::new();
                for i in 0..success_count {
                    batch = batch.add(format!("success_{}", i), "test.success", json!({}));
                }
                for i in 0..error_count {
                    batch = batch.add(format!("error_{}", i), "test.fail", json!({"fail": true}));
                }

                let result = execute_batch(batch, Arc::new(router), &config).await;
                assert!(result.is_ok());

                let (response, metrics) = result.unwrap();
                assert_eq!(metrics.total_requests, success_count + error_count);
                assert_eq!(metrics.success_count, success_count);
                assert_eq!(metrics.error_count, error_count);
                assert_eq!(response.success_count(), success_count);
                assert_eq!(response.error_count(), error_count);
            });
        }
    }

    // Property 20: Batch size limit enforcement
    #[test]
    fn test_batch_size_limit_enforcement() {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let router = Arc::new(create_test_router().compile());
            let config = RpcConfig::default()
                .with_batch_config(BatchConfig::default().with_max_batch_size(5));

            // Exactly at limit - should succeed
            let mut batch = BatchRequest::new();
            for i in 0..5 {
                batch = batch.add(format!("req_{}", i), "test.success", json!({}));
            }
            assert!(execute_batch(batch, router.clone(), &config).await.is_ok());

            // Over limit - should fail
            let mut batch = BatchRequest::new();
            for i in 0..6 {
                batch = batch.add(format!("req_{}", i), "test.success", json!({}));
            }
            assert!(execute_batch(batch, router, &config).await.is_err());
        });
    }

    // Property 21: Batch size validation precedes request validation
    #[test]
    fn test_batch_size_validation_first() {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let router = create_test_router().compile();
            let config = RpcConfig::default()
                .with_batch_config(BatchConfig::default().with_max_batch_size(1));

            // Create batch with 2 requests (exceeds limit)
            // Even if individual requests are invalid, batch size check should fail first
            let batch = BatchRequest::new()
                .add("1", "", json!({})) // Invalid path
                .add("2", "", json!({})); // Invalid path

            let result = execute_batch(batch, Arc::new(router), &config).await;
            assert!(result.is_err());

            let err = result.unwrap_err();
            // Should be batch size error, not path validation error
            assert!(err.message.contains("exceeds maximum") || err.message.contains("Batch"));
        });
    }

    // Property 22: Batch failure context preservation
    #[test]
    fn test_batch_failure_context_preservation() {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let router = create_test_router().compile();
            let config = RpcConfig::default();

            let batch = BatchRequest::new()
                .add("req_1", "test.success", json!({}))
                .add("req_2", "test.fail", json!({"fail": true}))
                .add("req_3", "test.success", json!({}));

            let result = execute_batch(batch, Arc::new(router), &config).await;
            assert!(result.is_ok());

            let (response, _) = result.unwrap();

            // Check that error context is preserved
            let error_result = response.get("req_2").unwrap();
            assert!(error_result.is_error());

            let error = error_result.get_error().unwrap();
            assert_eq!(error.code.as_str(), "INTERNAL_ERROR");
            assert_eq!(error.message, "Mock error");
        });
    }

    // Unit tests for execution modes
    #[test]
    fn test_parallel_execution_mode() {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let router = create_test_router().compile();
            let config = RpcConfig::default()
                .with_batch_config(BatchConfig::default().with_parallel_execution(true));

            let batch = BatchRequest::new()
                .add("1", "test.success", json!({}))
                .add("2", "test.success", json!({}))
                .add("3", "test.success", json!({}));

            let result = execute_batch(batch, Arc::new(router), &config).await;
            assert!(result.is_ok());

            let (response, metrics) = result.unwrap();
            assert_eq!(metrics.success_count, 3);
            assert_eq!(response.len(), 3);
        });
    }

    #[test]
    fn test_sequential_execution_mode() {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let router = create_test_router().compile();
            let config = RpcConfig::default()
                .with_batch_config(BatchConfig::default().with_parallel_execution(false));

            let batch = BatchRequest::new()
                .add("1", "test.success", json!({}))
                .add("2", "test.success", json!({}))
                .add("3", "test.success", json!({}));

            let result = execute_batch(batch, Arc::new(router), &config).await;
            assert!(result.is_ok());

            let (response, metrics) = result.unwrap();
            assert_eq!(metrics.success_count, 3);
            assert_eq!(response.len(), 3);

            // Verify order is preserved in sequential mode
            assert_eq!(response.results[0].id, "1");
            assert_eq!(response.results[1].id, "2");
            assert_eq!(response.results[2].id, "3");
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_batch_config_default() {
        let config = BatchConfig::default();
        assert_eq!(config.max_batch_size, 100);
        assert!(config.parallel_execution);
    }

    #[test]
    fn test_batch_config_builder() {
        let config = BatchConfig::new()
            .with_max_batch_size(50)
            .with_parallel_execution(false);
        assert_eq!(config.max_batch_size, 50);
        assert!(!config.parallel_execution);
    }

    #[test]
    fn test_batch_config_validation() {
        let config = BatchConfig::new().with_max_batch_size(0);
        assert!(config.validate().is_err());

        let config = BatchConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_batch_request_builder() {
        let batch = BatchRequest::new()
            .add("1", "user.get", json!({"id": 1}))
            .add("2", "user.list", json!(null));

        assert_eq!(batch.len(), 2);
        assert!(!batch.is_empty());
    }

    #[test]
    fn test_batch_request_validation() {
        let config = BatchConfig::new().with_max_batch_size(2);

        // Empty batch should fail
        let empty = BatchRequest::new();
        assert!(empty.validate(&config).is_err());

        // Valid batch should pass
        let valid = BatchRequest::new().add("1", "test", json!(null));
        assert!(valid.validate(&config).is_ok());

        // Oversized batch should fail
        let oversized = BatchRequest::new()
            .add("1", "test", json!(null))
            .add("2", "test", json!(null))
            .add("3", "test", json!(null));
        assert!(oversized.validate(&config).is_err());
    }

    #[test]
    fn test_batch_result_success() {
        let result = BatchResult::success("1", json!({"name": "test"}));
        assert!(result.is_success());
        assert!(!result.is_error());
        assert_eq!(result.data(), Some(&json!({"name": "test"})));
        assert!(result.get_error().is_none());
    }

    #[test]
    fn test_batch_result_error() {
        let result = BatchResult::error("1", RpcError::not_found("Not found"));
        assert!(!result.is_success());
        assert!(result.is_error());
        assert!(result.data().is_none());
        assert!(result.get_error().is_some());
    }

    #[test]
    fn test_batch_response() {
        let response = BatchResponse::new(vec![
            BatchResult::success("1", json!({"id": 1})),
            BatchResult::error("2", RpcError::not_found("Not found")),
            BatchResult::success("3", json!({"id": 3})),
        ]);

        assert_eq!(response.len(), 3);
        assert_eq!(response.success_count(), 2);
        assert_eq!(response.error_count(), 1);
        assert!(!response.all_success());
        assert!(response.has_errors());
        assert!(response.get("1").is_some());
        assert!(response.get("4").is_none());
    }

    #[test]
    fn test_batch_serialization() {
        let batch = BatchRequest::new().add("1", "user.get", json!({"id": 1}));

        let json = serde_json::to_string(&batch).unwrap();
        let parsed: BatchRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed.requests[0].id, "1");
        assert_eq!(parsed.requests[0].path, "user.get");
    }

    #[test]
    fn test_batch_response_serialization() {
        let response = BatchResponse::new(vec![
            BatchResult::success("1", json!({"name": "test"})),
            BatchResult::error("2", RpcError::not_found("Not found")),
        ]);

        let json = serde_json::to_string(&response).unwrap();
        let parsed: BatchResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.len(), 2);
        assert!(parsed.results[0].is_success());
        assert!(parsed.results[1].is_error());
    }
}
