//! Application types
//!
//! These types are mirrored in TypeScript at: src/generated/types.ts

use serde::{Deserialize, Serialize};

// =============================================================================
// User Types
// =============================================================================

/// User entity
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: u32,
    pub name: String,
    pub email: String,
    pub created_at: String,
}

impl User {
    pub fn new(id: u32, name: impl Into<String>, email: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            email: email.into(),
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}

/// Input for getting a user by ID
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
}

/// Input for deleting a user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteUserInput {
    pub id: u32,
}

// =============================================================================
// General Types
// =============================================================================

/// Input for greeting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GreetInput {
    pub name: String,
}

/// Health check response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}
