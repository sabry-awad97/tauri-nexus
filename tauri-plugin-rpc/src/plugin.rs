//! Tauri plugin integration

use crate::RpcError;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tauri::{
    plugin::{Builder, TauriPlugin},
    Manager, Runtime, State, 
};

/// Type-erased router trait
pub trait DynRouter: Send + Sync {
    fn call<'a>(
        &'a self,
        path: &'a str,
        input: serde_json::Value,
    ) -> Pin<Box<dyn Future<Output = Result<serde_json::Value, RpcError>> + Send + 'a>>;
    
    fn procedures(&self) -> Vec<String>;
}

/// Wrapper to store router in Tauri state
pub struct RouterState(pub Arc<dyn DynRouter>);

/// Tauri command to handle RPC calls
#[tauri::command]
async fn rpc_call(
    path: String,
    input: serde_json::Value,
    state: State<'_, RouterState>,
) -> Result<serde_json::Value, String> {
    match state.0.call(&path, input).await {
        Ok(data) => Ok(data),
        Err(e) => Err(serde_json::to_string(&e).unwrap_or_else(|_| e.to_string())),
    }
}

/// Tauri command to list available procedures
#[tauri::command]
fn rpc_procedures(state: State<'_, RouterState>) -> Vec<String> {
    state.0.procedures()
}

/// Initialize the RPC plugin with a router
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
