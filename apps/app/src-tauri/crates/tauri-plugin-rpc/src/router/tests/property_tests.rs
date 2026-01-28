//! Property-based tests for router

use crate::{Context, RpcResult, router::Router};
use proptest::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Default)]
struct TestContext;

#[derive(Debug, Deserialize, Serialize)]
struct EchoInput {
    value: String,
}

async fn echo_handler(_ctx: Context<TestContext>, input: EchoInput) -> RpcResult<EchoInput> {
    Ok(input)
}

/// Property 10: Procedure Builder Backward Compatibility
/// The new fluent API should produce identical results to the old API
/// for equivalent procedure definitions.
#[test]
fn prop_backward_compatibility_query_results() {
    proptest!(|(value in "[a-zA-Z0-9]{1,20}")| {
        let rt = tokio::runtime::Runtime::new().unwrap();

        // Old API
        let old_router = Router::new()
            .context(TestContext)
            .query("test", echo_handler);

        // New API
        let new_router = Router::new()
            .context(TestContext)
            .procedure("test")
            .input::<EchoInput>()
            .query(echo_handler);

        let input = serde_json::json!({"value": value});

        let old_result = rt.block_on(old_router.call("test", input.clone()));
        let new_result = rt.block_on(new_router.call("test", input));

        prop_assert!(old_result.is_ok());
        prop_assert!(new_result.is_ok());
        prop_assert_eq!(old_result.unwrap(), new_result.unwrap());
    });
}

/// Property: Multiple procedures can be chained
#[test]
fn prop_multiple_procedures_registered() {
    proptest!(|(count in 1usize..5)| {
        let mut router = Router::new().context(TestContext);

        for i in 0..count {
            let path = format!("proc{}", i);
            router = router
                .procedure(&path)
                .input::<EchoInput>()
                .query(echo_handler);
        }

        let procedures = router.procedures();
        prop_assert_eq!(procedures.len(), count);

        for i in 0..count {
            let path = format!("proc{}", i);
            prop_assert!(procedures.contains(&path));
        }
    });
}

/// Property: Procedure chain preserves procedure type
#[test]
fn prop_procedure_type_preserved() {
    proptest!(|(is_query in proptest::bool::ANY)| {
        let router = if is_query {
            Router::new()
                .context(TestContext)
                .procedure("test")
                .input::<EchoInput>()
                .query(echo_handler)
        } else {
            Router::new()
                .context(TestContext)
                .procedure("test")
                .input::<EchoInput>()
                .mutation(echo_handler)
        };

        // Both should be callable (not subscriptions)
        prop_assert!(!router.is_subscription("test"));
    });
}
