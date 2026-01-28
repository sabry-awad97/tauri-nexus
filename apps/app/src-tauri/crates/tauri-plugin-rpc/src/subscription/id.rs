//! Subscription ID types and utilities

use crate::subscription::errors::ParseError;
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// A unique, time-ordered subscription identifier based on UUID v7.
///
/// UUID v7 provides:
/// - Time-ordered IDs (sortable by creation time)
/// - Cryptographically random bits for uniqueness
/// - Standard UUID format for interoperability
///
/// # Example
/// ```rust,ignore
/// let id = SubscriptionId::new();
/// println!("Subscription: {}", id); // sub_01234567-89ab-7cde-8f01-234567890abc
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SubscriptionId(Uuid);

impl SubscriptionId {
    /// Create a new subscription ID using UUID v7.
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    /// Create a subscription ID from an existing UUID.
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Get the underlying UUID.
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }

    /// Convert to the underlying UUID.
    pub fn into_uuid(self) -> Uuid {
        self.0
    }

    /// Parse a subscription ID from a string.
    ///
    /// This method requires the "sub_" prefix for consistency.
    /// Use `parse_lenient()` if you need to accept both formats.
    ///
    /// # Example
    /// ```rust,ignore
    /// // Valid - with prefix
    /// let id = SubscriptionId::parse("sub_01234567-89ab-7cde-8f01-234567890abc")?;
    ///
    /// // Invalid - without prefix
    /// let result = SubscriptionId::parse("01234567-89ab-7cde-8f01-234567890abc");
    /// assert!(result.is_err());
    /// ```
    pub fn parse(s: &str) -> Result<Self, ParseError> {
        if let Some(uuid_str) = s.strip_prefix("sub_") {
            Uuid::parse_str(uuid_str)
                .map(Self)
                .map_err(ParseError::InvalidUuid)
        } else {
            Err(ParseError::MissingPrefix)
        }
    }

    /// Parse a subscription ID from a string, accepting both formats.
    ///
    /// This lenient version accepts:
    /// - With prefix: "sub_01234567-89ab-7cde-8f01-234567890abc"
    /// - Without prefix: "01234567-89ab-7cde-8f01-234567890abc"
    ///
    /// Use this for backward compatibility when migrating existing code.
    ///
    /// # Example
    /// ```rust,ignore
    /// // Both formats work
    /// let id1 = SubscriptionId::parse_lenient("sub_01234567-89ab-7cde-8f01-234567890abc")?;
    /// let id2 = SubscriptionId::parse_lenient("01234567-89ab-7cde-8f01-234567890abc")?;
    /// ```
    pub fn parse_lenient(s: &str) -> Result<Self, ParseError> {
        let uuid_str = s.strip_prefix("sub_").unwrap_or(s);
        Uuid::parse_str(uuid_str)
            .map(Self)
            .map_err(ParseError::InvalidUuid)
    }
}

impl Default for SubscriptionId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for SubscriptionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "sub_{}", self.0)
    }
}

impl From<Uuid> for SubscriptionId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<SubscriptionId> for String {
    fn from(id: SubscriptionId) -> Self {
        id.to_string()
    }
}

/// Generate a unique subscription ID using UUID v7.
///
/// This is a convenience function that creates a new [`SubscriptionId`].
///
/// # Example
/// ```rust,ignore
/// let id = generate_subscription_id();
/// assert!(id.to_string().starts_with("sub_"));
/// ```
pub fn generate_subscription_id() -> SubscriptionId {
    SubscriptionId::new()
}
