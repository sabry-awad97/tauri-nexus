//! Tauri plugin integration

use crate::RpcError;
use crate::batch::{BatchConfig, BatchRequest, BatchResponse, BatchResult};
use crate::config::RpcConfig;
use crate::subscription::{
    Event, SubscriptionContext, SubscriptionEvent, SubscriptionId, SubscriptionManager,
    generate_subscription_id,
};
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tauri::{
    AppHandle, Emitter, Manager, Runtime, State,
    plugin::{Builder, TauriPlugin},
};
use tokio::sync::mpsc;

// =============================================================================
// Type Aliases
// =============================================================================

/// Future type for subscription results
pub type SubscriptionFuture<'a> = Pin<
    Box<
        dyn Future<Output = Result<mpsc::Receiver<Event<serde_json::Value>>, RpcError>> + Send + 'a,
    >,
>;

// =============================================================================
// Input Validation
// =============================================================================

/// Validate procedure path format.
pub fn validate_path(path: &str) -> Result<(), RpcError> {
    if path.is_empty() {
        return Err(RpcError::validation("Procedure path cannot be empty"));
    }
    if path.starts_with('.') || path.ends_with('.') {
        return Err(RpcError::validation(
            "Procedure path cannot start or end with a dot",
        ));
    }
    if path.contains("..") {
        return Err(RpcError::validation(
            "Procedure path cannot contain consecutive dots",
        ));
    }
    for ch in path.chars() {
        if !ch.is_ascii_alphanumeric() && ch != '_' && ch != '.' {
            return Err(RpcError::validation(format!(
                "Procedure path contains invalid character: '{}'",
                ch
            )));
        }
    }
    Ok(())
}

/// Validate input size against configuration limit.
pub fn validate_input_size(input: &serde_json::Value, config: &RpcConfig) -> Result<(), RpcError> {
    let size = serde_json::to_vec(input).map(|v| v.len()).unwrap_or(0);
    if size > config.max_input_size {
        return Err(RpcError::payload_too_large(format!(
            "Input size {} bytes exceeds maximum {} bytes",
            size, config.max_input_size
        )));
    }
    Ok(())
}

/// Validate subscription ID format when provided by client.
pub fn validate_subscription_id(id: &str) -> Result<SubscriptionId, RpcError> {
    if id.is_empty() {
        return Err(RpcError::validation("Subscription ID cannot be empty"));
    }
    SubscriptionId::parse(id)
        .map_err(|e| RpcError::validation(format!("Invalid subscription ID '{}': {}", id, e)))
}

/// Validate all inputs for an RPC call.
pub fn validate_rpc_input(
    path: &str,
    input: &serde_json::Value,
    config: &RpcConfig,
) -> Result<(), RpcError> {
    validate_path(path)?;
    validate_input_size(input, config)?;
    Ok(())
}

// =============================================================================
// Router Trait
// =============================================================================

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
    ) -> SubscriptionFuture<'a>;
}

// =============================================================================
// Plugin State
// =============================================================================

struct RouterState(Arc<dyn DynRouter>);
struct SubscriptionState(Arc<SubscriptionManager>);
struct ConfigState(RpcConfig);

// =============================================================================
// Request Types
// =============================================================================

/// Request payload for subscription operations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscribeRequest {
    /// Optional subscription ID (generated if empty)
    #[serde(default)]
    pub id: String,
    /// Procedure path
    pub path: String,
    /// Input data
    pub input: serde_json::Value,
    /// Last event ID for resumption
    #[serde(default)]
    pub last_event_id: Option<String>,
}

// =============================================================================
// Tauri Commands
// =============================================================================

#[tauri::command]
async fn rpc_call(
    path: String,
    input: serde_json::Value,
    state: State<'_, RouterState>,
    config: State<'_, ConfigState>,
) -> Result<serde_json::Value, String> {
    validate_rpc_input(&path, &input, &config.0)
        .map_err(|e| serde_json::to_string(&e).unwrap_or_else(|_| e.to_string()))?;
    state
        .0
        .call(&path, input)
        .await
        .map_err(|e| serde_json::to_string(&e).unwrap_or_else(|_| e.to_string()))
}

#[tauri::command]
async fn rpc_call_batch(
    batch: BatchRequest,
    state: State<'_, RouterState>,
    config: State<'_, ConfigState>,
) -> Result<BatchResponse, String> {
    // Use default batch config for now
    // TODO: Make batch config configurable via RpcConfig
    let batch_config = BatchConfig::default();

    // Validate batch
    batch
        .validate(&batch_config)
        .map_err(|e| serde_json::to_string(&e).unwrap_or_else(|_| e.to_string()))?;

    // Validate each request's path and input
    for req in &batch.requests {
        validate_rpc_input(&req.path, &req.input, &config.0)
            .map_err(|e| serde_json::to_string(&e).unwrap_or_else(|_| e.to_string()))?;
    }

    // Execute batch using parallel execution
    let futures: Vec<_> = batch
        .requests
        .iter()
        .map(|req| {
            let id = req.id.clone();
            let path = req.path.clone();
            let input = req.input.clone();
            let router = state.0.clone();
            async move {
                match router.call(&path, input).await {
                    Ok(data) => BatchResult::success(id, data),
                    Err(error) => BatchResult::error(id, error),
                }
            }
        })
        .collect();

    let results = futures::future::join_all(futures).await;
    Ok(BatchResponse::new(results))
}

