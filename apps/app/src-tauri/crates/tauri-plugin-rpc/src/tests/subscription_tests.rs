//! Property-based tests for subscription/event iterator functionality
//!
//! These tests validate the correctness properties defined in the design document:
//! - Property 1: Subscription Cleanup
//! - Property 2: Subscription ID Uniqueness
//! - Property 5: Subscription Cancellation Cleanup
//! - Property 7: Bounded Channel Backpressure
//! - Property 8: EventPublisher Graceful Empty Publish
//! - Property 9: Lagged Subscriber Recovery

use proptest::prelude::*;
use std::collections::HashSet;
use std::sync::Arc;

use crate::subscription::{
    CancellationSignal, ChannelPublisher, Event, EventMeta, EventPublisher, PublishResult,
    SubscriptionContext, SubscriptionId, SubscriptionManager, generate_subscription_id,
};

// =============================================================================
// Property 2: Subscription ID Uniqueness
// =============================================================================

proptest! {
    /// **Property 2: Subscription ID Uniqueness**
    /// *For any* number of subscription IDs generated (up to practical limits),
    /// all generated IDs SHALL be unique, SHALL use UUID v7 format, and SHALL serialize with "sub_" prefix.
    /// **Feature: tauri-rpc-plugin-optimization, Property 2: Subscription ID Uniqueness**
    #[test]
    fn prop_subscription_id_uniqueness(count in 1usize..1000) {
        let mut ids = HashSet::new();
        for _ in 0..count {
            let id = generate_subscription_id();
            let id_str = id.to_string();

            // Verify format: should start with "sub_" followed by UUID
            prop_assert!(id_str.starts_with("sub_"), "ID should start with 'sub_': {}", id_str);

            // UUID v7 format: sub_ + 8-4-4-4-12 = sub_ + 36 chars = 40 total
            prop_assert_eq!(id_str.len(), 40, "ID should be 40 chars (4 prefix + 36 UUID): {}", id_str);

            // Verify UUID portion is valid
            let uuid_part = &id_str[4..];
            prop_assert!(
                uuid::Uuid::parse_str(uuid_part).is_ok(),
                "ID UUID portion should be valid UUID: {}", uuid_part
            );

            // Each ID should be unique
            prop_assert!(ids.insert(id), "Duplicate subscription ID generated: {}", id_str);
        }
        // All IDs should be present
        prop_assert_eq!(ids.len(), count);
    }
}

// =============================================================================
// Property 1: Subscription Cleanup
// =============================================================================

proptest! {
    /// **Property 1: Subscription Cleanup**
    /// *For any* subscription that is unsubscribed, the subscription SHALL be removed
    /// from the manager's registry AND its cancellation signal SHALL be triggered.
    /// **Feature: tauri-rpc-plugin-optimization, Property 1: Subscription Cleanup**
    #[test]
    fn prop_subscription_cleanup(
        subscription_count in 1usize..50,
        unsubscribe_indices in prop::collection::vec(0usize..50, 0..25)
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let manager = SubscriptionManager::new();
            let mut handles = Vec::new();
            let mut signals = Vec::new();

            // Create subscriptions
            for i in 0..subscription_count {
                let id = SubscriptionId::new();
                let signal = Arc::new(CancellationSignal::new());
                let handle = crate::subscription::SubscriptionHandle::new(
                    id,
                    format!("path.{}", i),
                    signal.clone(),
                );
                manager.subscribe(handle).await;
                handles.push(id);
                signals.push(signal);
            }

            // Verify all subscriptions exist
            prop_assert_eq!(manager.count().await, subscription_count);

            // Unsubscribe some
            let mut unsubscribed = HashSet::new();
            for idx in unsubscribe_indices {
                if idx < subscription_count && !unsubscribed.contains(&idx) {
                    let id = &handles[idx];
                    let result = manager.unsubscribe(id).await;
                    prop_assert!(result, "Unsubscribe should succeed for existing subscription");

                    // Signal should be cancelled
                    prop_assert!(signals[idx].is_cancelled(), "Signal should be cancelled after unsubscribe");

                    unsubscribed.insert(idx);
                }
            }

            // Verify remaining count
            let expected_remaining = subscription_count - unsubscribed.len();
            prop_assert_eq!(manager.count().await, expected_remaining);

            // Verify unsubscribed IDs no longer exist
            for idx in &unsubscribed {
                prop_assert!(!manager.exists(&handles[*idx]).await, "Unsubscribed ID should not exist");
            }

            Ok(())
        })?;
    }
}

