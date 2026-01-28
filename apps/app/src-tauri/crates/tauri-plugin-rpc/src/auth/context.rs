//! Authentication context extension

use crate::auth::types::AuthResult;

// =============================================================================
// Auth Context Extension
// =============================================================================

/// Extension trait for adding auth result to context.
///
/// This allows middleware to store authentication results
/// that can be accessed by handlers downstream.
///
/// # Note
///
/// This trait is currently a placeholder. Full implementation
/// requires modifications to the request/context system to support
/// request-scoped storage.
///
/// # Future Implementation
///
/// The planned implementation will:
/// 1. Add request IDs to the `Request` struct
/// 2. Store `AuthResult` in request-scoped storage
/// 3. Allow handlers to retrieve auth info via this trait
///
/// # Example (Future)
///
/// ```rust,ignore
/// use tauri_plugin_rpc::auth::AuthContextExt;
///
/// async fn my_handler(ctx: Context<AppContext>, input: MyInput) -> RpcResult<MyOutput> {
///     // Get auth info from context
///     if let Some(auth) = ctx.auth() {
///         println!("User: {:?}", auth.user_id);
///         println!("Roles: {:?}", auth.roles);
///     }
///     // ... handler logic
/// }
/// ```
pub trait AuthContextExt {
    /// Get the auth result from context metadata.
    ///
    /// Returns `None` if no authentication has been performed
    /// or if the request is unauthenticated.
    fn auth(&self) -> Option<AuthResult>;
}

// Note: Implementation will be added in Phase 2, Task 2.1
// This requires:
// - Request ID generation
// - Request-scoped storage mechanism
// - Integration with middleware system
