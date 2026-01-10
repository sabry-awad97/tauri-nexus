//! Shared types - automatically exported to TypeScript via ts-rs

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// User entity
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../guest-js/bindings/")]
pub struct User {
    pub id: u32,
    pub name: String,
    pub email: String,
    #[serde(rename = "createdAt")]
    #[ts(rename = "createdAt")]
    pub created_at: String,
}

/// Input for creating a user
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../guest-js/bindings/")]
pub struct CreateUserInput {
    pub name: String,
    pub email: String,
}

/// Input for updating a user
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../guest-js/bindings/")]
pub struct UpdateUserInput {
    pub id: u32,
    pub name: Option<String>,
    pub email: Option<String>,
}

/// Paginated response
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../guest-js/bindings/", concrete(T = User))]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub total: u32,
    pub page: u32,
    #[serde(rename = "totalPages")]
    #[ts(rename = "totalPages")]
    pub total_pages: u32,
}

/// Success response
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../guest-js/bindings/")]
pub struct SuccessResponse {
    pub success: bool,
    pub message: Option<String>,
}

/// Pagination input
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../guest-js/bindings/")]
pub struct PaginationInput {
    pub page: Option<u32>,
    pub limit: Option<u32>,
}

impl PaginationInput {
    pub fn page(&self) -> u32 {
        self.page.unwrap_or(1)
    }
    pub fn limit(&self) -> u32 {
        self.limit.unwrap_or(10)
    }
}
