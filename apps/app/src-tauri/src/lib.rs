//! Tauri Application

pub mod rpc;

use rpc::create_router;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_rpc::init(create_router()))
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
