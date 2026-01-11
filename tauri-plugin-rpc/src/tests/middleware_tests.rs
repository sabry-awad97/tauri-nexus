//! Middleware tests - Property-based tests for middleware chain execution
//!
//! Tests that middleware chains execute in the correct order (onion model),
//! support early return, and properly propagate errors.

use proptest::prelude::*;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::{
    Context, Router, RpcError, RpcResult,
    middleware::{Next, Request, Response},
};

// =============================================================================
// Test Helpers
// =============================================================================

/// A simple context for testing
#[derive(Clone, Default)]
struct TestContext {
    /// Tracks the order of middleware execution
    execution_log: Arc<Mutex<Vec<String>>>,
}

/// Create a middleware that logs its execution order
fn create_logging_middleware(
    name: String,
) -> impl Fn(
    Context<TestContext>,
    Request,
    Next<TestContext>,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = RpcResult<Response>> + Send>>
+ Send
+ Sync
+ 'static {
    move |ctx: Context<TestContext>, req: Request, next: Next<TestContext>| {
        let name = name.clone();
        Box::pin(async move {
            // Log entry
            {
                let mut log = ctx.inner().execution_log.lock().await;
                log.push(format!("{}_enter", name));
            }

            // Call next
            let result = next(ctx.clone(), req).await;

            // Log exit
            {
                let mut log = ctx.inner().execution_log.lock().await;
                log.push(format!("{}_exit", name));
            }

            result
        })
    }
}

/// Create a middleware that returns early without calling next
fn create_early_return_middleware(
    name: String,
    return_value: serde_json::Value,
) -> impl Fn(
    Context<TestContext>,
    Request,
    Next<TestContext>,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = RpcResult<Response>> + Send>>
+ Send
+ Sync
+ 'static {
    move |ctx: Context<TestContext>, _req: Request, _next: Next<TestContext>| {
        let name = name.clone();
        let return_value = return_value.clone();
        Box::pin(async move {
            // Log entry
            {
                let mut log = ctx.inner().execution_log.lock().await;
                log.push(format!("{}_early_return", name));
            }

            // Return early without calling next
            Ok(return_value)
        })
    }
}

/// Create a middleware that returns an error
fn create_error_middleware(
    name: String,
    error_message: String,
) -> impl Fn(
    Context<TestContext>,
    Request,
    Next<TestContext>,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = RpcResult<Response>> + Send>>
+ Send
+ Sync
+ 'static {
    move |ctx: Context<TestContext>, _req: Request, _next: Next<TestContext>| {
        let name = name.clone();
        let error_message = error_message.clone();
        Box::pin(async move {
            // Log entry
            {
                let mut log = ctx.inner().execution_log.lock().await;
                log.push(format!("{}_error", name));
            }

            // Return error without calling next
            Err(RpcError::middleware(error_message))
        })
    }
}

/// Simple handler that logs and returns success
async fn test_handler(ctx: Context<TestContext>, _input: ()) -> RpcResult<String> {
    let mut log = ctx.inner().execution_log.lock().await;
    log.push("handler".to_string());
    Ok("success".to_string())
}

// =============================================================================
// Property-Based Tests
// =============================================================================

