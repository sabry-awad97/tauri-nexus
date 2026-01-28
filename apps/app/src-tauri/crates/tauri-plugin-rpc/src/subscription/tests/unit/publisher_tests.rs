// Unit tests for EventPublisher and EventSubscriber

#[cfg(test)]
mod tests {
    use crate::subscription::PublishResult;
    use crate::subscription::*;

    #[test]
    fn test_publisher_new() {
        let publisher = EventPublisher::<String>::new(256);
        assert_eq!(publisher.subscriber_count(), 0);
    }

    #[test]
    fn test_publisher_subscribe() {
        let publisher = EventPublisher::<String>::new(256);
        let _subscriber = publisher.subscribe();
        assert_eq!(publisher.subscriber_count(), 1);
    }

    #[tokio::test]
    async fn test_publisher_publish_no_subscribers() {
        let publisher = EventPublisher::<String>::new(256);
        let result = publisher.publish_data("test".to_string());

        // Should return NoSubscribers when no subscribers
        assert_eq!(result, PublishResult::NoSubscribers);
        assert!(!result.is_published());
    }

    #[tokio::test]
    async fn test_publisher_publish_with_subscriber() {
        let publisher = EventPublisher::<String>::new(256);
        let mut subscriber = publisher.subscribe();

        let result = publisher.publish_data("test message".to_string());
        assert!(result.is_published());
        assert_eq!(result.subscriber_count(), 1);

        let event = subscriber.recv().await;
        assert!(event.is_some());
        assert_eq!(event.unwrap().data, "test message");
    }
}
