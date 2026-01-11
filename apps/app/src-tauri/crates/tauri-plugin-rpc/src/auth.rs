//! Authentication and Authorization middleware for RPC operations
//!
//! This module provides a flexible authentication and authorization system
//! for securing RPC procedures.
//!
//! # Overview
//!
//! The auth system consists of:
//! - [`AuthResult`] - Result of authentication containing user info and roles
//! - [`AuthProvider`] - Trait for implementing custom authentication logic
//! - [`AuthRule`] - Rules for protecting specific procedures
//! - [`AuthConfig`] - Configuration for auth middleware
//!
//! # Example
//!
//! ```rust,ignore
//! use tauri_plugin_rpc::auth::*;
//! use tauri_plugin_rpc::prelude::*;
//!
//! // Implement a custom auth provider
//! struct TokenAuthProvider;
//!
//! impl AuthProvider for TokenAuthProvider {
//!     async fn authenticate(&self, request: &Request) -> AuthResult {
//!         // Extract token from request and validate
//!         if let Some(token) = request.input.get("token").and_then(|v| v.as_str()) {
//!             if token == "valid-token" {
//!                 return AuthResult::authenticated("user-123")
//!                     .with_roles(vec!["user", "admin"]);
//!             }
//!         }
//!         AuthResult::unauthenticated()
//!     }
//! }
//!
//! // Configure auth rules
//! let config = AuthConfig::new()
//!     .public("health")
//!     .public("auth.login")
//!     .requires_auth("user.*")
//!     .requires_roles("admin.*", vec!["admin"]);
//! ```

use crate::middleware::{MiddlewareFn, Next, Request, from_fn};
use crate::{Context, RpcError};
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

// =============================================================================
// Auth Result
// =============================================================================

/// Result of an authentication attempt.
///
/// Contains information about whether the user is authenticated,
/// their identity, and their roles/permissions.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuthResult {
    /// Whether the user is authenticated
    pub authenticated: bool,
    /// User identifier (if authenticated)
    pub user_id: Option<String>,
    /// User's roles/permissions
    pub roles: Vec<String>,
    /// Additional metadata about the authenticated user
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl AuthResult {
    /// Create an unauthenticated result.
    pub fn unauthenticated() -> Self {
        Self {
            authenticated: false,
            user_id: None,
            roles: Vec::new(),
            metadata: None,
        }
    }

    /// Create an authenticated result with a user ID.
    pub fn authenticated(user_id: impl Into<String>) -> Self {
        Self {
            authenticated: true,
            user_id: Some(user_id.into()),
            roles: Vec::new(),
            metadata: None,
        }
    }

    /// Add roles to the auth result.
    pub fn with_roles(mut self, roles: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.roles = roles.into_iter().map(|r| r.into()).collect();
        self
    }

    /// Add a single role to the auth result.
    pub fn with_role(mut self, role: impl Into<String>) -> Self {
        self.roles.push(role.into());
        self
    }

    /// Add metadata to the auth result.
    pub fn with_metadata(mut self, metadata: impl Serialize) -> Self {
        self.metadata = serde_json::to_value(metadata).ok();
        self
    }

    /// Check if the user has a specific role.
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r == role)
    }

    /// Check if the user has any of the specified roles.
    pub fn has_any_role(&self, roles: &[&str]) -> bool {
        roles.iter().any(|r| self.has_role(r))
    }

    /// Check if the user has all of the specified roles.
    pub fn has_all_roles(&self, roles: &[&str]) -> bool {
        roles.iter().all(|r| self.has_role(r))
    }
}

// =============================================================================
// Auth Provider Trait
// =============================================================================

