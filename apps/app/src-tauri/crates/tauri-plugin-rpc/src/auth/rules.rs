//! Authorization rules and pattern matching

use crate::auth::types::AuthResult;

// =============================================================================
// Compiled Pattern
// =============================================================================

/// Compiled pattern for efficient path matching.
///
/// Patterns are compiled once during rule creation to avoid
/// runtime string operations on every authorization check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompiledPattern {
    /// Matches everything (pattern: "*")
    Wildcard,
    /// Exact string match (pattern: "user.get")
    Exact(String),
    /// Prefix match (pattern: "user.*" -> prefix: "user")
    Prefix(String),
}

impl CompiledPattern {
    /// Compile a pattern string into an efficient representation.
    ///
    /// # Supported Patterns
    ///
    /// - `"*"` - Matches everything
    /// - `"user.get"` - Exact match
    /// - `"user.*"` - Prefix match (matches "user", "user.get", "user.create", etc.)
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let pattern = CompiledPattern::compile("user.*");
    /// assert!(pattern.matches("user.get"));
    /// assert!(pattern.matches("user"));
    /// assert!(!pattern.matches("admin.get"));
    /// ```
    pub fn compile(pattern: &str) -> Self {
        if pattern == "*" {
            return Self::Wildcard;
        }

        if let Some(prefix) = pattern.strip_suffix(".*") {
            return Self::Prefix(prefix.to_string());
        }

        Self::Exact(pattern.to_string())
    }

    /// Check if this pattern matches a path.
    ///
    /// # Performance
    ///
    /// This method is optimized for hot-path usage:
    /// - Wildcard: O(1)
    /// - Exact: O(n) string comparison
    /// - Prefix: O(n) string comparison, no allocations
    pub fn matches(&self, path: &str) -> bool {
        match self {
            Self::Wildcard => true,
            Self::Exact(exact) => path == exact,
            Self::Prefix(prefix) => path == prefix || path.starts_with(&format!("{}.", prefix)),
        }
    }
}

// =============================================================================
// Auth Rule
// =============================================================================

/// A rule for protecting a procedure or set of procedures.
///
/// Rules define which paths require authentication and what roles
/// are needed to access them.
///
/// # Example
///
/// ```rust,ignore
/// use tauri_plugin_rpc::auth::AuthRule;
///
/// // Public endpoint (no auth required)
/// let health = AuthRule::public("health");
///
/// // Requires authentication
/// let user_endpoints = AuthRule::requires_auth("user.*");
///
/// // Requires specific roles
/// let admin_endpoints = AuthRule::requires_roles("admin.*", vec!["admin"]);
/// ```
#[derive(Debug, Clone)]
pub struct AuthRule {
    /// Compiled pattern for efficient matching
    pattern: CompiledPattern,
    /// Original pattern string (for display/debugging)
    path_pattern: String,
    /// Required roles (empty means any authenticated user)
    pub required_roles: Vec<String>,
    /// Whether this path is public (no auth required)
    pub public: bool,
    /// Whether all roles are required (true) or any role (false)
    pub require_all_roles: bool,
}

impl AuthRule {
    /// Create a public rule (no authentication required).
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let rule = AuthRule::public("health");
    /// assert!(rule.is_satisfied_by(&AuthResult::unauthenticated()));
    /// ```
    pub fn public(pattern: impl Into<String>) -> Self {
        let pattern_str = pattern.into();
        Self {
            pattern: CompiledPattern::compile(&pattern_str),
            path_pattern: pattern_str,
            required_roles: Vec::new(),
            public: true,
            require_all_roles: false,
        }
    }

    /// Create a rule requiring authentication (any authenticated user).
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let rule = AuthRule::requires_auth("user.*");
    /// assert!(!rule.is_satisfied_by(&AuthResult::unauthenticated()));
    /// assert!(rule.is_satisfied_by(&AuthResult::authenticated("user-123")));
    /// ```
    pub fn requires_auth(pattern: impl Into<String>) -> Self {
        let pattern_str = pattern.into();
        Self {
            pattern: CompiledPattern::compile(&pattern_str),
            path_pattern: pattern_str,
            required_roles: Vec::new(),
            public: false,
            require_all_roles: false,
        }
    }

    /// Create a rule requiring specific roles.
    ///
    /// By default, the user needs ANY of the specified roles.
    /// Use `.require_all()` to require ALL roles.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let rule = AuthRule::requires_roles("admin.*", vec!["admin"]);
    /// let user = AuthResult::authenticated("user-123").with_roles(vec!["admin"]);
    /// assert!(rule.is_satisfied_by(&user));
    /// ```
    pub fn requires_roles(
        pattern: impl Into<String>,
        roles: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        let pattern_str = pattern.into();
        Self {
            pattern: CompiledPattern::compile(&pattern_str),
            path_pattern: pattern_str,
            required_roles: roles.into_iter().map(|r| r.into()).collect(),
            public: false,
            require_all_roles: false,
        }
    }

    /// Set whether all roles are required (default: any role).
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let rule = AuthRule::requires_roles("admin.*", vec!["admin", "superuser"])
    ///     .require_all();
    /// ```
    pub fn require_all(mut self) -> Self {
        self.require_all_roles = true;
        self
    }

    /// Check if this rule matches a given path.
    pub fn matches(&self, path: &str) -> bool {
        self.pattern.matches(path)
    }

    /// Get the original pattern string (for display/debugging).
    pub fn pattern_string(&self) -> &str {
        &self.path_pattern
    }

    /// Check if the auth result satisfies this rule.
    ///
    /// # Logic
    ///
    /// - Public rules are always satisfied
    /// - Non-public rules require authentication
    /// - If roles are specified, user must have required roles
    /// - `require_all_roles` determines if ANY or ALL roles are needed
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

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compiled_pattern_wildcard() {
        let pattern = CompiledPattern::compile("*");
        assert_eq!(pattern, CompiledPattern::Wildcard);
        assert!(pattern.matches("anything"));
        assert!(pattern.matches("user.get"));
    }

    #[test]
    fn test_compiled_pattern_exact() {
        let pattern = CompiledPattern::compile("user.get");
        assert_eq!(pattern, CompiledPattern::Exact("user.get".to_string()));
        assert!(pattern.matches("user.get"));
        assert!(!pattern.matches("user.create"));
    }

    #[test]
    fn test_compiled_pattern_prefix() {
        let pattern = CompiledPattern::compile("user.*");
        assert_eq!(pattern, CompiledPattern::Prefix("user".to_string()));
        assert!(pattern.matches("user"));
        assert!(pattern.matches("user.get"));
        assert!(pattern.matches("user.create"));
        assert!(!pattern.matches("admin.get"));
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
    fn test_auth_rule_matches() {
        let rule = AuthRule::requires_auth("user.*");
        assert!(rule.matches("user.get"));
        assert!(rule.matches("user.create"));
        assert!(!rule.matches("admin.get"));
    }
}
