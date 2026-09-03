pub mod application;
pub mod commands;
pub mod domain;
pub mod platform;
pub mod singbox;
pub mod storage;
pub mod subscription;

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![commands::bootstrap_status])
        .run(tauri::generate_context!())
        .expect("failed to run Veyra");
}
