const COMMANDS: &[&str] = &[
    "greet",
    "get_user",
    "list_users",
    "create_user",
    "update_user",
    "delete_user",
];

fn main() {
    tauri_plugin::Builder::new(COMMANDS).build();
}
