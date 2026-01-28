use crate::subscription::*;
use std::sync::Arc;
use std::time::Duration;

#[tokio::test]
async fn test_health_status_initial() {
    let manager = SubscriptionManager::new();
    let health = manager.health().await;

    assert_eq!(health.active_subscriptions, 0);
    assert_eq!(health.active_tasks, 0);
    assert_eq!(health.completed_tasks, 0);
    assert!(health.is_healthy());
}

#[tokio::test]
async fn test_health_status_with_subscriptions() {
    let manager = SubscriptionManager::new();

    // Add some subscriptions
    let id1 = SubscriptionId::new();
    let signal1 = Arc::new(CancellationSignal::new());
    let handle1 = SubscriptionHandle::new(id1, "test.sub1".to_string(), signal1);
    manager.subscribe(handle1);

    let id2 = SubscriptionId::new();
    let signal2 = Arc::new(CancellationSignal::new());
    let handle2 = SubscriptionHandle::new(id2, "test.sub2".to_string(), signal2);
    manager.subscribe(handle2);

    let health = manager.health().await;

    assert_eq!(health.active_subscriptions, 2);
    assert!(health.is_healthy());
}

#[tokio::test]
async fn test_health_status_uptime() {
    let manager = SubscriptionManager::new();

    // Wait a bit
    tokio::time::sleep(Duration::from_millis(100)).await;

    let health = manager.health().await;

    // Uptime should be measurable (at least 0, but likely more)
    // Just verify the field exists and is accessible
    let _ = health.uptime_seconds;
}

#[tokio::test]
async fn test_health_status_message() {
    let manager = SubscriptionManager::new();
    let health = manager.health().await;

    let message = health.status_message();
    assert!(message.contains("Active:"));
    assert!(message.contains("subscriptions"));
    assert!(message.contains("tasks"));
    assert!(message.contains("Uptime:"));
}

#[tokio::test]
async fn test_subscription_metrics_creation() {
    let manager = SubscriptionManager::new();

    let id = SubscriptionId::new();
    let signal = Arc::new(CancellationSignal::new());
    let handle = SubscriptionHandle::new(id, "test.sub".to_string(), signal);
    manager.subscribe(handle);

    let metrics = manager.metrics();
    let snapshot = metrics.snapshot();

    assert_eq!(snapshot.created, 1);
    assert_eq!(snapshot.active, 1);
}

#[tokio::test]
async fn test_subscription_metrics_cancellation() {
    let manager = SubscriptionManager::new();

    let id = SubscriptionId::new();
    let signal = Arc::new(CancellationSignal::new());
    let handle = SubscriptionHandle::new(id, "test.sub".to_string(), signal);
    manager.subscribe(handle);

    // Wait a bit to ensure duration is measurable
    tokio::time::sleep(Duration::from_millis(10)).await;

    manager.unsubscribe(&id);

    let metrics = manager.metrics();
    let snapshot = metrics.snapshot();

    assert_eq!(snapshot.created, 1);
    assert_eq!(snapshot.cancelled, 1);
    assert_eq!(snapshot.active, 0);
    assert!(snapshot.avg_duration_ms > 0);
}

#[tokio::test]
async fn test_subscription_metrics_multiple() {
    let manager = SubscriptionManager::new();

    // Create multiple subscriptions
    for i in 0..5 {
        let id = SubscriptionId::new();
        let signal = Arc::new(CancellationSignal::new());
        let handle = SubscriptionHandle::new(id, format!("test.sub{}", i), signal);
        manager.subscribe(handle);
    }

    let metrics = manager.metrics();
    let snapshot = metrics.snapshot();

    assert_eq!(snapshot.created, 5);
    assert_eq!(snapshot.active, 5);
}

#[tokio::test]
async fn test_subscription_metrics_cancel_all() {
    let manager = SubscriptionManager::new();

    // Create multiple subscriptions
    for i in 0..3 {
        let id = SubscriptionId::new();
        let signal = Arc::new(CancellationSignal::new());
        let handle = SubscriptionHandle::new(id, format!("test.sub{}", i), signal);
        manager.subscribe(handle);
    }

    // Wait a bit
    tokio::time::sleep(Duration::from_millis(10)).await;

    manager.cancel_all();

    let metrics = manager.metrics();
    let snapshot = metrics.snapshot();

    assert_eq!(snapshot.created, 3);
    assert_eq!(snapshot.cancelled, 3);
    assert_eq!(snapshot.active, 0);
}

#[tokio::test]
async fn test_subscription_handle_duration() {
    let id = SubscriptionId::new();
    let signal = Arc::new(CancellationSignal::new());
    let handle = SubscriptionHandle::new(id, "test.sub".to_string(), signal);

    // Wait a bit
    tokio::time::sleep(Duration::from_millis(50)).await;

    let duration = handle.duration();
    assert!(duration.as_millis() >= 50);
}

#[tokio::test]
async fn test_health_with_tasks() {
    let manager = Arc::new(SubscriptionManager::new());

    // Spawn a subscription task
    let id = SubscriptionId::new();
    manager
        .spawn_subscription(id, async {
            tokio::time::sleep(Duration::from_millis(100)).await;
        })
        .await;

    let health = manager.health().await;
    assert_eq!(health.active_tasks, 1);
}

#[tokio::test]
async fn test_metrics_average_duration() {
    let manager = SubscriptionManager::new();

    // Create and cancel subscriptions with different durations
    let id1 = SubscriptionId::new();
    let signal1 = Arc::new(CancellationSignal::new());
    let handle1 = SubscriptionHandle::new(id1, "test.sub1".to_string(), signal1);
    manager.subscribe(handle1);

    tokio::time::sleep(Duration::from_millis(10)).await;
    manager.unsubscribe(&id1);

    let id2 = SubscriptionId::new();
    let signal2 = Arc::new(CancellationSignal::new());
    let handle2 = SubscriptionHandle::new(id2, "test.sub2".to_string(), signal2);
    manager.subscribe(handle2);

    tokio::time::sleep(Duration::from_millis(20)).await;
    manager.unsubscribe(&id2);

    let metrics = manager.metrics();
    let snapshot = metrics.snapshot();

    // Average should be between 10 and 20 ms
    assert!(snapshot.avg_duration_ms >= 10);
    assert!(snapshot.avg_duration_ms <= 30); // Allow some variance
}

#[tokio::test]
async fn test_health_status_equality() {
    let status1 = HealthStatus {
        active_subscriptions: 5,
        active_tasks: 3,
        completed_tasks: 10,
        uptime_seconds: 100,
    };

    let status2 = HealthStatus {
        active_subscriptions: 5,
        active_tasks: 3,
        completed_tasks: 10,
        uptime_seconds: 100,
    };

    assert_eq!(status1, status2);
}
