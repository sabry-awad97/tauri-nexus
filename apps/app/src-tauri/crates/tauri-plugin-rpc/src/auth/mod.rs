//! Authentication and Authorization middleware for RPC operations
//!
//! This module provides a flexible authentication and authorization system
//! for securing RPC procedures.
//!
//! # Quick Start
//!
//! ```rust,ignore
//! use tauri_plugin_rpc::auth::*;
//! use tauri_plugin_rpc::prelude::*;
//!
//! // 1. Implement a custom auth provider
//! struct TokenAuthProvider {
//!     secret: String,
//! }
//!
//! impl AuthProvider for TokenAuthProvider {
//!     fn authenticate(&self, request: &Request) -> Pin<Box<dyn Future<Output = AuthResult> + Send + '_>> {
//!         Box::pin(async move {
//!             // Extract token from request and validate
//!             if let Some(token) = request.input.get("token").and_then(|v| v.as_str()) {
//!                 if self.validate_token(token) {
//!                     return AuthResult::authenticated("user-123")
//!                         .with_roles(vec!["user", "admin"]);
//!                 }
//!             }
//!             AuthResult::unauthenticated()
//!         })
//!     }
//! }
//!
//! // 2. Configure auth rules
//! let config = AuthConfig::new()
//!     .public("health")
//!     .public("auth.login")
//!     .requires_auth("user.*")
//!     .requires_roles("admin.*", vec!["admin"]);
//!
//! // 3. Apply middleware to router
//! let router = Router::new()
//!     .middleware(auth_with_config(TokenAuthProvider { secret: "...".into() }, config))
//!     .query("health", health_handler)
//!     .query("user.profile", user_profile_handler)
//!     .query("admin.users", admin_users_handler);
//! ```
//!
//! # Common Patterns
//!
//! ## Public Endpoints
//!
//! ```rust,ignore
//! let config = AuthConfig::new()
//!     .public("health")
//!     .public("auth.login")
//!     .public("auth.register");
//! ```
//!
//! ## Authenticated Endpoints
//!
//! ```rust,ignore
//! let config = AuthConfig::new()
//!     .authenticated("user.*")
//!     .authenticated("profile.*");
//! ```
//!
//! ## Role-Based Access
//!
//! ```rust,ignore
//! let config = AuthConfig::new()
//!     .admin_only("admin.*")
//!     .any_role("moderator.*", &["admin", "moderator"])
//!     .all_roles("superadmin.*", &["admin", "superuser"]);
//! ```
//!
//! ## Mixed Configuration
//!
//! ```rust,ignore
//! let config = AuthConfig::new()
//!     // Public endpoints
//!     .public_many(vec!["health", "auth.login", "auth.register"])
//!     // User endpoints require authentication
//!     .authenticated("user.*")
//!     // Admin endpoints require admin role
//!     .admin_only("admin.*")
//!     // Moderator endpoints require admin OR moderator role
//!     .any_role("moderator.*", &["admin", "moderator"]);
//! ```
//!
//! # Security Considerations
//!
//! ## Rule Order Matters
//!
//! Rules are evaluated in order, and the **first matching rule wins**.
//! Place more specific rules before general ones:
//!
//! ```rust,ignore
//! let config = AuthConfig::new()
//!     .public("admin.health")        // Specific: admin.health is public
//!     .admin_only("admin.*");        // General: other admin.* require admin role
//! ```
//!
//! ## Default is Secure
//!
//! By default, all endpoints require authentication. Use `public()` to
//! explicitly allow unauthenticated access:
//!
//! ```rust,ignore
//! // Secure by default
//! let config = AuthConfig::new();
//! // All endpoints require authentication unless explicitly made public
//! ```
//!
//! ## Use HTTPS in Production
//!
//! Always use HTTPS in production to protect credentials in transit.
//! Authentication tokens, API keys, and other credentials should never
//! be sent over unencrypted connections.
//!
//! ## Validate Tokens Server-Side
//!
//! Always validate authentication tokens on the server. Never trust
//! client-provided authentication information without verification:
//!
//! ```rust,ignore
//! impl AuthProvider for JwtAuthProvider {
//!     fn authenticate(&self, request: &Request) -> Pin<Box<dyn Future<Output = AuthResult> + Send + '_>> {
//!         Box::pin(async move {
//!             if let Some(token) = extract_token(request) {
//!                 // ✅ Validate signature, expiration, issuer, etc.
//!                 if let Ok(claims) = validate_jwt(&token, &self.secret) {
//!                     return AuthResult::authenticated(claims.sub)
//!                         .with_roles(claims.roles);
//!                 }
//!             }
//!             AuthResult::unauthenticated()
//!         })
//!     }
//! }
//! ```
//!
//! ## Don't Expose Sensitive Information
//!
//! Error messages should not reveal sensitive information:
//! - ✅ "Authentication required"
//! - ✅ "Access denied. Required roles: admin"
//! - ❌ "Invalid token signature"
//! - ❌ "User 'admin' does not exist"
//!
//! # Architecture
//!
//! The auth system consists of several components:
//!
//! - [`AuthResult`] - Result of authentication containing user info and roles
//! - [`AuthProvider`] - Trait for implementing custom authentication logic
//! - [`AuthRule`] - Rules for protecting specific procedures
//! - [`AuthConfig`] - Configuration for auth middleware
//! - [`auth_middleware`] - Middleware for authentication only
//! - [`auth_with_config`] - Middleware for authentication + authorization
//! - [`requires_roles`] - Middleware for role checking
//!
//! # Module Organization
//!
//! - [`types`] - Core types (AuthResult, AuthorizationResult)
//! - [`provider`] - AuthProvider trait and built-in implementations
//! - [`rules`] - AuthRule and pattern matching
//! - [`config`] - AuthConfig builder
//! - [`middleware`] - Middleware functions
//! - [`context`] - Context extension for accessing auth in handlers

pub mod config;
pub mod context;
pub mod middleware;
pub mod provider;
pub mod rules;
pub mod types;

// Re-export public API
pub use config::AuthConfig;
pub use context::AuthContextExt;
pub use middleware::{auth_middleware, auth_with_config, requires_roles};
pub use provider::{AlwaysAuthProvider, AuthProvider, NoAuthProvider};
pub use rules::{AuthRule, CompiledPattern};
pub use types::{AuthResult, AuthorizationResult};

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
        /// Property: Authentication Validation
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

        /// Property: Authorization Role Enforcement
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
            use rules::CompiledPattern;

            // Global wildcard should match everything
            let wildcard = CompiledPattern::compile("*");
            prop_assert!(wildcard.matches(&path));

            // Exact match should only match itself
            let exact = CompiledPattern::compile(&path);
            prop_assert!(exact.matches(&path));

            // Wildcard patterns
            if path.contains('.') {
                let prefix = path.split('.').next().unwrap();
                let pattern = CompiledPattern::compile(&format!("{}.*", prefix));
                prop_assert!(pattern.matches(&path));
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

// Note: Integration tests have been moved to tests/auth_integration.rs
// This keeps the module focused on property-based tests and unit tests.
