use crate::subscription::*;

#[test]
fn test_backpressure_strategy_default() {
    assert_eq!(
        BackpressureStrategy::default(),
        BackpressureStrategy::DropOldest
    );
}

#[test]
fn test_backpressure_strategy_description() {
    let strategies = vec![
        BackpressureStrategy::DropOldest,
        BackpressureStrategy::DropNewest,
        BackpressureStrategy::Error,
    ];

    for strategy in strategies {
        assert!(!strategy.description().is_empty());
    }
}

#[tokio::test]
async fn test_event_publisher_with_strategy() {
    let publisher =
        EventPublisher::<String>::with_strategy(Capacity::Small, BackpressureStrategy::DropNewest);

    assert_eq!(publisher.strategy(), BackpressureStrategy::DropNewest);
    assert_eq!(publisher.capacity(), Capacity::Small);
}

#[tokio::test]
async fn test_event_publisher_with_capacity() {
    let publisher = EventPublisher::<String>::with_capacity(Capacity::Large);

    assert_eq!(publisher.capacity(), Capacity::Large);
    assert_eq!(publisher.strategy(), BackpressureStrategy::default());
}

#[tokio::test]
async fn test_event_publisher_metrics() {
    let publisher = EventPublisher::<String>::new(32);
    let _subscriber = publisher.subscribe();

    let result = publisher.publish_data("test".to_string());
    assert!(result.is_published());

    let metrics = publisher.metrics();
    let snapshot = metrics.snapshot();
    assert_eq!(snapshot.published, 1);
}

#[tokio::test]
async fn test_batch_publish_all_success() {
    let publisher = EventPublisher::<String>::new(32);
    let _subscriber = publisher.subscribe();

    let events = vec![
        Event::new("msg1".to_string()),
        Event::new("msg2".to_string()),
        Event::new("msg3".to_string()),
    ];

    let result = publisher.publish_batch(events);

    assert_eq!(result.success_count, 3);
    assert_eq!(result.failure_count, 0);
    assert!(result.is_complete_success());
    assert!(!result.is_complete_failure());
    assert!(!result.is_partial_success());
    assert_eq!(result.success_rate(), 1.0);
}

#[tokio::test]
async fn test_batch_publish_no_subscribers() {
    let publisher = EventPublisher::<String>::new(32);

    let events = vec![
        Event::new("msg1".to_string()),
        Event::new("msg2".to_string()),
    ];

    let result = publisher.publish_batch(events);

    assert_eq!(result.success_count, 0);
    assert_eq!(result.failure_count, 2);
    assert!(result.is_complete_failure());
    assert!(!result.is_complete_success());
    assert_eq!(result.success_rate(), 0.0);
}

#[tokio::test]
async fn test_batch_publish_empty() {
    let publisher = EventPublisher::<String>::new(32);
    let _subscriber = publisher.subscribe();

    let events = vec![];
    let result = publisher.publish_batch(events);

    assert_eq!(result.success_count, 0);
    assert_eq!(result.failure_count, 0);
    assert_eq!(result.total_count(), 0);
}

#[tokio::test]
async fn test_batch_publish_metrics() {
    let publisher = EventPublisher::<String>::new(32);
    let _subscriber = publisher.subscribe();

    let events = vec![
        Event::new("msg1".to_string()),
        Event::new("msg2".to_string()),
        Event::new("msg3".to_string()),
    ];

    publisher.publish_batch(events);

    let metrics = publisher.metrics();
    let snapshot = metrics.snapshot();

    // Should have 3 individual publishes + 3 batch_published
    assert_eq!(snapshot.published, 3);
    assert_eq!(snapshot.batch_published, 3);
}

#[test]
fn test_batch_publish_result_methods() {
    let result = BatchPublishResult::new(7, 3, 5);

    assert_eq!(result.total_count(), 10);
    assert_eq!(result.success_rate(), 0.7);
    assert!(result.is_partial_success());
    assert!(!result.is_complete_success());
    assert!(!result.is_complete_failure());
}

#[tokio::test]
async fn test_channel_publisher_with_capacity() {
    let publisher = ChannelPublisher::<String>::with_capacity(Capacity::Large);

    let _sub = publisher.subscribe("test-channel");
    let result = publisher.publish_data("test-channel", "message".to_string());

    assert!(result.is_ok());
    assert!(result.unwrap().is_published());
}

#[tokio::test]
async fn test_event_publisher_default_uses_medium_capacity() {
    let publisher = EventPublisher::<String>::default();
    assert_eq!(publisher.capacity(), Capacity::Medium);
}

#[tokio::test]
async fn test_channel_publisher_default_uses_medium_capacity() {
    let publisher = ChannelPublisher::<String>::default();
    // We can't directly check capacity, but we can verify it works
    let _sub = publisher.subscribe("test");
    assert_eq!(publisher.channels().len(), 1);
}

#[tokio::test]
async fn test_subscriber_lag_tracking() {
    let publisher = EventPublisher::<String>::new(2); // Very small capacity
    let mut subscriber = publisher.subscribe();

    // Fill the channel beyond capacity to cause lag
    for i in 0..10 {
        publisher.publish_data(format!("msg{}", i));
    }

    // Receive one message - should trigger lag detection
    let _ = subscriber.recv().await;

    // Check that lag was tracked
    let lag = subscriber.lag_count();
    assert!(lag > 0, "Expected lag to be tracked, got {}", lag);
}
