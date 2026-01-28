// Configuration types for subscription module

use std::time::Duration;

/// Configuration for subscription manager operations
#[derive(Debug, Clone)]
pub struct ManagerConfig {
    /// Timeout for subscribe operations
    pub subscribe_timeout: Duration,
    /// Timeout for unsubscribe operations
    pub unsubscribe_timeout: Duration,
    /// Interval for periodic cleanup of completed tasks
    pub cleanup_interval: Duration,
}

impl Default for ManagerConfig {
    fn default() -> Self {
        Self {
            subscribe_timeout: Duration::from_secs(30),
            unsubscribe_timeout: Duration::from_secs(5),
            cleanup_interval: Duration::from_secs(60),
        }
    }
}

impl ManagerConfig {
    /// Create a new manager configuration with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the subscribe timeout
    pub fn with_subscribe_timeout(mut self, timeout: Duration) -> Self {
        self.subscribe_timeout = timeout;
        self
    }

    /// Set the unsubscribe timeout
    pub fn with_unsubscribe_timeout(mut self, timeout: Duration) -> Self {
        self.unsubscribe_timeout = timeout;
        self
    }

    /// Set the cleanup interval
    pub fn with_cleanup_interval(mut self, interval: Duration) -> Self {
        self.cleanup_interval = interval;
        self
    }
}

/// Capacity presets for event publishers
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Capacity {
    /// Small capacity (32 events) - for low-frequency events
    Small,
    /// Medium capacity (256 events) - default for most use cases
    #[default]
    Medium,
    /// Large capacity (1024 events) - for high-frequency events
    Large,
    /// Extra large capacity (4096 events) - for very high throughput
    XLarge,
    /// Custom capacity
    Custom(usize),
}

impl Capacity {
    /// Get the numeric capacity value
    pub fn value(&self) -> usize {
        match self {
            Capacity::Small => 32,
            Capacity::Medium => 256,
            Capacity::Large => 1024,
            Capacity::XLarge => 4096,
            Capacity::Custom(size) => *size,
        }
    }
}

impl From<usize> for Capacity {
    fn from(size: usize) -> Self {
        match size {
            32 => Capacity::Small,
            256 => Capacity::Medium,
            1024 => Capacity::Large,
            4096 => Capacity::XLarge,
            _ => Capacity::Custom(size),
        }
    }
}

impl From<Capacity> for usize {
    fn from(capacity: Capacity) -> Self {
        capacity.value()
    }
}

/// Comprehensive subscription configuration
#[derive(Debug, Clone)]
pub struct SubscriptionConfig {
    /// Manager configuration
    pub manager: ManagerConfig,
    /// Default publisher capacity
    pub publisher_capacity: Capacity,
    /// Enable metrics collection
    pub enable_metrics: bool,
}

impl Default for SubscriptionConfig {
    fn default() -> Self {
        Self {
            manager: ManagerConfig::default(),
            publisher_capacity: Capacity::Medium,
            enable_metrics: true,
        }
    }
}

impl SubscriptionConfig {
    /// Create a new subscription configuration with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the manager configuration
    pub fn with_manager_config(mut self, config: ManagerConfig) -> Self {
        self.manager = config;
        self
    }

    /// Set the publisher capacity
    pub fn with_publisher_capacity(mut self, capacity: Capacity) -> Self {
        self.publisher_capacity = capacity;
        self
    }

    /// Enable or disable metrics collection
    pub fn with_metrics(mut self, enable: bool) -> Self {
        self.enable_metrics = enable;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manager_config_default() {
        let config = ManagerConfig::default();
        assert_eq!(config.subscribe_timeout, Duration::from_secs(30));
        assert_eq!(config.unsubscribe_timeout, Duration::from_secs(5));
        assert_eq!(config.cleanup_interval, Duration::from_secs(60));
    }

    #[test]
    fn test_manager_config_builder() {
        let config = ManagerConfig::new()
            .with_subscribe_timeout(Duration::from_secs(10))
            .with_unsubscribe_timeout(Duration::from_secs(2))
            .with_cleanup_interval(Duration::from_secs(30));

        assert_eq!(config.subscribe_timeout, Duration::from_secs(10));
        assert_eq!(config.unsubscribe_timeout, Duration::from_secs(2));
        assert_eq!(config.cleanup_interval, Duration::from_secs(30));
    }

    #[test]
    fn test_capacity_values() {
        assert_eq!(Capacity::Small.value(), 32);
        assert_eq!(Capacity::Medium.value(), 256);
        assert_eq!(Capacity::Large.value(), 1024);
        assert_eq!(Capacity::XLarge.value(), 4096);
        assert_eq!(Capacity::Custom(512).value(), 512);
    }

    #[test]
    fn test_capacity_from_usize() {
        assert_eq!(Capacity::from(32), Capacity::Small);
        assert_eq!(Capacity::from(256), Capacity::Medium);
        assert_eq!(Capacity::from(1024), Capacity::Large);
        assert_eq!(Capacity::from(4096), Capacity::XLarge);
        assert_eq!(Capacity::from(512), Capacity::Custom(512));
    }

    #[test]
    fn test_capacity_to_usize() {
        let size: usize = Capacity::Medium.into();
        assert_eq!(size, 256);
    }

    #[test]
    fn test_subscription_config_default() {
        let config = SubscriptionConfig::default();
        assert_eq!(config.publisher_capacity, Capacity::Medium);
        assert!(config.enable_metrics);
    }

    #[test]
    fn test_subscription_config_builder() {
        let config = SubscriptionConfig::new()
            .with_publisher_capacity(Capacity::Large)
            .with_metrics(false);

        assert_eq!(config.publisher_capacity, Capacity::Large);
        assert!(!config.enable_metrics);
    }
}
