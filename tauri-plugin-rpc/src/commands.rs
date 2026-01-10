//! Plugin commands

use crate::{error::Error, types::*, RpcState};
use tauri::{command, State};

type Result<T> = std::result::Result<T, Error>;

/// Greet a user
#[command]
pub fn greet(name: String) -> Result<String> {
    Ok(format!("Hello, {}! Greeted from Tauri RPC Plugin!", name))
}

/// Get user by ID
#[command]
pub fn get_user(id: u32, state: State<RpcState>) -> Result<User> {
    let users = state.users.lock().map_err(|_| Error::LockError)?;
    users
        .iter()
        .find(|u| u.id == id)
        .cloned()
        .ok_or(Error::NotFound(format!("User {}", id)))
}

/// List users with pagination
#[command]
pub fn list_users(
    pagination: Option<PaginationInput>,
    state: State<RpcState>,
) -> Result<PaginatedResponse<User>> {
    let users = state.users.lock().map_err(|_| Error::LockError)?;
    let pagination = pagination.unwrap_or(PaginationInput { page: None, limit: None });

    let page = pagination.page();
    let limit = pagination.limit();
    let total = users.len() as u32;
    let total_pages = (total + limit - 1) / limit;

    let start = ((page - 1) * limit) as usize;
    let end = (start + limit as usize).min(users.len());

    let data = if start < users.len() {
        users[start..end].to_vec()
    } else {
        vec![]
    };

    Ok(PaginatedResponse {
        data,
        total,
        page,
        total_pages,
    })
}

/// Create a new user
#[command]
pub fn create_user(input: CreateUserInput, state: State<RpcState>) -> Result<User> {
    let mut users = state.users.lock().map_err(|_| Error::LockError)?;
    let mut next_id = state.next_id.lock().map_err(|_| Error::LockError)?;

    let user = User {
        id: *next_id,
        name: input.name,
        email: input.email,
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    *next_id += 1;
    users.push(user.clone());

    Ok(user)
}

/// Update a user
#[command]
pub fn update_user(input: UpdateUserInput, state: State<RpcState>) -> Result<User> {
    let mut users = state.users.lock().map_err(|_| Error::LockError)?;

    let user = users
        .iter_mut()
        .find(|u| u.id == input.id)
        .ok_or(Error::NotFound(format!("User {}", input.id)))?;

    if let Some(name) = input.name {
        user.name = name;
    }
    if let Some(email) = input.email {
        user.email = email;
    }

    Ok(user.clone())
}

/// Delete a user
#[command]
pub fn delete_user(id: u32, state: State<RpcState>) -> Result<SuccessResponse> {
    let mut users = state.users.lock().map_err(|_| Error::LockError)?;
    let len_before = users.len();
    users.retain(|u| u.id != id);

    if users.len() < len_before {
        Ok(SuccessResponse {
            success: true,
            message: Some(format!("User {} deleted", id)),
        })
    } else {
        Err(Error::NotFound(format!("User {}", id)))
    }
}
