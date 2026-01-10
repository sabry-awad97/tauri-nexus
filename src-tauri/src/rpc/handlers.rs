//! RPC Handlers
//!
//! Define your handlers here and register them in create_router().

use super::*;
use tauri_plugin_rpc::middleware::{Next, Request, Response};

// =============================================================================
// Middleware
// =============================================================================

/// Logging middleware - logs all RPC calls
pub async fn logging(
    ctx: Context<AppContext>,
    req: Request,
    next: Next<AppContext>,
) -> RpcResult<Response> {
    let start = std::time::Instant::now();
    println!("→ [{}] {}", req.procedure_type, req.path);
    
    let result = next(ctx, req.clone()).await;
    let duration = start.elapsed();
    
    match &result {
        Ok(_) => println!("← [{}] {} ({:?})", req.procedure_type, req.path, duration),
        Err(e) => println!("✗ [{}] {} - {} ({:?})", req.procedure_type, req.path, e.code, duration),
    }
    
    result
}

// =============================================================================
// Router
// =============================================================================

/// Create the application router
pub fn create_router() -> Router<AppContext> {
    Router::new()
        .context(AppContext::new())
        .middleware(logging)
        // Root procedures
        .query("health", health_handler)
        .query("greet", greet_handler)
        // User procedures
        .merge("user", user_router())
}

/// User sub-router
fn user_router() -> Router<AppContext> {
    Router::new()
        .context(AppContext::new())
        .query("get", get_user)
        .query("list", list_users)
        .mutation("create", create_user)
        .mutation("update", update_user)
        .mutation("delete", delete_user)
}

// =============================================================================
// Root Handlers
// =============================================================================

async fn health_handler(_ctx: Context<AppContext>, _: ()) -> RpcResult<HealthResponse> {
    Ok(HealthResponse {
        status: "ok".into(),
        version: env!("CARGO_PKG_VERSION").into(),
    })
}

async fn greet_handler(_ctx: Context<AppContext>, input: GreetInput) -> RpcResult<String> {
    if input.name.is_empty() {
        return Err(RpcError::validation("Name cannot be empty"));
    }
    Ok(format!("Hello, {}! 👋", input.name))
}

// =============================================================================
// User Handlers
// =============================================================================

async fn get_user(ctx: Context<AppContext>, input: GetUserInput) -> RpcResult<User> {
    ctx.db
        .get_user(input.id)
        .ok_or_else(|| RpcError::not_found(format!("User {} not found", input.id)))
}

async fn list_users(ctx: Context<AppContext>, _: ()) -> RpcResult<Vec<User>> {
    Ok(ctx.db.list_users())
}

async fn create_user(ctx: Context<AppContext>, input: CreateUserInput) -> RpcResult<User> {
    // Validation
    if input.name.trim().is_empty() {
        return Err(RpcError::validation("Name is required"));
    }
    if !input.email.contains('@') {
        return Err(RpcError::validation("Invalid email format"));
    }

    ctx.db
        .create_user(&input.name, &input.email)
        .ok_or_else(|| RpcError::internal("Failed to create user"))
}

async fn update_user(ctx: Context<AppContext>, input: UpdateUserInput) -> RpcResult<User> {
    // Validation
    if let Some(ref email) = input.email {
        if !email.contains('@') {
            return Err(RpcError::validation("Invalid email format"));
        }
    }

    ctx.db
        .update_user(input.id, input.name.as_deref(), input.email.as_deref())
        .ok_or_else(|| RpcError::not_found(format!("User {} not found", input.id)))
}

async fn delete_user(ctx: Context<AppContext>, input: DeleteUserInput) -> RpcResult<SuccessResponse> {
    if ctx.db.delete_user(input.id) {
        Ok(SuccessResponse::ok(format!("User {} deleted", input.id)))
    } else {
        Err(RpcError::not_found(format!("User {} not found", input.id)))
    }
}
