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
}
