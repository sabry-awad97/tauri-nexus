//! Common types for RPC operations

use serde::{Deserialize, Deserializer, Serialize};

// =============================================================================
// NoInput - Empty Input Type with Good DX
// =============================================================================

/// A type representing no input for RPC handlers.
///
/// This type accepts both `null` and `{}` from JSON, making it flexible for
/// frontend callers. Use this instead of `()` for handlers that don't need input.
///
/// # Example
///
/// ```rust,ignore
/// use tauri_plugin_rpc::prelude::*;
///
/// // Handler with no input
/// async fn health_check(_ctx: Context<AppContext>, _: NoInput) -> RpcResult<String> {
///     Ok("healthy".to_string())
/// }
///
/// // Subscription with no input
/// async fn time_stream(
///     _ctx: Context<AppContext>,
///     sub_ctx: SubscriptionContext,
///     _: NoInput,
/// ) -> RpcResult<EventStream<String>> {
///     // ...
/// }
/// ```
///
/// # Frontend Usage
///
/// Both of these work:
/// ```typescript
/// await rpc.health();           // sends null
/// await rpc.health({});         // sends {}
/// await subscribe('stream.time', {});
/// await subscribe('stream.time', null);
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NoInput;

impl Serialize for NoInput {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_none()
    }
}

impl<'de> Deserialize<'de> for NoInput {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct NoInputVisitor;

        impl<'de> serde::de::Visitor<'de> for NoInputVisitor {
            type Value = NoInput;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("null, unit, or empty object")
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(NoInput)
            }

            fn visit_none<E>(self) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(NoInput)
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                // Consume any fields in the map (ignore them)
                while map
                    .next_entry::<serde::de::IgnoredAny, serde::de::IgnoredAny>()?
                    .is_some()
                {}
                Ok(NoInput)
            }
        }

        deserializer.deserialize_any(NoInputVisitor)
    }
}

// =============================================================================
// Paginated Response
// =============================================================================

/// Paginated response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginatedResponse<T> {
    /// Items in the current page
    pub data: Vec<T>,
    /// Total number of items
    pub total: u32,
    /// Current page number (1-indexed)
    pub page: u32,
    /// Total number of pages
    pub total_pages: u32,
}

impl<T> PaginatedResponse<T> {
    /// Create a new paginated response
    pub fn new(data: Vec<T>, total: u32, page: u32, limit: u32) -> Self {
        let total_pages = if limit > 0 { total.div_ceil(limit) } else { 1 };
        Self {
            data,
            total,
            page,
            total_pages,
        }
    }

    /// Check if there's a next page
    pub fn has_next(&self) -> bool {
        self.page < self.total_pages
    }

    /// Check if there's a previous page
    pub fn has_prev(&self) -> bool {
        self.page > 1
    }
}

/// Success response for operations without data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessResponse {
    /// Whether the operation succeeded
    pub success: bool,
    /// Optional message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl SuccessResponse {
    /// Create a success response with a message
    pub fn ok(message: impl Into<String>) -> Self {
        Self {
            success: true,
            message: Some(message.into()),
        }
    }

    /// Create a simple success response
    pub fn success() -> Self {
        Self {
            success: true,
            message: None,
        }
    }

    /// Create a failure response
    pub fn fail(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: Some(message.into()),
        }
    }
}

impl Default for SuccessResponse {
    fn default() -> Self {
        Self::success()
    }
}

/// Pagination input parameters
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PaginationInput {
    /// Page number (1-indexed)
    pub page: Option<u32>,
    /// Items per page
    pub limit: Option<u32>,
}

impl PaginationInput {
    /// Get page number (defaults to 1)
    pub fn page(&self) -> u32 {
        self.page.unwrap_or(1).max(1)
    }

    /// Get limit (defaults to 10, max 100)
    pub fn limit(&self) -> u32 {
        self.limit.unwrap_or(10).clamp(1, 100)
    }

    /// Get offset for database queries
    pub fn offset(&self) -> u32 {
        (self.page() - 1) * self.limit()
    }

    /// Create pagination with specific values
    pub fn new(page: u32, limit: u32) -> Self {
        Self {
            page: Some(page),
            limit: Some(limit),
        }
    }
}
