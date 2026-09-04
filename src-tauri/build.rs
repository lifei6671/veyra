fn main() {
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "bootstrap_status",
            "runtime_observation_snapshot",
            "start_managed_observation_runtime",
            "stop_managed_observation_runtime",
            "show_main_window",
        ]),
    ))
    .expect("failed to build the Tauri application manifest");
}
