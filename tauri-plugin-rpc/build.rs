const COMMANDS: &[&str] = &["rpc_call"];

fn main() {
    tauri_plugin::Builder::new(COMMANDS).build();
}
