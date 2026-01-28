
//! Integration tests for authentication and authorization
//!
//! These tests verify end-to-end behavior of the auth system
//! with a real router and middleware stack.

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::future::Future;
use std::pin::Pin;
use tauri_plugin_rpc::auth::{
    auth_middleware, auth_with_config, requires_roles, AuthConfig, AuthProvider, AuthResult,
};
use tauri_plugin_rpc::{Context, Router, RpcErrorCode, RpcResult};

// =============================================================================
// Test Context and Handlers
// =============================================================================

#[derive(Clone, Default)]
struct TestContext;

#[derive(Debug, Deserialize)]
struct TestInput {
    #[allow(dead_code)]
    value: String,
}

#[derive(Debug, Serialize)]
struct TestOutput {
    result: String,
}

async fn test_handler(_ctx: Context<TestContext>, _input: TestInput) -> RpcResult<TestOutput> {
    Ok(TestOutput {
        result: "success".to_string(),
    })
}

async fn health_handler(_ctx: Context<TestContext>, _input: ()) -> RpcResult<String> {
    Ok("healthy".to_string())
}

// =============================================================================
// Test Auth Provider
// =============================================================================

/// Auth provider that checks for a "token" field in the input
#[derive(Clone)]
struct TestAuthProvider {
    valid_token: String,
    user_id: String,
    roles: Vec<String>,
}

impl TestAuthProvider {
    fn new(valid_token: &str, user_id: &str, roles: Vec<&str>) -> Self {
        Self {
            valid_token: valid_token.to_string(),
            user_id: user_id.to_string(),
            roles: roles.into_iter().map(|s| s.to_string()).collect(),
        }
    }
}

impl AuthProvider for TestAuthProvider {
    fn authenticate(
        &self,
        request: &tauri_plugin_rpc::middleware::Request,
    ) -> Pin<Box<dyn Future<Output = AuthResult> + Send + '_>> {
        let token = request
            .input
            .get("token")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let valid_token = self.valid_token.clone();
        let user_id = self.user_id.clone();
        let roles = self.roles.clone();

        Box::pin(async move {
            if let Some(t) = token
                && t == valid_token
            {
                return AuthResult::authenticated(user_id).with_roles(roles);
            }
            AuthResult::unauthenticated()
        })
    }
}

// =============================================================================
// Integration Tests
// =============================================================================

#[tokio::test]
async fn test_auth_middleware_allows_authenticated() {
    let provider = TestAuthProvider::new("secret", "user-123", vec!["user"]);

    let router = Router::new()
        .context(TestContext)
        .middleware_fn(auth_middleware(provider))
        .query("test", test_handler);

    let result = router
        .call(
            "test",
            json!({
                "token": "secret",
                "value": "test"
            }),
        )
        .await;

    assert!(result.is_ok());
    let output = result.unwrap();
    assert_eq!(output["result"], "success");
}

#[tokio::test]
async fn test_auth_middleware_rejects_unauthenticated() {
    let provider = TestAuthProvider::new("secret", "user-123", vec!["user"]);

    let router = Router::new()
        .context(TestContext)
        .middleware_fn(auth_middleware(provider))
        .query("test", test_handler);

    let result = router
        .call(
            "test",
            json!({
                "token": "wrong",
                "value": "test"
            }),
        )
        .await;

    assert!(result.is_err());
    let error = result.unwrap_err();
    assert_eq!(error.code, RpcErrorCode::Unauthorized);
    assert!(error.message.contains("Authentication required"));
    assert!(error.message.contains("test"));
}

#[tokio::test]
async fn test_auth_with_config_public_path() {
    let provider = TestAuthProvider::new("secret", "user-123", vec!["user"]);
    let config = AuthConfig::new().public("health");

    let router = Router::new()
        .context(TestContext)
        .middleware_fn(auth_with_config(provider, config))
        .query("health", health_handler)
        .query("test", test_handler);

    // Health should be accessible without auth
    let result = router.call("health", json!(null)).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "healthy");

    // Test should require auth
    let result = router
        .call(
            "test",
            json!({
                "value": "test"
            }),
        )
        .await;
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert_eq!(error.code, RpcErrorCode::Unauthorized);
}