/// Trait for implementing custom authentication logic.
///
/// Implement this trait to define how users are authenticated
/// based on the incoming request.
///
/// # Example
///
/// ```rust,ignore
/// use tauri_plugin_rpc::auth::{AuthProvider, AuthResult};
/// use tauri_plugin_rpc::middleware::Request;
///
/// struct JwtAuthProvider {
///     secret: String,
/// }
///
/// impl AuthProvider for JwtAuthProvider {
///     async fn authenticate(&self, request: &Request) -> AuthResult {
///         // Extract and validate JWT from request
///         if let Some(token) = extract_token(request) {
///             if let Ok(claims) = validate_jwt(&token, &self.secret) {
///                 return AuthResult::authenticated(claims.sub)
///                     .with_roles(claims.roles);
///             }
///         }
///         AuthResult::unauthenticated()
///     }
/// }
/// ```
pub trait AuthProvider: Send + Sync {
    /// Authenticate a request and return the auth result.
    fn authenticate(
        &self,
        request: &Request,
    ) -> Pin<Box<dyn Future<Output = AuthResult> + Send + '_>>;
}

/// A simple auth provider that always returns unauthenticated.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoAuthProvider;

impl AuthProvider for NoAuthProvider {
    fn authenticate(
        &self,
        _request: &Request,
    ) -> Pin<Box<dyn Future<Output = AuthResult> + Send + '_>> {
        Box::pin(async { AuthResult::unauthenticated() })
    }
}

/// A simple auth provider that always returns authenticated with given user.
#[derive(Debug, Clone)]
pub struct AlwaysAuthProvider {
    user_id: String,
    roles: Vec<String>,
}

impl AlwaysAuthProvider {
    /// Create a new always-authenticated provider.
    pub fn new(user_id: impl Into<String>) -> Self {
        Self {
            user_id: user_id.into(),
            roles: Vec::new(),
        }
    }

    /// Add roles to the provider.
    pub fn with_roles(mut self, roles: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.roles = roles.into_iter().map(|r| r.into()).collect();
        self
    }
}

impl AuthProvider for AlwaysAuthProvider {
    fn authenticate(
        &self,
        _request: &Request,
    ) -> Pin<Box<dyn Future<Output = AuthResult> + Send + '_>> {
        let result = AuthResult::authenticated(self.user_id.clone()).with_roles(self.roles.clone());
        Box::pin(async move { result })
    }
}

// =============================================================================
// Auth Rule
// =============================================================================

/// A rule for protecting a procedure or set of procedures.
#[derive(Debug, Clone)]
pub struct AuthRule {
    /// Pattern to match procedure paths (supports wildcards like "user.*")
    pub path_pattern: String,
    /// Required roles (empty means any authenticated user)
    pub required_roles: Vec<String>,
    /// Whether this path is public (no auth required)
    pub public: bool,
    /// Whether all roles are required (true) or any role (false)
    pub require_all_roles: bool,
}

impl AuthRule {
    /// Create a public rule (no authentication required).
    pub fn public(pattern: impl Into<String>) -> Self {
        Self {
            path_pattern: pattern.into(),
            required_roles: Vec::new(),
            public: true,
            require_all_roles: false,
        }
    }

    /// Create a rule requiring authentication (any authenticated user).
    pub fn requires_auth(pattern: impl Into<String>) -> Self {
        Self {
            path_pattern: pattern.into(),
            required_roles: Vec::new(),
            public: false,
            require_all_roles: false,
        }
    }

    /// Create a rule requiring specific roles.
    pub fn requires_roles(
        pattern: impl Into<String>,
        roles: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            path_pattern: pattern.into(),
            required_roles: roles.into_iter().map(|r| r.into()).collect(),
            public: false,
            require_all_roles: false,
        }
    }

    /// Set whether all roles are required (default: any role).
    pub fn require_all(mut self) -> Self {
        self.require_all_roles = true;
        self
    }

    /// Check if this rule matches a given path.
    pub fn matches(&self, path: &str) -> bool {
        pattern_matches(&self.path_pattern, path)
    }

    /// Check if the auth result satisfies this rule.
    pub fn is_satisfied_by(&self, auth: &AuthResult) -> bool {
        if self.public {
            return true;
        }

        if !auth.authenticated {
            return false;
        }

        if self.required_roles.is_empty() {
            return true;
        }

        let role_refs: Vec<&str> = self.required_roles.iter().map(|s| s.as_str()).collect();
        if self.require_all_roles {
            auth.has_all_roles(&role_refs)
        } else {
            auth.has_any_role(&role_refs)
        }
    }
}

