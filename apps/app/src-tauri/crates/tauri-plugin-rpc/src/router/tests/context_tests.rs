//! Tests for context transformation chains

use crate::{
    Context, RpcError, RpcResult, router::Router, validation::{FieldError, Validate, ValidationResult}
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Default)]
struct TestContext {
    value: i32,
}

#[derive(Clone)]
struct AuthContext {
    user_id: String,
    original_value: i32,
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

async fn auth_handler(ctx: Context<AuthContext>, input: TestInput) -> RpcResult<TestOutput> {
    Ok(TestOutput {
        message: format!(
            "Hello, {}! User: {}, Value: {}",
            input.name,
            ctx.inner().user_id,
            ctx.inner().original_value
        ),
    })
}

#[tokio::test]
async fn test_context_transformation_basic() {
    let router = Router::new()
        .context(TestContext { value: 42 })
        .procedure("users.profile")
        .context(|ctx: Context<TestContext>| async move {
            Ok(AuthContext {
                user_id: "user123".to_string(),
                original_value: ctx.inner().value,
            })
        })
        .input::<TestInput>()
        .query(auth_handler);

    let result = router
        .call("users.profile", serde_json::json!({"name": "World"}))
        .await;
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap()["message"],
        "Hello, World! User: user123, Value: 42"
    );
}

#[tokio::test]
async fn test_context_transformation_with_error() {
    let router = Router::new()
        .context(TestContext { value: 42 })
        .procedure("users.profile")
        .context(|_ctx: Context<TestContext>| async move {
            Err::<AuthContext, _>(RpcError::unauthorized("Not authenticated"))
        })
        .input::<TestInput>()
        .query(auth_handler);

    let result = router
        .call("users.profile", serde_json::json!({"name": "World"}))
        .await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code, crate::RpcErrorCode::Unauthorized);
}

#[tokio::test]
async fn test_context_transformation_with_validation() {
    async fn validated_auth_handler(
        ctx: Context<AuthContext>,
        input: ValidatedInput,
    ) -> RpcResult<TestOutput> {
        Ok(TestOutput {
            message: format!(
                "Hello, {} (age {})! User: {}",
                input.name,
                input.age,
                ctx.inner().user_id
            ),
        })
    }

    let router = Router::new()
        .context(TestContext { value: 42 })
        .procedure("users.profile")
        .context(|ctx: Context<TestContext>| async move {
            Ok(AuthContext {
                user_id: "user456".to_string(),
                original_value: ctx.inner().value,
            })
        })
        .input_validated::<ValidatedInput>()
        .query(validated_auth_handler);

    // Valid input
    let result = router
        .call(
            "users.profile",
            serde_json::json!({"name": "Alice", "age": 30}),
        )
        .await;
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap()["message"],
        "Hello, Alice (age 30)! User: user456"
    );

    // Invalid input - validation should still work
    let result = router
        .call("users.profile", serde_json::json!({"name": "", "age": 200}))
        .await;
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().code,
        crate::RpcErrorCode::ValidationError
    );
}

#[tokio::test]
async fn test_context_transformation_mutation() {
    let router = Router::new()
        .context(TestContext { value: 100 })
        .procedure("users.update")
        .context(|ctx: Context<TestContext>| async move {
            Ok(AuthContext {
                user_id: "mutator".to_string(),
                original_value: ctx.inner().value,
            })
        })
        .input::<TestInput>()
        .mutation(auth_handler);

    let result = router
        .call("users.update", serde_json::json!({"name": "Test"}))
        .await;
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap()["message"],
        "Hello, Test! User: mutator, Value: 100"
    );
}

#[tokio::test]
async fn test_context_transformation_with_output_transformer() {
    let router = Router::new()
        .context(TestContext { value: 42 })
        .procedure("users.profile")
        .context(|ctx: Context<TestContext>| async move {
            Ok(AuthContext {
                user_id: "user789".to_string(),
                original_value: ctx.inner().value,
            })
        })
        .input::<TestInput>()
        .output(|value| {
            serde_json::json!({
                "data": value,
                "transformed": true
            })
        })
        .query(auth_handler);

    let result = router
        .call("users.profile", serde_json::json!({"name": "World"}))
        .await;
    assert!(result.is_ok());

    let output = result.unwrap();
    assert_eq!(output["transformed"], true);
    assert_eq!(
        output["data"]["message"],
        "Hello, World! User: user789, Value: 42"
    );
}

#[tokio::test]
async fn test_context_transformation_with_middleware() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    let middleware_count = Arc::new(AtomicUsize::new(0));
    let count_clone = middleware_count.clone();

    let router = Router::new()
        .context(TestContext { value: 42 })
        .procedure("users.profile")
        .use_middleware(move |ctx, req, next| {
            let count = count_clone.clone();
            async move {
                count.fetch_add(1, Ordering::SeqCst);
                next(ctx, req).await
            }
        })
        .context(|ctx: Context<TestContext>| async move {
            Ok(AuthContext {
                user_id: "user_with_mw".to_string(),
                original_value: ctx.inner().value,
            })
        })
        .input::<TestInput>()
        .query(auth_handler);

    let result = router
        .call("users.profile", serde_json::json!({"name": "World"}))
        .await;
    assert!(result.is_ok());
    assert_eq!(middleware_count.load(Ordering::SeqCst), 1);
    assert_eq!(
        result.unwrap()["message"],
        "Hello, World! User: user_with_mw, Value: 42"
    );
}
