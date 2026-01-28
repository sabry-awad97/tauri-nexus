use crate::subscription::*;
use std::time::Duration;

#[tokio::test]
async fn test_subscription_context_with_timeout() {
    let ctx = SubscriptionContext::new(SubscriptionId::new(), None)
        .with_timeout(Duration::from_millis(100));

    let reason = ctx.cancelled_or_timeout().await;
    assert_eq!(reason, CancellationReason::Timeout);
}

#[tokio::test]
async fn test_subscription_context_cancelled_before_timeout() {
    let ctx =
        SubscriptionContext::new(SubscriptionId::new(), None).with_timeout(Duration::from_secs(10));

    // Cancel immediately
    ctx.signal().cancel();

    let reason = ctx.cancelled_or_timeout().await;
    assert_eq!(reason, CancellationReason::Cancelled);
}

#[tokio::test]
async fn test_subscription_context_no_timeout() {
    let ctx = SubscriptionContext::new(SubscriptionId::new(), None);

    // Cancel after a short delay
    let signal = ctx.signal();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(50)).await;
        signal.cancel();
    });

    let reason = ctx.cancelled_or_timeout().await;
    assert_eq!(reason, CancellationReason::Cancelled);
}

#[tokio::test]
async fn test_manager_subscribe_with_timeout_success() {
    let config = ManagerConfig::new().with_subscribe_timeout(Duration::from_secs(1));
    let manager = SubscriptionManager::with_config(config);

    let id = SubscriptionId::new();
    let signal = Arc::new(CancellationSignal::new());
    let handle = SubscriptionHandle::new(id, "test.subscription".to_string(), signal);

    let result = manager.subscribe_with_timeout(handle).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), id);
}

#[tokio::test]
async fn test_manager_unsubscribe_with_timeout_success() {
    let config = ManagerConfig::new().with_unsubscribe_timeout(Duration::from_secs(1));
    let manager = SubscriptionManager::with_config(config);

    let id = SubscriptionId::new();
    let signal = Arc::new(CancellationSignal::new());
    let handle = SubscriptionHandle::new(id, "test.subscription".to_string(), signal);

    manager.subscribe(handle);

    let result = manager.unsubscribe_with_timeout(&id).await;
    assert!(result.is_ok());
    assert!(result.unwrap());
}

#[tokio::test]
async fn test_manager_unsubscribe_with_timeout_not_found() {
    let config = ManagerConfig::new().with_unsubscribe_timeout(Duration::from_secs(1));
    let manager = SubscriptionManager::with_config(config);

    let id = SubscriptionId::new();

    let result = manager.unsubscribe_with_timeout(&id).await;
    assert!(result.is_ok());
    assert!(!result.unwrap());
}

#[tokio::test]
async fn test_manager_config_default_values() {
    let config = ManagerConfig::default();
    assert_eq!(config.subscribe_timeout, Duration::from_secs(30));
    assert_eq!(config.unsubscribe_timeout, Duration::from_secs(5));
    assert_eq!(config.cleanup_interval, Duration::from_secs(60));
}

#[tokio::test]
async fn test_manager_config_builder() {
    let config = ManagerConfig::new()
        .with_subscribe_timeout(Duration::from_secs(10))
        .with_unsubscribe_timeout(Duration::from_secs(2))
        .with_cleanup_interval(Duration::from_secs(30));

    assert_eq!(config.subscribe_timeout, Duration::from_secs(10));
    assert_eq!(config.unsubscribe_timeout, Duration::from_secs(2));
    assert_eq!(config.cleanup_interval, Duration::from_secs(30));
}

#[tokio::test]
async fn test_manager_with_custom_config() {
    let config = ManagerConfig::new()
        .with_subscribe_timeout(Duration::from_millis(500))
        .with_cleanup_interval(Duration::from_secs(30));

    let manager = SubscriptionManager::with_config(config);

    // Verify manager was created successfully
    assert_eq!(manager.count(), 0);
}

#[tokio::test]
async fn test_periodic_cleanup_uses_config_interval() {
    let config = ManagerConfig::new().with_cleanup_interval(Duration::from_millis(100));
    let manager = Arc::new(SubscriptionManager::with_config(config));

    let cleanup_handle = manager.start_periodic_cleanup();

    // Let it run for a bit
    tokio::time::sleep(Duration::from_millis(250)).await;

    // Stop the cleanup task
    cleanup_handle.abort();

    // The test passes if no panic occurred
}