/// Check if a pattern matches a path.
/// Supports:
/// - Exact match: "user.get" matches "user.get"
/// - Wildcard suffix: "user.*" matches "user.get", "user.create", etc.
/// - Global wildcard: "*" matches everything
fn pattern_matches(pattern: &str, path: &str) -> bool {
    if pattern == "*" {
        return true;
    }

    if let Some(prefix) = pattern.strip_suffix(".*") {
        return path == prefix || path.starts_with(&format!("{}.", prefix));
    }

    pattern == path
}

// =============================================================================
// Auth Config
// =============================================================================

/// Configuration for authentication and authorization.
///
/// Defines rules for which procedures require authentication
/// and what roles are needed.
#[derive(Debug, Clone, Default)]
pub struct AuthConfig {
    /// List of auth rules (evaluated in order)
    pub rules: Vec<AuthRule>,
    /// Whether procedures are public by default
    pub default_public: bool,
}

impl AuthConfig {
    /// Create a new auth config with default settings.
    /// By default, all procedures require authentication.
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            default_public: false,
        }
    }

    /// Create a config where all procedures are public by default.
    pub fn public_by_default() -> Self {
        Self {
            rules: Vec::new(),
            default_public: true,
        }
    }

    /// Add a public rule (no authentication required).
    pub fn public(mut self, pattern: impl Into<String>) -> Self {
        self.rules.push(AuthRule::public(pattern));
        self
    }

    /// Add a rule requiring authentication.
    pub fn requires_auth(mut self, pattern: impl Into<String>) -> Self {
        self.rules.push(AuthRule::requires_auth(pattern));
        self
    }

    /// Add a rule requiring specific roles (any of the roles).
    pub fn requires_roles(
        mut self,
        pattern: impl Into<String>,
        roles: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.rules.push(AuthRule::requires_roles(pattern, roles));
        self
    }

    /// Add a rule requiring all specified roles.
    pub fn requires_all_roles(
        mut self,
        pattern: impl Into<String>,
        roles: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.rules
            .push(AuthRule::requires_roles(pattern, roles).require_all());
        self
    }

    /// Add a custom rule.
    pub fn rule(mut self, rule: AuthRule) -> Self {
        self.rules.push(rule);
        self
    }

    /// Find the first matching rule for a path.
    pub fn find_rule(&self, path: &str) -> Option<&AuthRule> {
        self.rules.iter().find(|rule| rule.matches(path))
    }

    /// Check if a path is authorized for the given auth result.
    pub fn is_authorized(&self, path: &str, auth: &AuthResult) -> AuthorizationResult {
        if let Some(rule) = self.find_rule(path) {
            if rule.is_satisfied_by(auth) {
                AuthorizationResult::Allowed
            } else if !auth.authenticated {
                AuthorizationResult::Unauthorized
            } else {
                AuthorizationResult::Forbidden(rule.required_roles.clone())
            }
        } else if self.default_public || auth.authenticated {
            AuthorizationResult::Allowed
        } else {
            AuthorizationResult::Unauthorized
        }
    }
}

