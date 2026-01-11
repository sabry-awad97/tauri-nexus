const COMMANDS: &[&str] = &[
    "rpc_call",
    "rpc_call_batch",
    "rpc_procedures",
    "rpc_subscribe",
    "rpc_unsubscribe",
    "rpc_subscription_count",
];

fn main() {
    tauri_plugin::Builder::new(COMMANDS).build();
}