proptest! {
    /// **Property 10: Middleware Execution Order**
    /// *For any* router with middleware added in order [M1, M2, M3], when a request
    /// is processed, the middleware SHALL execute in the order M1 → M2 → M3 → Handler → M3 → M2 → M1 (onion model).
    /// **Feature: tauri-rpc-plugin-optimization, Property 10: Middleware Execution Order**
    #[test]
    fn prop_middleware_execution_order(middleware_count in 1usize..5) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let test_ctx = TestContext::default();

            // Build router with N middleware
            let mut router = Router::new().context(test_ctx.clone());

            for i in 0..middleware_count {
                let name = format!("M{}", i + 1);
                router = router.middleware(create_logging_middleware(name));
            }

            let router = router.query("test", test_handler);

            // Call the procedure
            let result = router.call("test", serde_json::json!(null)).await;
            prop_assert!(result.is_ok(), "Call should succeed");

            // Verify execution order
            let log = test_ctx.execution_log.lock().await;

            // Expected order: M1_enter, M2_enter, ..., MN_enter, handler, MN_exit, ..., M2_exit, M1_exit
            let mut expected = Vec::new();
            for i in 0..middleware_count {
                expected.push(format!("M{}_enter", i + 1));
            }
            expected.push("handler".to_string());
            for i in (0..middleware_count).rev() {
                expected.push(format!("M{}_exit", i + 1));
            }

            prop_assert_eq!(
                log.as_slice(),
                expected.as_slice(),
                "Middleware should execute in onion order"
            );

            Ok(())
        })?;
    }

    /// **Property 11: Middleware Early Return**
    /// *For any* middleware that returns a response without calling `next`, the downstream
    /// middleware and handler SHALL NOT be invoked, and the returned response SHALL be the final response.
    /// **Feature: tauri-rpc-plugin-optimization, Property 11: Middleware Early Return**
    #[test]
    fn prop_middleware_early_return(
        early_return_position in 0usize..3,
        total_middleware in 1usize..5,
    ) {
        // Ensure early_return_position is valid
        let early_return_position = early_return_position % total_middleware.max(1);

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let test_ctx = TestContext::default();
            let expected_return = serde_json::json!({"early": true, "position": early_return_position});

            // Build router with middleware, one of which returns early
            let mut router = Router::new().context(test_ctx.clone());

            for i in 0..total_middleware {
                if i == early_return_position {
                    let name = format!("M{}", i + 1);
                    router = router.middleware(create_early_return_middleware(name, expected_return.clone()));
                } else {
                    let name = format!("M{}", i + 1);
                    router = router.middleware(create_logging_middleware(name));
                }
            }

            let router = router.query("test", test_handler);

            // Call the procedure
            let result = router.call("test", serde_json::json!(null)).await;
            prop_assert!(result.is_ok(), "Call should succeed with early return");
            prop_assert_eq!(result.unwrap(), expected_return, "Should return early return value");

            // Verify that downstream middleware and handler were NOT called
            let log = test_ctx.execution_log.lock().await;

            // Should only have entries for middleware before the early return
            // Plus the early return entry itself
            let mut expected_entries = Vec::new();
            for i in 0..early_return_position {
                expected_entries.push(format!("M{}_enter", i + 1));
            }
            expected_entries.push(format!("M{}_early_return", early_return_position + 1));
            // Exit entries for middleware that entered before early return
            for i in (0..early_return_position).rev() {
                expected_entries.push(format!("M{}_exit", i + 1));
            }

            prop_assert_eq!(
                log.as_slice(),
                expected_entries.as_slice(),
                "Only middleware before early return should execute"
            );

            // Verify handler was NOT called
            prop_assert!(
                !log.contains(&"handler".to_string()),
                "Handler should not be called when middleware returns early"
            );

            Ok(())
        })?;
    }

    /// **Property 4: Middleware Error Propagation**
    /// *For any* middleware that returns an error, the caller SHALL receive that exact error
    /// (code and message preserved) without the downstream handler being invoked.
    /// **Feature: tauri-rpc-plugin-optimization, Property 4: Middleware Error Propagation**
    #[test]
    fn prop_middleware_error_propagation(
        error_position in 0usize..3,
        total_middleware in 1usize..5,
        error_message in "[a-zA-Z0-9 ]{1,50}",
    ) {
        // Ensure error_position is valid
        let error_position = error_position % total_middleware.max(1);

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let test_ctx = TestContext::default();

            // Build router with middleware, one of which returns an error
            let mut router = Router::new().context(test_ctx.clone());

            for i in 0..total_middleware {
                if i == error_position {
                    let name = format!("M{}", i + 1);
                    router = router.middleware(create_error_middleware(name, error_message.clone()));
                } else {
                    let name = format!("M{}", i + 1);
                    router = router.middleware(create_logging_middleware(name));
                }
            }

            let router = router.query("test", test_handler);

            // Call the procedure
            let result = router.call("test", serde_json::json!(null)).await;

            // Should return an error
            prop_assert!(result.is_err(), "Call should fail with middleware error");

            let err = result.unwrap_err();
            prop_assert_eq!(
                err.message,
                error_message,
                "Error message should be preserved"
            );

            // Verify that downstream middleware and handler were NOT called
            let log = test_ctx.execution_log.lock().await;

            // Handler should NOT be called
            prop_assert!(
                !log.contains(&"handler".to_string()),
                "Handler should not be called when middleware returns error"
            );

            // Middleware after the error should NOT have entered
            for i in (error_position + 1)..total_middleware {
                prop_assert!(
                    !log.contains(&format!("M{}_enter", i + 1)),
                    "Middleware after error should not execute"
                );
            }

            Ok(())
        })?;
    }
}