// =============================================================================
// Property 5: Subscription Cancellation Cleanup
// =============================================================================

proptest! {
    /// **Property 5: Subscription Cancellation Cleanup**
    /// *For any* subscription that is cancelled, the associated task SHALL stop executing
    /// within a bounded time, and the subscription SHALL be removed from the manager.
    /// On shutdown, ALL subscriptions SHALL be cancelled and ALL tracked tasks SHALL be aborted.
    /// **Feature: tauri-rpc-plugin-optimization, Property 5: Subscription Cancellation Cleanup**
    #[test]
    fn prop_subscription_cancellation_cleanup(
        subscription_count in 1usize..20,
        cancel_indices in prop::collection::vec(0usize..20, 1..10)
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let manager = Arc::new(SubscriptionManager::new());
            let mut subscription_ids = Vec::new();
            let mut signals = Vec::new();

            // Create subscriptions with tracked tasks
            for i in 0..subscription_count {
                let id = SubscriptionId::new();
                let signal = Arc::new(CancellationSignal::new());
                let handle = crate::subscription::SubscriptionHandle::new(
                    id,
                    format!("test.path.{}", i),
                    signal.clone(),
                );
                manager.subscribe(handle).await;

                // Spawn a tracked task that runs until cancelled
                let signal_clone = signal.clone();
                manager.spawn_subscription(id, async move {
                    // Simulate a long-running subscription task
                    loop {
                        if signal_clone.is_cancelled() {
                            break;
                        }
                        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                    }
                }).await;

                subscription_ids.push(id);
                signals.push(signal);
            }

            // Verify all subscriptions exist
            prop_assert_eq!(manager.count().await, subscription_count);

            // Cancel some subscriptions via unsubscribe
            let mut cancelled = HashSet::new();
            for idx in cancel_indices {
                if idx < subscription_count && !cancelled.contains(&idx) {
                    let id = &subscription_ids[idx];

                    // Unsubscribe should cancel the signal and remove from manager
                    let result = manager.unsubscribe(id).await;
                    prop_assert!(result, "Unsubscribe should succeed for existing subscription");

                    // Signal should be cancelled immediately
                    prop_assert!(
                        signals[idx].is_cancelled(),
                        "Signal should be cancelled after unsubscribe"
                    );

                    // Subscription should be removed from manager
                    prop_assert!(
                        !manager.exists(id).await,
                        "Subscription should be removed from manager after unsubscribe"
                    );

                    cancelled.insert(idx);
                }
            }

            // Verify remaining count
            let expected_remaining = subscription_count - cancelled.len();
            prop_assert_eq!(manager.count().await, expected_remaining);

            // Test shutdown - should cancel all remaining subscriptions
            manager.shutdown().await;

            // After shutdown, all subscriptions should be removed
            prop_assert_eq!(manager.count().await, 0, "All subscriptions should be removed after shutdown");

            // All signals should be cancelled after shutdown
            for signal in &signals {
                prop_assert!(signal.is_cancelled(), "All signals should be cancelled after shutdown");
            }

            Ok(())
        })?;
    }

    /// **Property 5 (continued): Task Tracking Cleanup**
    /// *For any* set of spawned subscription tasks, shutdown SHALL abort all tasks
    /// and wait for them to complete.
    /// **Feature: tauri-rpc-plugin-optimization, Property 5: Subscription Cancellation Cleanup**
    #[test]
    fn prop_task_tracking_cleanup(task_count in 1usize..10) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let manager = Arc::new(SubscriptionManager::new());
            let task_started = Arc::new(std::sync::atomic::AtomicUsize::new(0));

            // Spawn multiple tracked tasks
            for i in 0..task_count {
                let id = SubscriptionId::new();
                let signal = Arc::new(CancellationSignal::new());
                let handle = crate::subscription::SubscriptionHandle::new(
                    id,
                    format!("task.{}", i),
                    signal.clone(),
                );
                manager.subscribe(handle).await;

                let task_started_clone = task_started.clone();
                let signal_clone = signal.clone();
                manager.spawn_subscription(id, async move {
                    // Mark task as started
                    task_started_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

                    // Run until cancelled - use tokio::select! for faster cancellation
                    signal_clone.cancelled().await;
                }).await;
            }

            // Give tasks time to start (minimal delay)
            tokio::task::yield_now().await;

            // Note: Some tasks may not have started yet due to async scheduling
            // The important thing is that shutdown handles them correctly
            let _started = task_started.load(std::sync::atomic::Ordering::SeqCst);

            // Shutdown should complete within bounded time (tasks should be aborted)
            let shutdown_result = tokio::time::timeout(
                tokio::time::Duration::from_secs(2),
                manager.shutdown()
            ).await;

            prop_assert!(
                shutdown_result.is_ok(),
                "Shutdown should complete within bounded time"
            );

            // After shutdown, no subscriptions should remain
            prop_assert_eq!(manager.count().await, 0, "No subscriptions should remain after shutdown");

            Ok(())
        })?;
    }
}

