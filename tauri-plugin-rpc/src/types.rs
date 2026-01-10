//! Common types for RPC
//! 
//! These types have corresponding TypeScript definitions.
//! When modifying, update the TypeScript types as well.

use serde::{Deserialize, Serialize};

/// Generic paginated response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub total: u32,
    pub page: u32,
    pub total_pages: u32,
}

/// Generic success response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessResponse {
    pub success: bool,
    pub message: Option<String>,
}

impl SuccessResponse {
    pub fn ok(message: impl Into<String>) -> Self {
        Self { success: true, message: Some(message.into()) }
    }
    
    pub fn success() -> Self {
        Self { success: true, message: None }
    }
}

/// Pagination input
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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
    
    pub fn offset(&self) -> u32 {
        (self.page() - 1) * self.limit()
    }
}
