// Unit tests for SubscriptionManager

#[cfg(test)]
mod tests {
    use crate::subscription::*;

    #[tokio::test]
    async fn test_manager_new() {
        let manager = SubscriptionManager::new();
        assert_eq!(manager.count(), 0);
    }

    #[tokio::test]
    async fn test_manager_subscribe() {
        let manager = SubscriptionManager::new();
        let id = SubscriptionId::new();
        let signal = std::sync::Arc::new(CancellationSignal::new());
        let handle = SubscriptionHandle::new(id, "test.path".to_string(), signal);

        let registered_id = manager.subscribe(handle);
        assert_eq!(registered_id, id);
        assert_eq!(manager.count(), 1);
        assert!(manager.exists(&id));
    }

    #[tokio::test]
    async fn test_manager_unsubscribe() {
        let manager = SubscriptionManager::new();
        let id = SubscriptionId::new();
        let signal = std::sync::Arc::new(CancellationSignal::new());
        let handle = SubscriptionHandle::new(id, "test.path".to_string(), signal);

        manager.subscribe(handle);
        let removed = manager.unsubscribe(&id);

        assert!(removed);
        assert_eq!(manager.count(), 0);
        assert!(!manager.exists(&id));
    }
}