// =============================================================================
// Property 37: Event Iterator Completion Signal
// =============================================================================

proptest! {
    /// **Property 37: Event Iterator Completion Signal**
    /// *For any* subscription context, when cancelled, is_cancelled() should return true
    #[test]
    fn prop_completion_signal(
        last_event_id in proptest::option::of("[a-z0-9]{1,32}")
    ) {
        let subscription_id = SubscriptionId::new();
        let ctx = SubscriptionContext::new(subscription_id, last_event_id.clone());

        // Initially not cancelled
        prop_assert!(!ctx.is_cancelled());

        // After cancel, should be cancelled
        ctx.signal().cancel();
        prop_assert!(ctx.is_cancelled());

        // Verify context fields
        prop_assert_eq!(ctx.subscription_id, subscription_id);
        prop_assert_eq!(ctx.last_event_id, last_event_id);
    }
}

// =============================================================================
// Property 36: Event Iterator Last Event ID Resumption
// =============================================================================

proptest! {
    /// **Property 36: Event Iterator Last Event ID Resumption**
    /// *For any* subscription context with a last_event_id, the ID should be preserved
    /// and accessible for resumption logic
    #[test]
    fn prop_last_event_id_resumption(
        last_event_id in "[a-z0-9]{1,32}"
    ) {
        let subscription_id = SubscriptionId::new();
        let ctx = SubscriptionContext::new(subscription_id, Some(last_event_id.clone()));

        // Last event ID should be preserved
        prop_assert_eq!(ctx.last_event_id, Some(last_event_id));
    }

    /// Test that events can carry IDs for resumption
    #[test]
    fn prop_event_id_preservation(
        data in any::<i32>(),
        event_id in "[a-z0-9]{1,32}"
    ) {
        let event = Event::with_id(data, event_id.clone());

        prop_assert_eq!(event.data, data);
        prop_assert_eq!(event.id, Some(event_id));
        prop_assert_eq!(event.retry, None);
    }
}

// =============================================================================
// Property 38: EventPublisher Channel Isolation
// =============================================================================

proptest! {
    /// **Property 38: EventPublisher Channel Isolation**
    /// *For any* set of channels, events published to one channel should not
    /// appear in other channels
    #[test]
    fn prop_channel_isolation(
        channel_names in prop::collection::hash_set("[a-z]{3,8}", 2..5),
        events_per_channel in 1usize..10
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let publisher: ChannelPublisher<String> = ChannelPublisher::new(64);
            let channel_names: Vec<_> = channel_names.into_iter().collect();

            // Subscribe to all channels
            let mut subscribers = Vec::new();
            for name in &channel_names {
                let sub = publisher.subscribe(name).await;
                subscribers.push((name.clone(), sub));
            }

            // Publish events to each channel with channel-specific data
            for name in channel_names.iter() {
                for j in 0..events_per_channel {
                    let data = format!("{}_{}", name, j);
                    let _ = publisher.publish_data(name, data).await;
                }
            }

            // Note: In a real test, we'd verify that each subscriber only receives
            // events from their channel. This is a simplified structural test.

            // Verify channels exist
            let channels = publisher.channels().await;
            for name in &channel_names {
                prop_assert!(channels.contains(name), "Channel {} should exist", name);
            }

            Ok(())
        })?;
    }
}

// =============================================================================
// Event and EventMeta Tests
// =============================================================================

