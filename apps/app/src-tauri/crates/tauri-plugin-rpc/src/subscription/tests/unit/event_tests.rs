// Unit tests for Event types

#[cfg(test)]
mod tests {
    use crate::subscription::*;

    #[test]
    fn test_event_new() {
        let event = Event::new("test data");
        assert_eq!(event.data, "test data");
        assert!(event.id.is_none());
        assert!(event.retry.is_none());
    }

    #[test]
    fn test_event_with_id() {
        let event = Event::with_id("test data", "event-123");
        assert_eq!(event.data, "test data");
        assert_eq!(event.id, Some("event-123".to_string()));
    }

    #[test]
    fn test_event_with_meta() {
        let meta = EventMeta::with_id("event-456");
        let event = Event::new("test data").with_meta(meta);
        assert_eq!(event.id, Some("event-456".to_string()));
    }
}