#[tauri::command]
fn rpc_procedures(state: State<'_, RouterState>) -> Vec<String> {
    state.0.procedures()
}

#[tauri::command]
async fn rpc_subscribe<R: Runtime>(
    request: SubscribeRequest,
    app: AppHandle<R>,
    router_state: State<'_, RouterState>,
    sub_state: State<'_, SubscriptionState>,
    config: State<'_, ConfigState>,
) -> Result<String, String> {
    let SubscribeRequest {
        id,
        path,
        input,
        last_event_id,
    } = request;

    validate_rpc_input(&path, &input, &config.0)
        .map_err(|e| serde_json::to_string(&e).unwrap_or_else(|_| e.to_string()))?;

    let subscription_id = if id.is_empty() {
        generate_subscription_id()
    } else {
        validate_subscription_id(&id)
            .map_err(|e| serde_json::to_string(&e).unwrap_or_else(|_| e.to_string()))?
    };

    if !router_state.0.is_subscription(&path) {
        return Err(serde_json::to_string(&RpcError::bad_request(format!(
            "'{}' is not a subscription procedure",
            path
        )))
        .unwrap());
    }

    let sub_ctx = SubscriptionContext::new(subscription_id, last_event_id);
    let signal = sub_ctx.signal();
    let handle =
        crate::subscription::SubscriptionHandle::new(subscription_id, path.clone(), signal.clone());
    sub_state.0.subscribe(handle).await;

    let event_name = format!("rpc:subscription:{}", subscription_id);
    let router = router_state.0.clone();
    let sub_manager = sub_state.0.clone();

    // Use spawn_subscription for tracked task management instead of tokio::spawn
    // This ensures proper cleanup during shutdown
    sub_state
        .0
        .spawn_subscription(subscription_id, async move {
            match router.subscribe(&path, input, sub_ctx).await {
                Ok(mut stream) => {
                    while let Some(event) = stream.recv().await {
                        if signal.is_cancelled() {
                            break;
                        }
                        let sub_event = SubscriptionEvent::data(event);
                        if app.emit(&event_name, &sub_event).is_err() {
                            break;
                        }
                    }
                    if !signal.is_cancelled() {
                        let _ = app.emit(&event_name, &SubscriptionEvent::completed());
                    }
                }
                Err(err) => {
                    let _ = app.emit(&event_name, &SubscriptionEvent::error(err));
                }
            }
            sub_manager.unsubscribe(&subscription_id).await;
        })
        .await;

    Ok(subscription_id.to_string())
}

#[tauri::command]
async fn rpc_unsubscribe(
    id: String,
    sub_state: State<'_, SubscriptionState>,
) -> Result<bool, String> {
    let subscription_id = validate_subscription_id(&id)
        .map_err(|e| serde_json::to_string(&e).unwrap_or_else(|_| e.to_string()))?;
    Ok(sub_state.0.unsubscribe(&subscription_id).await)
}

#[tauri::command]
async fn rpc_subscription_count(sub_state: State<'_, SubscriptionState>) -> Result<usize, String> {
    Ok(sub_state.0.count().await)
}

// =============================================================================
// Plugin Initialization
// =============================================================================

/// Initialize the RPC plugin with a router
///
/// # Example
/// ```rust,ignore
/// tauri::Builder::default()
///     .plugin(tauri_plugin_rpc::init(create_router()))
///     .run(tauri::generate_context!())
/// ```
pub fn init<R, D>(router: D) -> TauriPlugin<R>
where
    R: Runtime,
    D: DynRouter + 'static,
{
    init_with_config(router, RpcConfig::default())
}

/// Initialize the RPC plugin with a router and custom configuration
///
/// # Panics
///
/// Panics if the configuration is invalid (e.g., max_input_size is 0).
/// Use `RpcConfig::validate()` to check configuration before passing it.
///
/// # Example
/// ```rust,ignore
/// use tauri_plugin_rpc::RpcConfig;
///
/// let config = RpcConfig::new()
///     .with_max_input_size(512 * 1024)
///     .with_debug_logging(true);
///
/// tauri::Builder::default()
///     .plugin(tauri_plugin_rpc::init_with_config(create_router(), config))
///     .run(tauri::generate_context!())
/// ```
pub fn init_with_config<R, D>(router: D, config: RpcConfig) -> TauriPlugin<R>
where
    R: Runtime,
    D: DynRouter + 'static,
{
    // Validate configuration at startup
    if let Err(e) = config.validate() {
        panic!("Invalid RPC configuration: {}", e);
    }

    let router: Arc<dyn DynRouter> = Arc::new(router);
    let subscription_manager = Arc::new(SubscriptionManager::new());

    Builder::new("rpc")
        .invoke_handler(tauri::generate_handler![
            rpc_call,
            rpc_call_batch,
            rpc_procedures,
            rpc_subscribe,
            rpc_unsubscribe,
            rpc_subscription_count
        ])
        .setup(move |app, _api| {
            app.manage(RouterState(router.clone()));
            app.manage(SubscriptionState(subscription_manager.clone()));
            app.manage(ConfigState(config.clone()));
            Ok(())
        })
        .build()
}
