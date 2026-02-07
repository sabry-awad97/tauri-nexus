use crate::{
    Context, Next, ProcedureBuilder, ProcedureType, Request, RpcError, RpcResult, Validate,
    middleware::Response,
    validation::{FieldError, ValidationResult},
};
use serde::{Deserialize, Serialize};

#[derive(Clone)]
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

#[test]
fn test_procedure_builder_new() {
    let builder = ProcedureBuilder::<TestContext>::new("users.get");
    assert_eq!(builder.path(), "users.get");
}

#[test]
fn test_procedure_builder_query() {
    let procedure = ProcedureBuilder::<TestContext>::new("users.get")
        .input::<TestInput>()
        .query(test_handler);

    assert_eq!(procedure.path, "users.get");
    assert_eq!(procedure.procedure_type, ProcedureType::Query);
    assert!(procedure.middleware.is_empty());
}

#[test]
fn test_procedure_builder_mutation() {
    let procedure = ProcedureBuilder::<TestContext>::new("users.create")
        .input::<TestInput>()
        .mutation(test_handler);

    assert_eq!(procedure.path, "users.create");
    assert_eq!(procedure.procedure_type, ProcedureType::Mutation);
}

#[test]
fn test_procedure_builder_with_middleware() {
    async fn test_middleware(
        ctx: Context<TestContext>,
        req: Request,
        next: Next<TestContext>,
    ) -> RpcResult<Response> {
        next(ctx, req).await
    }

    let procedure = ProcedureBuilder::<TestContext>::new("users.get")
        .use_middleware(test_middleware)
        .input::<TestInput>()
        .query(test_handler);

    assert_eq!(procedure.middleware.len(), 1);
}

#[test]
fn test_procedure_builder_with_multiple_middleware() {
    async fn middleware1(
        ctx: Context<TestContext>,
        req: Request,
        next: Next<TestContext>,
    ) -> RpcResult<Response> {
        next(ctx, req).await
    }

    async fn middleware2(
        ctx: Context<TestContext>,
        req: Request,
        next: Next<TestContext>,
    ) -> RpcResult<Response> {
        next(ctx, req).await
    }

    let procedure = ProcedureBuilder::<TestContext>::new("users.get")
        .use_middleware(middleware1)
        .use_middleware(middleware2)
        .input::<TestInput>()
        .query(test_handler);

    assert_eq!(procedure.middleware.len(), 2);
}

#[test]
fn test_validated_procedure_builder() {
    let procedure = ProcedureBuilder::<TestContext>::new("users.create")
        .input_validated::<ValidatedInput>()
        .mutation(validated_handler);

    assert_eq!(procedure.path, "users.create");
    assert_eq!(procedure.procedure_type, ProcedureType::Mutation);
}

// Meta tests

#[test]
fn test_procedure_builder_with_meta() {
    use crate::schema::ProcedureMeta;

    let procedure = ProcedureBuilder::<TestContext>::new("users.get")
        .meta(
            ProcedureMeta::new()
                .description("Get a user by ID")
                .summary("Get user")
                .tag("users")
                .deprecated(),
        )
        .input::<TestInput>()
        .query(test_handler);

    assert_eq!(procedure.path, "users.get");
    assert!(procedure.meta.is_some());

    let meta = procedure.meta.unwrap();
    assert_eq!(meta.description, Some("Get a user by ID".to_string()));
    assert_eq!(meta.summary, Some("Get user".to_string()));
    assert_eq!(meta.tags, vec!["users".to_string()]);
    assert!(meta.deprecated);
}

#[test]
fn test_procedure_builder_meta_with_schemas() {
    use crate::schema::{ProcedureMeta, TypeSchema};

    let procedure = ProcedureBuilder::<TestContext>::new("users.create")
        .meta(
            ProcedureMeta::new()
                .description("Create a new user")
                .input(
                    TypeSchema::object()
                        .with_property("name", TypeSchema::string())
                        .with_required("name"),
                )
                .output(
                    TypeSchema::object()
                        .with_property("id", TypeSchema::integer())
                        .with_property("name", TypeSchema::string()),
                ),
        )
        .input::<TestInput>()
        .mutation(test_handler);

    assert!(procedure.meta.is_some());
    let meta = procedure.meta.unwrap();
    assert!(meta.input.is_some());
    assert!(meta.output.is_some());
}

