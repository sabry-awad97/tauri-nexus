//! Tauri RPC Plugin
//!
//! Type-safe RPC with automatic TypeScript generation via ts-rs.

use tauri::{
    plugin::{Builder, TauriPlugin},
    Manager, Runtime,
};

mod commands;
mod error;
pub mod types;

pub use commands::*;
pub use error::*;
pub use types::*;

use std::sync::Mutex;

/// Plugin state
pub struct RpcState {
    pub users: Mutex<Vec<User>>,
    pub next_id: Mutex<u32>,
}

impl Default for RpcState {
    fn default() -> Self {
        Self {
            users: Mutex::new(vec![
                User {
                    id: 1,
                    name: "Alice".into(),
                    email: "alice@example.com".into(),
                    created_at: "2024-01-01T00:00:00Z".into(),
                },
                User {
                    id: 2,
                    name: "Bob".into(),
                    email: "bob@example.com".into(),
                    created_at: "2024-01-02T00:00:00Z".into(),
                },
            ]),
            next_id: Mutex::new(3),
        }
    }
}

/// Initialize the RPC plugin
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("rpc")
        .invoke_handler(tauri::generate_handler![
            commands::greet,
            commands::get_user,
            commands::list_users,
            commands::create_user,
            commands::update_user,
            commands::delete_user,
        ])
        .setup(|app, _api| {
            app.manage(RpcState::default());
            Ok(())
        })
        .build()
}

/// Extension trait for accessing the RPC plugin
pub trait RpcExt<R: Runtime> {
    fn rpc(&self) -> &RpcState;
}

impl<R: Runtime, T: Manager<R>> RpcExt<R> for T {
    fn rpc(&self) -> &RpcState {
        self.state::<RpcState>().inner()
    }
}
