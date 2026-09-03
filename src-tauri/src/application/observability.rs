//! 封闭的内存运行观测读侧。
//!
//! 此模块只保存本次进程内的受控摘要。它不读取 sidecar、网络、文件或持久化状态；
//! `MockOnly` 明确表示当前数据不是真实运行时观测。后续接入经批准的数据源时，仍须
//! 经过此 Port 的脱敏和单增 revision 语义，不能把原始日志或敏感运行细节交给 UI。

// Mock-only 控制入口只供此 Task 的单元验证使用；未接入真实运行时前，它们不会被生产路径调用。
#![allow(dead_code)]

use std::{
    collections::HashMap,
    sync::{Arc, Mutex, Weak},
};

use serde::Serialize;

/// 此交付的唯一观测来源，避免把内存测试数据误称为实时 sidecar 数据。
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ObservationSource {
    MockOnly,
}

/// 仅描述已进入观测模型的 capture 状态，而非操作系统或 sidecar 的事实声明。
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ObservedCaptureMode {
    NotObserved,
    Off,
    SystemProxy,
    RecoveryRequired,
}

/// 仅描述已进入观测模型的受管 sidecar 生命周期。
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ObservedSidecarLifecycle {
    NotObserved,
    Stopped,
    Ready,
    RecoveryRequired,
}

/// 速率与累计值均为非负字节计数，不包含连接目标或流量内容。
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
pub(crate) struct TrafficObservation {
    pub(crate) upload_bytes_per_second: u64,
    pub(crate) download_bytes_per_second: u64,
    pub(crate) upload_total_bytes: u64,
    pub(crate) download_total_bytes: u64,
}

/// 不关联系统 PID、进程路径或连接目标的聚合连接数。
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
pub(crate) struct ConnectionObservation {
    pub(crate) active: u32,
}

/// 脱敏摘要的严重级别。
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ObservationLogLevel {
    Info,
    Warning,
    Error,
}

/// 只允许 UI 消费这几个封闭类别，而不是 Core 原始日志分类。
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ObservationLogCategory {
    Runtime,
    Connectivity,
    Recovery,
}

/// 日志只保留分类、级别和白名单摘要；绝不保留原始 detail。
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ObservationLogSummary {
    pub(crate) level: ObservationLogLevel,
    pub(crate) category: ObservationLogCategory,
    pub(crate) message: String,
}

/// 提供给固定 IPC Snapshot 的完整、仅内存运行观测。
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct RuntimeObservationSnapshot {
    pub(crate) revision: u64,
    pub(crate) source: ObservationSource,
    pub(crate) capture_mode: ObservedCaptureMode,
    pub(crate) sidecar_lifecycle: ObservedSidecarLifecycle,
    pub(crate) traffic: TrafficObservation,
    pub(crate) connections: ConnectionObservation,
    pub(crate) latest_log: Option<ObservationLogSummary>,
}

/// 为固定事件接线保留的最新增量。它含完整安全读侧，而非任意事件载荷。
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct RuntimeObservationDelta {
    pub(crate) revision: u64,
    pub(crate) capture_mode: ObservedCaptureMode,
    pub(crate) sidecar_lifecycle: ObservedSidecarLifecycle,
    pub(crate) traffic: TrafficObservation,
    pub(crate) connections: ConnectionObservation,
    pub(crate) latest_log: Option<ObservationLogSummary>,
}

impl From<&RuntimeObservationSnapshot> for RuntimeObservationDelta {
    fn from(snapshot: &RuntimeObservationSnapshot) -> Self {
        Self {
            revision: snapshot.revision,
            capture_mode: snapshot.capture_mode,
            sidecar_lifecycle: snapshot.sidecar_lifecycle,
            traffic: snapshot.traffic,
            connections: snapshot.connections,
            latest_log: snapshot.latest_log.clone(),
        }
    }
}

/// Application 对固定 IPC/事件接线暴露的只读观测语义。
pub(crate) trait RuntimeObservationPort: Send + Sync {
    fn snapshot(&self) -> RuntimeObservationSnapshot;
    fn subscribe(&self) -> RuntimeObservationSubscription;
}

