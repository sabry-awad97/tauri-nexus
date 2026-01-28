//! Authentication provider trait and implementations

use crate::auth::types::AuthResult;
use crate::middleware::Request;
use std::future::Future;
use std::pin::Pin;

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
///     fn authenticate(
///         &self,
///         request: &Request,
///     ) -> Pin<Box<dyn Future<Output = AuthResult> + Send + '_>> {
///         Box::pin(async move {
///             // Extract and validate JWT from request
///             if let Some(token) = extract_token(request) {
///                 if let Ok(claims) = validate_jwt(&token, &self.secret) {
///                     return AuthResult::authenticated(claims.sub)
///                         .with_roles(claims.roles);
///                 }
///             }
///             AuthResult::unauthenticated()
///         })
///     }
/// }
/// ```
pub trait AuthProvider: Send + Sync {
    /// Authenticate a request and return the auth result.
    ///
    /// # Parameters
    ///
    /// - `request`: The incoming RPC request containing input data
    ///
    /// # Returns
    ///
    /// An `AuthResult` indicating whether the user is authenticated,
    /// their identity, and their roles.
    ///
    /// # Implementation Notes
    ///
    /// - Extract credentials from `request.input` (e.g., token, API key)
    /// - Validate credentials (check signature, expiration, etc.)
    /// - Return `AuthResult::authenticated()` with user info on success
    /// - Return `AuthResult::unauthenticated()` on failure
    /// - Never panic - always return a result
    fn authenticate(
        &self,
        request: &Request,
    ) -> Pin<Box<dyn Future<Output = AuthResult> + Send + '_>>;
}

// =============================================================================
// Built-in Providers
// =============================================================================

/// A simple auth provider that always returns unauthenticated.
///
/// Useful for testing or as a placeholder.
///
/// # Example
///
/// ```rust,ignore
/// use tauri_plugin_rpc::auth::NoAuthProvider;
///
/// let provider = NoAuthProvider;
/// ```
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
///
/// Useful for testing or development environments.
///
/// # Example
///
/// ```rust,ignore
/// use tauri_plugin_rpc::auth::AlwaysAuthProvider;
///
/// let provider = AlwaysAuthProvider::new("test-user")
///     .with_roles(vec!["admin", "user"]);
/// ```
///
/// # Security Warning
///
/// **Never use this in production!** This provider bypasses all authentication.
#[derive(Debug, Clone)]
pub struct AlwaysAuthProvider {
    user_id: String,
    roles: Vec<String>,
}

impl AlwaysAuthProvider {
    /// Create a new always-authenticated provider.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let provider = AlwaysAuthProvider::new("test-user");
    /// ```
    pub fn new(user_id: impl Into<String>) -> Self {
        Self {
            user_id: user_id.into(),
            roles: Vec::new(),
        }
    }

    /// Add roles to the provider.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let provider = AlwaysAuthProvider::new("test-user")
    ///     .with_roles(vec!["admin", "user"]);
    /// ```
    #[must_use = "This method returns a new AlwaysAuthProvider and does not modify self"]
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
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn create_test_request() -> Request {
        Request {
            path: "test".to_string(),
            procedure_type: crate::middleware::ProcedureType::Query,
            input: json!({}),
        }
    }

    #[tokio::test]
    async fn test_no_auth_provider() {
        let provider = NoAuthProvider;
        let request = create_test_request();
        let result = provider.authenticate(&request).await;
        assert!(!result.is_authenticated());
    }

    #[tokio::test]
    async fn test_always_auth_provider() {
        let provider = AlwaysAuthProvider::new("test-user").with_roles(vec!["admin", "user"]);
        let request = create_test_request();
        let result = provider.authenticate(&request).await;

        assert!(result.is_authenticated());
        assert_eq!(result.user_id(), Some("test-user"));
        assert!(result.has_role("admin"));
        assert!(result.has_role("user"));
    }
}
