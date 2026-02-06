//! Authentication and authorization middleware

use crate::auth::config::AuthConfig;
use crate::auth::provider::AuthProvider;
use crate::auth::types::AuthorizationResult;
use crate::middleware::{MiddlewareFn, Next, Request, from_fn};
use crate::{Context, RpcError};
use std::sync::Arc;

// =============================================================================
// Auth Middleware
// =============================================================================

/// Create an authentication middleware.
///
/// This middleware validates that the user is authenticated using the
/// provided auth provider. If authentication fails, it returns an
/// UNAUTHORIZED error.
///
/// # Example
///
/// ```rust,ignore
/// use tauri_plugin_rpc::auth::{auth_middleware, AlwaysAuthProvider};
/// use tauri_plugin_rpc::Router;
///
/// let provider = AlwaysAuthProvider::new("test-user");
/// let router = Router::new()
///     .middleware(auth_middleware(provider))
///     .query("protected", protected_handler);
/// ```
///
/// # Error Messages
///
/// Returns detailed error messages including:
/// - The procedure path that was accessed
/// - What authentication is required
pub fn auth_middleware<Ctx, P>(provider: P) -> MiddlewareFn<Ctx>
where
    Ctx: Clone + Send + Sync + 'static,
    P: AuthProvider + Clone + 'static,
{
    let middleware = move |ctx: Context<Ctx>, req: Request, next: Next<Ctx>| {
        let provider = provider.clone();
        let path = req.path.clone();
        async move {
            let auth_result = provider.authenticate(&req).await;

            if !auth_result.authenticated {
                tracing::debug!(
                    path = %path,
                    "Authentication failed: no valid credentials provided"
                );
                return Err(RpcError::unauthorized(format!(
                    "Authentication required to access '{}'. Please provide valid credentials.",
                    path
                )));
            }

            tracing::trace!(
                path = %path,
                user_id = ?auth_result.user_id,
                roles = ?auth_result.roles,
                "Authentication successful"
            );

            next(ctx, req).await
        }
    };
    from_fn(middleware)
}

/// Create an authorization middleware with configuration.
///
/// This middleware checks both authentication and authorization based
/// on the provided config. It returns UNAUTHORIZED for unauthenticated
/// users and FORBIDDEN for users lacking required roles.
///
/// # Example
///
/// ```rust,ignore
/// use tauri_plugin_rpc::auth::{auth_with_config, AuthConfig, AlwaysAuthProvider};
/// use tauri_plugin_rpc::Router;
///
/// let provider = AlwaysAuthProvider::new("test-user");
/// let config = AuthConfig::new()
///     .public("health")
///     .requires_roles("admin.*", vec!["admin"]);
///
/// let router = Router::new()
///     .middleware(auth_with_config(provider, config))
///     .query("health", health_handler)
///     .query("admin.users", admin_users_handler);
/// ```
///
/// # Error Messages
///
/// Returns detailed error messages including:
/// - The procedure path that was accessed
/// - What authentication/authorization is required
/// - Which roles are needed (for forbidden errors)
pub fn auth_with_config<Ctx, P>(provider: P, config: AuthConfig) -> MiddlewareFn<Ctx>
where
    Ctx: Clone + Send + Sync + 'static,
    P: AuthProvider + Clone + 'static,
{
    let config = Arc::new(config);
    let middleware = move |ctx: Context<Ctx>, req: Request, next: Next<Ctx>| {
        let provider = provider.clone();
        let config = Arc::clone(&config);
        let path = req.path.clone();
        async move {
            let auth_result = provider.authenticate(&req).await;

            match config.is_authorized(&req.path, &auth_result) {
                AuthorizationResult::Allowed => {
                    tracing::trace!(
                        path = %path,
                        user_id = ?auth_result.user_id,
                        "Authorization granted"
                    );
                    next(ctx, req).await
                }
                AuthorizationResult::Unauthorized => {
                    tracing::debug!(
                        path = %path,
                        "Authorization denied: authentication required for path '{}'",
                        path
                    );
                    Err(RpcError::unauthorized(format!(
                        "Authentication required to access '{}'. Please provide valid credentials.",
                        path
                    )))
                }
                AuthorizationResult::Forbidden(required_roles) => {
                    tracing::warn!(
                        path = %path,
                        user_id = ?auth_result.user_id,
                        user_roles = ?auth_result.roles,
                        required_roles = ?required_roles,
                        "Authorization denied: user lacks required roles"
                    );

                    let msg = if required_roles.is_empty() {
                        format!("Access denied to '{}'", path)
                    } else {
                        format!(
                            "Access denied to '{}'. Required roles: {}",
                            path,
                            required_roles.join(", ")
                        )
                    };
                    Err(RpcError::forbidden(msg))
                }
            }
        }
    };
    from_fn(middleware)
}

/// Create a simple role-checking middleware.
///
/// This middleware checks if the authenticated user has any of the
/// required roles. It should be used after an authentication middleware.
///
/// # Example
///
/// ```rust,ignore
/// use tauri_plugin_rpc::auth::{auth_middleware, requires_roles, AlwaysAuthProvider};
/// use tauri_plugin_rpc::Router;
///
/// let provider = AlwaysAuthProvider::new("test-user")
///     .with_roles(vec!["admin"]);
///
/// let router = Router::new()
///     .middleware(auth_middleware(provider.clone()))
///     .middleware(requires_roles(provider, vec!["admin".to_string()]))
///     .query("admin.users", admin_users_handler);
/// ```
///
/// # Error Messages
///
/// Returns detailed error messages including:
/// - The procedure path that was accessed
/// - Which roles are required
/// - What roles the user has (in logs only)
pub fn requires_roles<Ctx, P>(provider: P, roles: Vec<String>) -> MiddlewareFn<Ctx>
where
    Ctx: Clone + Send + Sync + 'static,
    P: AuthProvider + Clone + 'static,
{
    let roles = Arc::new(roles);
    let middleware = move |ctx: Context<Ctx>, req: Request, next: Next<Ctx>| {
        let provider = provider.clone();
        let roles = Arc::clone(&roles);
        let path = req.path.clone();
        async move {
            let auth_result = provider.authenticate(&req).await;

            if !auth_result.authenticated {
                tracing::debug!(
                    path = %path,
                    required_roles = ?roles.as_ref(),
                    "Role check failed: not authenticated"
                );
                return Err(RpcError::unauthorized(format!(
                    "Authentication required to access '{}'. Please provide valid credentials.",
                    path
                )));
            }

            let role_refs: Vec<&str> = roles.iter().map(|s| s.as_str()).collect();
            if !auth_result.has_any_role(&role_refs) {
                tracing::warn!(
                    path = %path,
                    user_id = ?auth_result.user_id,
                    user_roles = ?auth_result.roles,
                    required_roles = ?roles.as_ref(),
                    "Role check failed: insufficient roles"
                );
                return Err(RpcError::forbidden(format!(
                    "Access denied to '{}'. Required roles: {}",
                    path,
                    roles.join(", ")
                )));
            }

            tracing::trace!(
                path = %path,
                user_id = ?auth_result.user_id,
                "Role check passed"
            );

            next(ctx, req).await
        }
    };
    from_fn(middleware)
}
