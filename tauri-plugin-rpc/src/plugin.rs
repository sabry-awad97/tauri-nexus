//! Tauri plugin integration

use crate::RpcError;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tauri::{
    plugin::{Builder, TauriPlugin},
    Manager, Runtime, State,
};

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
}

/// Router state wrapper
struct RouterState(Arc<dyn DynRouter>);

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

    Builder::new("rpc")
        .invoke_handler(tauri::generate_handler![rpc_call, rpc_procedures])
        .setup(move |app, _api| {
            app.manage(RouterState(router.clone()));
            Ok(())
        })
        .build()
}
