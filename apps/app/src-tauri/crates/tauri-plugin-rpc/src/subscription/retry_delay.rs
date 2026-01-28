// Validated retry delay type

use crate::subscription::ValidationError;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// A validated retry delay for subscription events.
///
/// This type ensures retry delays are within a reasonable range (1ms to 1 hour)
/// to prevent misconfiguration.
///
/// # Example
/// ```rust,ignore
/// use std::time::Duration;
/// use tauri_plugin_rpc::subscription::RetryDelay;
///
/// // Create from milliseconds
/// let delay = RetryDelay::from_millis(5000)?; // 5 seconds
///
/// // Create from Duration
/// let delay = RetryDelay::new(Duration::from_secs(10))?;
///
/// // Out of range values are rejected
/// let result = RetryDelay::from_millis(5_000_000); // 5000 seconds
/// assert!(result.is_err());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RetryDelay(Duration);

impl RetryDelay {
    /// Minimum allowed retry delay (1 millisecond)
    pub const MIN: Duration = Duration::from_millis(1);

    /// Maximum allowed retry delay (1 hour)
    pub const MAX: Duration = Duration::from_secs(3600);

    /// Create a new retry delay with validation.
    ///
    /// # Errors
    /// Returns `ValidationError::RetryDelayOutOfRange` if the duration is
    /// outside the valid range [1ms, 1 hour].
    ///
    /// # Example
    /// ```rust,ignore
    /// let delay = RetryDelay::new(Duration::from_secs(30))?;
    /// ```
    pub fn new(duration: Duration) -> Result<Self, ValidationError> {
        if duration < Self::MIN || duration > Self::MAX {
            Err(ValidationError::RetryDelayOutOfRange {
                min: Self::MIN.as_millis() as u64,
                max: Self::MAX.as_millis() as u64,
                actual: duration.as_millis() as u64,
            })
        } else {
            Ok(Self(duration))
        }
    }

    /// Create a retry delay from milliseconds.
    ///
    /// # Errors
    /// Returns `ValidationError::RetryDelayOutOfRange` if the value is
    /// outside the valid range [1ms, 3600000ms].
    ///
    /// # Example
    /// ```rust,ignore
    /// let delay = RetryDelay::from_millis(5000)?; // 5 seconds
    /// ```
    pub fn from_millis(ms: u64) -> Result<Self, ValidationError> {
        Self::new(Duration::from_millis(ms))
    }

    /// Get the retry delay as a Duration.
    pub fn as_duration(&self) -> Duration {
        self.0
    }

    /// Get the retry delay in milliseconds.
    pub fn as_millis(&self) -> u64 {
        self.0.as_millis() as u64
    }

    /// Get the retry delay in seconds.
    pub fn as_secs(&self) -> u64 {
        self.0.as_secs()
    }
}

impl From<RetryDelay> for Duration {
    fn from(delay: RetryDelay) -> Self {
        delay.0
    }
}

impl TryFrom<Duration> for RetryDelay {
    type Error = ValidationError;

    fn try_from(duration: Duration) -> Result<Self, Self::Error> {
        Self::new(duration)
    }
}

impl TryFrom<u64> for RetryDelay {
    type Error = ValidationError;

    fn try_from(ms: u64) -> Result<Self, Self::Error> {
        Self::from_millis(ms)
    }
}

// Serialize as milliseconds
impl Serialize for RetryDelay {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_u64(self.as_millis())
    }
}

// Deserialize from milliseconds with validation
impl<'de> Deserialize<'de> for RetryDelay {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let ms = u64::deserialize(deserializer)?;
        Self::from_millis(ms).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_delay_valid() {
        let delay = RetryDelay::from_millis(5000).unwrap();
        assert_eq!(delay.as_millis(), 5000);
        assert_eq!(delay.as_secs(), 5);
    }

    #[test]
    fn test_retry_delay_min() {
        let delay = RetryDelay::from_millis(1).unwrap();
        assert_eq!(delay.as_millis(), 1);
    }

    #[test]
    fn test_retry_delay_max() {
        let delay = RetryDelay::from_millis(3_600_000).unwrap();
        assert_eq!(delay.as_secs(), 3600);
    }

    #[test]
    fn test_retry_delay_too_small() {
        let result = RetryDelay::from_millis(0);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ValidationError::RetryDelayOutOfRange { .. }
        ));
    }

    #[test]
    fn test_retry_delay_too_large() {
        let result = RetryDelay::from_millis(5_000_000);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ValidationError::RetryDelayOutOfRange { .. }
        ));
    }

    #[test]
    fn test_retry_delay_from_duration() {
        let duration = Duration::from_secs(30);
        let delay = RetryDelay::new(duration).unwrap();
        assert_eq!(delay.as_duration(), duration);
    }

    #[test]
    fn test_retry_delay_try_from_duration() {
        let duration = Duration::from_secs(10);
        let delay: RetryDelay = duration.try_into().unwrap();
        assert_eq!(delay.as_secs(), 10);
    }

    #[test]
    fn test_retry_delay_try_from_u64() {
        let delay: RetryDelay = 1000u64.try_into().unwrap();
        assert_eq!(delay.as_millis(), 1000);
    }

    #[test]
    fn test_retry_delay_serialize() {
        let delay = RetryDelay::from_millis(5000).unwrap();
        let json = serde_json::to_string(&delay).unwrap();
        assert_eq!(json, "5000");
    }

    #[test]
    fn test_retry_delay_deserialize() {
        let delay: RetryDelay = serde_json::from_str("5000").unwrap();
        assert_eq!(delay.as_millis(), 5000);
    }

    #[test]
    fn test_retry_delay_deserialize_invalid() {
        let result: Result<RetryDelay, _> = serde_json::from_str("5000000");
        assert!(result.is_err());
    }

    #[test]
    fn test_retry_delay_ordering() {
        let delay1 = RetryDelay::from_millis(1000).unwrap();
        let delay2 = RetryDelay::from_millis(2000).unwrap();
        assert!(delay1 < delay2);
    }
}
