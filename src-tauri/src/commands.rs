use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::application::managed_observation_runtime::{
    ManagedObservationRuntimeController, ManagedRuntimeStartResult, ManagedRuntimeStopResult,
};
use crate::application::observability::{
    InMemoryRuntimeObservations, ObservationLogCategory, ObservationLogLevel, ObservationSource,
    ObservedCaptureMode, ObservedSidecarLifecycle, RuntimeObservationDelta, RuntimeObservationPort,
    RuntimeObservationSnapshot, TrafficHistoryPoint,
};

const MAIN_WINDOW_LABEL: &str = "main";
const RUNTIME_OBSERVATION_DELTA_EVENT: &str = "runtime_observation_delta";

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapStatus {
    application: &'static str,
    status: &'static str,
}

#[tauri::command]
pub fn bootstrap_status() -> BootstrapStatus {
    BootstrapStatus {
        application: "Veyra",
        status: "ready",
    }
}

/// 仅返回封闭、脱敏且带明确来源标记的内存观测快照。
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeObservationResponse {
    source: &'static str,
    revision: u64,
    observed_at_ms: u64,
    traffic_history: Vec<TrafficHistoryPointResponse>,
    capture_mode: &'static str,
    sidecar_lifecycle: &'static str,
    upload_rate_bps: u64,
    download_rate_bps: u64,
    upload_total_bytes: u64,
    download_total_bytes: u64,
    connection_count: u32,
    log_summary: Vec<ObservationLogSummaryResponse>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrafficHistoryPointResponse {
    sampled_at_ms: u64,
    upload_rate_bps: u64,
    download_rate_bps: u64,
}

impl From<TrafficHistoryPoint> for TrafficHistoryPointResponse {
    fn from(point: TrafficHistoryPoint) -> Self {
        Self {
            sampled_at_ms: point.sampled_at_ms,
            upload_rate_bps: point.upload_rate_bps,
            download_rate_bps: point.download_rate_bps,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObservationLogSummaryResponse {
    category: &'static str,
    level: &'static str,
    occurrences: u32,
}

impl From<RuntimeObservationSnapshot> for RuntimeObservationResponse {
    fn from(snapshot: RuntimeObservationSnapshot) -> Self {
        let log_summary = snapshot
            .latest_log
            .as_ref()
            .map(|summary| ObservationLogSummaryResponse {
                category: map_log_category(summary.category),
                level: map_log_level(summary.level),
                occurrences: 1,
            })
            .into_iter()
            .collect();

        Self {
            source: map_source(snapshot.source),
            revision: snapshot.revision,
            observed_at_ms: snapshot.observed_at_ms,
            traffic_history: snapshot
                .traffic_history
                .into_iter()
                .map(Into::into)
                .collect(),
            capture_mode: map_capture_mode(snapshot.capture_mode),
            sidecar_lifecycle: map_sidecar_lifecycle(snapshot.sidecar_lifecycle),
            upload_rate_bps: snapshot.traffic.upload_bytes_per_second,
            download_rate_bps: snapshot.traffic.download_bytes_per_second,
            upload_total_bytes: snapshot.traffic.upload_total_bytes,
            download_total_bytes: snapshot.traffic.download_total_bytes,
            connection_count: snapshot.connections.active,
            log_summary,
        }
    }
}

impl From<RuntimeObservationDelta> for RuntimeObservationResponse {
    fn from(delta: RuntimeObservationDelta) -> Self {
        let log_summary = delta
            .latest_log
            .as_ref()
            .map(|summary| ObservationLogSummaryResponse {
                category: map_log_category(summary.category),
                level: map_log_level(summary.level),
                occurrences: 1,
            })
            .into_iter()
            .collect();

        Self {
            source: map_source(delta.source),
            revision: delta.revision,
            observed_at_ms: delta.observed_at_ms,
            traffic_history: delta.traffic_history.into_iter().map(Into::into).collect(),
            capture_mode: map_capture_mode(delta.capture_mode),
            sidecar_lifecycle: map_sidecar_lifecycle(delta.sidecar_lifecycle),
            upload_rate_bps: delta.traffic.upload_bytes_per_second,
            download_rate_bps: delta.traffic.download_bytes_per_second,
            upload_total_bytes: delta.traffic.upload_total_bytes,
            download_total_bytes: delta.traffic.download_total_bytes,
            connection_count: delta.connections.active,
            log_summary,
        }
    }
}

/// 固定、无参数的只读 IPC 入口；不接受 endpoint、命令、路径或 secret。
#[tauri::command]
pub(crate) fn runtime_observation_snapshot(
    observations: State<'_, InMemoryRuntimeObservations>,
) -> RuntimeObservationResponse {
    observations.snapshot().into()
}

/// 固定、零参数的受管观测运行时启动入口；配置、路径、端口和 secret 都由后端拥有。
#[tauri::command]
pub(crate) fn start_managed_observation_runtime(
    runtime: State<'_, ManagedObservationRuntimeController>,
) -> ManagedRuntimeStartResult {
    runtime.start()
}

/// 固定、零参数的受管观测运行时停止入口；只处理当前 controller 所拥有的 child。
#[tauri::command]
pub(crate) fn stop_managed_observation_runtime(
    runtime: State<'_, ManagedObservationRuntimeController>,
) -> ManagedRuntimeStopResult {
    runtime.stop()
}

/// 主窗口恢复失败时仅回传封闭错误类别，不泄露平台细节或路径。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ShowMainWindowError {
    MainWindowUnavailable,
    WindowOperationFailed,
    ObservationNotificationFailed,
}

/// 固定、无参数的主窗口恢复命令；只恢复已有窗口并发出当前安全快照。
#[tauri::command]
pub(crate) fn show_main_window(
    app: AppHandle,
    observations: State<'_, InMemoryRuntimeObservations>,
) -> Result<(), ShowMainWindowError> {
    restore_main_window(&app, &observations)
}

pub(crate) fn restore_main_window(
    app: &AppHandle,
    observations: &InMemoryRuntimeObservations,
) -> Result<(), ShowMainWindowError> {
    let window = app
        .get_webview_window(MAIN_WINDOW_LABEL)
        .ok_or(ShowMainWindowError::MainWindowUnavailable)?;
    window
        .show()
        .and_then(|_| window.set_focus())
        .map_err(|_| ShowMainWindowError::WindowOperationFailed)?;
    app.state::<crate::MainWindowVisibility>().mark_visible();
    app.emit_to(
        MAIN_WINDOW_LABEL,
        RUNTIME_OBSERVATION_DELTA_EVENT,
        RuntimeObservationResponse::from(observations.snapshot()),
    )
    .map_err(|_| ShowMainWindowError::ObservationNotificationFailed)
}

fn map_source(source: ObservationSource) -> &'static str {
    match source {
        ObservationSource::MockOnly => "mock",
        ObservationSource::ManagedSidecar => "runtime",
    }
}

fn map_capture_mode(mode: ObservedCaptureMode) -> &'static str {
    match mode {
        ObservedCaptureMode::NotObserved => "off",
        ObservedCaptureMode::Off => "off",
        ObservedCaptureMode::SystemProxy => "systemProxy",
        ObservedCaptureMode::RecoveryRequired => "recoveryRequired",
    }
}

fn map_sidecar_lifecycle(lifecycle: ObservedSidecarLifecycle) -> &'static str {
    match lifecycle {
        ObservedSidecarLifecycle::NotObserved => "notAttached",
        ObservedSidecarLifecycle::Stopped => "stopped",
        ObservedSidecarLifecycle::Ready => "ready",
        ObservedSidecarLifecycle::RecoveryRequired => "recoveryRequired",
    }
}