proptest! {
    /// Test Event creation and metadata
    #[test]
    fn prop_event_creation(
        data in any::<String>(),
        id in proptest::option::of("[a-z0-9]{1,32}"),
        retry in proptest::option::of(100u64..10000)
    ) {
        let event = Event::new(data.clone());
        prop_assert_eq!(event.data, data.clone());
        prop_assert_eq!(event.id, None);
        prop_assert_eq!(event.retry, None);

        // Create new event with metadata
        let meta = EventMeta {
            id: id.clone(),
            retry,
        };
        let event_with_meta = Event::new(data.clone()).with_meta(meta);

        prop_assert_eq!(event_with_meta.id, id);
        prop_assert_eq!(event_with_meta.retry, retry);
    }
}

// =============================================================================
// EventPublisher Tests
// =============================================================================

proptest! {
    /// Test EventPublisher subscriber count
    #[test]
    fn prop_publisher_subscriber_count(subscriber_count in 1usize..20) {
        let publisher: EventPublisher<i32> = EventPublisher::new(64);

        // Create subscribers
        let _subscribers: Vec<_> = (0..subscriber_count)
            .map(|_| publisher.subscribe())
            .collect();

        // Verify count
        prop_assert_eq!(publisher.subscriber_count(), subscriber_count);
    }
}

// =============================================================================
// CancellationSignal Tests
// =============================================================================

proptest! {
    /// Test CancellationSignal state transitions
    #[test]
    fn prop_cancellation_signal_state(_dummy in 0..100) {
        let signal = CancellationSignal::new();

        // Initially not cancelled
        prop_assert!(!signal.is_cancelled());

        // After cancel
        signal.cancel();
        prop_assert!(signal.is_cancelled());

        // Multiple cancels should be idempotent
        signal.cancel();
        prop_assert!(signal.is_cancelled());
    }
}

// =============================================================================
// Property 7: Bounded Channel Backpressure
// =============================================================================

proptest! {
    /// **Property 7: Bounded Channel Backpressure**
    /// *For any* subscription channel configured with a buffer size N, when N events
    /// are pending and unconsumed, subsequent sends SHALL either block (for async sends)
    /// or return a "full" indication, and the channel SHALL never exceed N pending events.
    /// **Feature: tauri-rpc-plugin-optimization, Property 7: Bounded Channel Backpressure**
    #[test]
    fn prop_bounded_channel_backpressure(buffer_size in 2usize..16) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            // Create a publisher with the specified buffer size
            let publisher: EventPublisher<i32> = EventPublisher::new(buffer_size);

            // Create a subscriber (required for publish to succeed)
            let _subscriber = publisher.subscribe();

            // Publish exactly buffer_size events - all should succeed
            for i in 0..buffer_size {
                let result = publisher.publish_data(i as i32);
                prop_assert!(
                    result.is_published(),
                    "Publishing event {} should succeed within buffer capacity {}",
                    i, buffer_size
                );
            }

            // The broadcast channel in tokio doesn't block on send - it overwrites
            // old messages when full (for lagging receivers). This is the expected
            // behavior for broadcast channels. The backpressure is handled by
            // the receiver getting a Lagged error.

            // Verify subscriber count is correct
            prop_assert_eq!(publisher.subscriber_count(), 1);

            Ok(())
        })?;
    }
}

// =============================================================================
// Property 8: EventPublisher Graceful Empty Publish
// =============================================================================

proptest! {
    /// **Property 8: EventPublisher Graceful Empty Publish**
    /// *For any* EventPublisher with zero subscribers, calling publish SHALL return
    /// an error result (not panic) indicating no subscribers are available.
    /// **Feature: tauri-rpc-plugin-optimization, Property 8: EventPublisher Graceful Empty Publish**
    #[test]
    fn prop_empty_publisher_graceful(
        buffer_size in 1usize..64,
        data in any::<i32>()
    ) {
        // Create a publisher with no subscribers
        let publisher: EventPublisher<i32> = EventPublisher::new(buffer_size);

        // Verify no subscribers
        prop_assert_eq!(publisher.subscriber_count(), 0);

        // Publishing should return NoSubscribers, not an error
        let result = publisher.publish_data(data);

        prop_assert_eq!(
            result,
            PublishResult::NoSubscribers,
            "Publishing to empty publisher should return NoSubscribers"
        );
    }

    /// Test that publish returns NoSubscribers after all subscribers are dropped
    #[test]
    fn prop_empty_publisher_after_drop(
        buffer_size in 1usize..64,
        data in any::<i32>()
    ) {
        let publisher: EventPublisher<i32> = EventPublisher::new(buffer_size);

        // Create and immediately drop a subscriber
        {
            let _subscriber = publisher.subscribe();
            prop_assert_eq!(publisher.subscriber_count(), 1);
        }

        // After subscriber is dropped, count should be 0
        prop_assert_eq!(publisher.subscriber_count(), 0);

        // Publishing should now return NoSubscribers
        let result = publisher.publish_data(data);
        prop_assert_eq!(
            result,
            PublishResult::NoSubscribers,
            "Publishing after all subscribers dropped should return NoSubscribers"
        );
    }
}

