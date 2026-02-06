//! Tauri plugin integration

use crate::RpcError;
use crate::batch::{BatchRequest, BatchResponse, execute_batch};
use crate::config::{PluginConfig, RpcConfig};
use crate::subscription::{
    Event, SubscriptionContext, SubscriptionEvent, SubscriptionManager, generate_subscription_id,
    handle_subscription_events, handle_subscription_events_buffered, subscription_event_name,
};
use crate::validation::{validate_rpc_input, validate_subscription_id};
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tauri::{
    AppHandle, Emitter, Manager, Runtime, State,
    plugin::{Builder, TauriPlugin},
};
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

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
// Helper Functions
// =============================================================================

/// Generate a unique request ID using UUID v7 (time-ordered).
///
/// UUID v7 provides:
/// - Time-based ordering for better database indexing
/// - Monotonic sorting within the same millisecond
/// - Compatibility with standard UUID format
///
/// # Time-Ordering Benefits
///
/// UUID v7 uses a timestamp-based prefix, which provides several advantages:
/// - **Database Performance**: Sequential IDs improve B-tree index performance
/// - **Sortability**: Natural chronological ordering without additional fields
/// - **Uniqueness**: Combines timestamp with random bits for collision resistance
/// - **Compatibility**: Standard UUID format works with existing systems
fn generate_request_id() -> uuid::Uuid {
    uuid::Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext))
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
struct PluginConfigState(PluginConfig);

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
    let request_id = generate_request_id();
    let start = std::time::Instant::now();

    debug!(
        request_id = %request_id,
        path = %path,
        "RPC call started"
    );

    validate_rpc_input(&path, &input, &config.0).map_err(|e| {
        warn!(
            request_id = %request_id,
            path = %path,
            error = %e,
            "RPC input validation failed"
        );
        serde_json::to_string(&e).unwrap_or_else(|_| e.to_string())
    })?;

    let result = state.0.call(&path, input).await.map_err(|e| {
        let duration = start.elapsed();
        warn!(
            request_id = %request_id,
            path = %path,
            error_code = %e.code,
            error_message = %e.message,
            duration_ms = %duration.as_millis(),
            "RPC call failed"
        );
        serde_json::to_string(&e).unwrap_or_else(|_| e.to_string())
    });

    if result.is_ok() {
        let duration = start.elapsed();
        debug!(
            request_id = %request_id,
            path = %path,
            duration_ms = %duration.as_millis(),
            "RPC call completed"
        );
    }

    result
}

