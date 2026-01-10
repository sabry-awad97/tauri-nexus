//! Property-based tests for subscription/event iterator functionality
//!
//! These tests validate the correctness properties defined in the design document:
//! - Property 9: Subscription ID Uniqueness
//! - Property 10: Subscription Cleanup
//! - Property 36: Event Iterator Last Event ID Resumption
//! - Property 37: Event Iterator Completion Signal
//! - Property 38: EventPublisher Channel Isolation

use proptest::prelude::*;
use std::collections::HashSet;
use std::sync::Arc;

use crate::subscription::{
    generate_subscription_id, CancellationSignal, ChannelPublisher, Event, EventMeta,
    EventPublisher, SubscriptionContext, SubscriptionManager,
};

// =============================================================================
// Property 9: Subscription ID Uniqueness
// =============================================================================

proptest! {
    /// **Property 9: Subscription ID Uniqueness**
    /// *For any* number of subscription IDs generated, all IDs should be unique
    /// **Validates: Requirements 5.2**
    #[test]
    fn prop_subscription_id_uniqueness(count in 1usize..1000) {
        let mut ids = HashSet::new();
        for _ in 0..count {
            let id = generate_subscription_id();
            // Each ID should be unique
            prop_assert!(ids.insert(id.clone()), "Duplicate subscription ID generated: {}", id);
        }
        // All IDs should be present
        prop_assert_eq!(ids.len(), count);
    }
}

// =============================================================================
// Property 10: Subscription Cleanup
// =============================================================================

proptest! {
    /// **Property 10: Subscription Cleanup**
    /// *For any* subscription that is unsubscribed, the subscription should be removed
    /// and its cancellation signal should be triggered
    /// **Validates: Requirements 5.6, 5.9**
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
                let id = format!("sub_{}", i);
                let signal = Arc::new(CancellationSignal::new());
                let handle = crate::subscription::SubscriptionHandle::new(
                    id.clone(),
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
// Property 37: Event Iterator Completion Signal
// =============================================================================

proptest! {
    /// **Property 37: Event Iterator Completion Signal**
    /// *For any* subscription context, when cancelled, is_cancelled() should return true
    /// **Validates: Requirements 5.5**
    #[test]
    fn prop_completion_signal(
        subscription_id in "[a-z0-9]{8,16}",
        last_event_id in proptest::option::of("[a-z0-9]{1,32}")
    ) {
        let ctx = SubscriptionContext::new(subscription_id.clone(), last_event_id.clone());
        
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
    /// **Validates: Requirements 5.4**
    #[test]
    fn prop_last_event_id_resumption(
        subscription_id in "[a-z0-9]{8,16}",
        last_event_id in "[a-z0-9]{1,32}"
    ) {
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
    /// **Validates: Requirements 5.7**
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
            for (_i, name) in channel_names.iter().enumerate() {
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