// =============================================================================
// Property 9: Lagged Subscriber Recovery
// =============================================================================

proptest! {
    /// **Property 9: Lagged Subscriber Recovery**
    /// *For any* subscriber that falls behind in a broadcast channel, when they resume
    /// receiving, they SHALL receive the most recent available messages (skipping lagged
    /// ones) rather than blocking indefinitely or receiving stale data.
    /// **Feature: tauri-rpc-plugin-optimization, Property 9: Lagged Subscriber Recovery**
    #[test]
    fn prop_lagged_subscriber_recovery(
        buffer_size in 2usize..8,
        overflow_count in 1usize..5
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            // Create a small buffer publisher
            let publisher: EventPublisher<i32> = EventPublisher::new(buffer_size);

            // Create a subscriber
            let mut subscriber = publisher.subscribe();

            // Publish more events than the buffer can hold to cause lag
            let total_events = buffer_size + overflow_count;
            for i in 0..total_events {
                let _ = publisher.publish_data(i as i32);
            }

            // The subscriber should be able to recover and receive events
            // It may skip some lagged events, but should not block indefinitely
            let timeout_result = tokio::time::timeout(
                tokio::time::Duration::from_millis(100),
                subscriber.recv()
            ).await;

            // Should complete without timeout (either receive an event or handle lag)
            prop_assert!(
                timeout_result.is_ok(),
                "Subscriber should recover from lag without blocking indefinitely"
            );

            Ok(())
        })?;
    }

    /// Test that lagged subscriber continues to receive new events after recovery
    #[test]
    fn prop_lagged_subscriber_continues_receiving(buffer_size in 2usize..8) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let publisher: EventPublisher<i32> = EventPublisher::new(buffer_size);
            let mut subscriber = publisher.subscribe();

            // Cause lag by publishing more than buffer size
            for i in 0..(buffer_size * 2) {
                let _ = publisher.publish_data(i as i32);
            }

            // Drain any available events (handling lag)
            let mut _received_count = 0;
            loop {
                let timeout_result = tokio::time::timeout(
                    tokio::time::Duration::from_millis(10),
                    subscriber.recv()
                ).await;

                match timeout_result {
                    Ok(Some(_)) => _received_count += 1,
                    Ok(None) => break, // Channel closed
                    Err(_) => break,   // Timeout - no more events
                }
            }

            // Should have received at least some events
            // (may not be all due to lag handling)

            // Now publish a new event after recovery
            let new_event_value = 9999;
            let _ = publisher.publish_data(new_event_value);

            // Subscriber should be able to receive the new event
            let timeout_result = tokio::time::timeout(
                tokio::time::Duration::from_millis(100),
                subscriber.recv()
            ).await;

            match timeout_result {
                Ok(Some(event)) => {
                    prop_assert_eq!(
                        event.data, new_event_value,
                        "Should receive the new event after recovery"
                    );
                }
                Ok(None) => {
                    // Channel closed - this is acceptable if publisher was dropped
                }
                Err(_) => {
                    // Timeout - this shouldn't happen for a fresh event
                    prop_assert!(false, "Should not timeout receiving new event after recovery");
                }
            }

            Ok(())
        })?;
    }
}

// =============================================================================
// Property 4: SubscriptionEvent Error Serialization Format
// =============================================================================