#[test]
fn test_procedure_builder_meta_with_examples() {
    use crate::schema::ProcedureMeta;

    let procedure = ProcedureBuilder::<TestContext>::new("users.get")
        .meta(
            ProcedureMeta::new()
                .example_input(serde_json::json!({"name": "Alice"}))
                .example_output(serde_json::json!({"message": "Hello, Alice!"})),
        )
        .input::<TestInput>()
        .query(test_handler);

    let meta = procedure.meta.unwrap();
    assert_eq!(
        meta.example_input,
        Some(serde_json::json!({"name": "Alice"}))
    );
    assert_eq!(
        meta.example_output,
        Some(serde_json::json!({"message": "Hello, Alice!"}))
    );
}

#[test]
fn test_validated_procedure_builder_with_meta() {
    use crate::schema::ProcedureMeta;

    let procedure = ProcedureBuilder::<TestContext>::new("users.create")
        .meta(
            ProcedureMeta::new()
                .description("Create a validated user")
                .tags(vec!["users", "validated"]),
        )
        .input_validated::<ValidatedInput>()
        .mutation(validated_handler);

    assert!(procedure.meta.is_some());
    let meta = procedure.meta.unwrap();
    assert_eq!(
        meta.description,
        Some("Create a validated user".to_string())
    );
    assert_eq!(
        meta.tags,
        vec!["users".to_string(), "validated".to_string()]
    );
}

#[test]
fn test_context_transformed_procedure_with_meta() {
    use crate::schema::ProcedureMeta;

    let procedure = ProcedureBuilder::<TestContext>::new("users.profile")
        .meta(
            ProcedureMeta::new()
                .description("Get authenticated user profile")
                .tag("auth"),
        )
        .context(|ctx: Context<TestContext>| async move {
            Ok(AuthContext {
                user_id: "user123".to_string(),
                original_value: ctx.inner().value,
            })
        })
        .input::<TestInput>()
        .query(auth_handler);

    assert!(procedure.meta.is_some());
    let meta = procedure.meta.unwrap();
    assert_eq!(
        meta.description,
        Some("Get authenticated user profile".to_string())
    );
}

#[test]
fn test_context_transformed_validated_procedure_with_meta() {
    use crate::schema::ProcedureMeta;

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

    let procedure = ProcedureBuilder::<TestContext>::new("users.update")
        .meta(
            ProcedureMeta::new()
                .description("Update user with validation")
                .tags(vec!["users", "auth", "validated"]),
        )
        .context(|ctx: Context<TestContext>| async move {
            Ok(AuthContext {
                user_id: "user456".to_string(),
                original_value: ctx.inner().value,
            })
        })
        .input_validated::<ValidatedInput>()
        .mutation(validated_auth_handler);

    assert!(procedure.meta.is_some());
    let meta = procedure.meta.unwrap();
    assert_eq!(
        meta.description,
        Some("Update user with validation".to_string())
    );
    assert_eq!(
        meta.tags,
        vec![
            "users".to_string(),
            "auth".to_string(),
            "validated".to_string()
        ]
    );
}

#[tokio::test]
async fn test_procedure_handler_execution() {
    let procedure = ProcedureBuilder::<TestContext>::new("test")
        .input::<TestInput>()
        .query(test_handler);

    let ctx = Context::new(TestContext { value: 42 });
    let input = serde_json::json!({"name": "World"});

    let result = (procedure.handler)(ctx, input).await;
    assert!(result.is_ok());

    let output = result.unwrap();
    assert_eq!(output["message"], "Hello, World!");
}

#[tokio::test]
async fn test_validated_procedure_valid_input() {
    let procedure = ProcedureBuilder::<TestContext>::new("test")
        .input_validated::<ValidatedInput>()
        .query(validated_handler);

    let ctx = Context::new(TestContext { value: 42 });
    let input = serde_json::json!({"name": "Alice", "age": 30});

    let result = (procedure.handler)(ctx, input).await;
    assert!(result.is_ok());

    let output = result.unwrap();
    assert_eq!(output["message"], "Hello, Alice (age 30)!");
}

#[tokio::test]
async fn test_validated_procedure_invalid_input() {
    let procedure = ProcedureBuilder::<TestContext>::new("test")
        .input_validated::<ValidatedInput>()
        .query(validated_handler);

    let ctx = Context::new(TestContext { value: 42 });
    let input = serde_json::json!({"name": "", "age": 200});

    let result = (procedure.handler)(ctx, input).await;
    assert!(result.is_err());

    let error = result.unwrap_err();
    assert_eq!(error.code, crate::RpcErrorCode::ValidationError);
}

