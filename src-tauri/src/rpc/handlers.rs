//! RPC handlers

use super::*;
use tauri_plugin_rpc::{Context, middleware::{Request, Next, Response}};

/// Logging middleware
pub async fn logging_middleware(
    ctx: Context<AppContext>,
    req: Request,
    next: Next<AppContext>,
) -> RpcResult<Response> {
    println!("[RPC] {:?} {}", req.procedure_type, req.path);
    let result = next(ctx, req.clone()).await;
    match &result {
        Ok(_) => println!("[RPC] {} -> OK", req.path),
        Err(e) => println!("[RPC] {} -> ERROR: {}", req.path, e),
    }
    result
}

/// Create the application router
pub fn create_router() -> Router<AppContext> {
    Router::new()
        .context(AppContext::new())
        .middleware(logging_middleware)
        // Root level procedures
        .query("greet", greet_handler)
        // User procedures
        .merge("user", user_router())
}

fn user_router() -> Router<AppContext> {
    Router::new()
        .context(AppContext::new())
        .query("get", get_user_handler)
        .query("list", list_users_handler)
        .mutation("create", create_user_handler)
        .mutation("update", update_user_handler)
        .mutation("delete", delete_user_handler)
}

// Handler implementations

async fn greet_handler(_ctx: Context<AppContext>, input: GreetInput) -> RpcResult<String> {
    Ok(format!("Hello, {}! From Tauri RPC!", input.name))
}

async fn get_user_handler(ctx: Context<AppContext>, input: GetUserInput) -> RpcResult<User> {
    ctx.db
        .get_user(input.id)
        .ok_or_else(|| RpcError::not_found(format!("User {} not found", input.id)))
}

async fn list_users_handler(ctx: Context<AppContext>, _input: ()) -> RpcResult<Vec<User>> {
    Ok(ctx.db.list_users())
}

async fn create_user_handler(ctx: Context<AppContext>, input: CreateUserInput) -> RpcResult<User> {
    ctx.db
        .create_user(input.name, input.email)
        .ok_or_else(|| RpcError::internal("Failed to create user"))
}

async fn update_user_handler(ctx: Context<AppContext>, input: UpdateUserInput) -> RpcResult<User> {
    ctx.db
        .update_user(input.id, input.name, input.email)
        .ok_or_else(|| RpcError::not_found(format!("User {} not found", input.id)))
}

async fn delete_user_handler(ctx: Context<AppContext>, input: DeleteUserInput) -> RpcResult<SuccessResponse> {
    if ctx.db.delete_user(input.id) {
        Ok(SuccessResponse::ok(format!("User {} deleted", input.id)))
    } else {
        Err(RpcError::not_found(format!("User {} not found", input.id)))
    }
}
