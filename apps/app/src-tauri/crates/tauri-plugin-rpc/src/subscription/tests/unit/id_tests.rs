// Unit tests for SubscriptionId

#[cfg(test)]
mod tests {
    use crate::subscription::*;

    #[test]
    fn test_subscription_id_new() {
        let id = SubscriptionId::new();
        assert!(id.to_string().starts_with("sub_"));
    }

    #[test]
    fn test_subscription_id_display() {
        let id = SubscriptionId::new();
        let display = id.to_string();
        assert!(display.starts_with("sub_"));
        assert_eq!(display.len(), 40); // "sub_" + 36 char UUID
    }

    #[test]
    fn test_subscription_id_from_uuid() {
        let uuid = uuid::Uuid::now_v7();
        let id = SubscriptionId::from_uuid(uuid);
        assert_eq!(id.as_uuid(), &uuid);
    }

    #[test]
    fn test_subscription_id_parse_with_prefix() {
        let id = SubscriptionId::new();
        let id_str = id.to_string();
        let parsed = SubscriptionId::parse(&id_str).unwrap();
        assert_eq!(parsed, id);
    }

    #[test]
    fn test_subscription_id_parse_without_prefix_fails() {
        let uuid = uuid::Uuid::now_v7();
        let uuid_str = uuid.to_string();
        let result = SubscriptionId::parse(&uuid_str);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ParseError::MissingPrefix));
    }

    #[test]
    fn test_subscription_id_parse_lenient_with_prefix() {
        let id = SubscriptionId::new();
        let id_str = id.to_string();
        let parsed = SubscriptionId::parse_lenient(&id_str).unwrap();
        assert_eq!(parsed, id);
    }

    #[test]
    fn test_subscription_id_parse_lenient_without_prefix() {
        let uuid = uuid::Uuid::now_v7();
        let uuid_str = uuid.to_string();
        let parsed = SubscriptionId::parse_lenient(&uuid_str).unwrap();
        assert_eq!(parsed.as_uuid(), &uuid);
    }

    #[test]
    fn test_subscription_id_parse_invalid_uuid() {
        let result = SubscriptionId::parse("sub_invalid-uuid");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ParseError::InvalidUuid(_)));
    }

    #[test]
    fn test_subscription_id_round_trip() {
        let id = SubscriptionId::new();
        let id_str = id.to_string();
        let parsed = SubscriptionId::parse(&id_str).unwrap();
        assert_eq!(parsed, id);
        assert_eq!(parsed.to_string(), id_str);
    }
}
