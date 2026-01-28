//! Middleware chain building utilities
//!
//! This module provides the core functionality for composing middleware
//! functions into execution chains.

use crate::middleware::{MiddlewareFn, Next};
use std::sync::Arc;

/// Build a middleware chain from a list of middleware functions and a final handler.
///
/// Middleware is applied in reverse order (last added = innermost), meaning
/// the first middleware in the list wraps all subsequent middleware.
///
/// # Arguments
/// * `middleware` - List of middleware functions in registration order
/// * `final_handler` - The innermost handler (Next function)
///
/// # Returns
/// A composed Next function with all middleware applied
///
/// # Example
/// ```rust,ignore
/// // Given middleware [M1, M2, M3] and handler H:
/// // Execution order: M1 → M2 → M3 → H → M3 → M2 → M1
/// let chain = build_middleware_chain(vec![m1, m2, m3], handler);
/// ```
pub fn build_middleware_chain<Ctx: Clone + Send + Sync + 'static>(
    middleware: Vec<MiddlewareFn<Ctx>>,
    final_handler: Next<Ctx>,
) -> Next<Ctx> {
    middleware
        .into_iter()
        .rev()
        .fold(final_handler, |next, mw| {
            Arc::new(move |ctx, req| {
                let mw = mw.clone();
                let next = next.clone();
                Box::pin(async move { (mw)(ctx, req, next).await })
            })
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Context,
        middleware::{ProcedureType, Request},
    };

    #[tokio::test]
    async fn test_middleware_chain_execution_order() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};

        #[derive(Clone)]
        struct TestCtx;

        let execution_order = Arc::new(AtomicUsize::new(0));

        // Create middleware that records execution order
        let order1 = execution_order.clone();
        let mw1: MiddlewareFn<TestCtx> = Arc::new(move |ctx, req, next| {
            let order = order1.clone();
            Box::pin(async move {
                assert_eq!(order.fetch_add(1, Ordering::SeqCst), 0);
                let result = next(ctx, req).await;
                assert_eq!(order.fetch_add(1, Ordering::SeqCst), 5);
                result
            })
        });

        let order2 = execution_order.clone();
        let mw2: MiddlewareFn<TestCtx> = Arc::new(move |ctx, req, next| {
            let order = order2.clone();
            Box::pin(async move {
                assert_eq!(order.fetch_add(1, Ordering::SeqCst), 1);
                let result = next(ctx, req).await;
                assert_eq!(order.fetch_add(1, Ordering::SeqCst), 4);
                result
            })
        });

        let order3 = execution_order.clone();
        let mw3: MiddlewareFn<TestCtx> = Arc::new(move |ctx, req, next| {
            let order = order3.clone();
            Box::pin(async move {
                assert_eq!(order.fetch_add(1, Ordering::SeqCst), 2);
                let result = next(ctx, req).await;
                assert_eq!(order.fetch_add(1, Ordering::SeqCst), 3);
                result
            })
        });

        // Final handler
        let final_handler: Next<TestCtx> = Arc::new(move |_ctx, _req| {
            Box::pin(async move { Ok(serde_json::json!({"result": "ok"})) })
        });

        // Build chain
        let chain = build_middleware_chain(vec![mw1, mw2, mw3], final_handler);

        // Execute
        let ctx = Context::new(TestCtx);
        let req = Request {
            path: "test".to_string(),
            procedure_type: ProcedureType::Query,
            input: serde_json::json!(null),
        };

        let result = chain(ctx, req).await;
        assert!(result.is_ok());
        assert_eq!(execution_order.load(Ordering::SeqCst), 6);
    }
}
