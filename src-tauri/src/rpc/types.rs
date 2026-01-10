//! App-specific types
//!
//! Define your types here. Mirror these in TypeScript at:
//! src/generated/types.ts

use serde::{Deserialize, Serialize};

/// User entity
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: u32,
    pub name: String,
    pub email: String,
    pub created_at: String,
}

/// Input for getting a user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetUserInput {
    pub id: u32,
}

/// Input for creating a user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserInput {
    pub name: String,
    pub email: String,
}

/// Input for updating a user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateUserInput {
    pub id: u32,
    pub name: Option<String>,
    pub email: Option<String>,
}

/// Input for deleting a user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteUserInput {
    pub id: u32,
}

/// Greet input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GreetInput {
    pub name: String,
}