#[tokio::test]
async fn test_auth_with_config_requires_roles() {
    let provider = TestAuthProvider::new("secret", "user-123", vec!["user"]);
    let config = AuthConfig::new()
        .public("health")
        .requires_roles("admin.*", vec!["admin"]);

    let router = Router::new()
        .context(TestContext)
        .middleware_fn(auth_with_config(provider, config))
        .query("health", health_handler)
        .query("admin.users", test_handler);

    // User without admin role should be forbidden
    let result = router
        .call(
            "admin.users",
            json!({
                "token": "secret",
                "value": "test"
            }),
        )
        .await;

    assert!(result.is_err());
    let error = result.unwrap_err();
    assert_eq!(error.code, RpcErrorCode::Forbidden);
    assert!(error.message.contains("admin.users"));
    assert!(error.message.contains("admin"));
}

#[tokio::test]
async fn test_auth_with_config_admin_access() {
    let provider = TestAuthProvider::new("admin-secret", "admin-123", vec!["admin", "user"]);
    let config = AuthConfig::new()
        .public("health")
        .requires_roles("admin.*", vec!["admin"]);

    let router = Router::new()
        .context(TestContext)
        .middleware_fn(auth_with_config(provider, config))
        .query("admin.users", test_handler);

    // Admin should have access
    let result = router
        .call(
            "admin.users",
            json!({
                "token": "admin-secret",
                "value": "test"
            }),
        )
        .await;

    assert!(result.is_ok());
    let output = result.unwrap();
    assert_eq!(output["result"], "success");
}

#[tokio::test]
async fn test_requires_roles_middleware() {
    let provider = TestAuthProvider::new("secret", "user-123", vec!["user"]);

    let router = Router::new()
        .context(TestContext)
        .middleware_fn(requires_roles(provider, vec!["admin".to_string()]))
        .query("admin.users", test_handler);

    // User without admin role should be forbidden
    let result = router
        .call(
            "admin.users",
            json!({
                "token": "secret",
                "value": "test"
            }),
        )
        .await;

    assert!(result.is_err());
    let error = result.unwrap_err();
    assert_eq!(error.code, RpcErrorCode::Forbidden);
    assert!(error.message.contains("admin"));
}

#[tokio::test]
async fn test_compiled_router_with_auth() {
    let provider = TestAuthProvider::new("secret", "user-123", vec!["user"]);
    let config = AuthConfig::new().public("health");

    let router = Router::new()
        .context(TestContext)
        .middleware_fn(auth_with_config(provider, config))
        .query("health", health_handler)
        .query("test", test_handler)
        .compile();

    // Health should be accessible without auth
    let result = router.call("health", json!(null)).await;
    assert!(result.is_ok());

    // Test should require auth
    let result = router
        .call(
            "test",
            json!({
                "value": "test"
            }),
        )
        .await;
    assert!(result.is_err());

    // Test with valid auth
    let result = router
        .call(
            "test",
            json!({
                "token": "secret",
                "value": "test"
            }),
        )
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_error_messages_include_context() {
    let provider = TestAuthProvider::new("secret", "user-123", vec!["user"]);
    let config = AuthConfig::new()
        .requires_auth("user.*")
        .requires_roles("admin.*", vec!["admin"]);

    let router = Router::new()
        .context(TestContext)
        .middleware_fn(auth_with_config(provider, config))
        .query("user.profile", test_handler)
        .query("admin.settings", test_handler);

    // Unauthorized error should include path
    let result = router.call("user.profile", json!({"value": "test"})).await;
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.message.contains("user.profile"));
    assert!(error.message.contains("Authentication required"));

    // Forbidden error should include path and required roles
    let result = router
        .call(
            "admin.settings",
            json!({"token": "secret", "value": "test"}),
        )
        .await;
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.message.contains("admin.settings"));
    assert!(error.message.contains("admin"));
}

