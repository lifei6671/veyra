//! 固定受管观测 sidecar 的应用拥有生命周期。
//!
//! 此控制器不是 System Proxy supervisor：它没有 CaptureMode、WinINet、TUN、UAC 或调用方参数。
//! 唯一 worker 串行拥有 child，避免 IPC、Tray 与采样之间出现旧 identity 的结果交错。

use std::{
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, SyncSender, TrySendError},
    thread,
    time::Duration,
};

use serde::Serialize;

use crate::{
    application::observability::{
        InMemoryRuntimeObservations, ManagedRuntimeFailure, ObservedSidecarLifecycle,
    },
    domain::{DnsPolicy, RouteTarget, RuntimeIntent},
    platform::windows::managed_sidecar_port::WindowsManagedSidecarPort,
    singbox::{
        ConfigCompiler, RuntimeProfile, SingBoxCompiler,
        managed_sidecar::generate_api_secret,
        runtime::{
            RuntimeObservationSidecarPort, SidecarError, SidecarLifecycle, SidecarPort,
            SidecarRuntime,
        },
    },
    storage::{JsonStateStore, StateStore},
};

const REQUEST_QUEUE_CAPACITY: usize = 1;
const RESPONSE_QUEUE_CAPACITY: usize = 1;
const SAMPLE_INTERVAL: Duration = Duration::from_secs(1);
const START_RESPONSE_TIMEOUT: Duration = Duration::from_secs(15);
const STOP_RESPONSE_TIMEOUT: Duration = Duration::from_secs(3);