fn map_log_category(category: ObservationLogCategory) -> &'static str {
    match category {
        ObservationLogCategory::Runtime => "runtime",
        ObservationLogCategory::Connectivity => "proxy",
        ObservationLogCategory::Recovery => "subscription",
    }
}

fn map_log_level(level: ObservationLogLevel) -> &'static str {
    match level {
        ObservationLogLevel::Info => "info",
        ObservationLogLevel::Warning => "warn",
        ObservationLogLevel::Error => "error",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::observability::{
        InMemoryRuntimeObservations, ObservationLogCategory, ObservationLogLevel,
        RuntimeObservationPort,
    };

    #[test]
    fn returns_only_fixed_bootstrap_information() {
        assert_eq!(
            bootstrap_status(),
            super::BootstrapStatus {
                application: "Veyra",
                status: "ready",
            }
        );
    }

    #[test]
    fn observation_response_exposes_only_fixed_safe_fields() {
        let observations = InMemoryRuntimeObservations::new_mock();
        observations.record_runtime_log(
            ObservationLogLevel::Error,
            ObservationLogCategory::Connectivity,
            "http://127.0.0.1:9090 fixture-secret-token",
        );

        let response = RuntimeObservationResponse::from(observations.snapshot());

        assert_eq!(response.source, "mock");
        assert_eq!(response.log_summary.len(), 1);
        assert_eq!(response.log_summary[0].category, "proxy");
        assert_eq!(response.log_summary[0].level, "error");
        let encoded = serde_json::to_string(&response).expect("observation response serializes");
        assert!(!encoded.contains("fixture-secret-token"));
        assert!(!encoded.contains("127.0.0.1"));
        assert!(!encoded.contains("message"));
    }

    #[test]
    fn delta_response_uses_the_same_fixed_safe_dto() {
        let observations = InMemoryRuntimeObservations::new_mock();
        let subscription = observations.subscribe();
        observations.record_connection_count(2);
        let delta = subscription
            .take_latest_delta()
            .expect("subscription receives the latest delta");

        let response = RuntimeObservationResponse::from(delta);

        assert_eq!(response.source, "mock");
        assert_eq!(response.connection_count, 2);
        assert_eq!(response.revision, 1);
    }

    #[test]
    fn traffic_history_response_has_only_fixed_timing_and_rate_fields() {
        let observations = InMemoryRuntimeObservations::new_mock();
        let mut snapshot = observations.snapshot();
        snapshot.observed_at_ms = 2_000;
        snapshot.traffic_history = vec![TrafficHistoryPoint {
            sampled_at_ms: 1_000,
            upload_rate_bps: 100,
            download_rate_bps: 200,
        }];
        let delta_response =
            RuntimeObservationResponse::from(RuntimeObservationDelta::from(&snapshot));
        let snapshot_response = RuntimeObservationResponse::from(snapshot);
        assert_eq!(delta_response, snapshot_response);
        assert_eq!(
            serde_json::to_value(snapshot_response).expect("safe response"),
            serde_json::json!({
                "source": "mock",
                "revision": 0,
                "observedAtMs": 2_000,
                "trafficHistory": [{
                    "sampledAtMs": 1_000,
                    "uploadRateBps": 100,
                    "downloadRateBps": 200,
                }],
                "captureMode": "off",
                "sidecarLifecycle": "notAttached",
                "uploadRateBps": 0,
                "downloadRateBps": 0,
                "uploadTotalBytes": 0,
                "downloadTotalBytes": 0,
                "connectionCount": 0,
                "logSummary": [],
            })
        );
    }

    #[test]
    fn stopped_managed_runtime_has_a_distinct_safe_lifecycle_value() {
        let observations = InMemoryRuntimeObservations::new_mock();
        observations.record_managed_stopped();

        let response = RuntimeObservationResponse::from(observations.snapshot());

        assert_eq!(response.source, "runtime");
        assert_eq!(response.sidecar_lifecycle, "stopped");
    }
}
