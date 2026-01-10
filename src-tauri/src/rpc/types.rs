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

// =============================================================================
// Subscription Types
// =============================================================================

/// Input for counter subscription
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CounterInput {
    /// Starting value
    #[serde(default)]
    pub start: i32,
    /// Maximum count (stops after reaching this)
    #[serde(default = "default_max_count")]
    pub max_count: i32,
    /// Interval in milliseconds between updates
    #[serde(default = "default_interval")]
    pub interval_ms: u64,
}

fn default_max_count() -> i32 {
    10
}
fn default_interval() -> u64 {
    1000
}

/// Counter event data
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CounterEvent {
    pub count: i32,
    pub timestamp: String,
}

/// Input for chat room subscription
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatRoomInput {
    pub room_id: String,
}

/// Chat message event
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    pub id: String,
    pub room_id: String,
    pub user_id: String,
    pub text: String,
    pub timestamp: String,
}

/// Input for sending a chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendMessageInput {
    pub room_id: String,
    pub text: String,
}

/// Input for stock price subscription
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockInput {
    pub symbols: Vec<String>,
}

/// Stock price event
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StockPrice {
    pub symbol: String,
    pub price: f64,
    pub change: f64,
    pub change_percent: f64,
    pub timestamp: String,
}

/// Empty input type for subscriptions that don't need parameters
/// Accepts `{}` from JSON (empty object)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EmptyInput {
    #[serde(skip)]
    _private: (),
}