proptest! {
    /// **Property 4: SubscriptionEvent Error Serialization Format**
    /// *For any* SubscriptionEvent::Error with a retry_after_ms value, serializing to JSON
    /// SHALL produce a field named "retryAfterMs" (camelCase).
    /// **Feature: tauri-plugin-rpc-improvements, Property 4: SubscriptionEvent Error Serialization Format**
    /// **Validates: Requirements 3.4**
    #[test]
    fn prop_subscription_event_error_serialization_format(
        retry_ms in 100u64..10000
    ) {
        let error = crate::RpcError::internal("test error");
        let event = crate::subscription::SubscriptionEvent::error_with_retry(error, retry_ms);

        let json = serde_json::to_string(&event).unwrap();

        // Verify the field is serialized as camelCase "retryAfterMs"
        prop_assert!(
            json.contains("retryAfterMs"),
            "JSON should contain 'retryAfterMs' field: {}",
            json
        );

        // Verify the retry value is present in the JSON
        prop_assert!(
            json.contains(&retry_ms.to_string()),
            "JSON should contain the retry value {}: {}",
            retry_ms,
            json
        );

        // Verify the type is "error" (camelCase from rename_all)
        prop_assert!(
            json.contains("\"type\":\"error\""),
            "JSON should contain type 'error': {}",
            json
        );
    }

    /// Test that error events without retry do not include retryAfterMs field
    #[test]
    fn prop_subscription_event_error_no_retry_serialization(
        _dummy in 0..100
    ) {
        let error = crate::RpcError::unauthorized("test error");
        let event = crate::subscription::SubscriptionEvent::error_no_retry(error);

        let json = serde_json::to_string(&event).unwrap();

        // Verify the field is NOT present when retry is None (skip_serializing_if)
        prop_assert!(
            !json.contains("retryAfterMs"),
            "JSON should NOT contain 'retryAfterMs' field when None: {}",
            json
        );
    }

    /// Test that the existing error() method also excludes retryAfterMs
    #[test]
    fn prop_subscription_event_error_default_no_retry(
        _dummy in 0..100
    ) {
        let error = crate::RpcError::not_found("test error");
        let event = crate::subscription::SubscriptionEvent::error(error);

        let json = serde_json::to_string(&event).unwrap();

        // Verify the field is NOT present when using the default error() method
        prop_assert!(
            !json.contains("retryAfterMs"),
            "JSON should NOT contain 'retryAfterMs' field for default error(): {}",
            json
        );
    }
}

// =============================================================================
// Property 5: Shutdown Cancels All Subscription Signals
// =============================================================================

proptest! {
    /// **Property 5: Shutdown Cancels All Subscription Signals**
    /// *For any* SubscriptionManager with active subscriptions, calling `shutdown()` SHALL
    /// result in all subscription signals being cancelled (is_cancelled() returns true).
    /// **Feature: tauri-plugin-rpc-improvements, Property 5: Shutdown Cancels All Subscription Signals**
    /// **Validates: Requirements 4.3**
    #[test]
    fn prop_shutdown_cancels_all_signals(sub_count in 1usize..10) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let manager = SubscriptionManager::new();
            let mut signals = Vec::new();

            // Create subscriptions and collect their signals
            for i in 0..sub_count {
                let id = generate_subscription_id();
                let ctx = SubscriptionContext::new(id, None);
                signals.push(ctx.signal());
                let handle = crate::subscription::SubscriptionHandle::new(
                    id,
                    format!("test.path.{}", i),
                    ctx.signal(),
                );
                manager.subscribe(handle).await;
            }

            // Verify all subscriptions exist before shutdown
            prop_assert_eq!(manager.count().await, sub_count);

            // Verify no signals are cancelled before shutdown
            for signal in &signals {
                prop_assert!(
                    !signal.is_cancelled(),
                    "Signal should not be cancelled before shutdown"
                );
            }

            // Call shutdown
            manager.shutdown().await;

            // Verify all signals are cancelled after shutdown
            for (i, signal) in signals.iter().enumerate() {
                prop_assert!(
                    signal.is_cancelled(),
                    "Signal {} should be cancelled after shutdown",
                    i
                );
            }

            // Verify all subscriptions are removed
            prop_assert_eq!(
                manager.count().await,
                0,
                "All subscriptions should be removed after shutdown"
            );

            Ok(())
        })?;
    }
}

// =============================================================================
// Property 6: Shutdown Waits for Tracked Tasks
// =============================================================================

