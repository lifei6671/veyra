fn main() {
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "bootstrap_status",
            "runtime_observation_snapshot",
            "start_managed_observation_runtime",
            "stop_managed_observation_runtime",
            "show_main_window",
            "list_subscriptions",
            "import_subscription",
            "update_subscription",
            "get_subscription_settings",
            "edit_subscription",
            "activate_subscription",
            "get_subscription_share_url",
            "delete_subscription",
            "get_subscription_document",
            "format_subscription_document",
            "save_subscription_document",
            "get_running_configuration",
        ]),
    ))
    .expect("failed to build the Tauri application manifest");
}
