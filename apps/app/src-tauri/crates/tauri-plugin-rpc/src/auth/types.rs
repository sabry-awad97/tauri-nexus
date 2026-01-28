//! Core authentication and authorization types

use serde::{Deserialize, Serialize};

// =============================================================================
// Auth Result
// =============================================================================

/// Result of an authentication attempt.
///
/// Contains information about whether the user is authenticated,
/// their identity, and their roles/permissions.
///
/// # Example
///
/// ```rust,ignore
/// use tauri_plugin_rpc::auth::AuthResult;
///
/// // Unauthenticated user
/// let guest = AuthResult::unauthenticated();
/// assert!(!guest.is_authenticated());
///
/// // Authenticated user with roles
/// let admin = AuthResult::authenticated("user-123")
///     .with_roles(vec!["admin", "user"]);
/// assert!(admin.has_role("admin"));
/// ```
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
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let result = AuthResult::unauthenticated();
    /// assert!(!result.is_authenticated());
    /// ```
    pub fn unauthenticated() -> Self {
        Self {
            authenticated: false,
            user_id: None,
            roles: Vec::new(),
            metadata: None,
        }
    }

    /// Create an authenticated result with a user ID.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let result = AuthResult::authenticated("user-123");
    /// assert!(result.is_authenticated());
    /// assert_eq!(result.user_id(), Some("user-123"));
    /// ```
    pub fn authenticated(user_id: impl Into<String>) -> Self {
        Self {
            authenticated: true,
            user_id: Some(user_id.into()),
            roles: Vec::new(),
            metadata: None,
        }
    }

    /// Check if the user is authenticated.
    pub fn is_authenticated(&self) -> bool {
        self.authenticated
    }

    /// Get the user ID if authenticated.
    pub fn user_id(&self) -> Option<&str> {
        self.user_id.as_deref()
    }

    /// Add roles to the auth result.
    #[must_use = "This method returns a new AuthResult and does not modify self"]
    pub fn with_roles(mut self, roles: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.roles = roles.into_iter().map(|r| r.into()).collect();
        self
    }

    /// Add a single role to the auth result.
    #[must_use = "This method returns a new AuthResult and does not modify self"]
    pub fn with_role(mut self, role: impl Into<String>) -> Self {
        self.roles.push(role.into());
        self
    }

    /// Add metadata to the auth result.
    #[must_use = "This method returns a new AuthResult and does not modify self"]
    pub fn with_metadata(mut self, metadata: impl Serialize) -> Self {
        self.metadata = serde_json::to_value(metadata).ok();
        self
    }

    /// Check if the user has a specific role.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let user = AuthResult::authenticated("user-123")
    ///     .with_roles(vec!["admin", "user"]);
    /// assert!(user.has_role("admin"));
    /// assert!(!user.has_role("guest"));
    /// ```
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r == role)
    }

    /// Check if the user has any of the specified roles.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let user = AuthResult::authenticated("user-123")
    ///     .with_roles(vec!["user"]);
    /// assert!(user.has_any_role(&["admin", "user"]));
    /// assert!(!user.has_any_role(&["admin", "superuser"]));
    /// ```
    pub fn has_any_role(&self, roles: &[&str]) -> bool {
        roles.iter().any(|r| self.has_role(r))
    }

    /// Check if the user has all of the specified roles.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let user = AuthResult::authenticated("user-123")
    ///     .with_roles(vec!["admin", "user"]);
    /// assert!(user.has_all_roles(&["admin", "user"]));
    /// assert!(!user.has_all_roles(&["admin", "superuser"]));
    /// ```
    pub fn has_all_roles(&self, roles: &[&str]) -> bool {
        roles.iter().all(|r| self.has_role(r))
    }
}

// =============================================================================
// Authorization Result
// =============================================================================

/// Result of an authorization check.
///
/// Indicates whether access is allowed, and if not, why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthorizationResult {
    /// Access is allowed
    Allowed,
    /// User is not authenticated
    Unauthorized,
    /// User is authenticated but lacks required roles
    Forbidden(Vec<String>),
}

impl AuthorizationResult {
    /// Check if access is allowed.
    pub fn is_allowed(&self) -> bool {
        matches!(self, Self::Allowed)
    }

    /// Get the required roles if access was forbidden.
    pub fn required_roles(&self) -> Option<&[String]> {
        match self {
            Self::Forbidden(roles) => Some(roles),
            _ => None,
        }
    }
}
