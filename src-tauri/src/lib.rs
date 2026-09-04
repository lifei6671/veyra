pub mod application;
pub mod commands;
pub mod domain;
pub mod platform;
pub mod singbox;
pub mod storage;
pub mod subscription;

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use tauri::{
    Emitter, Manager, WindowEvent,
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
};

use application::{
    managed_observation_runtime::ManagedObservationRuntimeController,
    observability::InMemoryRuntimeObservations,
};

const MAIN_WINDOW_LABEL: &str = "main";
const TRAY_SHOW_ID: &str = "show-main-window";
const TRAY_QUIT_ID: &str = "quit-application";
const RUNTIME_OBSERVATION_DELTA_EVENT: &str = "runtime_observation_delta";

/// 主窗口是否可见仅用于限制内存观测事件；它不保存到 AppState 或磁盘。
#[derive(Clone)]
pub(crate) struct MainWindowVisibility {
    visible: Arc<AtomicBool>,
}

impl MainWindowVisibility {
    fn visible() -> Self {
        Self {
            visible: Arc::new(AtomicBool::new(true)),
        }
    }

    fn mark_hidden(&self) {
        self.visible.store(false, Ordering::Release);
    }

    pub(crate) fn mark_visible(&self) {
        self.visible.store(true, Ordering::Release);
    }

    fn is_visible(&self) -> bool {
        self.visible.load(Ordering::Acquire)
    }
}

pub fn run() {
    tauri::Builder::default()
        .manage(InMemoryRuntimeObservations::new_mock())
        .manage(MainWindowVisibility::visible())
        .setup(|app| {
            let resource_root = app.path().resource_dir()?;
            let app_local_data_root = app.path().app_local_data_dir()?;
            let observations = (*app.state::<InMemoryRuntimeObservations>()).clone();
            app.manage(ManagedObservationRuntimeController::new(
                resource_root,
                app_local_data_root,
                observations,
            ));
            configure_tray(app)
        })
        .on_window_event(|window, event| {
            if window.label() == MAIN_WINDOW_LABEL
                && let WindowEvent::CloseRequested { api, .. } = event
            {
                api.prevent_close();
                if window.hide().is_ok() {
                    window.state::<MainWindowVisibility>().mark_hidden();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::bootstrap_status,
            commands::runtime_observation_snapshot,
            commands::start_managed_observation_runtime,
            commands::stop_managed_observation_runtime,
            commands::show_main_window
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Veyra");
}

fn configure_tray(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let observations = (*app.state::<InMemoryRuntimeObservations>()).clone();
    let visibility = (*app.state::<MainWindowVisibility>()).clone();
    let app_handle = app.handle().clone();
    observations.install_delta_sink(move |delta| {
        if visibility.is_visible() {
            let _ = app_handle.emit_to(
                MAIN_WINDOW_LABEL,
                RUNTIME_OBSERVATION_DELTA_EVENT,
                commands::RuntimeObservationResponse::from(delta),
            );
        }
    });

    let show = MenuItem::with_id(app, TRAY_SHOW_ID, "显示 Veyra", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, TRAY_QUIT_ID, "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;
    let mut tray = TrayIconBuilder::with_id("veyra-tray")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            TRAY_SHOW_ID => {
                let observations = app.state::<InMemoryRuntimeObservations>();
                let _ = commands::restore_main_window(app, &observations);
            }
            TRAY_QUIT_ID
                if app
                    .state::<ManagedObservationRuntimeController>()
                    .shutdown() =>
            {
                app.exit(0);
            }
            _ => {}
        });

    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::observability::RuntimeObservationPort;
    use std::sync::Mutex;

    #[test]
    fn hidden_window_drops_deltas_and_visible_window_resumes_without_backlog() {
        let observations = InMemoryRuntimeObservations::new_mock();
        let visibility = MainWindowVisibility::visible();
        let emitted = Arc::new(Mutex::new(Vec::new()));
        let emitted_revisions = Arc::clone(&emitted);
        let bridge_visibility = visibility.clone();
        observations.install_delta_sink(move |delta| {
            if bridge_visibility.is_visible() {
                emitted_revisions
                    .lock()
                    .expect("test event bridge lock")
                    .push(delta.revision);
            }
        });

        observations.record_connection_count(1);
        visibility.mark_hidden();
        observations.record_connection_count(2);
        visibility.mark_visible();
        observations.record_connection_count(3);

        assert_eq!(*emitted.lock().expect("test revisions lock"), vec![1, 3]);
        assert_eq!(observations.snapshot().connections.active, 3);
    }
}
