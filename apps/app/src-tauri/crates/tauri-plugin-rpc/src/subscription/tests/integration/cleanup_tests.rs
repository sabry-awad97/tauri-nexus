// Integration tests for resource cleanup

#[cfg(test)]
mod tests {
    use crate::subscription::*;

    #[tokio::test]
    async fn test_subscription_cleanup_on_cancel() {
        let manager = SubscriptionManager::new();
        let id = SubscriptionId::new();
        let signal = std::sync::Arc::new(CancellationSignal::new());
        let handle = SubscriptionHandle::new(id, "test.cleanup".to_string(), signal.clone());

        manager.subscribe(handle);
        assert_eq!(manager.count(), 1);

        // Cancel and cleanup
        signal.cancel();
        manager.unsubscribe(&id);

        assert_eq!(manager.count(), 0);
    }

    #[tokio::test]
    async fn test_manager_shutdown_cleans_all() {
        let manager = SubscriptionManager::new();

        // Create multiple subscriptions
        for i in 0..5 {
            let id = SubscriptionId::new();
            let signal = std::sync::Arc::new(CancellationSignal::new());
            let handle = SubscriptionHandle::new(id, format!("test.{}", i), signal);
            manager.subscribe(handle);
        }

        assert_eq!(manager.count(), 5);

        // Shutdown should clean everything
        manager.shutdown().await;

        assert_eq!(manager.count(), 0);
    }
}