/// UI 只能收到封闭生命周期结果，绝不收到路径、PID、secret 或底层错误。
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum ManagedRuntimeStartResult {
    Started,
    AlreadyRunning,
    StateUnavailable,
    ConfigurationFailed,
    StartFailed,
    Busy,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum ManagedRuntimeStopResult {
    Stopped,
    AlreadyStopped,
    StopFailed,
    Busy,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ShutdownResult {
    ShutdownComplete,
    ShutdownFailed,
}

struct ObservationCompilationInput {
    runtime: RuntimeIntent,
    default_target: RouteTarget,
}

enum WorkerRequest {
    Start {
        intent: ObservationCompilationInput,
        response: SyncSender<ManagedRuntimeStartResult>,
    },
    Stop {
        response: SyncSender<ManagedRuntimeStopResult>,
    },
    Shutdown {
        response: SyncSender<ShutdownResult>,
    },
}

/// App setup 时创建，所有 IPC 都是零参数。
pub(crate) struct ManagedObservationRuntimeController {
    state_file: PathBuf,
    requests: SyncSender<WorkerRequest>,
    observations: InMemoryRuntimeObservations,
}

impl ManagedObservationRuntimeController {
    /// 所有路径由 Tauri PathResolver 在 setup 中确定；此处不触发资源校验、进程或网络 I/O。
    pub(crate) fn new(
        resource_root: PathBuf,
        app_local_data_root: PathBuf,
        observations: InMemoryRuntimeObservations,
    ) -> Self {
        let (requests, receiver) = mpsc::sync_channel(REQUEST_QUEUE_CAPACITY);
        let state_file = app_local_data_root.join("state.json");
        let worker_observations = observations.clone();
        thread::spawn(move || {
            worker_loop(
                receiver,
                resource_root,
                app_local_data_root,
                worker_observations,
            )
        });
        Self {
            state_file,
            requests,
            observations,
        }
    }

    pub(crate) fn start(&self) -> ManagedRuntimeStartResult {
        let intent = match load_runtime_intent(&self.state_file) {
            Some(intent) => intent,
            None => {
                self.observations
                    .record_managed_failure(None, ManagedRuntimeFailure::Configuration);
                return ManagedRuntimeStartResult::StateUnavailable;
            }
        };
        let (response, receiver) = mpsc::sync_channel(RESPONSE_QUEUE_CAPACITY);
        match self
            .requests
            .try_send(WorkerRequest::Start { intent, response })
        {
            Ok(()) => receiver
                .recv_timeout(START_RESPONSE_TIMEOUT)
                .unwrap_or_else(|_| {
                    self.observations.record_managed_failure(
                        Some(ObservedSidecarLifecycle::RecoveryRequired),
                        ManagedRuntimeFailure::Worker,
                    );
                    ManagedRuntimeStartResult::StartFailed
                }),
            Err(TrySendError::Full(_) | TrySendError::Disconnected(_)) => {
                ManagedRuntimeStartResult::Busy
            }
        }
    }

    pub(crate) fn stop(&self) -> ManagedRuntimeStopResult {
        let (response, receiver) = mpsc::sync_channel(RESPONSE_QUEUE_CAPACITY);
        match self.requests.try_send(WorkerRequest::Stop { response }) {
            Ok(()) => receiver
                .recv_timeout(STOP_RESPONSE_TIMEOUT)
                .unwrap_or_else(|_| {
                    self.observations.record_managed_failure(
                        Some(ObservedSidecarLifecycle::RecoveryRequired),
                        ManagedRuntimeFailure::Worker,
                    );
                    ManagedRuntimeStopResult::StopFailed
                }),
            Err(TrySendError::Full(_) | TrySendError::Disconnected(_)) => {
                ManagedRuntimeStopResult::Busy
            }
        }
    }

    /// Tray Quit 只有 worker 确认停止当前 child 后才允许继续退出应用。
    pub(crate) fn shutdown(&self) -> bool {
        let (response, receiver) = mpsc::sync_channel(RESPONSE_QUEUE_CAPACITY);
        matches!(
            self.requests.try_send(WorkerRequest::Shutdown { response }),
            Ok(())
        ) && matches!(
            receiver.recv_timeout(STOP_RESPONSE_TIMEOUT),
            Ok(ShutdownResult::ShutdownComplete)
        )
    }
}

impl Drop for ManagedObservationRuntimeController {
    fn drop(&mut self) {
        let (response, _) = mpsc::sync_channel(RESPONSE_QUEUE_CAPACITY);
        let _ = self.requests.try_send(WorkerRequest::Shutdown { response });
    }
}

fn load_runtime_intent(state_file: &Path) -> Option<ObservationCompilationInput> {
    let store = JsonStateStore::new(state_file.to_path_buf()).ok()?;
    let state = store.load().ok()?;
    if state.nodes.is_empty() {
        return None;
    }
    Some(ObservationCompilationInput {
        runtime: RuntimeIntent::from_state(&state).ok()?,
        default_target: state.default_target,
    })
}

fn worker_loop(
    requests: Receiver<WorkerRequest>,
    resource_root: PathBuf,
    app_local_data_root: PathBuf,
    observations: InMemoryRuntimeObservations,
) {
    let mut runtime: Option<SidecarRuntime<WindowsManagedSidecarPort>> = None;
    loop {
        match requests.recv_timeout(SAMPLE_INTERVAL) {
            Ok(WorkerRequest::Start { intent, response }) => {
                let result = start_runtime(
                    &mut runtime,
                    &resource_root,
                    &app_local_data_root,
                    &intent,
                    &observations,
                );
                if response.send(result).is_err() {
                    observations.record_managed_failure(
                        Some(ObservedSidecarLifecycle::RecoveryRequired),
                        ManagedRuntimeFailure::Worker,
                    );
                }
            }
            Ok(WorkerRequest::Stop { response }) => {
                let result = stop_runtime(&mut runtime, &observations);
                if response.send(result).is_err() {
                    observations.record_managed_failure(
                        Some(ObservedSidecarLifecycle::RecoveryRequired),
                        ManagedRuntimeFailure::Worker,
                    );
                }
            }
            Ok(WorkerRequest::Shutdown { response }) => {
                let result = match stop_runtime(&mut runtime, &observations) {
                    ManagedRuntimeStopResult::Stopped
                    | ManagedRuntimeStopResult::AlreadyStopped => ShutdownResult::ShutdownComplete,
                    ManagedRuntimeStopResult::StopFailed | ManagedRuntimeStopResult::Busy => {
                        ShutdownResult::ShutdownFailed
                    }
                };
                let confirmed = response.send(result).is_ok();
                if !confirmed {
                    observations.record_managed_failure(
                        Some(ObservedSidecarLifecycle::RecoveryRequired),
                        ManagedRuntimeFailure::Worker,
                    );
                }
                if confirmed && result == ShutdownResult::ShutdownComplete {
                    return;
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => sample_runtime(runtime.as_mut(), &observations),
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                let _ = stop_runtime(&mut runtime, &observations);
                return;
            }
        }
    }
}

fn start_runtime(
    runtime: &mut Option<SidecarRuntime<WindowsManagedSidecarPort>>,
    resource_root: &Path,
    app_local_data_root: &Path,
    intent: &ObservationCompilationInput,
    observations: &InMemoryRuntimeObservations,
) -> ManagedRuntimeStartResult {
    if let Some(active) = runtime.as_mut() {
        return match active.snapshot().lifecycle {
            SidecarLifecycle::Ready => ManagedRuntimeStartResult::AlreadyRunning,
            SidecarLifecycle::Stopped => start_stopped_runtime(active, intent, observations),
            SidecarLifecycle::RecoveryRequired => {
                observations.record_managed_failure(
                    Some(ObservedSidecarLifecycle::RecoveryRequired),
                    ManagedRuntimeFailure::Start,
                );
                ManagedRuntimeStartResult::StartFailed
            }
        };
    }

    let port = match WindowsManagedSidecarPort::new(
        resource_root.to_path_buf(),
        app_local_data_root.to_path_buf(),
    ) {
        Ok(port) => port,
        Err(_) => {
            observations.record_managed_failure(None, ManagedRuntimeFailure::Start);
            return ManagedRuntimeStartResult::StartFailed;
        }
    };
    *runtime = Some(SidecarRuntime::new_observation_only(port));
    start_stopped_runtime(
        runtime.as_mut().expect("runtime was initialized"),
        intent,
        observations,
    )
}

fn start_stopped_runtime<P: SidecarPort>(
    runtime: &mut SidecarRuntime<P>,
    intent: &ObservationCompilationInput,
    observations: &InMemoryRuntimeObservations,
) -> ManagedRuntimeStartResult {
    let plan = match SingBoxCompiler.compile(
        &intent.runtime,
        &intent.default_target,
        DnsPolicy::System,
        RuntimeProfile::ObservationOnly,
    ) {
        Ok(plan) => plan,
        Err(_) => {
            observations.record_managed_failure(None, ManagedRuntimeFailure::Configuration);
            return ManagedRuntimeStartResult::ConfigurationFailed;
        }
    };
    let secret = match generate_api_secret() {
        Ok(secret) => secret,
        Err(_) => {
            observations.record_managed_failure(None, ManagedRuntimeFailure::Configuration);
            return ManagedRuntimeStartResult::ConfigurationFailed;
        }
    };
    let candidate = match plan.finalize(&secret) {
        Ok(candidate) => candidate,
        Err(_) => {
            observations.record_managed_failure(None, ManagedRuntimeFailure::Configuration);
            return ManagedRuntimeStartResult::ConfigurationFailed;
        }
    };
    match runtime.start_or_replace(candidate) {
        Ok(()) => {
            observations.record_managed_ready();
            ManagedRuntimeStartResult::Started
        }
        Err(error) => {
            let lifecycle = observed_lifecycle(runtime.snapshot().lifecycle);
            let configuration_failed = matches!(
                error,
                SidecarError::CandidateCheck | SidecarError::CandidatePrepare
            ) && lifecycle != ObservedSidecarLifecycle::RecoveryRequired;
            observations.record_managed_failure(
                Some(lifecycle),
                if configuration_failed {
                    ManagedRuntimeFailure::Configuration
                } else {
                    ManagedRuntimeFailure::Start
                },
            );
            if configuration_failed {
                ManagedRuntimeStartResult::ConfigurationFailed
            } else {
                ManagedRuntimeStartResult::StartFailed
            }
        }
    }
}

fn observed_lifecycle(lifecycle: SidecarLifecycle) -> ObservedSidecarLifecycle {
    match lifecycle {
        SidecarLifecycle::Stopped => ObservedSidecarLifecycle::Stopped,
        SidecarLifecycle::Ready => ObservedSidecarLifecycle::Ready,
        SidecarLifecycle::RecoveryRequired => ObservedSidecarLifecycle::RecoveryRequired,
    }
}

fn stop_runtime(
    runtime: &mut Option<SidecarRuntime<WindowsManagedSidecarPort>>,
    observations: &InMemoryRuntimeObservations,
) -> ManagedRuntimeStopResult {
    let Some(runtime) = runtime.as_mut() else {
        observations.record_managed_stopped();
        return ManagedRuntimeStopResult::AlreadyStopped;
    };
    match runtime.snapshot().lifecycle {
        SidecarLifecycle::Stopped => {
            observations.record_managed_stopped();
            ManagedRuntimeStopResult::AlreadyStopped
        }
        SidecarLifecycle::Ready | SidecarLifecycle::RecoveryRequired if runtime.stop().is_ok() => {
            observations.record_managed_stopped();
            ManagedRuntimeStopResult::Stopped
        }
        SidecarLifecycle::Ready | SidecarLifecycle::RecoveryRequired => {
            observations.record_managed_failure(
                Some(ObservedSidecarLifecycle::RecoveryRequired),
                ManagedRuntimeFailure::Stop,
            );
            ManagedRuntimeStopResult::StopFailed
        }
    }
}

fn sample_runtime(
    runtime: Option<&mut SidecarRuntime<WindowsManagedSidecarPort>>,
    observations: &InMemoryRuntimeObservations,
) {
    let Some(runtime) = runtime else {
        return;
    };
    if runtime.snapshot().lifecycle != SidecarLifecycle::Ready {
        return;
    }
    match runtime.with_active_port(|port, child| port.read_runtime_observation(child)) {
        Ok(Some(observation)) => observations.record_managed_observation(observation),
        Ok(None) => {}
        Err(_) => {
            let lifecycle = match runtime.recover_active_failure() {
                Ok(()) => observed_lifecycle(runtime.snapshot().lifecycle),
                Err(_) => ObservedSidecarLifecycle::RecoveryRequired,
            };
            observations
                .record_managed_failure(Some(lifecycle), ManagedRuntimeFailure::Observation);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };

    use super::*;
    use crate::application::observability::RuntimeObservationPort;

    static NEXT_TEST_ID: AtomicU64 = AtomicU64::new(0);

    #[derive(Default)]
    struct FailingPort {
        fault: &'static str,
        events: Vec<&'static str>,
    }

    impl SidecarPort for FailingPort {
        fn check(
            &mut self,
            config: &crate::singbox::GeneratedConfig,
        ) -> Result<(), crate::singbox::runtime::SidecarPortError> {
            config.validate_final().expect("valid final candidate");
            self.events.push("check");
            if self.fault == "check" {
                Err(crate::singbox::runtime::SidecarPortError)
            } else {
                Ok(())
            }
        }
        fn prepare(
            &mut self,
            _: &crate::singbox::GeneratedConfig,
        ) -> Result<(), crate::singbox::runtime::SidecarPortError> {
            Ok(())
        }
        fn run(
            &mut self,
        ) -> Result<
            crate::singbox::runtime::ManagedSidecar,
            crate::singbox::runtime::SidecarPortError,
        > {
            self.events.push("run");
            if self.fault == "run" {
                Err(crate::singbox::runtime::SidecarPortError)
            } else {
                Ok(crate::singbox::runtime::ManagedSidecar::from_port_identity(
                    1,
                ))
            }
        }
        fn ready(
            &mut self,
            _: &crate::singbox::runtime::ManagedSidecar,
        ) -> Result<(), crate::singbox::runtime::SidecarPortError> {
            self.events.push("ready");
            if matches!(self.fault, "ready" | "stop") {
                Err(crate::singbox::runtime::SidecarPortError)
            } else {
                Ok(())
            }
        }
        fn stop(
            &mut self,
            _: &crate::singbox::runtime::ManagedSidecar,
        ) -> Result<(), crate::singbox::runtime::SidecarPortError> {
            self.events.push("stop");
            if self.fault == "stop" {
                Err(crate::singbox::runtime::SidecarPortError)
            } else {
                Ok(())
            }
        }
        fn cancel_pending(&mut self) -> Result<(), crate::singbox::runtime::SidecarPortError> {
            Ok(())
        }
        fn has_pending_cleanup(&self) -> bool {
            false
        }
    }

    fn valid_compilation_input() -> ObservationCompilationInput {
        use crate::domain::*;
        let mut state = AppState::empty();
        state.subscriptions.push(Subscription {
            id: SubscriptionId("sub".into()),
            name: "fixture".into(),
        });
        state.providers.push(Provider {
            id: ProviderId("provider".into()),
            subscription_id: SubscriptionId("sub".into()),
            name: "fixture".into(),
        });
        state.nodes = crate::subscription::normalize_nodes(ProviderId("provider".into()),
            crate::subscription::parse_subscription(r#"{"outbounds":[{"type":"socks","tag":"fixture","server":"127.0.0.1","server_port":1080}]}"#)
                .expect("parse").nodes).expect("normalize");
        state.pools.push(NodePool {
            id: PoolId("default".into()),
            name: "default".into(),
            kind: PoolKind::Custom,
            sources: vec![PoolSource {
                provider_id: ProviderId("provider".into()),
                filter: NodeFilter::default(),
            }],
            selection: SelectionPolicy::Manual {
                selected_node_id: None,
            },
            enabled: true,
        });
        state.default_target = RouteTarget::Pool(PoolId("default".into()));
        ObservationCompilationInput {
            runtime: RuntimeIntent::from_state(&state).expect("valid state"),
            default_target: state.default_target,
        }
    }

    #[test]
    fn each_start_failure_reports_its_stage_and_actual_lifecycle_without_retry() {
        for (fault, result, lifecycle, message, runs) in [
            (
                "check",
                ManagedRuntimeStartResult::ConfigurationFailed,
                ObservedSidecarLifecycle::Stopped,
                "Configuration generation failed; candidate not applied",
                0,
            ),
            (
                "run",
                ManagedRuntimeStartResult::StartFailed,
                ObservedSidecarLifecycle::Stopped,
                "Core startup failed",
                1,
            ),
            (
                "ready",
                ManagedRuntimeStartResult::StartFailed,
                ObservedSidecarLifecycle::Stopped,
                "Core startup failed",
                1,
            ),
            (
                "stop",
                ManagedRuntimeStartResult::StartFailed,
                ObservedSidecarLifecycle::RecoveryRequired,
                "Core startup failed",
                1,
            ),
        ] {
            let observations = InMemoryRuntimeObservations::new_mock();
            let mut runtime = SidecarRuntime::new_observation_only(FailingPort {
                fault,
                ..Default::default()
            });
            assert_eq!(
                start_stopped_runtime(&mut runtime, &valid_compilation_input(), &observations),
                result,
                "{fault}"
            );
            let snapshot = observations.snapshot();
            assert_eq!(snapshot.sidecar_lifecycle, lifecycle, "{fault}");
            assert_eq!(snapshot.connections.active, 0);
            assert_eq!(snapshot.latest_log.expect("failure log").message, message);
            assert_eq!(
                runtime
                    .into_port()
                    .events
                    .iter()
                    .filter(|event| **event == "run")
                    .count(),
                runs
            );
        }
    }

    #[test]
    fn rejected_compilation_does_not_call_the_port_or_overwrite_a_healthy_observation() {
        let observations = InMemoryRuntimeObservations::new_mock();
        observations.record_managed_ready();
        observations.record_connection_count(2);
        let mut input = valid_compilation_input();
        input.default_target = RouteTarget::Unconfigured;
        let mut runtime = SidecarRuntime::new_observation_only(FailingPort::default());
        assert_eq!(
            start_stopped_runtime(&mut runtime, &input, &observations),
            ManagedRuntimeStartResult::ConfigurationFailed
        );
        assert!(runtime.into_port().events.is_empty());
        let snapshot = observations.snapshot();
        assert_eq!(snapshot.sidecar_lifecycle, ObservedSidecarLifecycle::Ready);
        assert_eq!(snapshot.connections.active, 2);
        assert_eq!(
            serde_json::to_string(&ManagedRuntimeStartResult::ConfigurationFailed)
                .expect("serialized result"),
            "\"configurationFailed\""
        );
    }

    fn isolated_test_root() -> PathBuf {
        std::env::temp_dir().join(format!(
            "veyra-managed-runtime-test-{}",
            NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed)
        ))
    }

    fn worker_resource_fixture() -> (PathBuf, PathBuf, PathBuf) {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join(format!(
                "task009-worker-{}-{}",
                std::process::id(),
                NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed)
            ));
        let resources = root.join("resources");
        let destination = resources.join("sing-box/1.14.0");
        std::fs::create_dir_all(&destination).expect("isolated resource directory");
        let cache = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("binaries/sing-box-1.14.0-windows-amd64");
        for file in ["LICENSE", "libcronet.dll", "sing-box.exe"] {
            std::fs::hard_link(cache.join(file), destination.join(file))
                .expect("fixed resource hard link");
        }
        let app_data = std::env::temp_dir().join(root.file_name().expect("owned fixture name"));
        std::fs::create_dir(&app_data).expect("isolated app data");
        (root, resources, app_data)
    }

    #[test]
    fn real_worker_serializes_start_start_and_start_stop_with_a_bounded_queue() {
        use crate::singbox::test_support::FIXED_CLASH_API_TEST_LOCK;
        let _lock = FIXED_CLASH_API_TEST_LOCK
            .lock()
            .expect("fixed API test lock");
        for queued_stop in [false, true] {
            let (root, resources, app_data) = worker_resource_fixture();
            let observations = InMemoryRuntimeObservations::new_mock();
            let (events, event_receiver) = mpsc::channel();
            observations.install_delta_sink(move |delta| {
                let _ = events.send(delta);
            });
            let (requests, receiver) = mpsc::sync_channel(REQUEST_QUEUE_CAPACITY);
            let worker_observations = observations.clone();
            let worker_data = app_data.clone();
            // 零容量响应只在测试中把 worker 暂停在 Ready 后，使队列交错可重复。
            let (first_response, first_receiver) = mpsc::sync_channel(0);
            requests
                .try_send(WorkerRequest::Start {
                    intent: valid_compilation_input(),
                    response: first_response,
                })
                .expect("first start queued");
            let worker = thread::spawn(move || {
                worker_loop(receiver, resources, worker_data, worker_observations)
            });
            let cleanup_data = app_data.clone();
            // 断言失败也先关闭请求/响应端，再等 owner 完成自身清理，最后重新抛出断言。
            let assertions = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
                let ready = event_receiver
                    .recv_timeout(START_RESPONSE_TIMEOUT)
                    .expect("ready event");
                assert_eq!(ready.sidecar_lifecycle, ObservedSidecarLifecycle::Ready);
                let instances = std::fs::read_dir(app_data.join("sidecar-runtime"))
                    .expect("private runtime directory")
                    .map(|entry| entry.expect("instance").path())
                    .collect::<Vec<_>>();
                assert_eq!(instances.len(), 1);
                let original_config =
                    std::fs::read(instances[0].join("config.json")).expect("final private bytes");
                let (second_start, second_start_receiver) =
                    mpsc::sync_channel(RESPONSE_QUEUE_CAPACITY);
                let (second_stop, second_stop_receiver) =
                    mpsc::sync_channel(RESPONSE_QUEUE_CAPACITY);
                if queued_stop {
                    requests
                        .try_send(WorkerRequest::Stop {
                            response: second_stop,
                        })
                        .expect("stop queued behind start");
                } else {
                    let mut invalid_input = valid_compilation_input();
                    invalid_input.default_target = RouteTarget::Unconfigured;
                    requests
                        .try_send(WorkerRequest::Start {
                            intent: invalid_input,
                            response: second_start,
                        })
                        .expect("repeat start queued");
                }
                let controller = ManagedObservationRuntimeController {
                    state_file: PathBuf::new(),
                    requests,
                    observations: observations.clone(),
                };
                assert_eq!(controller.stop(), ManagedRuntimeStopResult::Busy);
                assert_eq!(
                    first_receiver
                        .recv_timeout(START_RESPONSE_TIMEOUT)
                        .expect("first response"),
                    ManagedRuntimeStartResult::Started
                );
                if queued_stop {
                    assert_eq!(
                        second_stop_receiver
                            .recv_timeout(STOP_RESPONSE_TIMEOUT)
                            .expect("queued stop response"),
                        ManagedRuntimeStopResult::Stopped
                    );
                } else {
                    assert_eq!(
                        second_start_receiver
                            .recv_timeout(START_RESPONSE_TIMEOUT)
                            .expect("repeat response"),
                        ManagedRuntimeStartResult::AlreadyRunning
                    );
                    // 无效新输入也没有触发编译，原实例目录与最终字节保持不变。
                    assert!(
                        original_config
                            == std::fs::read(instances[0].join("config.json"))
                                .expect("unchanged private bytes")
                    );
                    assert_eq!(
                        std::fs::read_dir(app_data.join("sidecar-runtime"))
                            .expect("runtime directory")
                            .count(),
                        1
                    );
                    assert_eq!(controller.stop(), ManagedRuntimeStopResult::Stopped);
                }
                assert!(!instances[0].exists());
                assert_eq!(
                    observations.snapshot().sidecar_lifecycle,
                    ObservedSidecarLifecycle::Stopped
                );
                assert!(controller.shutdown());
            }));
            let worker_result = worker.join();
            std::fs::remove_dir_all(cleanup_data).expect("remove owned app data");
            std::fs::remove_dir_all(root).expect("remove owned fixture");
            worker_result.expect("worker joined");
            if let Err(panic) = assertions {
                std::panic::resume_unwind(panic);
            }
        }
    }

    #[test]
    fn real_worker_lost_start_response_retains_owned_child_until_manual_stop() {
        use crate::{
            application::observability::{
                ObservationLogCategory, ObservationLogLevel, TrafficObservation,
            },
            singbox::{
                clash_api::{
                    ClashConnectionSnapshot, ClashRuntimeObservation, ClashTrafficObservation,
                },
                test_support::FIXED_CLASH_API_TEST_LOCK,
            },
        };
        use std::sync::Mutex;

        let _lock = FIXED_CLASH_API_TEST_LOCK
            .lock()
            .expect("fixed API test lock");
        let (root, resources, app_data) = worker_resource_fixture();
        let observations = InMemoryRuntimeObservations::new_mock();
        // 旧图表数据仅为内存夹具，不把它记作真实核心正流量证据。
        observations.record_managed_observation(ClashRuntimeObservation {
            connections: ClashConnectionSnapshot {
                upload_total_bytes: 300,
                download_total_bytes: 400,
                connection_count: 2,
            },
            traffic: Some(ClashTrafficObservation {
                upload_bytes_per_second: 30,
                download_bytes_per_second: 40,
                upload_total_bytes: 300,
                download_total_bytes: 400,
            }),
            latest_log: None,
        });
        let previous = observations.snapshot();
        assert_eq!(previous.traffic_history.len(), 1);
        let (events, event_receiver) = mpsc::sync_channel(4);
        let (release, gate) = mpsc::sync_channel(1);
        let gate = Mutex::new(gate);
        observations.install_delta_sink(move |delta| {
            let paused = matches!(
                delta.sidecar_lifecycle,
                ObservedSidecarLifecycle::Ready | ObservedSidecarLifecycle::RecoveryRequired
            );
            let _ = events.try_send(delta);
            if paused {
                // 发布后暂停 owner，精确控制响应丢失交错；断言失败断开 gate 即可收尾。
                let _ = gate
                    .lock()
                    .expect("publication gate")
                    .recv_timeout(Duration::from_secs(10));
            }
        });
        let (requests, receiver) = mpsc::sync_channel(REQUEST_QUEUE_CAPACITY);
        let (response, lost_receiver) = mpsc::sync_channel(RESPONSE_QUEUE_CAPACITY);
        requests
            .try_send(WorkerRequest::Start {
                intent: valid_compilation_input(),
                response,
            })
            .expect("start queued");
        let worker_observations = observations.clone();
        let worker_data = app_data.clone();
        let (finished, finished_receiver) = mpsc::sync_channel(1);
        let worker = thread::spawn(move || {
            worker_loop(receiver, resources, worker_data, worker_observations);
            let _ = finished.try_send(());
        });
        let cleanup_data = app_data.clone();
        let assertions = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
            let ready = event_receiver
                .recv_timeout(START_RESPONSE_TIMEOUT)
                .expect("real core ready");
            assert_eq!(ready.sidecar_lifecycle, ObservedSidecarLifecycle::Ready);
            assert_eq!(ready.revision, previous.revision + 1);
            assert!(ready.traffic_history.is_empty());
            assert_eq!(ready.traffic, TrafficObservation::default());
            let instances = std::fs::read_dir(app_data.join("sidecar-runtime"))
                .expect("private runtime directory")
                .map(|entry| entry.expect("instance").path())
                .collect::<Vec<_>>();
            assert_eq!(instances.len(), 1);
            let config_path = instances[0].join("config.json");
            let original_config =
                std::fs::read(&config_path).expect("private checked configuration");

            // 核心已 Ready，但调用方丢失接收端；该失败不能丢弃仍运行的 child 所有权。
            drop(lost_receiver);
            release.try_send(()).expect("release ready publication");
            let recovery = event_receiver
                .recv_timeout(STOP_RESPONSE_TIMEOUT)
                .expect("lost response recovery");
            assert_eq!(
                recovery.sidecar_lifecycle,
                ObservedSidecarLifecycle::RecoveryRequired
            );
            assert_eq!(recovery.revision, ready.revision + 1);
            assert!(recovery.traffic_history.is_empty());
            assert_eq!(recovery.traffic, TrafficObservation::default());
            assert_eq!(recovery.connections.active, 0);
            let log = recovery.latest_log.as_ref().expect("closed worker summary");
            assert_eq!(log.level, ObservationLogLevel::Error);
            assert_eq!(log.category, ObservationLogCategory::Runtime);
            assert_eq!(log.message, "Runtime worker response unavailable");
            let snapshot = observations.snapshot();
            assert_eq!(snapshot.revision, recovery.revision);
            assert_eq!(snapshot.latest_log, recovery.latest_log);
            assert!(snapshot.traffic_history.is_empty());
            let config: serde_json::Value =
                serde_json::from_slice(&original_config).expect("private configuration JSON");
            let secret = config["experimental"]["clash_api"]["secret"]
                .as_str()
                .expect("private API secret");
            assert!(!secret.is_empty());
            let safe_delta = serde_json::to_string(&recovery).expect("safe recovery delta");
            assert!(!safe_delta.contains(secret));
            assert!(!safe_delta.contains(&app_data.to_string_lossy().to_string()));

            let mut invalid_input = valid_compilation_input();
            invalid_input.default_target = RouteTarget::Unconfigured;
            let (repeat_response, repeat_receiver) = mpsc::sync_channel(RESPONSE_QUEUE_CAPACITY);
            requests
                .try_send(WorkerRequest::Start {
                    intent: invalid_input,
                    response: repeat_response,
                })
                .expect("repeat start queued");
            release.try_send(()).expect("release recovery publication");
            assert_eq!(
                repeat_receiver
                    .recv_timeout(START_RESPONSE_TIMEOUT)
                    .expect("repeat start response"),
                ManagedRuntimeStartResult::AlreadyRunning
            );
            // 重复 Start 没有编译、替换实例或删除仍由 owner 持有的私有配置。
            assert!(original_config == std::fs::read(&config_path).expect("same private bytes"));
            assert_eq!(
                std::fs::read_dir(app_data.join("sidecar-runtime"))
                    .expect("runtime directory")
                    .count(),
                1
            );
            // 后续采样不再需要发布门；这里只验证响应丢失，不注入网络采样延迟。
            drop(release);
            let controller = ManagedObservationRuntimeController {
                state_file: PathBuf::new(),
                requests,
                observations: observations.clone(),
            };
            assert_eq!(controller.stop(), ManagedRuntimeStopResult::Stopped);
            assert!(!instances[0].exists());
            let stopped = observations.snapshot();
            assert_eq!(stopped.sidecar_lifecycle, ObservedSidecarLifecycle::Stopped);
            assert!(stopped.revision > recovery.revision);
            assert_eq!(stopped.traffic, TrafficObservation::default());
            assert!(stopped.traffic_history.is_empty());
            assert!(controller.shutdown());
        }));
        finished_receiver
            .recv_timeout(START_RESPONSE_TIMEOUT)
            .expect("owner finishes bounded cleanup after channels close");
        let worker_result = worker.join();
        let private_root = cleanup_data.join("sidecar-runtime");
        if private_root.exists() {
            assert_eq!(
                std::fs::read_dir(private_root)
                    .expect("owned runtime directory")
                    .count(),
                0,
                "retain fixture if owner did not clean its private resources"
            );
        }
        // 只删除本次夹具，先核对解析后的直接父目录，保留其它历史失败证据。
        for (owned_path, expected_parent) in [
            (cleanup_data, std::env::temp_dir()),
            (
                root,
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target"),
            ),
        ] {
            let resolved = owned_path.canonicalize().expect("resolved owned fixture");
            let expected_parent = expected_parent
                .canonicalize()
                .expect("resolved fixture parent");
            assert_eq!(resolved.parent(), Some(expected_parent.as_path()));
            assert!(
                resolved
                    .file_name()
                    .expect("fixture name")
                    .to_string_lossy()
                    .starts_with(&format!("task009-worker-{}-", std::process::id()))
            );
            std::fs::remove_dir_all(resolved).expect("remove this test's owned fixture");
        }
        worker_result.expect("worker joined");
        if let Err(panic) = assertions {
            std::panic::resume_unwind(panic);
        }
    }

    #[test]
    fn real_worker_finishes_pending_stream_before_stop_and_new_instance_sampling() {
        use crate::{
            application::observability::{RuntimeObservationDelta, TrafficObservation},
            singbox::{
                clash_api::stream_test_probe::{self, Event, Kind, Stage},
                test_support::FIXED_CLASH_API_TEST_LOCK,
            },
        };
        use sha2::{Digest, Sha256};
        use std::sync::{
            Arc, Mutex,
            atomic::{AtomicBool, AtomicUsize},
        };

        enum Trace {
            Stream(usize, Event),
            Delta(usize, RuntimeObservationDelta, [usize; 2]),
        }
        #[derive(Default)]
        struct Counts {
            active: [usize; 2],
            maximum: [usize; 2],
            started: [usize; 2],
            finished: [usize; 2],
            unbalanced: bool,
        }

        let _lock = FIXED_CLASH_API_TEST_LOCK
            .lock()
            .expect("fixed API test lock");
        let (root, resources, app_data) = worker_resource_fixture();
        let observations = InMemoryRuntimeObservations::new_mock();
        let phase = Arc::new(AtomicUsize::new(1));
        let counts = Arc::new(Mutex::new(Counts::default()));
        let (events, traces) = mpsc::sync_channel(32);
        let (release, gate) = mpsc::sync_channel(1);
        let gate = Mutex::new(gate);
        let gate_released = Arc::new(AtomicBool::new(false));
        let paused = AtomicBool::new(false);
        let delta_events = events.clone();
        let delta_phase = Arc::clone(&phase);
        let delta_counts = Arc::clone(&counts);
        observations.install_delta_sink(move |delta| {
            let active = delta_counts.lock().expect("client operation counts").active;
            let _ = delta_events.try_send(Trace::Delta(
                delta_phase.load(Ordering::SeqCst),
                delta,
                active,
            ));
        });
        let probe_counts = Arc::clone(&counts);
        let probe_phase = Arc::clone(&phase);
        let probe_released = Arc::clone(&gate_released);
        let probe = Arc::new(move |event: Event| {
            let index = match event.kind {
                Kind::Traffic => 0,
                Kind::Logs => 1,
            };
            {
                let mut counts = probe_counts.lock().expect("client operation counts");
                match event.stage {
                    Stage::Started => {
                        counts.active[index] += 1;
                        counts.started[index] += 1;
                        counts.maximum[index] = counts.maximum[index].max(counts.active[index]);
                    }
                    Stage::Finished => {
                        counts.unbalanced |= counts.active[index] == 0;
                        counts.active[index] = counts.active[index].saturating_sub(1);
                        counts.finished[index] += 1;
                    }
                    Stage::Pending => {}
                }
            }
            let first_pending = event.kind == Kind::Traffic
                && event.stage == Stage::Pending
                && !paused.swap(true, Ordering::SeqCst);
            let _ = events.try_send(Trace::Stream(probe_phase.load(Ordering::SeqCst), event));
            if first_pending {
                // 仅在真实 socket.next 已返回 Pending 后短暂停住 owner；不持计数锁。
                // 断言失败会断开 gate，owner 仍能处理 Stop/请求通道关闭并清理 child。
                let released = gate
                    .lock()
                    .expect("pending read gate")
                    .recv_timeout(Duration::from_millis(800))
                    .is_ok();
                probe_released.store(released, Ordering::SeqCst);
            }
        });
        let (requests, receiver) = mpsc::sync_channel(REQUEST_QUEUE_CAPACITY);
        let controller = ManagedObservationRuntimeController {
            state_file: PathBuf::new(),
            requests,
            observations: observations.clone(),
        };
        let worker_observations = observations.clone();
        let worker_data = app_data.clone();
        let (finished, finished_receiver) = mpsc::sync_channel(1);
        let worker = thread::spawn(move || {
            let _installation = stream_test_probe::install(probe);
            worker_loop(receiver, resources, worker_data, worker_observations);
            let _ = finished.try_send(());
        });
        let cleanup_data = app_data.clone();
        let assertions = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
            let start = || {
                let (response, receiver) = mpsc::sync_channel(RESPONSE_QUEUE_CAPACITY);
                controller
                    .requests
                    .try_send(WorkerRequest::Start {
                        intent: valid_compilation_input(),
                        response,
                    })
                    .expect("start queued");
                assert_eq!(
                    receiver
                        .recv_timeout(START_RESPONSE_TIMEOUT)
                        .expect("start response"),
                    ManagedRuntimeStartResult::Started
                );
            };
            let read_config = || {
                let instances = std::fs::read_dir(app_data.join("sidecar-runtime"))
                    .expect("private runtime directory")
                    .map(|entry| entry.expect("private instance").path())
                    .collect::<Vec<_>>();
                assert_eq!(instances.len(), 1);
                let bytes =
                    std::fs::read(instances[0].join("config.json")).expect("private checked bytes");
                let value: serde_json::Value =
                    serde_json::from_slice(&bytes).expect("private configuration JSON");
                let secret = value["experimental"]["clash_api"]["secret"]
                    .as_str()
                    .expect("private API secret")
                    .to_owned();
                assert!(!secret.is_empty());
                (
                    instances[0].clone(),
                    format!("{:x}", Sha256::digest(&bytes)),
                    secret,
                )
            };
            start();
            let Trace::Delta(1, ready, [0, 0]) = traces
                .recv_timeout(STOP_RESPONSE_TIMEOUT)
                .expect("first Ready delta")
            else {
                panic!("Ready precedes stream operations")
            };
            assert_eq!(ready.sidecar_lifecycle, ObservedSidecarLifecycle::Ready);
            assert!(ready.traffic_history.is_empty());
            let (old_instance, old_hash, old_secret) = read_config();
            let mut saw_pending = false;
            for _ in 0..8 {
                match traces
                    .recv_timeout(STOP_RESPONSE_TIMEOUT)
                    .expect("actual pending stream")
                {
                    Trace::Stream(
                        1,
                        Event {
                            kind: Kind::Traffic,
                            stage: Stage::Started,
                        },
                    ) => {}
                    Trace::Stream(
                        1,
                        Event {
                            kind: Kind::Traffic,
                            stage: Stage::Pending,
                        },
                    ) => {
                        saw_pending = true;
                        break;
                    }
                    _ => panic!("first traffic read must be pending before publication"),
                }
            }
            assert!(saw_pending);
            assert_eq!(counts.lock().expect("counts").active, [1, 0]);
            let (response, stop_receiver) = mpsc::sync_channel(RESPONSE_QUEUE_CAPACITY);
            controller
                .requests
                .try_send(WorkerRequest::Stop { response })
                .expect("Stop queued during network read");
            assert_eq!(controller.stop(), ManagedRuntimeStopResult::Busy);
            assert!(matches!(
                stop_receiver.try_recv(),
                Err(mpsc::TryRecvError::Empty)
            ));
            assert_eq!(observations.snapshot().revision, ready.revision);
            release.try_send(()).expect("release actual pending read");

            let mut sample_revision = None;
            let mut stopped_revision = None;
            for _ in 0..12 {
                match traces
                    .recv_timeout(STOP_RESPONSE_TIMEOUT)
                    .expect("sample then Stop trace")
                {
                    Trace::Stream(1, _) => assert!(sample_revision.is_none()),
                    Trace::Delta(1, delta, active) => {
                        assert_eq!(
                            active,
                            [0, 0],
                            "socket operations finish before publication"
                        );
                        match delta.sidecar_lifecycle {
                            ObservedSidecarLifecycle::Ready => {
                                assert!(sample_revision.is_none());
                                assert_eq!(delta.traffic_history.len(), 1);
                                assert!(delta.revision > ready.revision);
                                sample_revision = Some(delta.revision);
                            }
                            ObservedSidecarLifecycle::Stopped => {
                                assert!(
                                    delta.revision > sample_revision.expect("sample precedes Stop")
                                );
                                assert!(delta.traffic_history.is_empty());
                                assert_eq!(delta.traffic, TrafficObservation::default());
                                stopped_revision = Some(delta.revision);
                                break;
                            }
                            _ => panic!("normal stream and Stop must remain healthy"),
                        }
                    }
                    _ => panic!("no new instance phase before old Stop"),
                }
            }
            let stopped_revision = stopped_revision.expect("Stopped published");
            assert_eq!(
                stop_receiver
                    .recv_timeout(STOP_RESPONSE_TIMEOUT)
                    .expect("Stop response"),
                ManagedRuntimeStopResult::Stopped
            );
            assert!(
                gate_released.load(Ordering::SeqCst),
                "gate did not expire before Stop was queued"
            );
            assert!(!old_instance.exists());
            {
                let counts = counts.lock().expect("counts");
                assert_eq!(counts.active, [0, 0]);
                assert_eq!(counts.started, counts.finished);
                assert_eq!(counts.started, [1, 1]);
            }
            assert!(
                matches!(
                    traces.recv_timeout(SAMPLE_INTERVAL + Duration::from_millis(200)),
                    Err(mpsc::RecvTimeoutError::Timeout)
                ),
                "stopped worker creates no stream or old Delta"
            );
            assert_eq!(observations.snapshot().revision, stopped_revision);

            // 只有旧客户端操作全部释放且 Stop 已确认，才在同一 owner 开始新实例阶段。
            phase.store(2, Ordering::SeqCst);
            start();
            let Trace::Delta(2, new_ready, [0, 0]) = traces
                .recv_timeout(STOP_RESPONSE_TIMEOUT)
                .expect("new Ready delta")
            else {
                panic!("new Ready precedes new streams")
            };
            assert_eq!(new_ready.sidecar_lifecycle, ObservedSidecarLifecycle::Ready);
            assert!(new_ready.revision > stopped_revision);
            assert!(new_ready.traffic_history.is_empty());
            assert_eq!(new_ready.traffic, TrafficObservation::default());
            let (new_instance, new_hash, new_secret) = read_config();
            assert!(old_instance != new_instance);
            assert!(old_hash != new_hash);
            assert!(old_secret != new_secret);
            let mut new_sample_revision = None;
            for _ in 0..12 {
                match traces
                    .recv_timeout(STOP_RESPONSE_TIMEOUT)
                    .expect("new instance sample")
                {
                    Trace::Stream(2, _) => {}
                    Trace::Delta(2, delta, active) => {
                        assert_eq!(active, [0, 0]);
                        assert_eq!(delta.sidecar_lifecycle, ObservedSidecarLifecycle::Ready);
                        assert!(delta.revision > new_ready.revision);
                        assert_eq!(delta.traffic_history.len(), 1);
                        assert!(delta.traffic_history[0].sampled_at_ms >= new_ready.observed_at_ms);
                        let safe = serde_json::to_string(&delta).expect("safe Delta");
                        assert!(!safe.contains(&old_secret) && !safe.contains(&new_secret));
                        new_sample_revision = Some(delta.revision);
                        break;
                    }
                    _ => panic!("old phase cannot publish into new instance"),
                }
            }
            assert!(new_sample_revision.is_some());
            assert_eq!(controller.stop(), ManagedRuntimeStopResult::Stopped);
            let Trace::Delta(2, stopped, [0, 0]) = traces
                .recv_timeout(STOP_RESPONSE_TIMEOUT)
                .expect("new Stop delta")
            else {
                panic!("no stream remains after second sample")
            };
            assert_eq!(stopped.sidecar_lifecycle, ObservedSidecarLifecycle::Stopped);
            assert!(stopped.revision > new_sample_revision.expect("new sample"));
            assert!(stopped.traffic_history.is_empty());
            assert!(!new_instance.exists());
            {
                let counts = counts.lock().expect("counts");
                assert!(!counts.unbalanced);
                assert_eq!(counts.active, [0, 0]);
                assert_eq!(counts.maximum, [1, 1]);
                assert_eq!(counts.started, [2, 2]);
                assert_eq!(counts.started, counts.finished);
            }
            assert!(controller.shutdown());
            println!(
                "task009 pending-stream client_ops_max=1,1 client_ops_final=0,0 old_config_sha256={old_hash} new_config_sha256={new_hash}"
            );
        }));
        finished_receiver
            .recv_timeout(START_RESPONSE_TIMEOUT)
            .expect("owner completes bounded cleanup after channels close");
        let worker_result = worker.join();
        let private_root = cleanup_data.join("sidecar-runtime");
        if private_root.exists() {
            assert_eq!(
                std::fs::read_dir(private_root)
                    .expect("owned runtime directory")
                    .count(),
                0,
                "retain fixture if private resource cleanup was not confirmed"
            );
        }
        for (owned_path, expected_parent) in [
            (cleanup_data, std::env::temp_dir()),
            (
                root,
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target"),
            ),
        ] {
            let resolved = owned_path.canonicalize().expect("resolved owned fixture");
            let expected_parent = expected_parent
                .canonicalize()
                .expect("resolved fixture parent");
            assert_eq!(resolved.parent(), Some(expected_parent.as_path()));
            assert!(
                resolved
                    .file_name()
                    .expect("fixture name")
                    .to_string_lossy()
                    .starts_with(&format!("task009-worker-{}-", std::process::id()))
            );
            std::fs::remove_dir_all(resolved).expect("remove only this test's owned fixture");
        }
        worker_result.expect("worker joined");
        if let Err(panic) = assertions {
            std::panic::resume_unwind(panic);
        }
    }

    #[test]
    fn real_worker_samples_at_least_one_second_apart_and_publishes_nothing_after_stop() {
        use crate::singbox::test_support::FIXED_CLASH_API_TEST_LOCK;
        use std::{
            sync::{Mutex, atomic::AtomicUsize},
            time::Instant,
        };
        let _lock = FIXED_CLASH_API_TEST_LOCK
            .lock()
            .expect("fixed API test lock");
        let (root, resources, app_data) = worker_resource_fixture();
        let observations = InMemoryRuntimeObservations::new_mock();
        let (events, event_receiver) = mpsc::channel();
        let (release_sample, sample_gate) = mpsc::sync_channel(1);
        let sample_gate = Mutex::new(sample_gate);
        let event_count = AtomicUsize::new(0);
        observations.install_delta_sink(move |delta| {
            let index = event_count.fetch_add(1, Ordering::SeqCst);
            let _ = events.send((Instant::now(), delta));
            if index == 2 {
                // 暂停第二次采样发布的尾部，Stop 必须在同一 owner 完成采样后处理。
                // 测试断言提前失败时 gate 会断开；让 owner 返回接收循环并自行停止 child。
                let _ = sample_gate
                    .lock()
                    .expect("sample gate")
                    .recv_timeout(Duration::from_secs(5));
            }
        });
        let (requests, receiver) = mpsc::sync_channel(REQUEST_QUEUE_CAPACITY);
        let worker_observations = observations.clone();
        let worker_data = app_data.clone();
        let worker = thread::spawn(move || {
            worker_loop(receiver, resources, worker_data, worker_observations)
        });
        let assertions = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
            let (response, start_receiver) = mpsc::sync_channel(RESPONSE_QUEUE_CAPACITY);
            requests
                .try_send(WorkerRequest::Start {
                    intent: valid_compilation_input(),
                    response,
                })
                .expect("start queued");
            assert_eq!(
                start_receiver
                    .recv_timeout(START_RESPONSE_TIMEOUT)
                    .expect("start response"),
                ManagedRuntimeStartResult::Started
            );
            let (ready_at, ready) = event_receiver
                .recv_timeout(Duration::from_secs(5))
                .expect("ready delta");
            let (first_at, first) = event_receiver
                .recv_timeout(Duration::from_secs(5))
                .expect("first sample delta");
            let (second_at, second) = event_receiver
                .recv_timeout(Duration::from_secs(5))
                .expect("second sample delta");
            assert_eq!(ready.sidecar_lifecycle, ObservedSidecarLifecycle::Ready);
            assert_eq!(first.sidecar_lifecycle, ObservedSidecarLifecycle::Ready);
            assert_eq!(second.sidecar_lifecycle, ObservedSidecarLifecycle::Ready);
            assert!(first_at.duration_since(ready_at) >= SAMPLE_INTERVAL);
            assert!(second_at.duration_since(first_at) >= SAMPLE_INTERVAL);
            assert!(ready.revision < first.revision && first.revision < second.revision);
            let (response, stop_receiver) = mpsc::sync_channel(RESPONSE_QUEUE_CAPACITY);
            requests
                .try_send(WorkerRequest::Stop { response })
                .expect("stop queued during sample publication");
            assert!(matches!(
                stop_receiver.try_recv(),
                Err(mpsc::TryRecvError::Empty)
            ));
            release_sample.send(()).expect("release sampled event");
            assert_eq!(
                stop_receiver
                    .recv_timeout(STOP_RESPONSE_TIMEOUT)
                    .expect("stop response"),
                ManagedRuntimeStopResult::Stopped
            );
            let (_, stopped) = event_receiver
                .recv_timeout(Duration::from_secs(1))
                .expect("stopped delta");
            assert_eq!(stopped.sidecar_lifecycle, ObservedSidecarLifecycle::Stopped);
            assert!(stopped.revision > second.revision);
            assert!(matches!(
                event_receiver.recv_timeout(SAMPLE_INTERVAL + Duration::from_millis(200)),
                Err(mpsc::RecvTimeoutError::Timeout)
            ));
            assert_eq!(observations.snapshot().revision, stopped.revision);
            let (response, shutdown_receiver) = mpsc::sync_channel(RESPONSE_QUEUE_CAPACITY);
            requests
                .try_send(WorkerRequest::Shutdown { response })
                .expect("shutdown queued");
            assert_eq!(
                shutdown_receiver
                    .recv_timeout(STOP_RESPONSE_TIMEOUT)
                    .expect("shutdown response"),
                ShutdownResult::ShutdownComplete
            );
        }));
        let worker_result = worker.join();
        assert_eq!(
            std::fs::read_dir(app_data.join("sidecar-runtime"))
                .expect("runtime root")
                .count(),
            0
        );
        std::fs::remove_dir_all(app_data).expect("remove owned app data");
        std::fs::remove_dir_all(root).expect("remove owned fixture");
        worker_result.expect("worker joined");
        if let Err(panic) = assertions {
            std::panic::resume_unwind(panic);
        }
    }

    #[test]
    fn missing_state_fails_closed_without_asking_the_worker_to_start() {
        let root = isolated_test_root();
        let controller = ManagedObservationRuntimeController::new(
            root.join("resources"),
            root.join("app-data"),
            InMemoryRuntimeObservations::new_mock(),
        );

        assert_eq!(
            controller.start(),
            ManagedRuntimeStartResult::StateUnavailable
        );
        assert!(controller.shutdown());
    }

    #[test]
    fn stop_without_a_managed_child_is_closed_and_idempotent() {
        let root = isolated_test_root();
        let controller = ManagedObservationRuntimeController::new(
            root.join("resources"),
            root.join("app-data"),
            InMemoryRuntimeObservations::new_mock(),
        );

        assert_eq!(controller.stop(), ManagedRuntimeStopResult::AlreadyStopped);
        assert!(controller.shutdown());
    }

    #[test]
    fn lost_response_cannot_turn_a_stopped_observation_into_fresh_stop_confirmation() {
        for owner_finishes_before_response_loss in [false, true] {
            let observations = InMemoryRuntimeObservations::new_mock();
            observations.record_managed_stopped();
            let initial_revision = observations.snapshot().revision;
            let (requests, receiver) = mpsc::sync_channel(REQUEST_QUEUE_CAPACITY);
            let worker_observations = observations.clone();
            let worker = thread::spawn(move || {
                let WorkerRequest::Stop { response } = receiver.recv().expect("queued stop") else {
                    panic!("expected stop");
                };
                if owner_finishes_before_response_loss {
                    worker_observations.record_managed_stopped();
                }
                drop(response);
            });
            let controller = ManagedObservationRuntimeController {
                state_file: PathBuf::new(),
                requests,
                observations: observations.clone(),
            };
            assert_eq!(controller.stop(), ManagedRuntimeStopResult::StopFailed);
            worker.join().expect("fake worker exits");
            let snapshot = observations.snapshot();
            assert!(snapshot.revision > initial_revision);
            assert_eq!(
                snapshot.sidecar_lifecycle,
                ObservedSidecarLifecycle::RecoveryRequired
            );
            // 只有后续 owner 确认（例如手动 Stop）才重新发布 Stopped。
            observations.record_managed_stopped();
            assert_eq!(
                observations.snapshot().sidecar_lifecycle,
                ObservedSidecarLifecycle::Stopped
            );
        }
    }

    #[test]
    fn lost_shutdown_response_keeps_worker_available_for_a_confirmed_retry() {
        let (requests, receiver) = mpsc::sync_channel(REQUEST_QUEUE_CAPACITY);
        let observations = InMemoryRuntimeObservations::new_mock();
        let worker_observations = observations.clone();
        let worker = thread::spawn(move || {
            worker_loop(
                receiver,
                PathBuf::from("missing-resources"),
                PathBuf::from("missing-app-data"),
                worker_observations,
            )
        });

        let (lost_response, lost_receiver) = mpsc::sync_channel(RESPONSE_QUEUE_CAPACITY);
        drop(lost_receiver);
        requests
            .send(WorkerRequest::Shutdown {
                response: lost_response,
            })
            .expect("first shutdown is queued");

        let (retry_response, retry_receiver) = mpsc::sync_channel(RESPONSE_QUEUE_CAPACITY);
        requests
            .send(WorkerRequest::Shutdown {
                response: retry_response,
            })
            .expect("retry shutdown is queued");
        assert_eq!(
            retry_receiver
                .recv_timeout(STOP_RESPONSE_TIMEOUT)
                .expect("retry receives confirmation"),
            ShutdownResult::ShutdownComplete
        );
        worker
            .join()
            .expect("worker exits after confirmed shutdown");
        assert_eq!(
            observations.snapshot().sidecar_lifecycle,
            crate::application::observability::ObservedSidecarLifecycle::Stopped
        );
    }
    #[test]
    fn stored_default_target_is_carried_exactly_into_compilation() {
        use crate::domain::*;
        let root = isolated_test_root().join("default-target");
        let store = JsonStateStore::new(root.join("state.json")).expect("store");
        let mut state = AppState::empty();
        state.subscriptions.push(Subscription {
            id: SubscriptionId("sub".into()),
            name: "fixture".into(),
        });
        state.providers.push(Provider {
            id: ProviderId("provider".into()),
            subscription_id: SubscriptionId("sub".into()),
            name: "fixture".into(),
        });
        state.nodes = crate::subscription::normalize_nodes(ProviderId("provider".into()),
            crate::subscription::parse_subscription(r#"{"outbounds":[{"type":"socks","tag":"fixture","server":"example.invalid","server_port":1080}]}"#)
                .expect("parse").nodes).expect("normalize");
        for id in ["first", "chosen"] {
            state.pools.push(NodePool {
                id: PoolId(id.into()),
                name: id.into(),
                kind: PoolKind::Custom,
                sources: vec![PoolSource {
                    provider_id: ProviderId("provider".into()),
                    filter: NodeFilter::default(),
                }],
                selection: SelectionPolicy::Manual {
                    selected_node_id: None,
                },
                enabled: true,
            });
        }
        for target in [
            RouteTarget::Pool(PoolId("first".into())),
            RouteTarget::Unconfigured,
            RouteTarget::Direct,
            RouteTarget::Block,
        ] {
            state.default_target = target.clone();
            store.save(&state).expect("save exact target");
            let loaded =
                load_runtime_intent(&root.join("state.json")).expect("load validated input");
            assert_eq!(loaded.default_target, target);
            let compiled = SingBoxCompiler.compile(
                &loaded.runtime,
                &loaded.default_target,
                DnsPolicy::System,
                RuntimeProfile::ObservationOnly,
            );
            if matches!(target, RouteTarget::Pool(_)) {
                let config = compiled
                    .expect("chosen pool")
                    .finalize(&crate::singbox::managed_sidecar::test_api_secret())
                    .expect("final config");
                let value: serde_json::Value =
                    serde_json::from_slice(config.as_bytes()).expect("JSON");
                assert_eq!(value["route"]["final"], "pool-first");
            } else {
                assert!(compiled.is_err(), "never guess an enabled pool");
            }
        }
        std::fs::remove_dir_all(root).expect("remove isolated state fixture");
    }
}
