//! Generate TypeScript bindings via ts-rs
//! Run: cargo test -p tauri-plugin-rpc --test export_bindings

#[test]
fn export_bindings() {
    use tauri_plugin_rpc::types::*;
    use ts_rs::TS;

    User::export_all().expect("Failed to export User");
    CreateUserInput::export_all().expect("Failed to export CreateUserInput");
    UpdateUserInput::export_all().expect("Failed to export UpdateUserInput");
    PaginatedResponse::<User>::export_all().expect("Failed to export PaginatedResponse");
    SuccessResponse::export_all().expect("Failed to export SuccessResponse");
    PaginationInput::export_all().expect("Failed to export PaginationInput");

    println!("✅ TypeScript bindings exported to guest-js/bindings/");
}
