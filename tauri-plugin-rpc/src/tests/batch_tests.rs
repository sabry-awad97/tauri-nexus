//! Batch processing tests
//!
//! Tests for batch request processing including result ordering
//! and error isolation properties.

use crate::batch::{BatchConfig, BatchRequest};
use crate::{Context, EmptyContext, Router, RpcError, RpcResult};
use proptest::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::json;

// =============================================================================
// Test Handlers
// =============================================================================

#[derive(Debug, Clone, Deserialize, Serialize)]
struct GetInput {
    id: i32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct GetOutput {
    id: i32,
    name: String,
}

async fn get_handler(_ctx: Context<EmptyContext>, input: GetInput) -> RpcResult<GetOutput> {
    // Simulate a lookup - IDs 1-100 exist, others don't
    if input.id > 0 && input.id <= 100 {
        Ok(GetOutput {
            id: input.id,
            name: format!("Item {}", input.id),
        })
    } else {
        Err(RpcError::not_found(format!("Item {} not found", input.id)))
    }
}

async fn echo_handler(
    _ctx: Context<EmptyContext>,
    input: serde_json::Value,
) -> RpcResult<serde_json::Value> {
    Ok(input)
}

async fn fail_handler(
    _ctx: Context<EmptyContext>,
    _input: serde_json::Value,
) -> RpcResult<serde_json::Value> {
    Err(RpcError::internal("Always fails"))
}

fn create_test_router() -> Router<EmptyContext> {
    Router::new()
        .context(EmptyContext)
        .query("item.get", get_handler)
        .query("echo", echo_handler)
        .query("fail", fail_handler)
}

// =============================================================================
// Unit Tests
// =============================================================================

#[tokio::test]
async fn test_batch_single_request() {
    let router = create_test_router().compile();
    let config = BatchConfig::default();

    let batch = BatchRequest::new().add("1", "item.get", json!({"id": 1}));

    let response = router.call_batch(batch, &config).await.unwrap();

    assert_eq!(response.len(), 1);
    assert!(response.results[0].is_success());
    assert_eq!(response.results[0].id, "1");
}

#[tokio::test]
async fn test_batch_multiple_requests() {
    let router = create_test_router().compile();
    let config = BatchConfig::default();

    let batch = BatchRequest::new()
        .add("1", "item.get", json!({"id": 1}))
        .add("2", "item.get", json!({"id": 2}))
        .add("3", "item.get", json!({"id": 3}));

    let response = router.call_batch(batch, &config).await.unwrap();

    assert_eq!(response.len(), 3);
    assert!(response.all_success());
}

#[tokio::test]
async fn test_batch_result_ordering() {
    let router = create_test_router().compile();
    let config = BatchConfig::default();

    let batch = BatchRequest::new()
        .add("first", "echo", json!({"order": 1}))
        .add("second", "echo", json!({"order": 2}))
        .add("third", "echo", json!({"order": 3}));

    let response = router.call_batch(batch, &config).await.unwrap();

    assert_eq!(response.results[0].id, "first");
    assert_eq!(response.results[1].id, "second");
    assert_eq!(response.results[2].id, "third");
}

#[tokio::test]
async fn test_batch_error_isolation() {
    let router = create_test_router().compile();
    let config = BatchConfig::default();

    let batch = BatchRequest::new()
        .add("1", "item.get", json!({"id": 1})) // Success
        .add("2", "fail", json!(null)) // Fail
        .add("3", "item.get", json!({"id": 3})); // Success

    let response = router.call_batch(batch, &config).await.unwrap();

    assert_eq!(response.len(), 3);
    assert!(response.results[0].is_success(), "First should succeed");
    assert!(response.results[1].is_error(), "Second should fail");
    assert!(response.results[2].is_success(), "Third should succeed");
    assert_eq!(response.success_count(), 2);
    assert_eq!(response.error_count(), 1);
}

#[tokio::test]
async fn test_batch_all_failures() {
    let router = create_test_router().compile();
    let config = BatchConfig::default();

    let batch = BatchRequest::new()
        .add("1", "fail", json!(null))
        .add("2", "fail", json!(null))
        .add("3", "fail", json!(null));

    let response = router.call_batch(batch, &config).await.unwrap();

    assert_eq!(response.len(), 3);
    assert!(response.results.iter().all(|r| r.is_error()));
    assert_eq!(response.error_count(), 3);
}

#[tokio::test]
async fn test_batch_max_size_exceeded() {
    let router = create_test_router().compile();
    let config = BatchConfig::new().with_max_batch_size(2);

    let batch = BatchRequest::new()
        .add("1", "echo", json!(null))
        .add("2", "echo", json!(null))
        .add("3", "echo", json!(null));

    let result = router.call_batch(batch, &config).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_batch_empty_rejected() {
    let router = create_test_router().compile();
    let config = BatchConfig::default();

    let batch = BatchRequest::new();

    let result = router.call_batch(batch, &config).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_batch_sequential_execution() {
    let router = create_test_router().compile();
    let config = BatchConfig::new().with_parallel_execution(false);

    let batch = BatchRequest::new()
        .add("1", "echo", json!({"n": 1}))
        .add("2", "echo", json!({"n": 2}))
        .add("3", "echo", json!({"n": 3}));

    let response = router.call_batch(batch, &config).await.unwrap();

    assert_eq!(response.len(), 3);
    assert!(response.all_success());
    // Verify ordering is maintained
    assert_eq!(response.results[0].id, "1");
    assert_eq!(response.results[1].id, "2");
    assert_eq!(response.results[2].id, "3");
}

#[tokio::test]
async fn test_batch_procedure_not_found() {
    let router = create_test_router().compile();
    let config = BatchConfig::default();

    let batch = BatchRequest::new().add("1", "nonexistent", json!(null));

    let response = router.call_batch(batch, &config).await.unwrap();

    assert_eq!(response.len(), 1);
    assert!(response.results[0].is_error());
}

#[tokio::test]
async fn test_batch_mixed_procedures() {
    let router = create_test_router().compile();
    let config = BatchConfig::default();

    let batch = BatchRequest::new()
        .add("1", "item.get", json!({"id": 1}))
        .add("2", "echo", json!({"test": true}))
        .add("3", "item.get", json!({"id": 999})); // Not found

    let response = router.call_batch(batch, &config).await.unwrap();

    assert_eq!(response.len(), 3);
    assert!(response.results[0].is_success());
    assert!(response.results[1].is_success());
    assert!(response.results[2].is_error());
}

// =============================================================================
// Property-Based Tests
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Property 3: Batch Result Ordering**
    /// *For any* batch request containing N procedure calls, the batch response
    /// SHALL contain exactly N results in the same order as input requests.
    /// **Feature: tauri-rpc-framework, Property 3: Batch Result Ordering**
    /// **Validates: Requirements 8.2, 8.3**
    #[test]
    fn prop_batch_result_ordering(
        ids in prop::collection::vec("[a-z0-9]{1,10}", 1..20)
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let router = create_test_router().compile();
            let config = BatchConfig::default();

            // Create batch with unique IDs
            let mut batch = BatchRequest::new();
            for id in &ids {
                batch = batch.add(id.clone(), "echo", json!({"id": id}));
            }

            let response = router.call_batch(batch, &config).await.unwrap();

            // Property: Result count equals request count
            prop_assert_eq!(
                response.len(),
                ids.len(),
                "Response should have same number of results as requests"
            );

            // Property: Results are in same order as requests
            for (i, id) in ids.iter().enumerate() {
                prop_assert_eq!(
                    &response.results[i].id,
                    id,
                    "Result {} should have ID '{}', got '{}'",
                    i,
                    id,
                    response.results[i].id
                );
            }

            Ok(())
        })?;
    }