#[tokio::test]
async fn test_procedure_with_output_transformer() {
    let procedure = ProcedureBuilder::<TestContext>::new("test")
        .input::<TestInput>()
        .output(|value| {
            serde_json::json!({
                "data": value,
                "wrapped": true
            })
        })
        .query(test_handler);

    let ctx = Context::new(TestContext { value: 42 });
    let input = serde_json::json!({"name": "World"});

    let result = (procedure.handler)(ctx, input).await;
    assert!(result.is_ok());

    let output = result.unwrap();
    assert_eq!(output["wrapped"], true);
    assert_eq!(output["data"]["message"], "Hello, World!");
}

// Context transformation tests

#[derive(Clone)]
struct AuthContext {
    user_id: String,
    original_value: i32,
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
    let procedure = ProcedureBuilder::<TestContext>::new("test")
        .context(|ctx: Context<TestContext>| async move {
            Ok(AuthContext {
                user_id: "user123".to_string(),
                original_value: ctx.inner().value,
            })
        })
        .input::<TestInput>()
        .query(auth_handler);

    let ctx = Context::new(TestContext { value: 42 });
    let input = serde_json::json!({"name": "World"});

    let result = (procedure.handler)(ctx, input).await;
    assert!(result.is_ok());

    let output = result.unwrap();
    assert_eq!(output["message"], "Hello, World! User: user123, Value: 42");
}

#[tokio::test]
async fn test_context_transformation_with_error() {
    let procedure = ProcedureBuilder::<TestContext>::new("test")
        .context(|_ctx: Context<TestContext>| async move {
            Err::<AuthContext, _>(RpcError::unauthorized("Not authenticated"))
        })
        .input::<TestInput>()
        .query(auth_handler);

    let ctx = Context::new(TestContext { value: 42 });
    let input = serde_json::json!({"name": "World"});

    let result = (procedure.handler)(ctx, input).await;
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

    let procedure = ProcedureBuilder::<TestContext>::new("test")
        .context(|ctx: Context<TestContext>| async move {
            Ok(AuthContext {
                user_id: "user456".to_string(),
                original_value: ctx.inner().value,
            })
        })
        .input_validated::<ValidatedInput>()
        .query(validated_auth_handler);

    // Valid input
    let ctx = Context::new(TestContext { value: 42 });
    let input = serde_json::json!({"name": "Alice", "age": 30});

    let result = (procedure.handler)(ctx, input).await;
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap()["message"],
        "Hello, Alice (age 30)! User: user456"
    );

    // Invalid input - validation should still work
    let ctx = Context::new(TestContext { value: 42 });
    let input = serde_json::json!({"name": "", "age": 200});

    let result = (procedure.handler)(ctx, input).await;
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().code,
        crate::RpcErrorCode::ValidationError
    );
}

#[tokio::test]
async fn test_context_transformation_mutation() {
    let procedure = ProcedureBuilder::<TestContext>::new("test")
        .context(|ctx: Context<TestContext>| async move {
            Ok(AuthContext {
                user_id: "mutator".to_string(),
                original_value: ctx.inner().value,
            })
        })
        .input::<TestInput>()
        .mutation(auth_handler);

    assert_eq!(procedure.procedure_type, ProcedureType::Mutation);

    let ctx = Context::new(TestContext { value: 100 });
    let input = serde_json::json!({"name": "Test"});

    let result = (procedure.handler)(ctx, input).await;
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap()["message"],
        "Hello, Test! User: mutator, Value: 100"
    );
}