/// 订阅者只保存一个可被下一次更新覆盖的最新 Delta，因而不会积压隐藏窗口期间的事件。
pub(crate) struct RuntimeObservationSubscription {
    id: u64,
    subscribers: Weak<Mutex<SubscriberState>>,
    latest: Arc<Mutex<Option<RuntimeObservationDelta>>>,
}

impl RuntimeObservationSubscription {
    /// 读取并消费最近一次增量；没有更新或已隐藏/取消时返回 `None`。
    pub(crate) fn take_latest_delta(&self) -> Option<RuntimeObservationDelta> {
        self.latest.lock().ok()?.take()
    }
}

impl Drop for RuntimeObservationSubscription {
    fn drop(&mut self) {
        if let Some(subscribers) = self.subscribers.upgrade()
            && let Ok(mut subscribers) = subscribers.lock()
        {
            subscribers.entries.remove(&self.id);
        }
    }
}

#[derive(Default)]
struct SubscriberState {
    next_id: u64,
    entries: HashMap<u64, Arc<Mutex<Option<RuntimeObservationDelta>>>>,
}

struct ObservationState {
    snapshot: RuntimeObservationSnapshot,
    subscribers: Arc<Mutex<SubscriberState>>,
    delta_sink: Option<RuntimeObservationDeltaSink>,
}

type RuntimeObservationDeltaSink = Arc<dyn Fn(RuntimeObservationDelta) + Send + Sync>;

/// Mock-only 内存 Port。测试和后续固定 IPC 都可从其取得 Snapshot；它不做任何 I/O。
#[derive(Clone)]
pub(crate) struct InMemoryRuntimeObservations {
    state: Arc<Mutex<ObservationState>>,
}

impl Default for InMemoryRuntimeObservations {
    fn default() -> Self {
        Self::new_mock()
    }
}

impl InMemoryRuntimeObservations {
    pub(crate) fn new_mock() -> Self {
        Self {
            state: Arc::new(Mutex::new(ObservationState {
                snapshot: RuntimeObservationSnapshot {
                    revision: 0,
                    source: ObservationSource::MockOnly,
                    capture_mode: ObservedCaptureMode::NotObserved,
                    sidecar_lifecycle: ObservedSidecarLifecycle::NotObserved,
                    traffic: TrafficObservation::default(),
                    connections: ConnectionObservation::default(),
                    latest_log: None,
                },
                subscribers: Arc::new(Mutex::new(SubscriberState::default())),
                delta_sink: None,
            })),
        }
    }

    /// 应用层只可安装一个受控 Delta 接收器；它不接受调用方参数，也不缓存历史事件。
    pub(crate) fn install_delta_sink(
        &self,
        sink: impl Fn(RuntimeObservationDelta) + Send + Sync + 'static,
    ) {
        if let Ok(mut state) = self.state.lock() {
            state.delta_sink = Some(Arc::new(sink));
        }
    }

    /// 模拟受管运行时状态变化；不触发任何真实 sidecar 或平台代理操作。
    pub(crate) fn record_runtime_state(
        &self,
        capture_mode: ObservedCaptureMode,
        sidecar_lifecycle: ObservedSidecarLifecycle,
    ) {
        self.update(|snapshot| {
            snapshot.capture_mode = capture_mode;
            snapshot.sidecar_lifecycle = sidecar_lifecycle;
        });
    }

    /// 以受控数字更新速率和累计流量，不接受流量内容、连接地址或接口参数。
    pub(crate) fn record_traffic(
        &self,
        upload_bytes_per_second: u64,
        download_bytes_per_second: u64,
        uploaded_bytes: u64,
        downloaded_bytes: u64,
    ) {
        self.update(|snapshot| {
            snapshot.traffic.upload_bytes_per_second = upload_bytes_per_second;
            snapshot.traffic.download_bytes_per_second = download_bytes_per_second;
            snapshot.traffic.upload_total_bytes = snapshot
                .traffic
                .upload_total_bytes
                .saturating_add(uploaded_bytes);
            snapshot.traffic.download_total_bytes = snapshot
                .traffic
                .download_total_bytes
                .saturating_add(downloaded_bytes);
        });
    }

    /// 仅更新聚合连接数量；不建立或保存任何连接历史。
    pub(crate) fn record_connection_count(&self, active: u32) {
        self.update(|snapshot| snapshot.connections.active = active);
    }