/// Result of an authorization check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthorizationResult {
    /// Access is allowed
    Allowed,
    /// User is not authenticated
    Unauthorized,
    /// User is authenticated but lacks required roles
    Forbidden(Vec<String>),
}

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
/// use tauri_plugin_rpc::auth::{auth_middleware, TokenAuthProvider};
///
/// let router = Router::new()
///     .middleware(auth_middleware(TokenAuthProvider::new()))
///     .query("protected", protected_handler);
/// ```
pub fn auth_middleware<Ctx, P>(provider: P) -> MiddlewareFn<Ctx>
where
    Ctx: Clone + Send + Sync + 'static,
    P: AuthProvider + Clone + 'static,
{
    from_fn(move |ctx: Context<Ctx>, req: Request, next: Next<Ctx>| {
        let provider = provider.clone();
        async move {
            let auth_result = provider.authenticate(&req).await;

            if !auth_result.authenticated {
                return Err(RpcError::unauthorized("Authentication required"));
            }

            next(ctx, req).await
        }
    })
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
/// use tauri_plugin_rpc::auth::{auth_with_config, AuthConfig, TokenAuthProvider};
///
/// let config = AuthConfig::new()
///     .public("health")
///     .requires_roles("admin.*", vec!["admin"]);
///
/// let router = Router::new()
///     .middleware(auth_with_config(TokenAuthProvider::new(), config))
///     .query("health", health_handler)
///     .query("admin.users", admin_users_handler);
/// ```
pub fn auth_with_config<Ctx, P>(provider: P, config: AuthConfig) -> MiddlewareFn<Ctx>
where
    Ctx: Clone + Send + Sync + 'static,
    P: AuthProvider + Clone + 'static,
{
    let config = Arc::new(config);
    from_fn(move |ctx: Context<Ctx>, req: Request, next: Next<Ctx>| {
        let provider = provider.clone();
        let config = Arc::clone(&config);
        async move {
            let auth_result = provider.authenticate(&req).await;

            match config.is_authorized(&req.path, &auth_result) {
                AuthorizationResult::Allowed => next(ctx, req).await,
                AuthorizationResult::Unauthorized => {
                    Err(RpcError::unauthorized("Authentication required"))
                }
                AuthorizationResult::Forbidden(required_roles) => {
                    let msg = if required_roles.is_empty() {
                        "Access denied".to_string()
                    } else {
                        format!(
                            "Access denied. Required roles: {}",
                            required_roles.join(", ")
                        )
                    };
                    Err(RpcError::forbidden(msg))
                }
            }
        }
    })
}

/// Create a simple role-checking middleware.
///
/// This middleware checks if the authenticated user has any of the
/// required roles. It should be used after an authentication middleware.
///
/// # Example
///
/// ```rust,ignore
/// use tauri_plugin_rpc::auth::{auth_middleware, requires_roles, TokenAuthProvider};
///
/// let router = Router::new()
///     .middleware(auth_middleware(TokenAuthProvider::new()))
///     .middleware(requires_roles(vec!["admin"]))
///     .query("admin.users", admin_users_handler);
/// ```
pub fn requires_roles<Ctx, P>(provider: P, roles: Vec<String>) -> MiddlewareFn<Ctx>
where
    Ctx: Clone + Send + Sync + 'static,
    P: AuthProvider + Clone + 'static,
{
    let roles = Arc::new(roles);
    from_fn(move |ctx: Context<Ctx>, req: Request, next: Next<Ctx>| {
        let provider = provider.clone();
        let roles = Arc::clone(&roles);
        async move {
            let auth_result = provider.authenticate(&req).await;

            if !auth_result.authenticated {
                return Err(RpcError::unauthorized("Authentication required"));
            }

            let role_refs: Vec<&str> = roles.iter().map(|s| s.as_str()).collect();
            if !auth_result.has_any_role(&role_refs) {
                return Err(RpcError::forbidden(format!(
                    "Access denied. Required roles: {}",
                    roles.join(", ")
                )));
            }

            next(ctx, req).await
        }
    })
}

// =============================================================================
// Auth Context Extension
// =============================================================================

/// Extension trait for adding auth result to context.
///
/// This allows middleware to store auth information that can be
/// accessed by handlers.
pub trait AuthContextExt {
    /// Get the auth result from context metadata.
    fn auth(&self) -> Option<AuthResult>;
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_result_unauthenticated() {
        let result = AuthResult::unauthenticated();
        assert!(!result.authenticated);
        assert!(result.user_id.is_none());
        assert!(result.roles.is_empty());
    }

    #[test]
    fn test_auth_result_authenticated() {
        let result = AuthResult::authenticated("user-123");
        assert!(result.authenticated);
        assert_eq!(result.user_id, Some("user-123".to_string()));
    }

    #[test]
    fn test_auth_result_with_roles() {
        let result = AuthResult::authenticated("user-123").with_roles(vec!["admin", "user"]);
        assert!(result.has_role("admin"));
        assert!(result.has_role("user"));
        assert!(!result.has_role("guest"));
    }

    #[test]
    fn test_auth_result_has_any_role() {
        let result = AuthResult::authenticated("user-123").with_roles(vec!["user"]);
        assert!(result.has_any_role(&["admin", "user"]));
        assert!(!result.has_any_role(&["admin", "superuser"]));
    }

    #[test]
    fn test_auth_result_has_all_roles() {
        let result = AuthResult::authenticated("user-123").with_roles(vec!["admin", "user"]);
        assert!(result.has_all_roles(&["admin", "user"]));
        assert!(!result.has_all_roles(&["admin", "superuser"]));
    }

    #[test]
    fn test_pattern_matches_exact() {
        assert!(pattern_matches("user.get", "user.get"));
        assert!(!pattern_matches("user.get", "user.create"));
    }

    #[test]
    fn test_pattern_matches_wildcard() {
        assert!(pattern_matches("user.*", "user.get"));
        assert!(pattern_matches("user.*", "user.create"));
        assert!(pattern_matches("user.*", "user"));
        assert!(!pattern_matches("user.*", "admin.get"));
    }

    #[test]
    fn test_pattern_matches_global() {
        assert!(pattern_matches("*", "anything"));
        assert!(pattern_matches("*", "user.get"));
    }

    #[test]
    fn test_auth_rule_public() {
        let rule = AuthRule::public("health");
        assert!(rule.public);
        assert!(rule.is_satisfied_by(&AuthResult::unauthenticated()));
    }

    #[test]
    fn test_auth_rule_requires_auth() {
        let rule = AuthRule::requires_auth("user.*");
        assert!(!rule.public);
        assert!(!rule.is_satisfied_by(&AuthResult::unauthenticated()));
        assert!(rule.is_satisfied_by(&AuthResult::authenticated("user-123")));
    }

    #[test]
    fn test_auth_rule_requires_roles() {
        let rule = AuthRule::requires_roles("admin.*", vec!["admin"]);
        assert!(!rule.is_satisfied_by(&AuthResult::unauthenticated()));
        assert!(!rule.is_satisfied_by(&AuthResult::authenticated("user-123")));
        assert!(
            rule.is_satisfied_by(&AuthResult::authenticated("user-123").with_roles(vec!["admin"]))
        );
    }

    #[test]
    fn test_auth_rule_requires_all_roles() {
        let rule = AuthRule::requires_roles("admin.*", vec!["admin", "superuser"]).require_all();
        assert!(
            !rule.is_satisfied_by(&AuthResult::authenticated("user-123").with_roles(vec!["admin"]))
        );
        assert!(rule.is_satisfied_by(
            &AuthResult::authenticated("user-123").with_roles(vec!["admin", "superuser"])
        ));
    }

    #[test]
    fn test_auth_config_public() {
        let config = AuthConfig::new().public("health");
        let auth = AuthResult::unauthenticated();
        assert_eq!(
            config.is_authorized("health", &auth),
            AuthorizationResult::Allowed
        );
    }

    #[test]
    fn test_auth_config_requires_auth() {
        let config = AuthConfig::new().requires_auth("user.*");
        let unauth = AuthResult::unauthenticated();
        let auth = AuthResult::authenticated("user-123");

        assert_eq!(
            config.is_authorized("user.get", &unauth),
            AuthorizationResult::Unauthorized
        );
        assert_eq!(
            config.is_authorized("user.get", &auth),
            AuthorizationResult::Allowed
        );
    }

    #[test]
    fn test_auth_config_requires_roles() {
        let config = AuthConfig::new().requires_roles("admin.*", vec!["admin"]);
        let user = AuthResult::authenticated("user-123").with_roles(vec!["user"]);
        let admin = AuthResult::authenticated("admin-123").with_roles(vec!["admin"]);

        assert_eq!(
            config.is_authorized("admin.users", &user),
            AuthorizationResult::Forbidden(vec!["admin".to_string()])
        );
        assert_eq!(
            config.is_authorized("admin.users", &admin),
            AuthorizationResult::Allowed
        );
    }

    #[test]
    fn test_auth_config_default_public() {
        let config = AuthConfig::public_by_default();
        let unauth = AuthResult::unauthenticated();
        assert_eq!(
            config.is_authorized("any.path", &unauth),
            AuthorizationResult::Allowed
        );
    }

    #[test]
    fn test_auth_config_default_requires_auth() {
        let config = AuthConfig::new();
        let unauth = AuthResult::unauthenticated();
        let auth = AuthResult::authenticated("user-123");

        assert_eq!(
            config.is_authorized("any.path", &unauth),
            AuthorizationResult::Unauthorized
        );
        assert_eq!(
            config.is_authorized("any.path", &auth),
            AuthorizationResult::Allowed
        );
    }

    #[test]
    fn test_auth_config_rule_order() {
        // More specific rules should be added first
        let config = AuthConfig::new()
            .public("admin.health")
            .requires_roles("admin.*", vec!["admin"]);

        let unauth = AuthResult::unauthenticated();

        // admin.health should be public (first matching rule)
        assert_eq!(
            config.is_authorized("admin.health", &unauth),
            AuthorizationResult::Allowed
        );

        // admin.users should require admin role
        assert_eq!(
            config.is_authorized("admin.users", &unauth),
            AuthorizationResult::Unauthorized
        );
    }
}

// =============================================================================
// Property-Based Tests
// =============================================================================

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    // Strategy for generating user IDs
    fn user_id_strategy() -> impl Strategy<Value = String> {
        "[a-zA-Z0-9_-]{1,20}".prop_map(|s| s)
    }

    // Strategy for generating role names
    fn role_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("admin".to_string()),
            Just("user".to_string()),
            Just("guest".to_string()),
            Just("moderator".to_string()),
            Just("superuser".to_string()),
        ]
    }

    // Strategy for generating a list of roles
    fn roles_strategy() -> impl Strategy<Value = Vec<String>> {
        prop::collection::vec(role_strategy(), 0..5)
    }

    // Strategy for generating procedure paths
    fn path_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("health".to_string()),
            Just("user.get".to_string()),
            Just("user.create".to_string()),
            Just("user.delete".to_string()),
            Just("admin.users".to_string()),
            Just("admin.settings".to_string()),
            Just("public.info".to_string()),
        ]
    }

    // Strategy for generating auth results
    fn auth_result_strategy() -> impl Strategy<Value = AuthResult> {
        prop_oneof![
            Just(AuthResult::unauthenticated()),
            (user_id_strategy(), roles_strategy()).prop_map(|(user_id, roles)| {
                AuthResult::authenticated(user_id).with_roles(roles)
            }),
        ]
    }

    proptest! {
        /// Property 11: Authentication Validation
        /// Unauthenticated users should be denied access to protected procedures.
        /// Authenticated users should be granted access to procedures they have permission for.
        #[test]
        fn prop_authentication_validation(
            path in path_strategy(),
            auth in auth_result_strategy(),
        ) {
            // Default config requires authentication for all paths
            let config = AuthConfig::new();

            let result = config.is_authorized(&path, &auth);

            if auth.authenticated {
                // Authenticated users should be allowed (no specific role requirements)
                prop_assert_eq!(result, AuthorizationResult::Allowed);
            } else {
                // Unauthenticated users should be denied
                prop_assert_eq!(result, AuthorizationResult::Unauthorized);
            }
        }

        /// Property: Public paths allow unauthenticated access
        #[test]
        fn prop_public_paths_allow_unauthenticated(
            auth in auth_result_strategy(),
        ) {
            let config = AuthConfig::new().public("health");

            // Health should always be allowed
            let result = config.is_authorized("health", &auth);
            prop_assert_eq!(result, AuthorizationResult::Allowed);
        }

        /// Property 12: Authorization Role Enforcement
        /// Users without required roles should be denied access.
        /// Users with required roles should be granted access.
        #[test]
        fn prop_authorization_role_enforcement(
            user_id in user_id_strategy(),
            user_roles in roles_strategy(),
        ) {
            let config = AuthConfig::new()
                .requires_roles("admin.*", vec!["admin"]);

            let auth = AuthResult::authenticated(user_id).with_roles(user_roles.clone());
            let result = config.is_authorized("admin.users", &auth);

            if user_roles.contains(&"admin".to_string()) {
                prop_assert_eq!(result, AuthorizationResult::Allowed);
            } else {
                prop_assert_eq!(result, AuthorizationResult::Forbidden(vec!["admin".to_string()]));
            }
        }

        /// Property: Role checking is consistent
        #[test]
        fn prop_role_checking_consistent(
            user_id in user_id_strategy(),
            roles in roles_strategy(),
            check_role in role_strategy(),
        ) {
            let auth = AuthResult::authenticated(user_id).with_roles(roles.clone());

            // has_role should be consistent with the roles list
            let has_role = auth.has_role(&check_role);
            let in_list = roles.contains(&check_role);

            prop_assert_eq!(has_role, in_list);
        }

        /// Property: has_any_role is true iff at least one role matches
        #[test]
        fn prop_has_any_role_correct(
            user_id in user_id_strategy(),
            user_roles in roles_strategy(),
            check_roles in prop::collection::vec(role_strategy(), 1..4),
        ) {
            let auth = AuthResult::authenticated(user_id).with_roles(user_roles.clone());
            let check_refs: Vec<&str> = check_roles.iter().map(|s| s.as_str()).collect();

            let has_any = auth.has_any_role(&check_refs);
            let expected = check_roles.iter().any(|r| user_roles.contains(r));

            prop_assert_eq!(has_any, expected);
        }

        /// Property: has_all_roles is true iff all roles match
        #[test]
        fn prop_has_all_roles_correct(
            user_id in user_id_strategy(),
            user_roles in roles_strategy(),
            check_roles in prop::collection::vec(role_strategy(), 1..4),
        ) {
            let auth = AuthResult::authenticated(user_id).with_roles(user_roles.clone());
            let check_refs: Vec<&str> = check_roles.iter().map(|s| s.as_str()).collect();

            let has_all = auth.has_all_roles(&check_refs);
            let expected = check_roles.iter().all(|r| user_roles.contains(r));

            prop_assert_eq!(has_all, expected);
        }

        /// Property: Pattern matching is consistent
        #[test]
        fn prop_pattern_matching_consistent(
            path in path_strategy(),
        ) {
            // Global wildcard should match everything
            prop_assert!(pattern_matches("*", &path));

            // Exact match should only match itself
            prop_assert!(pattern_matches(&path, &path));

            // Wildcard patterns
            if path.contains('.') {
                let prefix = path.split('.').next().unwrap();
                let pattern = format!("{}.*", prefix);
                prop_assert!(pattern_matches(&pattern, &path));
            }
        }

        /// Property: Rule satisfaction is consistent with auth state
        #[test]
        fn prop_rule_satisfaction_consistent(
            auth in auth_result_strategy(),
        ) {
            // Public rules should always be satisfied
            let public_rule = AuthRule::public("test");
            prop_assert!(public_rule.is_satisfied_by(&auth));

            // Auth-required rules should only be satisfied by authenticated users
            let auth_rule = AuthRule::requires_auth("test");
            prop_assert_eq!(auth_rule.is_satisfied_by(&auth), auth.authenticated);
        }

        /// Property: Config rule order matters - first match wins
        #[test]
        fn prop_config_rule_order_first_match_wins(
            auth in auth_result_strategy(),
        ) {
            // Create config with conflicting rules - public first, then requires auth
            let config = AuthConfig::new()
                .public("test.path")
                .requires_auth("test.*");

            // The public rule should win for exact match
            let result = config.is_authorized("test.path", &auth);
            prop_assert_eq!(result, AuthorizationResult::Allowed);

            // But other test.* paths should require auth
            let result2 = config.is_authorized("test.other", &auth);
            if auth.authenticated {
                prop_assert_eq!(result2, AuthorizationResult::Allowed);
            } else {
                prop_assert_eq!(result2, AuthorizationResult::Unauthorized);
            }
        }
    }
}

// =============================================================================
// Integration Tests
// =============================================================================

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::{Context, Router, RpcErrorCode, RpcResult};
    use serde::{Deserialize, Serialize};
    use serde_json::json;

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
            request: &Request,
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
        assert_eq!(result.unwrap_err().code, RpcErrorCode::Unauthorized);
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
}
