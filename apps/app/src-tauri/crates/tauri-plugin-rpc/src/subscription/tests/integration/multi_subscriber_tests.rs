// Integration tests for multiple subscribers

#[cfg(test)]
mod tests {
    use crate::subscription::*;

    #[tokio::test]
    async fn test_multiple_subscribers_receive_same_event() {
        let publisher = EventPublisher::<String>::new(256);
        let mut sub1 = publisher.subscribe();
        let mut sub2 = publisher.subscribe();
        let mut sub3 = publisher.subscribe();

        let _ = publisher.publish_data("broadcast message".to_string());

        let event1 = sub1.recv().await;
        let event2 = sub2.recv().await;
        let event3 = sub3.recv().await;

        assert!(event1.is_some());
        assert!(event2.is_some());
        assert!(event3.is_some());

        assert_eq!(event1.unwrap().data, "broadcast message");
        assert_eq!(event2.unwrap().data, "broadcast message");
        assert_eq!(event3.unwrap().data, "broadcast message");
    }
}
