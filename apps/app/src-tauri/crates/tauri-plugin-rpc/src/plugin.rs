//! Tauri plugin integration

use crate::RpcError;
use crate::batch::{BatchRequest, BatchResponse, BatchResult};
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
use tracing::{debug, info, warn};

// =============================================================================
// Configuration Constants
// =============================================================================

/// Default timeout for subscription shutdown operations.
///
/// This timeout prevents the application from hanging during shutdown
/// if subscriptions don't clean up promptly.
const SHUTDOWN_TIMEOUT_SECS: u64 = 5;

/// Prefix for subscription event names in Tauri event system.
///
/// Format: "rpc:subscription:{subscription_id}"
const SUBSCRIPTION_EVENT_PREFIX: &str = "rpc:subscription:";

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

/// Convert RpcError to String for Tauri command responses.
///
/// Attempts to serialize as JSON first, falls back to Display trait.
/// This ensures consistent error formatting across all commands.
fn serialize_error(error: &RpcError) -> String {
    serde_json::to_string(error).unwrap_or_else(|_| error.to_string())
}

/// Generate a unique request ID using UUID v7 (time-ordered).
///
/// UUID v7 provides:
/// - Time-based ordering for better database indexing
/// - Monotonic sorting within the same millisecond
/// - Compatibility with standard UUID format
fn generate_request_id() -> String {
    uuid::Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext)).to_string()
}

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

    // Use iterator methods for cleaner validation
    if let Some(invalid_char) = path
        .chars()
        .find(|&ch| !ch.is_ascii_alphanumeric() && ch != '_' && ch != '.')
    {
        return Err(RpcError::validation(format!(
            "Procedure path contains invalid character: '{}'",
            invalid_char
        )));
    }

    Ok(())
}

/// Validate input size against configuration limit.
///
/// Uses heuristics to avoid unnecessary serialization for small inputs:
/// - Null: 4 bytes
/// - Boolean: 5 bytes  
/// - Number: ~20 bytes (conservative estimate)
/// - String: length + 2 (quotes)
/// - Small arrays/objects: skip serialization if obviously small
pub fn validate_input_size(input: &serde_json::Value, config: &RpcConfig) -> Result<(), RpcError> {
    use serde_json::Value;
    
    // Fast path: estimate size for simple types
    let estimated_size = match input {
        Value::Null => 4,
        Value::Bool(_) => 5,
        Value::Number(_) => 20, // Conservative estimate
        Value::String(s) => s.len() + 2, // Add quotes
        Value::Array(arr) if arr.is_empty() => 2, // "[]"
        Value::Object(obj) if obj.is_empty() => 2, // "{}"
        _ => {
            // Complex type: need actual serialization
            let size = serde_json::to_vec(input)
                .map(|v| v.len())
                .unwrap_or(0);
            
            if size > config.max_input_size {
                return Err(RpcError::payload_too_large(format!(
                    "Input size {} bytes exceeds maximum {} bytes",
                    size, config.max_input_size
                )));
            }
            return Ok(());
        }
    };
    
    // Early return for small inputs
    if estimated_size > config.max_input_size {
        return Err(RpcError::payload_too_large(format!(
            "Input size ~{} bytes exceeds maximum {} bytes",
            estimated_size, config.max_input_size
        )));
    }
    
    Ok(())
}

