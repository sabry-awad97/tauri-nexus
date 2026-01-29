//! Error types for cache operations

/// Errors that can occur during cache operations
#[derive(Debug, thiserror::Error)]
pub enum CacheError {
    /// Failed to serialize or deserialize cache value
    #[error("Failed to serialize cache value: {0}")]
    SerializationError(String),

    /// Cache entry has expired
    #[error("Cache entry expired")]
    EntryExpired,

    /// Cache is disabled
    #[error("Cache is disabled")]
    CacheDisabled,

    /// Invalid pattern provided
    #[error("Invalid pattern: {0}")]
    InvalidPattern(String),

    /// Failed to acquire lock
    #[error("Lock acquisition failed: {0}")]
    LockError(String),
}

/// Result type for cache operations
pub type CacheResult<T> = Result<T, CacheError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_error_serialization_error_display() {
        let error = CacheError::SerializationError("invalid JSON".to_string());
        assert_eq!(
            error.to_string(),
            "Failed to serialize cache value: invalid JSON"
        );
    }

    #[test]
    fn test_cache_error_entry_expired_display() {
        let error = CacheError::EntryExpired;
        assert_eq!(error.to_string(), "Cache entry expired");
    }

    #[test]
    fn test_cache_error_cache_disabled_display() {
        let error = CacheError::CacheDisabled;
        assert_eq!(error.to_string(), "Cache is disabled");
    }

    #[test]
    fn test_cache_error_invalid_pattern_display() {
        let error = CacheError::InvalidPattern("bad.*pattern".to_string());
        assert_eq!(error.to_string(), "Invalid pattern: bad.*pattern");
    }

    #[test]
    fn test_cache_error_lock_error_display() {
        let error = CacheError::LockError("timeout".to_string());
        assert_eq!(error.to_string(), "Lock acquisition failed: timeout");
    }

    #[test]
    fn test_cache_error_is_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}
        assert_send::<CacheError>();
        assert_sync::<CacheError>();
    }

    #[test]
    fn test_cache_result_ok() {
        // Test that CacheResult<T> works correctly with Ok values
        fn returns_ok() -> CacheResult<i32> {
            Ok(42)
        }
        let result = returns_ok();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_cache_result_err() {
        // Test that CacheResult<T> works correctly with Err values
        fn returns_err() -> CacheResult<i32> {
            Err(CacheError::CacheDisabled)
        }
        let result = returns_err();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CacheError::CacheDisabled));
    }

    #[test]
    fn test_cache_error_debug_format() {
        let error = CacheError::SerializationError("test".to_string());
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("SerializationError"));
        assert!(debug_str.contains("test"));
    }
}
