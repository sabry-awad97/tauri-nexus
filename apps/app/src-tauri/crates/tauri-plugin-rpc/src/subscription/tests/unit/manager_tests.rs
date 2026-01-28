// Unit tests for SubscriptionManager

#[cfg(test)]
mod tests {
    use crate::subscription::*;

    #[tokio::test]
    async fn test_manager_new() {
        let manager = SubscriptionManager::new();
        assert_eq!(manager.count().await, 0);
    }

    #[tokio::test]
    async fn test_manager_subscribe() {
        let manager = SubscriptionManager::new();
        let id = SubscriptionId::new();
        let signal = std::sync::Arc::new(CancellationSignal::new());
        let handle = SubscriptionHandle::new(id, "test.path".to_string(), signal);

        let registered_id = manager.subscribe(handle).await;
        assert_eq!(registered_id, id);
        assert_eq!(manager.count().await, 1);
        assert!(manager.exists(&id).await);
    }

    #[tokio::test]
    async fn test_manager_unsubscribe() {
        let manager = SubscriptionManager::new();
        let id = SubscriptionId::new();
        let signal = std::sync::Arc::new(CancellationSignal::new());
        let handle = SubscriptionHandle::new(id, "test.path".to_string(), signal);

        manager.subscribe(handle).await;
        let removed = manager.unsubscribe(&id).await;

        assert!(removed);
        assert_eq!(manager.count().await, 0);
        assert!(!manager.exists(&id).await);
    }
}
