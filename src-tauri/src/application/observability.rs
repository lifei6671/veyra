//! 封闭的内存运行观测读侧。
//!
//! 此模块只保存本次进程内的受控摘要。它不读取 sidecar、网络、文件或持久化状态；
//! 受管 sidecar 的摘要必须先由后端 bridge 映射进来，再经过此 Port 的脱敏和单增 revision
//! 语义。它绝不把原始日志或敏感运行细节交给 UI。

// Mock-only 控制入口只供此 Task 的单元验证使用；未接入真实运行时前，它们不会被生产路径调用。
#![allow(dead_code)]

use std::{
    collections::HashMap,
    sync::{Arc, Mutex, Weak},
    time::Instant,
};

use serde::Serialize;

use crate::singbox::clash_api::{ClashLogCategory, ClashLogLevel, ClashRuntimeObservation};

/// 当前观测摘要的来源；默认 Mock-only，只有受管 bridge 成功映射后才标注为实时 sidecar。
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ObservationSource {
    MockOnly,
    ManagedSidecar,
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

const TRAFFIC_HISTORY_WINDOW_MS: u64 = 600_000;
const TRAFFIC_HISTORY_CAPACITY: usize = 600;

/// 成功采样发布时刻和聚合速率；不携带连接或核心实例信息。
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct TrafficHistoryPoint {
    pub(crate) sampled_at_ms: u64,
    pub(crate) upload_rate_bps: u64,
    pub(crate) download_rate_bps: u64,
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

/// 失败阶段只允许固定摘要，调用方无法传入 Core 输出或凭据。
#[derive(Clone, Copy)]
pub(crate) enum ManagedRuntimeFailure {
    Configuration,
    Start,
    Stop,
    Observation,
    Worker,
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
    pub(crate) observed_at_ms: u64,
    pub(crate) traffic_history: Vec<TrafficHistoryPoint>,
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
    pub(crate) observed_at_ms: u64,
    pub(crate) traffic_history: Vec<TrafficHistoryPoint>,
    pub(crate) source: ObservationSource,
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
            observed_at_ms: snapshot.observed_at_ms,
            traffic_history: snapshot.traffic_history.clone(),
            source: snapshot.source,
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
    clock_origin: Instant,
    #[cfg(test)]
    observed_time_override: Option<u64>,
    snapshot: RuntimeObservationSnapshot,
    subscribers: Arc<Mutex<SubscriberState>>,
    delta_sink: Option<RuntimeObservationDeltaSink>,
}

impl ObservationState {
    fn refresh_observed_time(&mut self) {
        let elapsed_ms = self.clock_origin.elapsed().as_millis() as u64;
        #[cfg(test)]
        let elapsed_ms = self.observed_time_override.unwrap_or(elapsed_ms);
        self.snapshot.observed_at_ms = elapsed_ms;
    }

    fn prune_traffic_history(&mut self) {
        let oldest = self
            .snapshot
            .observed_at_ms
            .saturating_sub(TRAFFIC_HISTORY_WINDOW_MS);
        self.snapshot
            .traffic_history
            .retain(|point| point.sampled_at_ms >= oldest);
        let excess = self
            .snapshot
            .traffic_history
            .len()
            .saturating_sub(TRAFFIC_HISTORY_CAPACITY);
        self.snapshot.traffic_history.drain(..excess);
    }
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
                clock_origin: Instant::now(),
                #[cfg(test)]
                observed_time_override: None,
                snapshot: RuntimeObservationSnapshot {
                    revision: 0,
                    observed_at_ms: 0,
                    traffic_history: Vec::new(),
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

    /// 将已由固定后端 bridge 脱敏的单次采样写入内存 DTO；不接受原始 API 响应或网络参数。
    pub(crate) fn record_managed_observation(&self, observation: ClashRuntimeObservation) {
        self.update(|snapshot| {
            snapshot.source = ObservationSource::ManagedSidecar;
            snapshot.sidecar_lifecycle = ObservedSidecarLifecycle::Ready;
            snapshot.connections.active = observation.connections.connection_count;
            if let Some(traffic) = observation.traffic {
                snapshot.traffic = TrafficObservation {
                    upload_bytes_per_second: traffic.upload_bytes_per_second,
                    download_bytes_per_second: traffic.download_bytes_per_second,
                    upload_total_bytes: traffic.upload_total_bytes,
                    download_total_bytes: traffic.download_total_bytes,
                };
                let point = TrafficHistoryPoint {
                    sampled_at_ms: snapshot.observed_at_ms,
                    upload_rate_bps: traffic.upload_bytes_per_second,
                    download_rate_bps: traffic.download_bytes_per_second,
                };
                // 同毫秒发布只覆盖末点，保证读侧时间严格递增。
                if let Some(last) = snapshot.traffic_history.last_mut()
                    && last.sampled_at_ms == point.sampled_at_ms
                {
                    *last = point;
                } else {
                    snapshot.traffic_history.push(point);
                }
            }
            if let Some(log) = observation.latest_log {
                snapshot.latest_log = Some(ObservationLogSummary {
                    level: map_managed_log_level(log.level),
                    category: map_managed_log_category(log.category),
                    message: log.message.to_owned(),
                });
            }
        });
    }

    /// 只由受管运行时 owner 在 Ready 后写入；不触发网络，也不声明任何捕获模式变更。
    pub(crate) fn record_managed_ready(&self) {
        self.update(|snapshot| {
            snapshot.source = ObservationSource::ManagedSidecar;
            snapshot.capture_mode = ObservedCaptureMode::Off;
            snapshot.sidecar_lifecycle = ObservedSidecarLifecycle::Ready;
            snapshot.traffic = TrafficObservation::default();
            snapshot.traffic_history.clear();
            snapshot.connections = ConnectionObservation::default();
            snapshot.latest_log = None;
        });
    }

    /// 停止后不保留旧 child 的流量、连接或日志摘要。
    pub(crate) fn record_managed_stopped(&self) {
        self.update(|snapshot| {
            snapshot.source = ObservationSource::ManagedSidecar;
            snapshot.capture_mode = ObservedCaptureMode::Off;
            snapshot.sidecar_lifecycle = ObservedSidecarLifecycle::Stopped;
            snapshot.traffic = TrafficObservation::default();
            snapshot.traffic_history.clear();
            snapshot.connections = ConnectionObservation::default();
            snapshot.latest_log = None;
        });
    }

    /// 固定 API/流失败只能更新为封闭恢复状态，不保存底层错误或网络细节。
    pub(crate) fn record_managed_recovery(&self) {
        self.update(|snapshot| {
            snapshot.source = ObservationSource::ManagedSidecar;
            snapshot.sidecar_lifecycle = ObservedSidecarLifecycle::RecoveryRequired;
            snapshot.traffic = TrafficObservation::default();
            snapshot.traffic_history.clear();
            snapshot.connections = ConnectionObservation::default();
            snapshot.latest_log = None;
        });
    }

    /// 只有串行 owner 可以提交生命周期；IPC 边界只追加日志，避免覆盖更新的运行状态。
    pub(crate) fn record_managed_failure(
        &self,
        lifecycle: Option<ObservedSidecarLifecycle>,
        failure: ManagedRuntimeFailure,
    ) {
        self.update(|snapshot| {
            if let Some(lifecycle) = lifecycle {
                snapshot.source = ObservationSource::ManagedSidecar;
                snapshot.capture_mode = ObservedCaptureMode::Off;
                snapshot.sidecar_lifecycle = lifecycle;
                if lifecycle != ObservedSidecarLifecycle::Ready {
                    snapshot.traffic = TrafficObservation::default();
                    snapshot.traffic_history.clear();
                    snapshot.connections = ConnectionObservation::default();
                }
            }
            snapshot.latest_log = Some(ObservationLogSummary {
                level: ObservationLogLevel::Error,
                category: ObservationLogCategory::Runtime,
                message: match failure {
                    ManagedRuntimeFailure::Configuration => {
                        "Configuration generation failed; candidate not applied"
                    }
                    ManagedRuntimeFailure::Start => "Core startup failed",
                    ManagedRuntimeFailure::Stop => "Core stop incomplete",
                    ManagedRuntimeFailure::Observation => "Runtime observation failed",
                    ManagedRuntimeFailure::Worker => "Runtime worker response unavailable",
                }
                .to_owned(),
            });
        });
    }

    fn update(&self, change: impl FnOnce(&mut RuntimeObservationSnapshot)) {
        let (delta, subscribers, delta_sink) = {
            let Ok(mut state) = self.state.lock() else {
                return;
            };
            state.refresh_observed_time();
            change(&mut state.snapshot);
            state.prune_traffic_history();
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

fn map_managed_log_level(level: ClashLogLevel) -> ObservationLogLevel {
    match level {
        ClashLogLevel::Info => ObservationLogLevel::Info,
        ClashLogLevel::Warning => ObservationLogLevel::Warning,
        ClashLogLevel::Error => ObservationLogLevel::Error,
    }
}

fn map_managed_log_category(category: ClashLogCategory) -> ObservationLogCategory {
    match category {
        ClashLogCategory::Runtime => ObservationLogCategory::Runtime,
        ClashLogCategory::Recovery => ObservationLogCategory::Recovery,
    }
}

impl RuntimeObservationPort for InMemoryRuntimeObservations {
    fn snapshot(&self) -> RuntimeObservationSnapshot {
        self.state
            .lock()
            .map(|mut state| {
                state.refresh_observed_time();
                state.prune_traffic_history();
                state.snapshot.clone()
            })
            .unwrap_or_else(|_| RuntimeObservationSnapshot {
                revision: 0,
                observed_at_ms: 0,
                traffic_history: Vec::new(),
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

    fn set_observed_time(observations: &InMemoryRuntimeObservations, elapsed_ms: u64) {
        observations
            .state
            .lock()
            .expect("observation state")
            .observed_time_override = Some(elapsed_ms);
    }

    fn managed_sample(upload: u64, download: u64) -> ClashRuntimeObservation {
        ClashRuntimeObservation {
            connections: crate::singbox::clash_api::ClashConnectionSnapshot {
                upload_total_bytes: 900,
                download_total_bytes: 1_800,
                connection_count: 2,
            },
            traffic: Some(crate::singbox::clash_api::ClashTrafficObservation {
                upload_bytes_per_second: upload,
                download_bytes_per_second: download,
                upload_total_bytes: 900,
                download_total_bytes: 1_800,
            }),
            latest_log: None,
        }
    }

    #[test]
    fn mock_default_is_honest_about_missing_real_observation_source() {
        let observations = InMemoryRuntimeObservations::new_mock();
        set_observed_time(&observations, 0);

        assert_eq!(
            observations.snapshot(),
            RuntimeObservationSnapshot {
                revision: 0,
                observed_at_ms: 0,
                traffic_history: Vec::new(),
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
    fn traffic_history_is_bounded_and_uses_successful_publication_times() {
        let observations = InMemoryRuntimeObservations::new_mock();
        for index in 0..605 {
            set_observed_time(&observations, index * 500);
            observations.record_managed_observation(managed_sample(index, index * 2));
        }
        let snapshot = observations.snapshot();
        assert_eq!(snapshot.observed_at_ms, 302_000);
        assert_eq!(snapshot.traffic_history.len(), 600);
        assert_eq!(snapshot.traffic_history[0].sampled_at_ms, 2_500);
        assert_eq!(snapshot.traffic_history[599].sampled_at_ms, 302_000);
        assert_eq!(snapshot.traffic_history[599].upload_rate_bps, 604);
        assert_eq!(snapshot.traffic_history[599].download_rate_bps, 1_208);
        assert!(
            snapshot
                .traffic_history
                .windows(2)
                .all(|points| points[0].sampled_at_ms < points[1].sampled_at_ms)
        );
    }

    #[test]
    fn snapshot_expires_history_without_revision_event_or_new_sample() {
        let observations = InMemoryRuntimeObservations::new_mock();
        set_observed_time(&observations, 1_000);
        observations.record_managed_observation(managed_sample(10, 20));
        let subscription = observations.subscribe();

        set_observed_time(&observations, 601_000);
        let boundary = observations.snapshot();
        assert_eq!(boundary.traffic_history.len(), 1);
        assert_eq!(boundary.observed_at_ms, 601_000);
        set_observed_time(&observations, 601_001);
        let expired = observations.snapshot();
        assert!(expired.traffic_history.is_empty());
        assert_eq!(expired.observed_at_ms, 601_001);
        assert_eq!(expired.revision, boundary.revision);
        assert_eq!(expired.traffic, boundary.traffic);
        assert_eq!(subscription.take_latest_delta(), None);
    }

    #[test]
    fn only_managed_traffic_samples_append_and_updates_also_expire_history() {
        let observations = InMemoryRuntimeObservations::new_mock();
        set_observed_time(&observations, 1_000);
        observations.record_managed_observation(managed_sample(10, 20));
        let original = observations.snapshot().traffic_history;
        set_observed_time(&observations, 2_000);
        observations.record_connection_count(7);
        observations.record_runtime_log(
            ObservationLogLevel::Info,
            ObservationLogCategory::Runtime,
            "runtime observation",
        );
        observations.record_traffic(99, 98, 0, 0);
        let mut no_traffic = managed_sample(0, 0);
        no_traffic.traffic = None;
        observations.record_managed_observation(no_traffic);
        assert_eq!(observations.snapshot().traffic_history, original);

        let subscription = observations.subscribe();
        set_observed_time(&observations, 601_001);
        observations.record_connection_count(8);
        let delta = subscription.take_latest_delta().expect("connection delta");
        assert!(delta.traffic_history.is_empty());
        assert_eq!(delta.observed_at_ms, 601_001);
    }

    #[test]
    fn same_millisecond_replaces_last_point_and_zero_rates_are_valid() {
        let observations = InMemoryRuntimeObservations::new_mock();
        set_observed_time(&observations, 500);
        observations.record_managed_observation(managed_sample(10, 20));
        observations.record_managed_observation(managed_sample(0, 0));
        assert_eq!(
            observations.snapshot().traffic_history,
            vec![TrafficHistoryPoint {
                sampled_at_ms: 500,
                upload_rate_bps: 0,
                download_rate_bps: 0,
            }]
        );
    }

    #[test]
    fn hidden_window_keeps_history_in_owner_and_latest_delta_carries_whole_window() {
        let observations = InMemoryRuntimeObservations::new_mock();
        let visible = observations.subscribe();
        drop(visible);
        set_observed_time(&observations, 1_000);
        observations.record_managed_observation(managed_sample(1, 2));
        set_observed_time(&observations, 181_000);
        observations.record_managed_observation(managed_sample(3, 4));
        let restored = observations.subscribe();
        assert_eq!(restored.take_latest_delta(), None);
        assert_eq!(observations.snapshot().traffic_history.len(), 2);

        set_observed_time(&observations, 182_000);
        observations.record_managed_observation(managed_sample(5, 6));
        set_observed_time(&observations, 183_000);
        observations.record_managed_observation(managed_sample(7, 8));
        let latest = restored.take_latest_delta().expect("latest delta");
        assert_eq!(latest.revision, 4);
        assert_eq!(latest.traffic_history.len(), 4);
        assert_eq!(
            latest.traffic_history,
            observations.snapshot().traffic_history
        );
        assert_eq!(restored.take_latest_delta(), None);
    }

    #[test]
    fn healthy_configuration_failure_preserves_history_but_core_changes_clear_it() {
        let transitions: [fn(&InMemoryRuntimeObservations); 5] = [
            InMemoryRuntimeObservations::record_managed_ready,
            InMemoryRuntimeObservations::record_managed_stopped,
            InMemoryRuntimeObservations::record_managed_recovery,
            |observations| {
                observations.record_managed_failure(
                    Some(ObservedSidecarLifecycle::Stopped),
                    ManagedRuntimeFailure::Start,
                )
            },
            |observations| {
                observations.record_managed_failure(
                    Some(ObservedSidecarLifecycle::RecoveryRequired),
                    ManagedRuntimeFailure::Stop,
                )
            },
        ];
        for transition in transitions {
            let observations = InMemoryRuntimeObservations::new_mock();
            set_observed_time(&observations, 1_000);
            observations.record_managed_observation(managed_sample(10, 20));
            let before = observations.snapshot();
            set_observed_time(&observations, 2_000);
            observations.record_managed_failure(None, ManagedRuntimeFailure::Configuration);
            assert_eq!(
                observations.snapshot().traffic_history,
                before.traffic_history
            );
            assert_eq!(observations.snapshot().traffic, before.traffic);
            let subscription = observations.subscribe();
            transition(&observations);
            let delta = subscription.take_latest_delta().expect("transition delta");
            assert!(delta.traffic_history.is_empty());
            assert_eq!(delta.traffic, TrafficObservation::default());
            assert!(observations.snapshot().traffic_history.is_empty());
        }
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

    #[test]
    fn managed_bridge_summary_updates_only_the_existing_safe_observation_fields() {
        let observations = InMemoryRuntimeObservations::new_mock();
        let subscription = observations.subscribe();

        observations.record_managed_observation(ClashRuntimeObservation {
            connections: crate::singbox::clash_api::ClashConnectionSnapshot {
                upload_total_bytes: 99,
                download_total_bytes: 88,
                connection_count: 3,
            },
            traffic: Some(crate::singbox::clash_api::ClashTrafficObservation {
                upload_bytes_per_second: 10,
                download_bytes_per_second: 20,
                upload_total_bytes: 99,
                download_total_bytes: 88,
            }),
            latest_log: Some(crate::singbox::clash_api::ClashLogSummary {
                level: ClashLogLevel::Warning,
                category: ClashLogCategory::Runtime,
                message: "sidecar log observed",
            }),
        });

        let snapshot = observations.snapshot();
        assert_eq!(snapshot.source, ObservationSource::ManagedSidecar);
        assert_eq!(snapshot.sidecar_lifecycle, ObservedSidecarLifecycle::Ready);
        assert_eq!(snapshot.connections.active, 3);
        assert_eq!(snapshot.traffic.upload_bytes_per_second, 10);
        assert_eq!(snapshot.traffic.download_bytes_per_second, 20);
        assert_eq!(snapshot.traffic.upload_total_bytes, 99);
        assert_eq!(snapshot.traffic.download_total_bytes, 88);
        let delta = subscription
            .take_latest_delta()
            .expect("managed summary delta");
        assert_eq!(delta.traffic, snapshot.traffic);
        let serialized = serde_json::to_value(&delta.traffic).expect("safe traffic DTO");
        assert_eq!(
            serialized,
            serde_json::json!({
                "upload_bytes_per_second": 10,
                "download_bytes_per_second": 20,
                "upload_total_bytes": 99,
                "download_total_bytes": 88
            })
        );
        assert_eq!(
            snapshot.latest_log,
            Some(ObservationLogSummary {
                level: ObservationLogLevel::Warning,
                category: ObservationLogCategory::Runtime,
                message: "sidecar log observed".to_owned(),
            })
        );
    }

    #[test]
    fn managed_bridge_failure_is_only_a_closed_recovery_state() {
        let observations = InMemoryRuntimeObservations::new_mock();

        observations.record_managed_recovery();

        let snapshot = observations.snapshot();
        assert_eq!(snapshot.source, ObservationSource::ManagedSidecar);
        assert_eq!(
            snapshot.sidecar_lifecycle,
            ObservedSidecarLifecycle::RecoveryRequired
        );
        assert_eq!(snapshot.latest_log, None);
    }

    #[test]
    fn managed_stop_clears_previous_safe_runtime_summaries() {
        let observations = InMemoryRuntimeObservations::new_mock();
        observations.record_managed_ready();
        observations.record_connection_count(3);
        observations.record_traffic(2, 4, 10, 20);

        observations.record_managed_stopped();
        let snapshot = observations.snapshot();

        assert_eq!(snapshot.source, ObservationSource::ManagedSidecar);
        assert_eq!(snapshot.capture_mode, ObservedCaptureMode::Off);
        assert_eq!(
            snapshot.sidecar_lifecycle,
            ObservedSidecarLifecycle::Stopped
        );
        assert_eq!(snapshot.connections.active, 0);
        assert_eq!(snapshot.traffic, TrafficObservation::default());
        assert_eq!(snapshot.latest_log, None);
    }

    #[test]
    fn configuration_failure_preserves_healthy_state_and_uses_a_fixed_error_summary() {
        let observations = InMemoryRuntimeObservations::new_mock();
        observations.record_managed_ready();
        observations.record_traffic(2, 4, 10, 20);
        observations.record_connection_count(3);
        let before = observations.snapshot();

        observations.record_managed_failure(None, ManagedRuntimeFailure::Configuration);
        let after = observations.snapshot();
        assert_eq!(after.sidecar_lifecycle, ObservedSidecarLifecycle::Ready);
        assert_eq!(after.traffic, before.traffic);
        assert_eq!(after.connections, before.connections);
        let log = after.latest_log.expect("failure log");
        assert_eq!(log.level, ObservationLogLevel::Error);
        assert_eq!(
            log.message,
            "Configuration generation failed; candidate not applied"
        );
    }

    #[test]
    fn failed_start_or_cleanup_clears_stale_traffic_and_records_actual_lifecycle() {
        for lifecycle in [
            ObservedSidecarLifecycle::Stopped,
            ObservedSidecarLifecycle::RecoveryRequired,
        ] {
            let observations = InMemoryRuntimeObservations::new_mock();
            observations.record_managed_ready();
            observations.record_traffic(2, 4, 10, 20);
            observations.record_connection_count(3);
            let subscription = observations.subscribe();

            observations.record_managed_failure(Some(lifecycle), ManagedRuntimeFailure::Start);
            let after = observations.snapshot();
            assert_eq!(after.sidecar_lifecycle, lifecycle);
            assert_eq!(after.traffic, TrafficObservation::default());
            assert_eq!(after.connections.active, 0);
            assert_eq!(
                after.latest_log.as_ref().expect("failure log").message,
                "Core startup failed"
            );
            let delta = subscription.take_latest_delta().expect("failure delta");
            assert_eq!(delta.sidecar_lifecycle, lifecycle);
            assert_eq!(delta.traffic, after.traffic);
            assert_eq!(delta.latest_log, after.latest_log);
        }
    }
}
