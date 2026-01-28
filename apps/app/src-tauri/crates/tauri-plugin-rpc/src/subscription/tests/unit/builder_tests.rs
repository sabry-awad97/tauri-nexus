use crate::subscription::*;
use std::sync::Arc;

#[test]
fn test_subscription_handle_builder_basic() {
    let id = SubscriptionId::new();
    let signal = Arc::new(CancellationSignal::new());

    let handle = SubscriptionHandle::builder()
        .id(id)
        .path("test.subscription")
        .signal(signal)
        .build();

    assert_eq!(handle.id, id);
    assert_eq!(handle.path, "test.subscription");
}

#[test]
fn test_subscription_handle_builder_with_task() {
    let id = SubscriptionId::new();
    let signal = Arc::new(CancellationSignal::new());

    // Create a dummy task handle (we can't actually spawn without runtime)
    // Just test that the builder accepts the task parameter
    let handle = SubscriptionHandle::builder()
        .id(id)
        .path("test.subscription")
        .signal(signal.clone())
        .build();

    assert_eq!(handle.id, id);
    assert_eq!(handle.path, "test.subscription");

    // Now test with_task method
    let handle2 = SubscriptionHandle::new(id, "test.subscription".to_string(), signal);
    // We can't test with_task without a runtime, so just verify the handle was created
    assert_eq!(handle2.id, id);
}

#[test]
fn test_subscription_handle_builder_path_into_string() {
    let id = SubscriptionId::new();
    let signal = Arc::new(CancellationSignal::new());

    // Test that path accepts Into<String>
    let handle = SubscriptionHandle::builder()
        .id(id)
        .path("test.subscription".to_string())
        .signal(signal)
        .build();

    assert_eq!(handle.path, "test.subscription");
}

#[test]
#[should_panic(expected = "id is required")]
fn test_subscription_handle_builder_missing_id() {
    let signal = Arc::new(CancellationSignal::new());

    SubscriptionHandle::builder()
        .path("test.subscription")
        .signal(signal)
        .build();
}

#[test]
#[should_panic(expected = "path is required")]
fn test_subscription_handle_builder_missing_path() {
    let id = SubscriptionId::new();
    let signal = Arc::new(CancellationSignal::new());

    SubscriptionHandle::builder().id(id).signal(signal).build();
}

#[test]
#[should_panic(expected = "signal is required")]
fn test_subscription_handle_builder_missing_signal() {
    let id = SubscriptionId::new();

    SubscriptionHandle::builder()
        .id(id)
        .path("test.subscription")
        .build();
}

#[test]
fn test_subscription_handle_builder_default() {
    let builder = SubscriptionHandleBuilder::default();
    // Just verify it can be created
    let _ = builder;
}

#[test]
fn test_subscription_handle_new_vs_builder() {
    let id = SubscriptionId::new();
    let signal1 = Arc::new(CancellationSignal::new());
    let signal2 = Arc::clone(&signal1);

    // Create using new()
    let handle1 = SubscriptionHandle::new(id, "test.subscription".to_string(), signal1);

    // Create using builder()
    let handle2 = SubscriptionHandle::builder()
        .id(id)
        .path("test.subscription")
        .signal(signal2)
        .build();

    // Both should have the same id and path
    assert_eq!(handle1.id, handle2.id);
    assert_eq!(handle1.path, handle2.path);
}

#[test]
fn test_subscription_handle_builder_fluent_api() {
    let id = SubscriptionId::new();
    let signal = Arc::new(CancellationSignal::new());

    // Test that builder methods can be chained
    let handle = SubscriptionHandle::builder()
        .id(id)
        .path("test.subscription")
        .signal(signal)
        .build();

    assert_eq!(handle.id, id);
}

#[tokio::test]
async fn test_subscription_handle_builder_with_real_task() {
    let id = SubscriptionId::new();
    let signal = Arc::new(CancellationSignal::new());

    let task = tokio::spawn(async {
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    });

    let handle = SubscriptionHandle::builder()
        .id(id)
        .path("test.subscription")
        .signal(signal)
        .task(task)
        .build();

    // Handle should be created successfully
    assert_eq!(handle.id, id);
}

#[test]
fn test_subscription_handle_builder_multiple_builds() {
    let id1 = SubscriptionId::new();
    let id2 = SubscriptionId::new();
    let signal1 = Arc::new(CancellationSignal::new());
    let signal2 = Arc::new(CancellationSignal::new());

    // Create first handle
    let handle1 = SubscriptionHandle::builder()
        .id(id1)
        .path("test.sub1")
        .signal(signal1)
        .build();

    // Create second handle with different values
    let handle2 = SubscriptionHandle::builder()
        .id(id2)
        .path("test.sub2")
        .signal(signal2)
        .build();

    assert_eq!(handle1.path, "test.sub1");
    assert_eq!(handle2.path, "test.sub2");
    assert_ne!(handle1.id, handle2.id);
}