#[tokio::test]
async fn test_rule_order_precedence() {
    let provider = TestAuthProvider::new("secret", "user-123", vec!["user"]);

    // More specific rule (public) should win over general rule (requires auth)
    let config = AuthConfig::new()
        .public("admin.health")
        .requires_roles("admin.*", vec!["admin"]);

    let router = Router::new()
        .context(TestContext)
        .middleware_fn(auth_with_config(provider, config))
        .query("admin.health", health_handler)
        .query("admin.users", test_handler);

    // admin.health should be public (first matching rule)
    let result = router.call("admin.health", json!(null)).await;
    assert!(result.is_ok());

    // admin.users should require admin role
    let result = router
        .call(
            "admin.users",
            json!({"token": "secret", "value": "test"}),
        )
        .await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code, RpcErrorCode::Forbidden);
}

#[tokio::test]
async fn test_complex_config_with_multiple_rules() {
    let provider = TestAuthProvider::new("secret", "user-123", vec!["user", "moderator"]);
    let config = AuthConfig::new()
        .public_many(vec!["health", "auth.login", "auth.register"])
        .authenticated("user.*")
        .any_role("moderator.*", &["admin", "moderator"])
        .admin_only("admin.*");

    let router = Router::new()
        .context(TestContext)
        .middleware_fn(auth_with_config(provider, config))
        .query("health", health_handler)
        .query("auth.login", health_handler)
        .query("user.profile", test_handler)
        .query("moderator.posts", test_handler)
        .query("admin.settings", test_handler);

    // Public endpoints
    assert!(router.call("health", json!(null)).await.is_ok());
    assert!(router.call("auth.login", json!(null)).await.is_ok());

    // User endpoints (requires auth)
    assert!(router
        .call("user.profile", json!({"value": "test"}))
        .await
        .is_err());
    assert!(router
        .call(
            "user.profile",
            json!({"token": "secret", "value": "test"})
        )
        .await
        .is_ok());

    // Moderator endpoints (requires moderator or admin role)
    assert!(router
        .call(
            "moderator.posts",
            json!({"token": "secret", "value": "test"})
        )
        .await
        .is_ok());

    // Admin endpoints (requires admin role)
    let result = router
        .call(
            "admin.settings",
            json!({"token": "secret", "value": "test"}),
        )
        .await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code, RpcErrorCode::Forbidden);
}

#[tokio::test]
async fn test_ergonomic_helper_methods() {
    let provider = TestAuthProvider::new("secret", "admin-123", vec!["admin"]);

    // Test all ergonomic helpers
    let config = AuthConfig::new()
        .public("health")
        .authenticated("user.*")
        .admin_only("admin.*")
        .any_role("moderator.*", &["admin", "moderator"])
        .all_roles("superadmin.*", &["admin", "superuser"]);

    let router = Router::new()
        .context(TestContext)
        .middleware_fn(auth_with_config(provider, config))
        .query("health", health_handler)
        .query("user.profile", test_handler)
        .query("admin.users", test_handler)
        .query("moderator.posts", test_handler)
        .query("superadmin.system", test_handler);

    // Public
    assert!(router.call("health", json!(null)).await.is_ok());

    // Authenticated
    assert!(router
        .call(
            "user.profile",
            json!({"token": "secret", "value": "test"})
        )
        .await
        .is_ok());

    // Admin only
    assert!(router
        .call(
            "admin.users",
            json!({"token": "secret", "value": "test"})
        )
        .await
        .is_ok());

    // Any role (admin has admin role)
    assert!(router
        .call(
            "moderator.posts",
            json!({"token": "secret", "value": "test"})
        )
        .await
        .is_ok());

    // All roles (admin lacks superuser role)
    let result = router
        .call(
            "superadmin.system",
            json!({"token": "secret", "value": "test"}),
        )
        .await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code, RpcErrorCode::Forbidden);
}