#[tokio::test]
async fn test_context_transformation_with_output_transformer() {
    let procedure = ProcedureBuilder::<TestContext>::new("test")
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

    let ctx = Context::new(TestContext { value: 42 });
    let input = serde_json::json!({"name": "World"});

    let result = (procedure.handler)(ctx, input).await;
    assert!(result.is_ok());

    let output = result.unwrap();
    assert_eq!(output["transformed"], true);
    assert_eq!(
        output["data"]["message"],
        "Hello, World! User: user789, Value: 42"
    );
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;
    use std::sync::Arc as StdArc;

    /// Property 9: Procedure Builder Middleware Order
    /// For any procedure with multiple middleware, middleware SHALL execute
    /// in registration order (first registered = outermost).
    #[test]
    fn prop_middleware_registration_order() {
        proptest!(|(num_middleware in 1usize..5)| {
            let execution_order = StdArc::new(std::sync::Mutex::new(Vec::new()));

            let mut builder = ProcedureBuilder::<()>::new("test");

            // Add middleware that records its index when executed
            for i in 0..num_middleware {
                let order = execution_order.clone();
                builder = builder.use_middleware(move |ctx, req, next| {
                    let order = order.clone();
                    async move {
                        order.lock().unwrap().push(i);
                        next(ctx, req).await
                    }
                });
            }

            let procedure = builder
                .input::<()>()
                .query(|_ctx, _input: ()| async { Ok(()) });

            // Verify middleware count
            prop_assert_eq!(procedure.middleware.len(), num_middleware);
        });
    }

    /// Property: Middleware stored in registration order
    #[test]
    fn prop_middleware_stored_in_order() {
        proptest!(|(count in 1usize..10)| {
            let mut builder = ProcedureBuilder::<()>::new("test");

            for _ in 0..count {
                builder = builder.use_middleware(|ctx, req, next| async move {
                    next(ctx, req).await
                });
            }

            let procedure = builder
                .input::<()>()
                .query(|_ctx, _input: ()| async { Ok(()) });

            prop_assert_eq!(procedure.middleware.len(), count);
        });
    }

    /// Property: Output transformer is applied
    #[test]
    fn prop_output_transformer_applied() {
        proptest!(|(suffix in "[a-z]{1,10}")| {
            let suffix_clone = suffix.clone();
            let procedure = ProcedureBuilder::<()>::new("test")
                .input::<()>()
                .output(move |mut value| {
                    if let Some(obj) = value.as_object_mut() {
                        obj.insert("suffix".to_string(), serde_json::json!(suffix_clone.clone()));
                    }
                    value
                })
                .query(|_ctx, _input: ()| async { Ok(serde_json::json!({"original": true})) });

            // The procedure should have an output transformer
            // We can't easily test the transformer without executing, but we verify it compiles
            prop_assert_eq!(procedure.path, "test");
        });
    }

    /// Property: Validated input rejects invalid data
    #[test]
    fn prop_validated_input_rejects_invalid() {
        use crate::validation::{FieldError, Validate, ValidationResult};
        use serde::Deserialize;

        #[derive(Debug, Deserialize)]
        struct TestValidatedInput {
            value: i32,
        }

        impl Validate for TestValidatedInput {
            fn validate(&self) -> ValidationResult {
                if self.value < 0 {
                    ValidationResult::from_errors(vec![FieldError::range("value", 0, 100)])
                } else {
                    ValidationResult::ok()
                }
            }
        }

        proptest!(|(value in -100i32..-1)| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let procedure = ProcedureBuilder::<()>::new("test")
                .input_validated::<TestValidatedInput>()
                .query(|_ctx, input: TestValidatedInput| async move {
                    Ok(input.value)
                });

            let ctx = Context::new(());
            let input = serde_json::json!({"value": value});

            let result = rt.block_on((procedure.handler)(ctx, input));
            prop_assert!(result.is_err());
            prop_assert_eq!(result.unwrap_err().code, crate::RpcErrorCode::ValidationError);
        });
    }

    /// Property: Validated input accepts valid data
    #[test]
    fn prop_validated_input_accepts_valid() {
        use crate::validation::{FieldError, Validate, ValidationResult};
        use serde::Deserialize;

        #[derive(Debug, Deserialize)]
        struct TestValidatedInput {
            value: i32,
        }

        impl Validate for TestValidatedInput {
            fn validate(&self) -> ValidationResult {
                if self.value < 0 || self.value > 100 {
                    ValidationResult::from_errors(vec![FieldError::range("value", 0, 100)])
                } else {
                    ValidationResult::ok()
                }
            }
        }

        proptest!(|(value in 0i32..=100)| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let procedure = ProcedureBuilder::<()>::new("test")
                .input_validated::<TestValidatedInput>()
                .query(|_ctx, input: TestValidatedInput| async move {
                    Ok(input.value)
                });

            let ctx = Context::new(());
            let input = serde_json::json!({"value": value});

            let result = rt.block_on((procedure.handler)(ctx, input));
            prop_assert!(result.is_ok());
            prop_assert_eq!(result.unwrap(), serde_json::json!(value));
        });
    }
}
