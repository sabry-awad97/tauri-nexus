//! Authentication and authorization configuration

use crate::auth::rules::AuthRule;
use crate::auth::types::{AuthResult, AuthorizationResult};

// =============================================================================
// Auth Config
// =============================================================================

/// Configuration for authentication and authorization.
///
/// Defines rules for which procedures require authentication
/// and what roles are needed.
///
/// # Rule Evaluation
///
/// Rules are evaluated in order, and the **first matching rule** wins.
/// This means more specific rules should be added before general ones.
///
/// # Example
///
/// ```rust,ignore
/// use tauri_plugin_rpc::auth::AuthConfig;
///
/// let config = AuthConfig::new()
///     // Public endpoints
///     .public("health")
///     .public("auth.login")
///     // User endpoints require authentication
///     .requires_auth("user.*")
///     // Admin endpoints require admin role
///     .requires_roles("admin.*", vec!["admin"]);
/// ```
///
/// # Security Considerations
///
/// - **Rule order matters**: Place specific rules before general ones
/// - **Default is secure**: By default, all endpoints require authentication
/// - **Use HTTPS**: Always use HTTPS in production to protect credentials
/// - **Validate tokens**: Implement proper token validation in your AuthProvider
#[derive(Debug, Clone, Default)]
pub struct AuthConfig {
    /// List of auth rules (evaluated in order)
    pub rules: Vec<AuthRule>,
    /// Whether procedures are public by default
    pub default_public: bool,
}

impl AuthConfig {
    /// Create a new auth config with default settings.
    ///
    /// By default, all procedures require authentication.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let config = AuthConfig::new()
    ///     .public("health")
    ///     .requires_auth("user.*");
    /// ```
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            default_public: false,
        }
    }

    /// Create a config where all procedures are public by default.
    ///
    /// Use this when you want to explicitly protect specific endpoints
    /// rather than protecting everything by default.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let config = AuthConfig::public_by_default()
    ///     .requires_auth("user.*")
    ///     .requires_roles("admin.*", vec!["admin"]);
    /// ```
    pub fn public_by_default() -> Self {
        Self {
            rules: Vec::new(),
            default_public: true,
        }
    }

    /// Add a public rule (no authentication required).
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let config = AuthConfig::new()
    ///     .public("health")
    ///     .public("auth.login");
    /// ```
    #[must_use = "This method returns a new AuthConfig and does not modify self"]
    pub fn public(mut self, pattern: impl Into<String>) -> Self {
        self.rules.push(AuthRule::public(pattern));
        self
    }

    /// Add multiple public patterns at once.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let config = AuthConfig::new()
    ///     .public_many(vec!["health", "auth.login", "auth.register"]);
    /// ```
    #[must_use = "This method returns a new AuthConfig and does not modify self"]
    pub fn public_many(mut self, patterns: impl IntoIterator<Item = impl Into<String>>) -> Self {
        for pattern in patterns {
            self.rules.push(AuthRule::public(pattern));
        }
        self
    }

    /// Add a rule requiring authentication.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let config = AuthConfig::new()
    ///     .requires_auth("user.*");
    /// ```
    #[must_use = "This method returns a new AuthConfig and does not modify self"]
    pub fn requires_auth(mut self, pattern: impl Into<String>) -> Self {
        self.rules.push(AuthRule::requires_auth(pattern));
        self
    }

    /// Add a rule requiring specific roles (any of the roles).
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let config = AuthConfig::new()
    ///     .requires_roles("admin.*", vec!["admin", "superuser"]);
    /// ```
    #[must_use = "This method returns a new AuthConfig and does not modify self"]
    pub fn requires_roles(
        mut self,
        pattern: impl Into<String>,
        roles: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.rules.push(AuthRule::requires_roles(pattern, roles));
        self
    }

    /// Add a rule requiring all specified roles.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let config = AuthConfig::new()
    ///     .requires_all_roles("admin.*", vec!["admin", "superuser"]);
    /// ```
    #[must_use = "This method returns a new AuthConfig and does not modify self"]
    pub fn requires_all_roles(
        mut self,
        pattern: impl Into<String>,
        roles: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.rules
            .push(AuthRule::requires_roles(pattern, roles).require_all());
        self
    }

    /// Add a rule requiring any of the specified roles (alias for `requires_roles`).
    ///
    /// This is more explicit than `requires_roles` for readability.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let config = AuthConfig::new()
    ///     .any_role("moderator.*", &["admin", "moderator"]);
    /// ```
    #[must_use = "This method returns a new AuthConfig and does not modify self"]
    pub fn any_role(self, pattern: impl Into<String>, roles: &[&str]) -> Self {
        self.requires_roles(pattern, roles.iter().copied())
    }

    /// Add a rule requiring all of the specified roles (alias for `requires_all_roles`).
    ///
    /// This is more explicit than `requires_all_roles` for readability.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let config = AuthConfig::new()
    ///     .all_roles("admin.*", &["admin", "superuser"]);
    /// ```
    #[must_use = "This method returns a new AuthConfig and does not modify self"]
    pub fn all_roles(self, pattern: impl Into<String>, roles: &[&str]) -> Self {
        self.requires_all_roles(pattern, roles.iter().copied())
    }

    /// Common pattern: admin-only endpoints.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let config = AuthConfig::new()
    ///     .admin_only("admin.*");
    /// ```
    #[must_use = "This method returns a new AuthConfig and does not modify self"]
    pub fn admin_only(self, pattern: impl Into<String>) -> Self {
        self.requires_roles(pattern, vec!["admin"])
    }

    /// Common pattern: authenticated users only (alias for `requires_auth`).
    ///
    /// This is more explicit than `requires_auth` for readability.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let config = AuthConfig::new()
    ///     .authenticated("user.*");
    /// ```
    #[must_use = "This method returns a new AuthConfig and does not modify self"]
    pub fn authenticated(self, pattern: impl Into<String>) -> Self {
        self.requires_auth(pattern)
    }

    /// Add a custom rule.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let custom_rule = AuthRule::requires_roles("custom.*", vec!["custom"])
    ///     .require_all();
    /// let config = AuthConfig::new()
    ///     .rule(custom_rule);
    /// ```
    #[must_use = "This method returns a new AuthConfig and does not modify self"]
    pub fn rule(mut self, rule: AuthRule) -> Self {
        self.rules.push(rule);
        self
    }

    /// Find the first matching rule for a path.
    ///
    /// Returns `None` if no rule matches (use default behavior).
    pub fn find_rule(&self, path: &str) -> Option<&AuthRule> {
        self.rules.iter().find(|rule| rule.matches(path))
    }

    /// Check if a path is authorized for the given auth result.
    ///
    /// # Returns
    ///
    /// - `Allowed`: Access is granted
    /// - `Unauthorized`: User is not authenticated
    /// - `Forbidden(roles)`: User lacks required roles
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

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn test_ergonomic_helpers() {
        let config = AuthConfig::new()
            .admin_only("admin.*")
            .authenticated("user.*")
            .any_role("moderator.*", &["admin", "moderator"])
            .all_roles("superadmin.*", &["admin", "superuser"]);

        let admin = AuthResult::authenticated("admin-123").with_roles(vec!["admin"]);
        let user = AuthResult::authenticated("user-123").with_roles(vec!["user"]);

        assert_eq!(
            config.is_authorized("admin.users", &admin),
            AuthorizationResult::Allowed
        );
        assert_eq!(
            config.is_authorized("user.profile", &user),
            AuthorizationResult::Allowed
        );
    }
}
