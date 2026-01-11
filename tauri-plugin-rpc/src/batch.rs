//! Batch request processing for RPC operations
//!
//! This module provides types and utilities for processing multiple RPC calls
//! in a single request, reducing IPC overhead.
//!
//! # Example
//!
//! ```rust,ignore
//! use tauri_plugin_rpc::batch::{BatchRequest, SingleRequest, BatchConfig};
//!
//! let batch = BatchRequest {
//!     requests: vec![
//!         SingleRequest { id: "1".into(), path: "user.get".into(), input: json!({"id": 1}) },
//!         SingleRequest { id: "2".into(), path: "user.list".into(), input: json!(null) },
//!     ],
//! };
//!
//! let config = BatchConfig::default();
//! let results = router.call_batch(batch, &config).await;
//! ```

use crate::RpcError;
use serde::{Deserialize, Serialize};

// =============================================================================
// Batch Configuration
// =============================================================================

/// Configuration for batch request processing.
#[derive(Debug, Clone)]
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
        Self::default()
    }

    /// Set the maximum batch size.
    pub fn with_max_batch_size(mut self, size: usize) -> Self {
        self.max_batch_size = size;
        self
    }

    /// Set whether to execute requests in parallel.
    pub fn with_parallel_execution(mut self, parallel: bool) -> Self {
        self.parallel_execution = parallel;
        self
    }

    /// Validate the batch configuration.
    pub fn validate(&self) -> Result<(), String> {
        if self.max_batch_size == 0 {
            return Err("max_batch_size must be greater than 0".to_string());
        }
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
    pub input: serde_json::Value,
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
        self.requests.push(SingleRequest {
            id: id.into(),
            path: path.into(),
            input,
        });
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
            return Err(RpcError::bad_request("Batch request cannot be empty"));
        }
        if self.requests.len() > config.max_batch_size {
            return Err(RpcError::bad_request(format!(
                "Batch size {} exceeds maximum allowed size {}",
                self.requests.len(),
                config.max_batch_size
            )));
        }
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
        Self {
            id: id.into(),
            result: BatchResultData::Success { data },
        }
    }

    /// Create an error batch result.
    pub fn error(id: impl Into<String>, error: RpcError) -> Self {
        Self {
            id: id.into(),
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