#[tauri::command]
async fn rpc_call_batch(
    batch: BatchRequest,
    state: State<'_, RouterState>,
    config: State<'_, ConfigState>,
) -> Result<BatchResponse, String> {
    // Use the new batch processor module
    let (response, metrics) = execute_batch(batch, state.0.clone(), &config.0)
        .await
        .map_err(|e| serde_json::to_string(&e).unwrap_or_else(|_| e.to_string()))?;

    debug!(
        total = metrics.total_requests,
        success = metrics.success_count,
        errors = metrics.error_count,
        duration_ms = metrics.duration_ms,
        "Batch execution completed"
    );

    Ok(response)
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
    plugin_config: State<'_, PluginConfigState>,
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
        warn!(
            subscription_id = %subscription_id,
            path = %path,
            "Attempted to subscribe to non-subscription procedure"
        );
        let error = RpcError::bad_request(format!("'{}' is not a subscription procedure", path));
        return Err(serde_json::to_string(&error).unwrap_or_else(|_| error.to_string()));
    }

    info!(
        subscription_id = %subscription_id,
        path = %path,
        last_event_id = ?last_event_id,
        "Subscription started"
    );

    let sub_ctx = SubscriptionContext::new(subscription_id, last_event_id);
    let signal = sub_ctx.signal();
    let handle =
        crate::subscription::SubscriptionHandle::new(subscription_id, path.clone(), signal.clone());
    sub_state.0.subscribe(handle);

    // Use the new subscription_event_name function from subscription_lifecycle
    let event_name =
        subscription_event_name(&plugin_config.0.subscription_event_prefix, &subscription_id);
    let router = router_state.0.clone();
    let sub_manager = sub_state.0.clone();
    let path_clone = path.clone();
    let app_clone = app.clone();
    let plugin_config_clone = plugin_config.0.clone();

    // Use spawn_subscription for tracked task management
    sub_state
        .0
        .spawn_subscription(subscription_id, async move {
            match router.subscribe(&path_clone, input, sub_ctx).await {
                Ok(stream) => {
                    // Use buffered handler if buffering is enabled
                    let _metrics = if plugin_config_clone.is_buffering_enabled() {
                        handle_subscription_events_buffered(
                            app_clone,
                            subscription_id,
                            path_clone,
                            event_name,
                            stream,
                            signal,
                            &plugin_config_clone,
                        )
                        .await
                    } else {
                        handle_subscription_events(
                            app_clone,
                            subscription_id,
                            path_clone,
                            event_name,
                            stream,
                            signal,
                        )
                        .await
                    };
                }
                Err(err) => {
                    warn!(
                        subscription_id = %subscription_id,
                        path = %path_clone,
                        error_code = %err.code,
                        error_message = %err.message,
                        "Subscription error"
                    );
                    let _ = app_clone.emit(&event_name, &SubscriptionEvent::error(err));
                }
            }
            sub_manager.unsubscribe(&subscription_id);
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

    let result = sub_state.0.unsubscribe(&subscription_id);

    if result {
        info!(
            subscription_id = %subscription_id,
            "Subscription unsubscribed"
        );
    } else {
        debug!(
            subscription_id = %subscription_id,
            "Unsubscribe called for non-existent subscription"
        );
    }

    Ok(result)
}

#[tauri::command]
async fn rpc_subscription_count(sub_state: State<'_, SubscriptionState>) -> Result<usize, String> {
    Ok(sub_state.0.count())
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
/// use tauri_plugin_rpc::{RpcConfig, PluginConfig};
/// use std::time::Duration;
///
/// let rpc_config = RpcConfig::new()
///     .with_max_input_size(512 * 1024)
///     .with_debug_logging(true);
///
/// let plugin_config = PluginConfig::new()
///     .with_shutdown_timeout(Duration::from_secs(10));
///
/// tauri::Builder::default()
///     .plugin(tauri_plugin_rpc::init_with_config(create_router(), rpc_config))
///     .run(tauri::generate_context!())
/// ```
pub fn init_with_config<R, D>(router: D, config: RpcConfig) -> TauriPlugin<R>
where
    R: Runtime,
    D: DynRouter + 'static,
{
    init_with_full_config(router, config, PluginConfig::default())
}

/// Initialize the RPC plugin with a router and full configuration (RPC + Plugin)
///
/// # Panics
///
/// Panics if either configuration is invalid.
///
/// # Example
/// ```rust,ignore
/// use tauri_plugin_rpc::{RpcConfig, PluginConfig};
/// use std::time::Duration;
///
/// let rpc_config = RpcConfig::new()
///     .with_max_input_size(512 * 1024);
///
/// let plugin_config = PluginConfig::new()
///     .with_shutdown_timeout(Duration::from_secs(10))
///     .with_event_prefix("custom:events:");
///
/// tauri::Builder::default()
///     .plugin(tauri_plugin_rpc::init_with_full_config(
///         create_router(),
///         rpc_config,
///         plugin_config
///     ))
///     .run(tauri::generate_context!())
/// ```
pub fn init_with_full_config<R, D>(
    router: D,
    config: RpcConfig,
    plugin_config: PluginConfig,
) -> TauriPlugin<R>
where
    R: Runtime,
    D: DynRouter + 'static,
{
    // Validate configurations at startup
    if let Err(e) = config.validate() {
        panic!("Invalid RPC configuration: {}", e);
    }
    if let Err(e) = plugin_config.validate() {
        panic!("Invalid Plugin configuration: {}", e);
    }

    let router: Arc<dyn DynRouter> = Arc::new(router);
    let subscription_manager = Arc::new(SubscriptionManager::new());
    let shutdown_manager = subscription_manager.clone();
    let shutdown_timeout = plugin_config.shutdown_timeout;

    // Log plugin initialization
    info!(
        max_input_size = config.max_input_size,
        channel_buffer = config.default_channel_buffer,
        debug_logging = config.debug_logging,
        shutdown_timeout_secs = shutdown_timeout.as_secs(),
        event_prefix = %plugin_config.subscription_event_prefix,
        "RPC plugin initializing"
    );

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
            let procedure_count = router.procedures().len();
            debug!(
                procedure_count = procedure_count,
                "RPC plugin setup complete"
            );
            app.manage(RouterState(router.clone()));
            app.manage(SubscriptionState(subscription_manager.clone()));
            app.manage(ConfigState(config.clone()));
            app.manage(PluginConfigState(plugin_config.clone()));
            Ok(())
        })
        .on_drop(move |_app| {
            info!("RPC plugin shutting down");

            let manager = shutdown_manager.clone();

            // Try to use current runtime handle first
            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                handle.spawn(async move {
                    // Use the shutdown method on SubscriptionManager
                    let result = manager.shutdown_plugin(shutdown_timeout).await;

                    if result.is_success() {
                        debug!(
                            cancelled = result.cancelled_count,
                            duration_ms = result.duration_ms,
                            "Subscription shutdown completed successfully"
                        );
                    } else {
                        warn!(
                            active = result.active_subscriptions,
                            cancelled = result.cancelled_count,
                            failed = result.failed_count,
                            "Subscription shutdown completed with issues: {}",
                            result.status_message()
                        );
                    }
                });
            } else {
                // Fallback: create new runtime
                std::thread::spawn(move || {
                    match tokio::runtime::Runtime::new() {
                        Ok(rt) => {
                            rt.block_on(async {
                                let result = manager.shutdown_plugin(shutdown_timeout).await;

                                if result.is_success() {
                                    debug!(
                                        cancelled = result.cancelled_count,
                                        duration_ms = result.duration_ms,
                                        "Subscription shutdown completed successfully"
                                    );
                                } else {
                                    warn!(
                                        active = result.active_subscriptions,
                                        cancelled = result.cancelled_count,
                                        failed = result.failed_count,
                                        "Subscription shutdown completed with issues: {}",
                                        result.status_message()
                                    );
                                }
                            });
                        }
                        Err(e) => {
                            warn!(
                                error = %e,
                                "Failed to create runtime for shutdown, subscriptions may not clean up properly"
                            );
                        }
                    }
                });
            }
        })
        .build()
}