/// Validate subscription ID format when provided by client.
///
/// This function accepts both formats for backward compatibility:
/// - With prefix: "sub_01234567-89ab-7cde-8f01-234567890abc"
/// - Without prefix: "01234567-89ab-7cde-8f01-234567890abc"
pub fn validate_subscription_id(id: &str) -> Result<SubscriptionId, RpcError> {
    if id.is_empty() {
        return Err(RpcError::validation("Subscription ID cannot be empty"));
    }
    SubscriptionId::parse_lenient(id)
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
// Subscription Event Handling
// =============================================================================

/// Handle subscription event stream and emit events to frontend.
///
/// This function manages the event loop for a subscription:
/// - Receives events from the stream
/// - Checks for cancellation
/// - Emits events to the frontend via Tauri event system
/// - Handles completion and errors
///
/// Returns the total number of events processed.
async fn handle_subscription_events<R: Runtime>(
    app: AppHandle<R>,
    subscription_id: SubscriptionId,
    path: String,
    event_name: String,
    mut stream: mpsc::Receiver<Event<serde_json::Value>>,
    signal: Arc<crate::subscription::CancellationSignal>,
) -> u64 {
    let mut event_count = 0u64;

    while let Some(event) = stream.recv().await {
        if signal.is_cancelled() {
            debug!(
                subscription_id = %subscription_id,
                path = %path,
                event_count = %event_count,
                "Subscription cancelled"
            );
            break;
        }

        event_count += 1;
        let sub_event = SubscriptionEvent::data(event);

        if app.emit(&event_name, &sub_event).is_err() {
            debug!(
                subscription_id = %subscription_id,
                path = %path,
                "Subscription emit failed, closing"
            );
            break;
        }
    }

    // Send completion event if not cancelled
    if !signal.is_cancelled() {
        info!(
            subscription_id = %subscription_id,
            path = %path,
            event_count = %event_count,
            "Subscription completed"
        );
        let _ = app.emit(&event_name, &SubscriptionEvent::completed());
    }

    event_count
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
        serialize_error(&e)
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
        serialize_error(&e)
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
    // Use batch config from RpcConfig
    let batch_config = &config.0.batch_config;

    // Validate batch
    if let Err(e) = batch.validate(batch_config) {
        warn!(error = %e, "Batch validation failed");
        return Err(serialize_error(&e));
    }

    // Validate each request's path and input
    for req in &batch.requests {
        if let Err(e) = validate_rpc_input(&req.path, &req.input, &config.0) {
            warn!(
                request_id = %req.id,
                path = %req.path,
                error = %e,
                "Batch request validation failed"
            );
            return Err(serialize_error(&e));
        }
    }

    debug!(batch_size = batch.requests.len(), "Executing batch request");

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
                    Ok(data) => {
                        debug!(request_id = %id, path = %path, "Batch request succeeded");
                        BatchResult::success(id, data)
                    }
                    Err(error) => {
                        warn!(
                            request_id = %id,
                            path = %path,
                            error_code = %error.code,
                            error_message = %error.message,
                            "Batch request failed"
                        );
                        BatchResult::error(id, error)
                    }
                }
            }
        })
        .collect();

    let results = futures::future::join_all(futures).await;
    let response = BatchResponse::new(results);

    debug!(
        success_count = response.success_count(),
        error_count = response.error_count(),
        "Batch request completed"
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
) -> Result<String, String> {
    let SubscribeRequest {
        id,
        path,
        input,
        last_event_id,
    } = request;

    validate_rpc_input(&path, &input, &config.0).map_err(|e| serialize_error(&e))?;

    let subscription_id = if id.is_empty() {
        generate_subscription_id()
    } else {
        validate_subscription_id(&id).map_err(|e| serialize_error(&e))?
    };

    if !router_state.0.is_subscription(&path) {
        warn!(
            subscription_id = %subscription_id,
            path = %path,
            "Attempted to subscribe to non-subscription procedure"
        );
        return Err(serialize_error(&RpcError::bad_request(format!(
            "'{}' is not a subscription procedure",
            path
        ))));
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

    let event_name = format!("{}{}", SUBSCRIPTION_EVENT_PREFIX, subscription_id);
    let router = router_state.0.clone();
    let sub_manager = sub_state.0.clone();
    let path_clone = path.clone();
    let app_clone = app.clone();

    // Use spawn_subscription for tracked task management instead of tokio::spawn
    // This ensures proper cleanup during shutdown
    sub_state
        .0
        .spawn_subscription(subscription_id, async move {
            match router.subscribe(&path_clone, input, sub_ctx).await {
                Ok(stream) => {
                    handle_subscription_events(
                        app_clone,
                        subscription_id,
                        path_clone,
                        event_name,
                        stream,
                        signal,
                    )
                    .await;
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
    let subscription_id = validate_subscription_id(&id).map_err(|e| serialize_error(&e))?;

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
    // Clone subscription manager for shutdown handler
    let shutdown_manager = subscription_manager.clone();

    // Log plugin initialization
    info!(
        max_input_size = config.max_input_size,
        channel_buffer = config.default_channel_buffer,
        debug_logging = config.debug_logging,
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
            Ok(())
        })
        .on_drop(move |_app| {
            info!("RPC plugin shutting down");
            
            let manager = shutdown_manager.clone();
            
            // Try to use current runtime handle first
            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                handle.spawn(async move {
                    let timeout = std::time::Duration::from_secs(SHUTDOWN_TIMEOUT_SECS);
                    if tokio::time::timeout(timeout, manager.shutdown()).await.is_err() {
                        warn!(
                            timeout_secs = SHUTDOWN_TIMEOUT_SECS,
                            "Subscription shutdown timed out"
                        );
                    } else {
                        debug!("Subscription shutdown completed successfully");
                    }
                });
            } else {
                // Fallback: create new runtime
                std::thread::spawn(move || {
                    match tokio::runtime::Runtime::new() {
                        Ok(rt) => {
                            rt.block_on(async {
                                let timeout = std::time::Duration::from_secs(SHUTDOWN_TIMEOUT_SECS);
                                if tokio::time::timeout(timeout, manager.shutdown()).await.is_err() {
                                    warn!(
                                        timeout_secs = SHUTDOWN_TIMEOUT_SECS,
                                        "Subscription shutdown timed out"
                                    );
                                } else {
                                    debug!("Subscription shutdown completed successfully");
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
