//! Tauri plugin integration

use crate::subscription::{
    Event, SubscriptionContext, SubscriptionEvent, SubscriptionManager,
    generate_subscription_id,
};
use crate::RpcError;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tauri::{
    plugin::{Builder, TauriPlugin},
    AppHandle, Emitter, Manager, Runtime, State,
};
use tokio::sync::mpsc;

/// Type-erased router trait for plugin storage
pub trait DynRouter: Send + Sync {
    /// Call a procedure by path
    fn call<'a>(
        &'a self,
        path: &'a str,
        input: serde_json::Value,
    ) -> Pin<Box<dyn Future<Output = Result<serde_json::Value, RpcError>> + Send + 'a>>;

    /// List all registered procedures
    fn procedures(&self) -> Vec<String>;

    /// Check if a path is a subscription
    fn is_subscription(&self, path: &str) -> bool;

    /// Start a subscription
    fn subscribe<'a>(
        &'a self,
        path: &'a str,
        input: serde_json::Value,
        ctx: SubscriptionContext,
    ) -> Pin<Box<dyn Future<Output = Result<mpsc::Receiver<Event<serde_json::Value>>, RpcError>> + Send + 'a>>;
}

/// Router state wrapper
struct RouterState(Arc<dyn DynRouter>);

/// Subscription manager state
struct SubscriptionState(Arc<SubscriptionManager>);

/// RPC call command
#[tauri::command]
async fn rpc_call(
    path: String,
    input: serde_json::Value,
    state: State<'_, RouterState>,
) -> Result<serde_json::Value, String> {
    state.0.call(&path, input).await.map_err(|e| {
        serde_json::to_string(&e).unwrap_or_else(|_| e.to_string())
    })
}

/// List available procedures
#[tauri::command]
fn rpc_procedures(state: State<'_, RouterState>) -> Vec<String> {
    state.0.procedures()
}

/// Subscribe to a streaming procedure
#[tauri::command]
async fn rpc_subscribe<R: Runtime>(
    id: String,
    path: String,
    input: serde_json::Value,
    last_event_id: Option<String>,
    app: AppHandle<R>,
    router_state: State<'_, RouterState>,
    sub_state: State<'_, SubscriptionState>,
) -> Result<String, String> {
    // Generate subscription ID if not provided
    let subscription_id = if id.is_empty() {
        generate_subscription_id()
    } else {
        id
    };

    // Check if path is a subscription
    if !router_state.0.is_subscription(&path) {
        return Err(serde_json::to_string(&RpcError::bad_request(
            format!("'{}' is not a subscription procedure", path),
        ))
        .unwrap());
    }

    // Create subscription context
    let sub_ctx = SubscriptionContext::new(subscription_id.clone(), last_event_id);
    let signal = sub_ctx.signal();

    // Create subscription handle
    let handle = crate::subscription::SubscriptionHandle::new(
        subscription_id.clone(),
        path.clone(),
        signal.clone(),
    );

    // Register subscription
    sub_state.0.subscribe(handle).await;

    // Start the subscription
    let event_name = format!("rpc:subscription:{}", subscription_id);
    let router = router_state.0.clone();
    let sub_manager = sub_state.0.clone();
    let sub_id = subscription_id.clone();

    tokio::spawn(async move {
        match router.subscribe(&path, input, sub_ctx).await {
            Ok(mut stream) => {
                // Stream events to frontend
                while let Some(event) = stream.recv().await {
                    if signal.is_cancelled() {
                        break;
                    }

                    let sub_event = SubscriptionEvent::data(event);
                    if app.emit(&event_name, &sub_event).is_err() {
                        break;
                    }
                }

                // Send completion event
                if !signal.is_cancelled() {
                    let _ = app.emit(&event_name, &SubscriptionEvent::completed());
                }
            }
            Err(err) => {
                // Send error event
                let _ = app.emit(&event_name, &SubscriptionEvent::error(err));
            }
        }

        // Cleanup
        sub_manager.unsubscribe(&sub_id).await;
    });

    Ok(subscription_id)
}

/// Unsubscribe from a streaming procedure
#[tauri::command]
async fn rpc_unsubscribe(
    id: String,
    sub_state: State<'_, SubscriptionState>,
) -> Result<bool, String> {
    Ok(sub_state.0.unsubscribe(&id).await)
}

/// Get active subscription count
#[tauri::command]
async fn rpc_subscription_count(sub_state: State<'_, SubscriptionState>) -> Result<usize, String> {
    Ok(sub_state.0.count().await)
}

/// Initialize the RPC plugin with a router
/// 
/// # Example
/// ```rust,ignore
/// tauri::Builder::default()
///     .plugin(tauri_plugin_rpc::init(create_router()))
///     .run(tauri::generate_context!())
/// ```
pub fn init<R: Runtime>(router: impl DynRouter + 'static) -> TauriPlugin<R> {
    let router: Arc<dyn DynRouter> = Arc::new(router);
    let subscription_manager = Arc::new(SubscriptionManager::new());

    Builder::new("rpc")
        .invoke_handler(tauri::generate_handler![
            rpc_call,
            rpc_procedures,
            rpc_subscribe,
            rpc_unsubscribe,
            rpc_subscription_count
        ])
        .setup(move |app, _api| {
            app.manage(RouterState(router.clone()));
            app.manage(SubscriptionState(subscription_manager.clone()));
            Ok(())
        })
        .build()
}
