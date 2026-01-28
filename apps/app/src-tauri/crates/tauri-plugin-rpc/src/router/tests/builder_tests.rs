//! Tests for procedure builder chains

use crate::{
    Context, RpcResult,
    router::Router,
    validation::{FieldError, Validate, ValidationResult},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Default)]
struct TestContext {
    #[allow(dead_code)]
    value: i32,
}

#[derive(Debug, Deserialize)]
struct TestInput {
    name: String,
}

#[derive(Debug, Serialize)]
struct TestOutput {
    message: String,
}

#[derive(Debug, Deserialize)]
struct ValidatedInput {
    name: String,
    age: i32,
}

impl Validate for ValidatedInput {
    fn validate(&self) -> ValidationResult {
        let mut errors = Vec::new();
        if self.name.is_empty() {
            errors.push(FieldError::required("name"));
        }
        if self.age < 0 || self.age > 150 {
            errors.push(FieldError::range("age", 0, 150));
        }
        ValidationResult::from_errors(errors)
    }
}

async fn test_handler(_ctx: Context<TestContext>, input: TestInput) -> RpcResult<TestOutput> {
    Ok(TestOutput {
        message: format!("Hello, {}!", input.name),
    })
}

async fn validated_handler(
    _ctx: Context<TestContext>,
    input: ValidatedInput,
) -> RpcResult<TestOutput> {
    Ok(TestOutput {
        message: format!("Hello, {} (age {})!", input.name, input.age),
    })
}

#[tokio::test]
async fn test_procedure_chain_query() {
    let router = Router::new()
        .context(TestContext::default())
        .procedure("users.get")
        .input::<TestInput>()
        .query(test_handler);

    let result = router
        .call("users.get", serde_json::json!({"name": "World"}))
        .await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap()["message"], "Hello, World!");
}

#[tokio::test]
async fn test_procedure_chain_mutation() {
    let router = Router::new()
        .context(TestContext::default())
        .procedure("users.create")
        .input::<TestInput>()
        .mutation(test_handler);

    let result = router
        .call("users.create", serde_json::json!({"name": "Alice"}))
        .await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap()["message"], "Hello, Alice!");
}

#[tokio::test]
async fn test_procedure_chain_with_validation() {
    let router = Router::new()
        .context(TestContext::default())
        .procedure("users.create")
        .input_validated::<ValidatedInput>()
        .mutation(validated_handler);

    // Valid input
    let result = router
        .call(
            "users.create",
            serde_json::json!({"name": "Bob", "age": 30}),
        )
        .await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap()["message"], "Hello, Bob (age 30)!");

    // Invalid input
    let result = router
        .call("users.create", serde_json::json!({"name": "", "age": 200}))
        .await;
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().code,
        crate::RpcErrorCode::ValidationError
    );
}

#[tokio::test]
async fn test_procedure_chain_with_output_transformer() {
    let router = Router::new()
        .context(TestContext::default())
        .procedure("users.get")
        .input::<TestInput>()
        .output(|value| {
            serde_json::json!({
                "data": value,
                "wrapped": true
            })
        })
        .query(test_handler);

    let result = router
        .call("users.get", serde_json::json!({"name": "World"}))
        .await;
    assert!(result.is_ok());
    let output = result.unwrap();
    assert_eq!(output["wrapped"], true);
    assert_eq!(output["data"]["message"], "Hello, World!");
}

#[tokio::test]
async fn test_procedure_chain_with_middleware() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    let call_count = Arc::new(AtomicUsize::new(0));
    let count_clone = call_count.clone();

    let router = Router::new()
        .context(TestContext::default())
        .procedure("users.get")
        .use_middleware(move |ctx, req, next| {
            let count = count_clone.clone();
            async move {
                count.fetch_add(1, Ordering::SeqCst);
                next(ctx, req).await
            }
        })
        .input::<TestInput>()
        .query(test_handler);

    let result = router
        .call("users.get", serde_json::json!({"name": "World"}))
        .await;
    assert!(result.is_ok());
    assert_eq!(call_count.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn test_procedure_chain_multiple_procedures() {
    let router = Router::new()
        .context(TestContext::default())
        .procedure("users.get")
        .input::<TestInput>()
        .query(test_handler)
        .procedure("users.create")
        .input::<TestInput>()
        .mutation(test_handler);

    // Both procedures should be registered
    let procedures = router.procedures();
    assert!(procedures.contains(&"users.get".to_string()));
    assert!(procedures.contains(&"users.create".to_string()));

    // Both should work
    let result1 = router
        .call("users.get", serde_json::json!({"name": "Get"}))
        .await;
    assert!(result1.is_ok());

    let result2 = router
        .call("users.create", serde_json::json!({"name": "Create"}))
        .await;
    assert!(result2.is_ok());
}

#[tokio::test]
async fn test_backward_compatibility_query() {
    // Old API
    let old_router = Router::new()
        .context(TestContext::default())
        .query("users.get", test_handler);

    // New API
    let new_router = Router::new()
        .context(TestContext::default())
        .procedure("users.get")
        .input::<TestInput>()
        .query(test_handler);

    let input = serde_json::json!({"name": "Test"});

    let old_result = old_router.call("users.get", input.clone()).await;
    let new_result = new_router.call("users.get", input).await;

    assert!(old_result.is_ok());
    assert!(new_result.is_ok());
    assert_eq!(old_result.unwrap(), new_result.unwrap());
}

#[tokio::test]
async fn test_backward_compatibility_mutation() {
    // Old API
    let old_router = Router::new()
        .context(TestContext::default())
        .mutation("users.create", test_handler);

    // New API
    let new_router = Router::new()
        .context(TestContext::default())
        .procedure("users.create")
        .input::<TestInput>()
        .mutation(test_handler);

    let input = serde_json::json!({"name": "Test"});

    let old_result = old_router.call("users.create", input.clone()).await;
    let new_result = new_router.call("users.create", input).await;

    assert!(old_result.is_ok());
    assert!(new_result.is_ok());
    assert_eq!(old_result.unwrap(), new_result.unwrap());
}

#[tokio::test]
async fn test_backward_compatibility_validated() {
    // Old API
    let old_router = Router::new()
        .context(TestContext::default())
        .mutation_validated("users.create", validated_handler);

    // New API
    let new_router = Router::new()
        .context(TestContext::default())
        .procedure("users.create")
        .input_validated::<ValidatedInput>()
        .mutation(validated_handler);

    // Valid input
    let valid_input = serde_json::json!({"name": "Test", "age": 25});
    let old_result = old_router.call("users.create", valid_input.clone()).await;
    let new_result = new_router.call("users.create", valid_input).await;

    assert!(old_result.is_ok());
    assert!(new_result.is_ok());
    assert_eq!(old_result.unwrap(), new_result.unwrap());

    // Invalid input - both should fail with validation error
    let invalid_input = serde_json::json!({"name": "", "age": 200});
    let old_result = old_router.call("users.create", invalid_input.clone()).await;
    let new_result = new_router.call("users.create", invalid_input).await;

    assert!(old_result.is_err());
    assert!(new_result.is_err());
    assert_eq!(old_result.unwrap_err().code, new_result.unwrap_err().code);
}