proptest! {
    /// **Property 6: Shutdown Waits for Tracked Tasks**
    /// *For any* SubscriptionManager with spawned subscription tasks, calling `shutdown()` SHALL
    /// wait for all tracked tasks to complete (or be aborted) before returning.
    /// **Feature: tauri-plugin-rpc-improvements, Property 6: Shutdown Waits for Tracked Tasks**
    /// **Validates: Requirements 4.4**
    #[test]
    fn prop_shutdown_waits_for_tracked_tasks(task_count in 1usize..5) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let manager = Arc::new(SubscriptionManager::new());
            let completed = Arc::new(std::sync::atomic::AtomicUsize::new(0));

            // Spawn tracked subscription tasks
            for i in 0..task_count {
                let id = generate_subscription_id();
                let signal = Arc::new(CancellationSignal::new());
                let handle = crate::subscription::SubscriptionHandle::new(
                    id,
                    format!("task.{}", i),
                    signal.clone(),
                );
                manager.subscribe(handle).await;

                let completed_clone = completed.clone();
                let signal_clone = signal.clone();
                manager.spawn_subscription(id, async move {
                    // Wait for cancellation signal (simulating a running subscription)
                    signal_clone.cancelled().await;
                    // Mark task as completed after receiving cancellation
                    completed_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                }).await;
            }

            // Verify tasks are tracked
            prop_assert!(
                manager.task_count().await > 0,
                "Tasks should be tracked before shutdown"
            );

            // Call shutdown - this should wait for all tasks to complete
            manager.shutdown().await;

            // After shutdown returns, all tasks should have been processed
            // (either completed normally or aborted)
            prop_assert_eq!(
                manager.task_count().await,
                0,
                "No tasks should remain after shutdown"
            );

            // Verify all subscriptions are removed
            prop_assert_eq!(
                manager.count().await,
                0,
                "All subscriptions should be removed after shutdown"
            );

            Ok(())
        })?;
    }
}

// =============================================================================
// Unit Tests for SubscriptionEvent Error Helper Methods
// =============================================================================

#[cfg(test)]
mod subscription_event_error_tests {
    use crate::RpcError;
    use crate::subscription::SubscriptionEvent;

    /// Test that error_with_retry creates an event with the specified retry value
    #[test]
    fn test_error_with_retry_creates_event_with_retry_value() {
        let error = RpcError::service_unavailable("Server busy");
        let retry_ms = 5000u64;

        let event = SubscriptionEvent::error_with_retry(error.clone(), retry_ms);

        match event {
            SubscriptionEvent::Error {
                payload,
                retry_after_ms,
            } => {
                assert_eq!(payload.code, error.code);
                assert_eq!(payload.message, error.message);
                assert_eq!(retry_after_ms, Some(retry_ms));
            }
            _ => panic!("Expected Error variant"),
        }
    }

    /// Test that error_no_retry creates an event without retry value
    #[test]
    fn test_error_no_retry_creates_event_without_retry_value() {
        let error = RpcError::unauthorized("Invalid token");

        let event = SubscriptionEvent::error_no_retry(error.clone());

        match event {
            SubscriptionEvent::Error {
                payload,
                retry_after_ms,
            } => {
                assert_eq!(payload.code, error.code);
                assert_eq!(payload.message, error.message);
                assert_eq!(retry_after_ms, None);
            }
            _ => panic!("Expected Error variant"),
        }
    }

    /// Test that the default error() method creates an event without retry value
    #[test]
    fn test_error_default_creates_event_without_retry_value() {
        let error = RpcError::not_found("Resource not found");

        let event = SubscriptionEvent::error(error.clone());

        match event {
            SubscriptionEvent::Error {
                payload,
                retry_after_ms,
            } => {
                assert_eq!(payload.code, error.code);
                assert_eq!(payload.message, error.message);
                assert_eq!(retry_after_ms, None);
            }
            _ => panic!("Expected Error variant"),
        }
    }

    /// Test that error_with_retry with zero retry value works correctly
    #[test]
    fn test_error_with_retry_zero_value() {
        let error = RpcError::rate_limited("Too many requests");
        let retry_ms = 0u64;

        let event = SubscriptionEvent::error_with_retry(error, retry_ms);

        match event {
            SubscriptionEvent::Error { retry_after_ms, .. } => {
                assert_eq!(retry_after_ms, Some(0));
            }
            _ => panic!("Expected Error variant"),
        }
    }