    /// **Property 4: Batch Error Isolation**
    /// *For any* batch request where some procedures fail, the Batch_Processor
    /// SHALL complete all non-failing procedures and return their results alongside errors.
    /// **Feature: tauri-rpc-framework, Property 4: Batch Error Isolation**
    /// **Validates: Requirements 8.4**
    #[test]
    fn prop_batch_error_isolation(
        // Generate a mix of valid IDs (1-100) and invalid IDs (>100)
        request_ids in prop::collection::vec(1i32..200, 1..15)
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let router = create_test_router().compile();
            let config = BatchConfig::default();

            // Create batch with mix of valid and invalid IDs
            let mut batch = BatchRequest::new();
            for (i, id) in request_ids.iter().enumerate() {
                batch = batch.add(
                    format!("req-{}", i),
                    "item.get",
                    json!({"id": id})
                );
            }

            let response = router.call_batch(batch, &config).await.unwrap();

            // Property: All requests get a result
            prop_assert_eq!(
                response.len(),
                request_ids.len(),
                "All requests should have results"
            );

            // Property: Valid IDs succeed, invalid IDs fail
            for (i, id) in request_ids.iter().enumerate() {
                let result = &response.results[i];
                let should_succeed = *id > 0 && *id <= 100;

                if should_succeed {
                    prop_assert!(
                        result.is_success(),
                        "ID {} should succeed but got error",
                        id
                    );
                } else {
                    prop_assert!(
                        result.is_error(),
                        "ID {} should fail but succeeded",
                        id
                    );
                }
            }

            // Property: Success count + error count = total count
            prop_assert_eq!(
                response.success_count() + response.error_count(),
                response.len(),
                "Success + error count should equal total"
            );

            Ok(())
        })?;
    }
}