    /// 原始日志 detail 只用于立即决定固定摘要，之后不会进入 Snapshot、Delta 或订阅缓存。
    pub(crate) fn record_runtime_log(
        &self,
        level: ObservationLogLevel,
        category: ObservationLogCategory,
        raw_detail: &str,
    ) {
        self.update(|snapshot| {
            snapshot.latest_log = Some(ObservationLogSummary {
                level,
                category,
                message: sanitized_summary(raw_detail),
            });
        });
    }

    fn update(&self, change: impl FnOnce(&mut RuntimeObservationSnapshot)) {
        let (delta, subscribers, delta_sink) = {
            let Ok(mut state) = self.state.lock() else {
                return;
            };
            change(&mut state.snapshot);
            state.snapshot.revision = state.snapshot.revision.saturating_add(1);
            (
                RuntimeObservationDelta::from(&state.snapshot),
                Arc::clone(&state.subscribers),
                state.delta_sink.clone(),
            )
        };

        if let Ok(mut subscribers) = subscribers.lock() {
            subscribers.entries.retain(|_, latest| {
                latest
                    .lock()
                    .map(|mut slot| {
                        *slot = Some(delta.clone());
                        true
                    })
                    .unwrap_or(false)
            });
        }
        if let Some(delta_sink) = delta_sink {
            delta_sink(delta);
        }
    }
}

impl RuntimeObservationPort for InMemoryRuntimeObservations {
    fn snapshot(&self) -> RuntimeObservationSnapshot {
        self.state
            .lock()
            .map(|state| state.snapshot.clone())
            .unwrap_or_else(|_| RuntimeObservationSnapshot {
                revision: 0,
                source: ObservationSource::MockOnly,
                capture_mode: ObservedCaptureMode::NotObserved,
                sidecar_lifecycle: ObservedSidecarLifecycle::NotObserved,
                traffic: TrafficObservation::default(),
                connections: ConnectionObservation::default(),
                latest_log: None,
            })
    }

    fn subscribe(&self) -> RuntimeObservationSubscription {
        let latest = Arc::new(Mutex::new(None));
        let subscribers = self
            .state
            .lock()
            .map(|state| Arc::clone(&state.subscribers))
            .unwrap_or_else(|_| Arc::new(Mutex::new(SubscriberState::default())));
        let id = subscribers
            .lock()
            .map(|mut state| {
                state.next_id = state.next_id.saturating_add(1);
                let id = state.next_id;
                state.entries.insert(id, Arc::clone(&latest));
                id
            })
            .unwrap_or(0);
        RuntimeObservationSubscription {
            id,
            subscribers: Arc::downgrade(&subscribers),
            latest,
        }
    }
}