// =============================================================================
// Unit Tests for Compiled Router
// =============================================================================

#[tokio::test]
async fn test_compiled_router_middleware_order() {
    let test_ctx = TestContext::default();

    let router = Router::new()
        .context(test_ctx.clone())
        .middleware(create_logging_middleware("M1".to_string()))
        .middleware(create_logging_middleware("M2".to_string()))
        .middleware(create_logging_middleware("M3".to_string()))
        .query("test", test_handler)
        .compile();

    let result = router.call("test", serde_json::json!(null)).await;
    assert!(result.is_ok());

    let log = test_ctx.execution_log.lock().await;
    let expected = vec![
        "M1_enter", "M2_enter", "M3_enter", "handler", "M3_exit", "M2_exit", "M1_exit",
    ];
    assert_eq!(log.as_slice(), expected.as_slice());
}

#[tokio::test]
async fn test_compiled_router_early_return() {
    let test_ctx = TestContext::default();
    let early_value = serde_json::json!({"early": true});

    let router = Router::new()
        .context(test_ctx.clone())
        .middleware(create_logging_middleware("M1".to_string()))
        .middleware(create_early_return_middleware(
            "M2".to_string(),
            early_value.clone(),
        ))
        .middleware(create_logging_middleware("M3".to_string()))
        .query("test", test_handler)
        .compile();

    let result = router.call("test", serde_json::json!(null)).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), early_value);

    let log = test_ctx.execution_log.lock().await;
    // M3 and handler should NOT be called
    assert!(!log.contains(&"M3_enter".to_string()));
    assert!(!log.contains(&"handler".to_string()));
}

#[tokio::test]
async fn test_compiled_router_error_propagation() {
    let test_ctx = TestContext::default();

    let router = Router::new()
        .context(test_ctx.clone())
        .middleware(create_logging_middleware("M1".to_string()))
        .middleware(create_error_middleware(
            "M2".to_string(),
            "test error".to_string(),
        ))
        .middleware(create_logging_middleware("M3".to_string()))
        .query("test", test_handler)
        .compile();

    let result = router.call("test", serde_json::json!(null)).await;
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert_eq!(err.message, "test error");

    let log = test_ctx.execution_log.lock().await;
    // M3 and handler should NOT be called
    assert!(!log.contains(&"M3_enter".to_string()));
    assert!(!log.contains(&"handler".to_string()));
}

#[tokio::test]
async fn test_no_middleware_direct_handler_call() {
    let test_ctx = TestContext::default();

    let router = Router::new()
        .context(test_ctx.clone())
        .query("test", test_handler)
        .compile();

    let result = router.call("test", serde_json::json!(null)).await;
    assert!(result.is_ok());

    let log = test_ctx.execution_log.lock().await;
    // Only handler should be called
    assert_eq!(log.as_slice(), &["handler"]);
}