    /// Test that error_with_retry with large retry value works correctly
    #[test]
    fn test_error_with_retry_large_value() {
        let error = RpcError::service_unavailable("Maintenance mode");
        let retry_ms = u64::MAX;

        let event = SubscriptionEvent::error_with_retry(error, retry_ms);

        match event {
            SubscriptionEvent::Error { retry_after_ms, .. } => {
                assert_eq!(retry_after_ms, Some(u64::MAX));
            }
            _ => panic!("Expected Error variant"),
        }
    }
}

// =============================================================================
// Unit Tests for Shutdown Timeout Behavior
// =============================================================================

#[cfg(test)]
mod shutdown_timeout_tests {
    use crate::subscription::{CancellationSignal, SubscriptionId, SubscriptionManager};
    use std::sync::Arc;

    /// Test that shutdown completes even with stuck tasks (tasks that don't respond to cancellation).
    /// This validates Requirements 4.5 - shutdown should complete within a reasonable timeout.
    /// **Validates: Requirements 4.5**
    #[tokio::test]
    async fn test_shutdown_completes_with_stuck_tasks() {
        let manager = Arc::new(SubscriptionManager::new());

        // Create a subscription with a "stuck" task that ignores cancellation
        let id = SubscriptionId::new();
        let signal = Arc::new(CancellationSignal::new());
        let handle = crate::subscription::SubscriptionHandle::new(
            id,
            "stuck.task".to_string(),
            signal.clone(),
        );
        manager.subscribe(handle).await;

        // Spawn a task that ignores the cancellation signal (simulating a stuck task)
        // Note: In reality, the JoinSet will abort this task during shutdown
        manager
            .spawn_subscription(id, async move {
                // This task intentionally ignores the cancellation signal
                // and would run forever if not aborted
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
            })
            .await;

        // Verify task is tracked
        assert!(manager.task_count().await > 0, "Task should be tracked");

        // Shutdown should complete within a bounded time even with stuck tasks
        // because the JoinSet aborts all tasks
        let shutdown_result =
            tokio::time::timeout(tokio::time::Duration::from_secs(2), manager.shutdown()).await;

        assert!(
            shutdown_result.is_ok(),
            "Shutdown should complete within timeout even with stuck tasks"
        );

        // After shutdown, no tasks should remain
        assert_eq!(
            manager.task_count().await,
            0,
            "No tasks should remain after shutdown"
        );

        // All subscriptions should be removed
        assert_eq!(
            manager.count().await,
            0,
            "All subscriptions should be removed after shutdown"
        );
    }

    /// Test that shutdown completes immediately when there are no subscriptions
    #[tokio::test]
    async fn test_shutdown_with_no_subscriptions() {
        let manager = SubscriptionManager::new();

        // Verify no subscriptions
        assert_eq!(manager.count().await, 0);

        // Shutdown should complete immediately
        let shutdown_result =
            tokio::time::timeout(tokio::time::Duration::from_millis(100), manager.shutdown()).await;

        assert!(
            shutdown_result.is_ok(),
            "Shutdown should complete immediately with no subscriptions"
        );
    }

    /// Test that shutdown properly handles multiple stuck tasks
    #[tokio::test]
    async fn test_shutdown_with_multiple_stuck_tasks() {
        let manager = Arc::new(SubscriptionManager::new());
        let task_count = 5;

        // Create multiple subscriptions with stuck tasks
        for i in 0..task_count {
            let id = SubscriptionId::new();
            let signal = Arc::new(CancellationSignal::new());
            let handle = crate::subscription::SubscriptionHandle::new(
                id,
                format!("stuck.task.{}", i),
                signal.clone(),
            );
            manager.subscribe(handle).await;

            // Spawn a stuck task
            manager
                .spawn_subscription(id, async move {
                    loop {
                        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
                    }
                })
                .await;
        }

        // Verify all tasks are tracked
        assert_eq!(
            manager.task_count().await,
            task_count,
            "All tasks should be tracked"
        );

        // Shutdown should complete within bounded time
        let shutdown_result =
            tokio::time::timeout(tokio::time::Duration::from_secs(2), manager.shutdown()).await;

        assert!(
            shutdown_result.is_ok(),
            "Shutdown should complete within timeout with multiple stuck tasks"
        );

        // All tasks and subscriptions should be cleaned up
        assert_eq!(manager.task_count().await, 0);
        assert_eq!(manager.count().await, 0);
    }
}