fn sanitized_summary(raw_detail: &str) -> String {
    let lower = raw_detail.to_ascii_lowercase();
    let has_sensitive_detail = [
        "secret",
        "token",
        "password",
        "uuid",
        "private",
        "reality",
        "authorization",
        "bearer",
        "://",
    ]
    .iter()
    .any(|needle| lower.contains(needle));

    if has_sensitive_detail {
        "Sensitive runtime detail omitted".to_owned()
    } else if raw_detail.trim().is_empty() {
        "Runtime event recorded without detail".to_owned()
    } else {
        "Runtime event recorded".to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_default_is_honest_about_missing_real_observation_source() {
        let observations = InMemoryRuntimeObservations::new_mock();

        assert_eq!(
            observations.snapshot(),
            RuntimeObservationSnapshot {
                revision: 0,
                source: ObservationSource::MockOnly,
                capture_mode: ObservedCaptureMode::NotObserved,
                sidecar_lifecycle: ObservedSidecarLifecycle::NotObserved,
                traffic: TrafficObservation::default(),
                connections: ConnectionObservation::default(),
                latest_log: None,
            }
        );
    }

    #[test]
    fn revisions_are_monotonic_across_runtime_traffic_and_connection_updates() {
        let observations = InMemoryRuntimeObservations::new_mock();

        observations.record_runtime_state(
            ObservedCaptureMode::SystemProxy,
            ObservedSidecarLifecycle::Ready,
        );
        observations.record_traffic(12, 34, 56, 78);
        observations.record_connection_count(3);

        let snapshot = observations.snapshot();
        assert_eq!(snapshot.revision, 3);
        assert_eq!(snapshot.capture_mode, ObservedCaptureMode::SystemProxy);
        assert_eq!(snapshot.sidecar_lifecycle, ObservedSidecarLifecycle::Ready);
        assert_eq!(snapshot.traffic.upload_bytes_per_second, 12);
        assert_eq!(snapshot.traffic.download_bytes_per_second, 34);
        assert_eq!(snapshot.traffic.upload_total_bytes, 56);
        assert_eq!(snapshot.traffic.download_total_bytes, 78);
        assert_eq!(snapshot.connections.active, 3);
    }

    #[test]
    fn traffic_totals_accumulate_without_connection_history() {
        let observations = InMemoryRuntimeObservations::new_mock();

        observations.record_traffic(10, 20, 30, 40);
        observations.record_traffic(11, 21, 50, 60);

        let snapshot = observations.snapshot();
        assert_eq!(snapshot.traffic.upload_bytes_per_second, 11);
        assert_eq!(snapshot.traffic.download_bytes_per_second, 21);
        assert_eq!(snapshot.traffic.upload_total_bytes, 80);
        assert_eq!(snapshot.traffic.download_total_bytes, 100);
        assert_eq!(snapshot.connections.active, 0);
    }

    #[test]
    fn log_summary_omits_fixture_secrets_and_raw_endpoint_details() {
        let observations = InMemoryRuntimeObservations::new_mock();
        let fixture_secret = "fixture-secret-token";

        observations.record_runtime_log(
            ObservationLogLevel::Error,
            ObservationLogCategory::Connectivity,
            "failed to connect to http://127.0.0.1:9090 with fixture-secret-token",
        );

        let summary = observations
            .snapshot()
            .latest_log
            .expect("sanitized log summary");
        assert_eq!(summary.message, "Sensitive runtime detail omitted");
        assert!(!summary.message.contains(fixture_secret));
        assert!(!summary.message.contains("127.0.0.1"));
    }

    #[test]
    fn no_subscription_means_no_delta_backlog_but_snapshot_remains_current() {
        let observations = InMemoryRuntimeObservations::new_mock();

        observations.record_connection_count(1);
        observations.record_connection_count(2);
        let subscription = observations.subscribe();

        assert_eq!(subscription.take_latest_delta(), None);
        assert_eq!(observations.snapshot().connections.active, 2);
    }

    #[test]
    fn subscription_keeps_only_latest_delta_and_drop_cancels_delivery() {
        let observations = InMemoryRuntimeObservations::new_mock();
        let subscription = observations.subscribe();

        observations.record_connection_count(1);
        observations.record_connection_count(2);
        let latest = subscription.take_latest_delta().expect("latest delta");
        assert_eq!(latest.revision, 2);
        assert_eq!(latest.connections.active, 2);
        assert_eq!(subscription.take_latest_delta(), None);

        drop(subscription);
        observations.record_connection_count(3);
        let restored = observations.subscribe();
        assert_eq!(restored.take_latest_delta(), None);
        assert_eq!(observations.snapshot().connections.active, 3);
    }

    #[test]
    fn restored_window_reads_current_snapshot_without_replaying_hidden_deltas() {
        let observations = InMemoryRuntimeObservations::new_mock();
        let visible = observations.subscribe();
        observations.record_traffic(1, 2, 3, 4);
        drop(visible);

        observations.record_traffic(5, 6, 7, 8);
        let restored = observations.subscribe();

        assert_eq!(restored.take_latest_delta(), None);
        let snapshot = observations.snapshot();
        assert_eq!(snapshot.revision, 2);
        assert_eq!(snapshot.traffic.upload_total_bytes, 10);
        assert_eq!(snapshot.traffic.download_total_bytes, 12);
    }

    #[test]
    fn installed_delta_sink_receives_each_new_delta_without_persisting_history() {
        let observations = InMemoryRuntimeObservations::new_mock();
        let revisions = Arc::new(Mutex::new(Vec::new()));
        let received = Arc::clone(&revisions);
        observations.install_delta_sink(move |delta| {
            received
                .lock()
                .expect("test sink lock")
                .push(delta.revision);
        });

        observations.record_connection_count(1);
        observations.record_connection_count(2);

        assert_eq!(*revisions.lock().expect("test revisions lock"), vec![1, 2]);
    }
}
