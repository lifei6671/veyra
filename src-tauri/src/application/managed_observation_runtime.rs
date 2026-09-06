//! 固定受管观测 sidecar 的应用拥有生命周期。
//!
//! 此控制器不是 System Proxy supervisor：它没有 CaptureMode、WinINet、TUN、UAC 或调用方参数。
//! 唯一 worker 串行拥有 child，避免 IPC、Tray 与采样之间出现旧 identity 的结果交错。

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, SyncSender, TrySendError},
    },
    thread,
    time::{Duration, Instant},
};

use serde::Serialize;

use crate::{
    application::{
        observability::{
            InMemoryRuntimeObservations, ManagedRuntimeFailure, ObservedSidecarLifecycle,
            RuntimeObservationPort, SubscriptionSwitch, SubscriptionSwitchErrorCode,
            SubscriptionSwitchStatus,
        },
        selected_subscription::{
            SelectedRuntimeProjection, SelectionProjectionError, project_selected_runtime,
        },
        state_access::{StateAccessError, StateAccessGate},
        subscription_management::{
            DocumentOperationError, DocumentSaveInput, SubscriptionManager, SubscriptionSummary,
            SubscriptionWriteGuard, summaries,
        },
        subscription_scheduler::SubscriptionScheduler,
    },
    domain::{AppState, DnsPolicy, NodeId, PoolId, SubscriptionId},
    platform::windows::managed_sidecar_port::WindowsManagedSidecarPort,
    singbox::{
        ConfigCompiler, GeneratedConfig, RuntimeProfile, SingBoxCompiler,
        clash_api::ClashApiClient,
        managed_sidecar::{api_secret_from_config, generate_api_secret},
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
const RUNNING_CONFIGURATION_MAX_BYTES: usize = 4 * 1024 * 1024;
const DOCUMENT_SAVE_DEADLINE: Duration = Duration::from_secs(30);
const SELECTOR_RESPONSE_TIMEOUT: Duration = Duration::from_secs(6);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ManualSelectionSavedOnlyReason {
    RuntimeStopped,
    RuntimeNotReady,
    NotInAppliedArtifact,
    DispatchUnavailable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ManualSelectionUnknownReason {
    ReadBackUnavailable,
    UnknownRuntimeNode,
    InstanceChanged,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ManualSelectionRuntimeResult {
    Applied,
    NotApplied { runtime_node_id: NodeId },
    SavedOnly(ManualSelectionSavedOnlyReason),
    ApplyUnknown(ManualSelectionUnknownReason),
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RuntimeTags {
    pool: String,
    node: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct AppliedRoutingIndex {
    configuration_generation: u64,
    pool_members: BTreeMap<PoolId, BTreeMap<NodeId, RuntimeTags>>,
}

impl AppliedRoutingIndex {
    fn from_projection(projection: &SelectedRuntimeProjection) -> Self {
        let pool_members = projection
            .runtime_intent
            .pools
            .iter()
            .map(|pool| {
                let members = pool
                    .members
                    .iter()
                    .map(|node_id| {
                        (
                            node_id.clone(),
                            RuntimeTags {
                                pool: format!("pool-{}", pool.id.0),
                                node: format!("node-{}", node_id.0),
                            },
                        )
                    })
                    .collect();
                (pool.id.clone(), members)
            })
            .collect();
        Self {
            configuration_generation: projection.selected_generation,
            pool_members,
        }
    }
}

/// UI 只能收到封闭生命周期结果，绝不收到路径、PID、secret 或底层错误。
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum ManagedRuntimeStartResult {
    Started,
    AlreadyRunning,
    SubscriptionSelectionRequired,
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

type ObservationCompilationInput = SelectedRuntimeProjection;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum ActivationError {
    InvalidInput,
    IdentityFailed,
    Busy,
    NotFound,
    StateUnavailable,
    SelectionConflict,
    ConfigurationFailed,
    SaveFailed,
    StopFailed,
    StartFailed,
    RecoveryRequired,
    StateChanged,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RoutingApplyTerminalFailure {
    ConfigurationFailed,
    StateChanged,
}

/// 仅进程内保存 routing Apply 的确定失败；generation/active subscription 绑定失败归属，
/// 避免后续结构保存把旧 operation 的失败误认为当前 desired configuration 的终态。
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RoutingApplyTerminal {
    pub(crate) operation_id: String,
    pub(crate) active_subscription_id: Option<String>,
    pub(crate) desired_generation: u64,
    pub(crate) failure: RoutingApplyTerminalFailure,
}

#[derive(Debug)]
pub(crate) enum ActivationResult {
    Activated {
        operation_id: String,
        subscription: SubscriptionSummary,
        generation: u64,
    },
    Reactivated {
        operation_id: String,
        subscription: SubscriptionSummary,
        generation: u64,
    },
    AlreadyCurrent {
        operation_id: String,
        subscription: SubscriptionSummary,
        generation: u64,
    },
    Pending {
        operation_id: String,
    },
    Error {
        operation_id: Option<String>,
        error: ActivationError,
    },
}

struct RuntimeGuard(Arc<AtomicBool>);
impl Drop for RuntimeGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}
struct RequestGuards {
    _runtime: RuntimeGuard,
    _subscription: Option<SubscriptionWriteGuard>,
}

struct ActivationRequest {
    operation_id: String,
    expected: AppState,
    candidate: AppState,
    projection: SelectedRuntimeProjection,
    subscription: SubscriptionSummary,
    config: GeneratedConfig,
    reactivation: bool,
    cancel: Arc<AtomicBool>,
    _guards: RequestGuards,
    response: SyncSender<ActivationResult>,
}

struct ManualSelectionRequest {
    pool_id: PoolId,
    node_id: NodeId,
    response: SyncSender<ManualSelectionRuntimeResult>,
    _guards: RequestGuards,
}

enum WorkerRequest {
    Start {
        intent: ObservationCompilationInput,
        response: SyncSender<ManagedRuntimeStartResult>,
        _guards: RequestGuards,
    },
    Stop {
        response: SyncSender<ManagedRuntimeStopResult>,
        _guards: RequestGuards,
    },
    Activate(Box<ActivationRequest>),
    ReconcileManualSelection(ManualSelectionRequest),
    ReadRunningConfiguration {
        response: SyncSender<RunningConfigurationResult>,
        _guards: RequestGuards,
    },
    SaveDocument {
        input: DocumentSaveInput,
        operation_id: String,
        deadline: Instant,
        response: SyncSender<DocumentSaveResult>,
        _runtime_guard: RuntimeGuard,
    },
    Shutdown {
        response: SyncSender<ShutdownResult>,
    },
}

struct WorkerContext {
    resource_root: PathBuf,
    app_local_data_root: PathBuf,
    observations: InMemoryRuntimeObservations,
    state_gate: StateAccessGate,
    subscriptions: Arc<SubscriptionManager>,
    shutdown_requested: Arc<AtomicBool>,
    shutdown_complete: Arc<AtomicBool>,
    selector_runtime_nodes: Arc<Mutex<BTreeMap<PoolId, NodeId>>>,
}

/// App setup 时创建；生命周期和订阅切换共享唯一 worker。
pub(crate) struct ManagedObservationRuntimeController {
    state_file: PathBuf,
    requests: SyncSender<WorkerRequest>,
    observations: InMemoryRuntimeObservations,
    state_gate: StateAccessGate,
    subscriptions: Arc<SubscriptionManager>,
    busy: Arc<AtomicBool>,
    shutdown_requested: Arc<AtomicBool>,
    shutdown_complete: Arc<AtomicBool>,
    scheduler: Option<Arc<SubscriptionScheduler>>,
    shutdown_waiting: Arc<AtomicBool>,
    selector_runtime_nodes: Arc<Mutex<BTreeMap<PoolId, NodeId>>>,
    routing_apply_terminal: Arc<Mutex<Option<RoutingApplyTerminal>>>,
}

impl ManagedObservationRuntimeController {
    /// 所有路径由 Tauri PathResolver 在 setup 中确定；此处不触发资源校验、进程或网络 I/O。
    pub(crate) fn new(
        resource_root: PathBuf,
        app_local_data_root: PathBuf,
        observations: InMemoryRuntimeObservations,
        state_gate: StateAccessGate,
        subscriptions: Arc<SubscriptionManager>,
        scheduler: Option<Arc<SubscriptionScheduler>>,
    ) -> Self {
        let (requests, receiver) = mpsc::sync_channel(REQUEST_QUEUE_CAPACITY);
        let state_file = app_local_data_root.join("state.json");
        let worker_observations = observations.clone();
        let worker_gate = state_gate.clone();
        let worker_subscriptions = Arc::clone(&subscriptions);
        let shutdown_requested = Arc::new(AtomicBool::new(false));
        let shutdown_complete = Arc::new(AtomicBool::new(false));
        let worker_shutdown = Arc::clone(&shutdown_requested);
        let worker_complete = Arc::clone(&shutdown_complete);
        let selector_runtime_nodes = Arc::new(Mutex::new(BTreeMap::new()));
        let worker_selector_runtime_nodes = Arc::clone(&selector_runtime_nodes);
        thread::spawn(move || {
            worker_loop(
                receiver,
                WorkerContext {
                    resource_root,
                    app_local_data_root,
                    observations: worker_observations,
                    state_gate: worker_gate,
                    subscriptions: worker_subscriptions,
                    shutdown_requested: worker_shutdown,
                    shutdown_complete: worker_complete,
                    selector_runtime_nodes: worker_selector_runtime_nodes,
                },
            )
        });
        Self {
            scheduler,
            shutdown_waiting: Arc::new(AtomicBool::new(false)),
            state_file,
            requests,
            observations,
            state_gate,
            subscriptions,
            busy: Arc::new(AtomicBool::new(false)),
            shutdown_requested,
            shutdown_complete,
            selector_runtime_nodes,
            routing_apply_terminal: Arc::new(Mutex::new(None)),
        }
    }

    pub(crate) fn start(&self) -> ManagedRuntimeStartResult {
        if self.is_shutting_down() {
            return ManagedRuntimeStartResult::Busy;
        }
        let guards = match self.acquire_guards(true) {
            Ok(guards) => guards,
            Err(_) => return ManagedRuntimeStartResult::Busy,
        };
        let intent = match load_runtime_intent(&self.state_file, &self.state_gate) {
            Ok(intent) => intent,
            Err(LoadRuntimeIntentError::Busy) => return ManagedRuntimeStartResult::Busy,
            Err(LoadRuntimeIntentError::SelectionRequired) => {
                return ManagedRuntimeStartResult::SubscriptionSelectionRequired;
            }
            Err(LoadRuntimeIntentError::Configuration) => {
                return ManagedRuntimeStartResult::ConfigurationFailed;
            }
            Err(LoadRuntimeIntentError::Unavailable) => {
                self.observations
                    .record_managed_failure(None, ManagedRuntimeFailure::Configuration);
                return ManagedRuntimeStartResult::StateUnavailable;
            }
        };
        let (response, receiver) = mpsc::sync_channel(RESPONSE_QUEUE_CAPACITY);
        match self.requests.try_send(WorkerRequest::Start {
            intent,
            response,
            _guards: guards,
        }) {
            Ok(()) => receiver
                .recv_timeout(START_RESPONSE_TIMEOUT)
                .unwrap_or_else(|_| {
                    self.observations
                        .record_managed_failure(None, ManagedRuntimeFailure::Worker);
                    ManagedRuntimeStartResult::StartFailed
                }),
            Err(TrySendError::Full(_) | TrySendError::Disconnected(_)) => {
                ManagedRuntimeStartResult::Busy
            }
        }
    }

    pub(crate) fn stop(&self) -> ManagedRuntimeStopResult {
        let guards = match self.acquire_guards(!self.is_shutting_down()) {
            Ok(guards) => guards,
            Err(_) => return ManagedRuntimeStopResult::Busy,
        };
        let (response, receiver) = mpsc::sync_channel(RESPONSE_QUEUE_CAPACITY);
        match self.requests.try_send(WorkerRequest::Stop {
            response,
            _guards: guards,
        }) {
            Ok(()) => receiver
                .recv_timeout(STOP_RESPONSE_TIMEOUT)
                .unwrap_or_else(|_| {
                    self.observations
                        .record_managed_failure(None, ManagedRuntimeFailure::Worker);
                    ManagedRuntimeStopResult::StopFailed
                }),
            Err(TrySendError::Full(_) | TrySendError::Disconnected(_)) => {
                ManagedRuntimeStopResult::Busy
            }
        }
    }

    /// Tray Quit 只有 worker 确认停止当前 child 后才允许继续退出应用。
    pub(crate) fn shutdown(&self) -> bool {
        if self.shutdown_complete.load(Ordering::Acquire) {
            return true;
        }
        if self
            .shutdown_waiting
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return false;
        }
        let _waiting = RuntimeGuard(Arc::clone(&self.shutdown_waiting));
        self.shutdown_requested.store(true, Ordering::Release);
        if self
            .scheduler
            .as_ref()
            .is_some_and(|scheduler| !scheduler.shutdown())
        {
            return false;
        }
        let (response, receiver) = mpsc::sync_channel(RESPONSE_QUEUE_CAPACITY);
        matches!(
            self.requests.try_send(WorkerRequest::Shutdown { response }),
            Ok(())
        ) && matches!(
            receiver.recv_timeout(STOP_RESPONSE_TIMEOUT),
            Ok(ShutdownResult::ShutdownComplete)
        )
    }

    pub(crate) fn is_shutting_down(&self) -> bool {
        self.shutdown_requested.load(Ordering::Acquire)
    }

    pub(crate) fn managed_proxy_port(&self) -> Option<std::num::NonZeroU16> {
        // 当前获批的 ObservationOnly 模式没有 mixed listener。
        None
    }

    pub(crate) fn running_configuration(&self) -> RunningConfigurationResult {
        let guards = match self.acquire_guards(false) {
            Ok(guards) => guards,
            Err(_) => return RunningConfigurationResult::Error(RunningConfigurationError::Busy),
        };
        let (response, receiver) = mpsc::sync_channel(RESPONSE_QUEUE_CAPACITY);
        if self
            .requests
            .try_send(WorkerRequest::ReadRunningConfiguration {
                response,
                _guards: guards,
            })
            .is_err()
        {
            return RunningConfigurationResult::Error(RunningConfigurationError::Busy);
        }
        receiver.recv().unwrap_or(RunningConfigurationResult::Error(
            RunningConfigurationError::RecoveryRequired,
        ))
    }

    pub(crate) fn save_document(&self, input: DocumentSaveInput) -> DocumentSaveResult {
        if self.is_shutting_down() {
            return DocumentSaveResult::Error {
                operation_id: None,
                error: DocumentSaveError::Busy,
            };
        }
        let mut entropy = [0u8; 16];
        if getrandom::fill(&mut entropy).is_err() {
            return DocumentSaveResult::Error {
                operation_id: None,
                error: DocumentSaveError::IdentityFailed,
            };
        }
        let operation_id = entropy
            .iter()
            .map(|value| format!("{value:02x}"))
            .collect::<String>();
        if self
            .busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return DocumentSaveResult::Error {
                operation_id: Some(operation_id),
                error: DocumentSaveError::Busy,
            };
        }
        let runtime_guard = RuntimeGuard(Arc::clone(&self.busy));
        let Some(deadline) = Instant::now().checked_add(DOCUMENT_SAVE_DEADLINE) else {
            return DocumentSaveResult::Error {
                operation_id: Some(operation_id),
                error: DocumentSaveError::OperationTimedOut,
            };
        };
        let (response, receiver) = mpsc::sync_channel(RESPONSE_QUEUE_CAPACITY);
        if self
            .requests
            .try_send(WorkerRequest::SaveDocument {
                input,
                operation_id: operation_id.clone(),
                deadline,
                response,
                _runtime_guard: runtime_guard,
            })
            .is_err()
        {
            return DocumentSaveResult::Error {
                operation_id: Some(operation_id),
                error: DocumentSaveError::Busy,
            };
        }
        receiver.recv().unwrap_or(DocumentSaveResult::Error {
            operation_id: Some(operation_id),
            error: DocumentSaveError::StateUnavailable,
        })
    }

    fn acquire_guards(&self, subscription: bool) -> Result<RequestGuards, ActivationError> {
        self.busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| ActivationError::Busy)?;
        let runtime = RuntimeGuard(Arc::clone(&self.busy));
        let subscription = if subscription {
            Some(
                self.subscriptions
                    .begin_write_owned()
                    .map_err(|_| ActivationError::Busy)?,
            )
        } else {
            None
        };
        Ok(RequestGuards {
            _runtime: runtime,
            _subscription: subscription,
        })
    }

    pub(crate) fn with_runtime_state<T>(
        &self,
        action: impl FnOnce(bool) -> T,
    ) -> Result<T, ActivationError> {
        let _guard = self.acquire_guards(false)?;
        let stopped = matches!(
            self.observations.snapshot().sidecar_lifecycle,
            ObservedSidecarLifecycle::Stopped
        );
        Ok(action(stopped))
    }

    pub(crate) fn activate(&self, id: String, force: bool) -> ActivationResult {
        self.activate_with_expected_generation(id, force, None)
    }

    /// Routing 的显式 Apply 在状态门释放后仍以冻结 structural tuple 做最后预检。
    /// 普通订阅 activate 不使用此入口，既有立即切换语义保持不变。
    pub(crate) fn apply_proxy_routing(
        &self,
        active_subscription_id: String,
        expected_desired_generation: u64,
    ) -> ActivationResult {
        self.activate_with_expected_generation(
            active_subscription_id,
            true,
            Some(expected_desired_generation),
        )
    }

    fn activate_with_expected_generation(
        &self,
        id: String,
        force: bool,
        expected_desired_generation: Option<u64>,
    ) -> ActivationResult {
        let mut entropy = [0u8; 16];
        if getrandom::fill(&mut entropy).is_err() {
            return ActivationResult::Error {
                operation_id: None,
                error: ActivationError::IdentityFailed,
            };
        }
        let operation_id = entropy
            .iter()
            .map(|value| format!("{value:02x}"))
            .collect::<String>();
        let mut observed_routing_tuple = None;
        let mut routing_observation_started = false;
        let prepared = (|| {
            if self.is_shutting_down() {
                return Err(ActivationError::Busy);
            }
            if id.trim().is_empty() || id.len() > 128 {
                return Err(ActivationError::InvalidInput);
            }
            let guards = self.acquire_guards(true)?;
            let expected = {
                let _access = self
                    .state_gate
                    .try_lock()
                    .map_err(|_| ActivationError::Busy)?;
                JsonStateStore::new(self.state_file.clone())
                    .and_then(|store| store.load())
                    .map_err(|_| ActivationError::StateUnavailable)?
            };
            if expected_desired_generation.is_some() {
                observed_routing_tuple = Some((
                    expected
                        .active_subscription_id
                        .as_ref()
                        .map(|active| active.0.clone()),
                    expected.active_configuration_generation,
                ));
            }
            if expected_desired_generation.is_some_and(|generation| {
                expected.active_configuration_generation != generation
                    || expected
                        .active_subscription_id
                        .as_ref()
                        .is_none_or(|active| active.0 != id)
            }) {
                return Err(ActivationError::StateChanged);
            }
            if expected_desired_generation.is_some() {
                self.clear_routing_apply_terminal();
                switch_phase(
                    &self.observations,
                    &operation_id,
                    SubscriptionSwitchStatus::Queued,
                    None,
                );
                routing_observation_started = true;
            }
            if !expected.subscriptions.iter().any(|item| item.id.0 == id) {
                return Err(ActivationError::NotFound);
            }
            let mut candidate = expected.clone();
            let same_selection = candidate
                .active_subscription_id
                .as_ref()
                .map(|item| &item.0)
                == Some(&id);
            if candidate
                .active_subscription_id
                .as_ref()
                .map(|item| &item.0)
                != Some(&id)
            {
                candidate.active_subscription_id = Some(SubscriptionId(id.clone()));
                candidate.active_configuration_generation = candidate
                    .active_configuration_generation
                    .checked_add(1)
                    .filter(|value| *value <= 9_007_199_254_740_991)
                    .ok_or(ActivationError::ConfigurationFailed)?;
            }
            let projection = project_selected_runtime(&candidate).map_err(map_projection_error)?;
            let snapshot = self.observations.snapshot();
            if should_return_already_current(
                force,
                &snapshot,
                &id,
                candidate.active_configuration_generation,
            ) {
                let subscription = selected_summary(&candidate, &id)?;
                return Ok((
                    None,
                    Some(ActivationResult::AlreadyCurrent {
                        operation_id: operation_id.clone(),
                        subscription,
                        generation: candidate.active_configuration_generation,
                    }),
                ));
            }
            let config = compile_projection(&projection)
                .map_err(|_| ActivationError::ConfigurationFailed)?;
            let subscription = selected_summary(&candidate, &id)?;
            let cancel = Arc::new(AtomicBool::new(false));
            let (response, receiver) = mpsc::sync_channel(RESPONSE_QUEUE_CAPACITY);
            let request = ActivationRequest {
                operation_id: operation_id.clone(),
                expected,
                candidate,
                projection,
                subscription,
                config,
                reactivation: force && same_selection,
                cancel: Arc::clone(&cancel),
                _guards: guards,
                response,
            };
            self.requests
                .try_send(WorkerRequest::Activate(Box::new(request)))
                .map_err(|_| ActivationError::Busy)?;
            Ok((Some((receiver, cancel)), None))
        })();
        match prepared {
            Err(error) => {
                if expected_desired_generation.is_some()
                    && let Some(failure) =
                        routing_apply_terminal_failure(error, routing_observation_started)
                {
                    let (active_subscription_id, desired_generation) = observed_routing_tuple
                        .unwrap_or_else(|| {
                            (
                                Some(id.clone()),
                                expected_desired_generation
                                    .expect("routing Apply always supplies a generation"),
                            )
                        });
                    self.record_routing_apply_terminal(RoutingApplyTerminal {
                        operation_id: operation_id.clone(),
                        active_subscription_id,
                        desired_generation,
                        failure,
                    });
                    let (status, error_code) = match failure {
                        RoutingApplyTerminalFailure::ConfigurationFailed => (
                            SubscriptionSwitchStatus::Failed,
                            Some(SubscriptionSwitchErrorCode::ConfigurationFailed),
                        ),
                        RoutingApplyTerminalFailure::StateChanged => (
                            SubscriptionSwitchStatus::Cancelled,
                            Some(SubscriptionSwitchErrorCode::Cancelled),
                        ),
                    };
                    // StateChanged can be detected before the normal queued phase. Seed the same
                    // operation first so the observation port accepts its terminal transition.
                    switch_phase(
                        &self.observations,
                        &operation_id,
                        SubscriptionSwitchStatus::Queued,
                        None,
                    );
                    switch_phase(&self.observations, &operation_id, status, error_code);
                }
                activation_error(&operation_id, error)
            }
            Ok((_, Some(result))) => result,
            Ok((Some((receiver, cancel)), None)) => {
                match receiver.recv_timeout(START_RESPONSE_TIMEOUT) {
                    Ok(result) => result,
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        cancel.store(true, Ordering::Release);
                        ActivationResult::Pending { operation_id }
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        activation_error(&operation_id, ActivationError::RecoveryRequired)
                    }
                }
            }
            _ => unreachable!("activation preflight returns a response or a receiver"),
        }
    }

    pub(crate) fn runtime_observation_snapshot(
        &self,
    ) -> crate::application::observability::RuntimeObservationSnapshot {
        self.observations.snapshot()
    }

    pub(crate) fn routing_apply_terminal(&self) -> Option<RoutingApplyTerminal> {
        self.routing_apply_terminal
            .lock()
            .map(|terminal| terminal.clone())
            .unwrap_or_default()
    }

    fn clear_routing_apply_terminal(&self) {
        if let Ok(mut terminal) = self.routing_apply_terminal.lock() {
            *terminal = None;
        }
    }

    fn record_routing_apply_terminal(&self, value: RoutingApplyTerminal) {
        if let Ok(mut terminal) = self.routing_apply_terminal.lock() {
            *terminal = Some(value);
        }
    }

    pub(crate) fn selector_runtime_nodes(&self) -> BTreeMap<PoolId, NodeId> {
        self.selector_runtime_nodes
            .lock()
            .map(|values| values.clone())
            .unwrap_or_default()
    }

    /// 持久 desired selection 已由调用方提交；本入口只协调当前 owned runtime。
    pub(crate) fn reconcile_manual_selection(
        &self,
        pool_id: PoolId,
        node_id: NodeId,
    ) -> ManualSelectionRuntimeResult {
        if self.is_shutting_down() {
            return ManualSelectionRuntimeResult::SavedOnly(
                ManualSelectionSavedOnlyReason::DispatchUnavailable,
            );
        }
        let guards = match self.acquire_guards(false) {
            Ok(guards) => guards,
            Err(_) => {
                return ManualSelectionRuntimeResult::SavedOnly(
                    ManualSelectionSavedOnlyReason::DispatchUnavailable,
                );
            }
        };
        let (response, receiver) = mpsc::sync_channel(RESPONSE_QUEUE_CAPACITY);
        let request = ManualSelectionRequest {
            pool_id,
            node_id,
            response,
            _guards: guards,
        };
        if self
            .requests
            .try_send(WorkerRequest::ReconcileManualSelection(request))
            .is_err()
        {
            return ManualSelectionRuntimeResult::SavedOnly(
                ManualSelectionSavedOnlyReason::DispatchUnavailable,
            );
        }
        match receiver.recv_timeout(SELECTOR_RESPONSE_TIMEOUT) {
            Ok(result) => result,
            Err(_) => ManualSelectionRuntimeResult::ApplyUnknown(
                ManualSelectionUnknownReason::ReadBackUnavailable,
            ),
        }
    }
}

fn should_return_already_current(
    force: bool,
    snapshot: &crate::application::observability::RuntimeObservationSnapshot,
    id: &str,
    generation: u64,
) -> bool {
    !force
        && snapshot.sidecar_lifecycle == ObservedSidecarLifecycle::Ready
        && snapshot.applied_subscription_id.as_deref() == Some(id)
        && snapshot.applied_configuration_generation == Some(generation)
}

impl Drop for ManagedObservationRuntimeController {
    fn drop(&mut self) {
        self.shutdown_requested.store(true, Ordering::Release);
        if let Some(scheduler) = &self.scheduler {
            scheduler.request_stop();
        }
        let (response, _) = mpsc::sync_channel(RESPONSE_QUEUE_CAPACITY);
        let _ = self.requests.try_send(WorkerRequest::Shutdown { response });
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LoadRuntimeIntentError {
    Busy,
    Unavailable,
    SelectionRequired,
    Configuration,
}

fn load_runtime_intent(
    state_file: &Path,
    state_gate: &StateAccessGate,
) -> Result<ObservationCompilationInput, LoadRuntimeIntentError> {
    let _access = state_gate.try_lock().map_err(|error| match error {
        StateAccessError::Busy => LoadRuntimeIntentError::Busy,
        StateAccessError::Unavailable => LoadRuntimeIntentError::Unavailable,
    })?;
    let store = JsonStateStore::new(state_file.to_path_buf())
        .map_err(|_| LoadRuntimeIntentError::Unavailable)?;
    let state = store
        .load()
        .map_err(|_| LoadRuntimeIntentError::Unavailable)?;
    if state.active_subscription_id.is_none() {
        return Err(LoadRuntimeIntentError::SelectionRequired);
    }
    project_selected_runtime(&state).map_err(|_| LoadRuntimeIntentError::Configuration)
}

fn worker_loop(requests: Receiver<WorkerRequest>, context: WorkerContext) {
    let observations = &context.observations;
    let mut runtime: Option<SidecarRuntime<WindowsManagedSidecarPort>> = None;
    let mut applied_routing_index: Option<AppliedRoutingIndex> = None;
    loop {
        match requests.recv_timeout(SAMPLE_INTERVAL) {
            Ok(WorkerRequest::Start {
                intent,
                response,
                _guards,
            }) => {
                let result = if context.shutdown_requested.load(Ordering::Acquire) {
                    ManagedRuntimeStartResult::Busy
                } else {
                    start_runtime(
                        &mut runtime,
                        &context.resource_root,
                        &context.app_local_data_root,
                        &intent,
                        observations,
                    )
                };
                if result == ManagedRuntimeStartResult::Started {
                    install_applied_routing_index(
                        &mut applied_routing_index,
                        &context.selector_runtime_nodes,
                        &intent,
                    );
                }
                if response.send(result).is_err() {
                    observations.record_managed_failure(None, ManagedRuntimeFailure::Worker);
                }
            }
            Ok(WorkerRequest::Stop { response, _guards }) => {
                let result = stop_runtime(&mut runtime, observations);
                if matches!(
                    result,
                    ManagedRuntimeStopResult::Stopped | ManagedRuntimeStopResult::AlreadyStopped
                ) {
                    clear_applied_routing_index(
                        &mut applied_routing_index,
                        &context.selector_runtime_nodes,
                    );
                }
                if response.send(result).is_err() {
                    observations.record_managed_failure(None, ManagedRuntimeFailure::Worker);
                }
            }
            Ok(WorkerRequest::Activate(request)) => {
                let projection = request.projection.clone();
                switch_phase(
                    observations,
                    &request.operation_id,
                    SubscriptionSwitchStatus::Queued,
                    None,
                );
                let response = request.response.clone();
                if runtime.is_none() {
                    match WindowsManagedSidecarPort::new(
                        context.resource_root.clone(),
                        context.app_local_data_root.clone(),
                    ) {
                        Ok(port) => runtime = Some(SidecarRuntime::new_observation_only(port)),
                        Err(_) => {
                            switch_phase(
                                observations,
                                &request.operation_id,
                                SubscriptionSwitchStatus::Failed,
                                Some(SubscriptionSwitchErrorCode::StartFailed),
                            );
                            let _ = response.send(activation_error(
                                &request.operation_id,
                                ActivationError::StartFailed,
                            ));
                            continue;
                        }
                    }
                }
                let result = apply_activation(
                    runtime.as_mut().expect("runtime initialized"),
                    *request,
                    &context,
                );
                if matches!(
                    result,
                    ActivationResult::Activated { .. } | ActivationResult::Reactivated { .. }
                ) {
                    install_applied_routing_index(
                        &mut applied_routing_index,
                        &context.selector_runtime_nodes,
                        &projection,
                    );
                } else if runtime.as_ref().is_some_and(|runtime| {
                    runtime.snapshot().lifecycle == SidecarLifecycle::Stopped
                }) {
                    clear_applied_routing_index(
                        &mut applied_routing_index,
                        &context.selector_runtime_nodes,
                    );
                }
                // 丢失等待者不回滚已经 Ready 的实例；权威观测仍保留最终结果。
                let _ = response.send(result);
            }
            Ok(WorkerRequest::ReconcileManualSelection(request)) => {
                let result = reconcile_manual_selection(
                    runtime.as_mut(),
                    applied_routing_index.as_ref(),
                    &context.observations,
                    &context.selector_runtime_nodes,
                    &request.pool_id,
                    &request.node_id,
                );
                let _ = request.response.send(result);
            }
            Ok(WorkerRequest::ReadRunningConfiguration { response, _guards }) => {
                let snapshot = observations.snapshot();
                let result = read_running_configuration(runtime.as_ref(), &snapshot);
                let _ = response.send(result);
            }
            Ok(WorkerRequest::SaveDocument {
                input,
                operation_id,
                deadline,
                response,
                _runtime_guard,
            }) => {
                let result = apply_document_save(
                    &mut runtime,
                    input,
                    operation_id,
                    deadline,
                    &context,
                    || {
                        WindowsManagedSidecarPort::new(
                            context.resource_root.clone(),
                            context.app_local_data_root.clone(),
                        )
                        .map(SidecarRuntime::new_observation_only)
                        .map_err(|_| ())
                    },
                    || {},
                );
                if matches!(
                    &result,
                    DocumentSaveResult::Ok {
                        apply: DocumentApplyResult::Ready { .. },
                        ..
                    }
                ) {
                    if let Ok(projection) = load_runtime_intent(
                        &context.app_local_data_root.join("state.json"),
                        &context.state_gate,
                    ) {
                        install_applied_routing_index(
                            &mut applied_routing_index,
                            &context.selector_runtime_nodes,
                            &projection,
                        );
                    }
                } else if runtime.as_ref().is_some_and(|runtime| {
                    runtime.snapshot().lifecycle == SidecarLifecycle::Stopped
                }) {
                    clear_applied_routing_index(
                        &mut applied_routing_index,
                        &context.selector_runtime_nodes,
                    );
                }
                // 接收端消失不取消已开始的保存；worker仍持有guard直至确定终态。
                let _ = response.send(result);
            }
            Ok(WorkerRequest::Shutdown { response }) => {
                let result = match stop_runtime(&mut runtime, observations) {
                    ManagedRuntimeStopResult::Stopped
                    | ManagedRuntimeStopResult::AlreadyStopped => ShutdownResult::ShutdownComplete,
                    ManagedRuntimeStopResult::StopFailed | ManagedRuntimeStopResult::Busy => {
                        ShutdownResult::ShutdownFailed
                    }
                };
                if result == ShutdownResult::ShutdownComplete {
                    clear_applied_routing_index(
                        &mut applied_routing_index,
                        &context.selector_runtime_nodes,
                    );
                }
                let complete = result == ShutdownResult::ShutdownComplete;
                context.shutdown_complete.store(complete, Ordering::Release);
                let _ = response.send(result);
                if complete {
                    return;
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => sample_runtime(runtime.as_mut(), observations),
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                let _ = stop_runtime(&mut runtime, observations);
                return;
            }
        }
    }
}

fn read_running_configuration<P: SidecarPort>(
    runtime: Option<&SidecarRuntime<P>>,
    snapshot: &crate::application::observability::RuntimeObservationSnapshot,
) -> RunningConfigurationResult {
    match runtime {
        Some(runtime)
            if runtime.snapshot().lifecycle == SidecarLifecycle::Ready
                && snapshot.sidecar_lifecycle == ObservedSidecarLifecycle::Ready =>
        {
            runtime
                .with_active_config(|config| {
                    if config.as_bytes().len() > RUNNING_CONFIGURATION_MAX_BYTES {
                        return RunningConfigurationResult::Error(
                            RunningConfigurationError::ContentTooLarge,
                        );
                    }
                    let (Some(id), Some(generation)) = (
                        snapshot.applied_subscription_id.clone(),
                        snapshot.applied_configuration_generation,
                    ) else {
                        return RunningConfigurationResult::Error(
                            RunningConfigurationError::RecoveryRequired,
                        );
                    };
                    String::from_utf8(config.as_bytes().to_vec()).map_or(
                        RunningConfigurationResult::Error(
                            RunningConfigurationError::RecoveryRequired,
                        ),
                        |content| RunningConfigurationResult::Ok {
                            content,
                            applied_subscription_id: id,
                            applied_configuration_generation: generation,
                        },
                    )
                })
                .unwrap_or(RunningConfigurationResult::Error(
                    RunningConfigurationError::RecoveryRequired,
                ))
        }
        Some(runtime) if runtime.snapshot().lifecycle == SidecarLifecycle::RecoveryRequired => {
            RunningConfigurationResult::Error(RunningConfigurationError::RecoveryRequired)
        }
        Some(runtime) if runtime.snapshot().lifecycle == SidecarLifecycle::Ready => {
            RunningConfigurationResult::Error(RunningConfigurationError::RecoveryRequired)
        }
        _ => RunningConfigurationResult::Error(RunningConfigurationError::NotRunning),
    }
}

fn install_applied_routing_index(
    index: &mut Option<AppliedRoutingIndex>,
    selector_runtime_nodes: &Mutex<BTreeMap<PoolId, NodeId>>,
    projection: &SelectedRuntimeProjection,
) {
    *index = Some(AppliedRoutingIndex::from_projection(projection));
    if let Ok(mut nodes) = selector_runtime_nodes.lock() {
        nodes.clear();
    }
}

fn clear_applied_routing_index(
    index: &mut Option<AppliedRoutingIndex>,
    selector_runtime_nodes: &Mutex<BTreeMap<PoolId, NodeId>>,
) {
    *index = None;
    if let Ok(mut nodes) = selector_runtime_nodes.lock() {
        nodes.clear();
    }
}

fn reconcile_manual_selection<P: SidecarPort>(
    runtime: Option<&mut SidecarRuntime<P>>,
    index: Option<&AppliedRoutingIndex>,
    observations: &InMemoryRuntimeObservations,
    selector_runtime_nodes: &Mutex<BTreeMap<PoolId, NodeId>>,
    pool_id: &PoolId,
    node_id: &NodeId,
) -> ManualSelectionRuntimeResult {
    let Some(runtime) = runtime else {
        return ManualSelectionRuntimeResult::SavedOnly(
            ManualSelectionSavedOnlyReason::RuntimeStopped,
        );
    };
    match runtime.snapshot().lifecycle {
        SidecarLifecycle::Stopped => {
            return ManualSelectionRuntimeResult::SavedOnly(
                ManualSelectionSavedOnlyReason::RuntimeStopped,
            );
        }
        SidecarLifecycle::RecoveryRequired => {
            return ManualSelectionRuntimeResult::SavedOnly(
                ManualSelectionSavedOnlyReason::RuntimeNotReady,
            );
        }
        SidecarLifecycle::Ready => {}
    }
    let observed = observations.snapshot();
    if observed.sidecar_lifecycle != ObservedSidecarLifecycle::Ready {
        return ManualSelectionRuntimeResult::SavedOnly(
            ManualSelectionSavedOnlyReason::RuntimeNotReady,
        );
    }
    let Some(index) = index.filter(|index| {
        observed.applied_configuration_generation == Some(index.configuration_generation)
    }) else {
        return ManualSelectionRuntimeResult::SavedOnly(
            ManualSelectionSavedOnlyReason::NotInAppliedArtifact,
        );
    };
    let Some(tags) = index
        .pool_members
        .get(pool_id)
        .and_then(|members| members.get(node_id))
        .cloned()
    else {
        return ManualSelectionRuntimeResult::SavedOnly(
            ManualSelectionSavedOnlyReason::NotInAppliedArtifact,
        );
    };
    let Some(instance_identity) = runtime
        .with_active_port(|_, child| Ok(child.identity()))
        .ok()
        .flatten()
    else {
        return ManualSelectionRuntimeResult::SavedOnly(
            ManualSelectionSavedOnlyReason::RuntimeNotReady,
        );
    };
    let Some(secret) = runtime
        .with_active_config(api_secret_from_config)
        .and_then(Result::ok)
    else {
        return ManualSelectionRuntimeResult::SavedOnly(
            ManualSelectionSavedOnlyReason::RuntimeNotReady,
        );
    };
    let client = match ClashApiClient::new(&secret) {
        Ok(client) => client,
        Err(_) => {
            return ManualSelectionRuntimeResult::SavedOnly(
                ManualSelectionSavedOnlyReason::DispatchUnavailable,
            );
        }
    };
    let runtime_tag = tauri::async_runtime::block_on(async {
        // PUT 的 transport/response 不是终态；同一 owned worker 总是继续 GET。
        let _ = client.write_selector(&tags.pool, &tags.node).await;
        client.read_selector(&tags.pool).await
    });
    let identity_after = runtime
        .with_active_port(|_, child| Ok(child.identity()))
        .ok()
        .flatten();
    if identity_after != Some(instance_identity)
        || observations.snapshot().applied_configuration_generation
            != Some(index.configuration_generation)
    {
        if let Ok(mut values) = selector_runtime_nodes.lock() {
            values.remove(pool_id);
        }
        return ManualSelectionRuntimeResult::ApplyUnknown(
            ManualSelectionUnknownReason::InstanceChanged,
        );
    }
    let runtime_tag = match runtime_tag {
        Ok(tag) => tag,
        Err(_) => {
            if let Ok(mut values) = selector_runtime_nodes.lock() {
                values.remove(pool_id);
            }
            return ManualSelectionRuntimeResult::ApplyUnknown(
                ManualSelectionUnknownReason::ReadBackUnavailable,
            );
        }
    };
    classify_selector_read_back(
        index,
        selector_runtime_nodes,
        pool_id,
        node_id,
        &runtime_tag,
    )
}

fn classify_selector_read_back(
    index: &AppliedRoutingIndex,
    selector_runtime_nodes: &Mutex<BTreeMap<PoolId, NodeId>>,
    pool_id: &PoolId,
    desired_node_id: &NodeId,
    runtime_tag: &str,
) -> ManualSelectionRuntimeResult {
    let runtime_node_id = index.pool_members.get(pool_id).and_then(|members| {
        members
            .iter()
            .find_map(|(node_id, tags)| (tags.node == runtime_tag).then(|| node_id.clone()))
    });
    let Some(runtime_node_id) = runtime_node_id else {
        if let Ok(mut values) = selector_runtime_nodes.lock() {
            values.remove(pool_id);
        }
        return ManualSelectionRuntimeResult::ApplyUnknown(
            ManualSelectionUnknownReason::UnknownRuntimeNode,
        );
    };
    if let Ok(mut values) = selector_runtime_nodes.lock() {
        values.insert(pool_id.clone(), runtime_node_id.clone());
    }
    if runtime_node_id == *desired_node_id {
        ManualSelectionRuntimeResult::Applied
    } else {
        ManualSelectionRuntimeResult::NotApplied { runtime_node_id }
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

fn map_projection_error(error: SelectionProjectionError) -> ActivationError {
    match error {
        SelectionProjectionError::SelectionRequired | SelectionProjectionError::NotFound => {
            ActivationError::NotFound
        }
        SelectionProjectionError::SelectionConflict => ActivationError::SelectionConflict,
        SelectionProjectionError::ConfigurationFailed => ActivationError::ConfigurationFailed,
    }
}

fn selected_summary(state: &AppState, id: &str) -> Result<SubscriptionSummary, ActivationError> {
    summaries(state)
        .map_err(|_| ActivationError::ConfigurationFailed)?
        .into_iter()
        .find(|item| item.id == id)
        .ok_or(ActivationError::NotFound)
}

fn activation_error(operation_id: &str, error: ActivationError) -> ActivationResult {
    ActivationResult::Error {
        operation_id: Some(operation_id.to_owned()),
        error,
    }
}

fn routing_apply_terminal_failure(
    error: ActivationError,
    observation_started: bool,
) -> Option<RoutingApplyTerminalFailure> {
    match error {
        ActivationError::NotFound
        | ActivationError::SelectionConflict
        | ActivationError::ConfigurationFailed => {
            Some(RoutingApplyTerminalFailure::ConfigurationFailed)
        }
        ActivationError::StateChanged => Some(RoutingApplyTerminalFailure::StateChanged),
        ActivationError::Busy if observation_started => {
            Some(RoutingApplyTerminalFailure::StateChanged)
        }
        _ => None,
    }
}

fn switch_phase(
    observations: &InMemoryRuntimeObservations,
    operation_id: &str,
    status: SubscriptionSwitchStatus,
    error_code: Option<SubscriptionSwitchErrorCode>,
) {
    observations.record_subscription_switch(SubscriptionSwitch {
        operation_id: operation_id.to_owned(),
        status,
        error_code,
    });
}

fn compile_projection(intent: &SelectedRuntimeProjection) -> Result<GeneratedConfig, ()> {
    let plan = SingBoxCompiler
        .compile(
            &intent.runtime_intent,
            &intent.projected_default_target,
            DnsPolicy::System,
            RuntimeProfile::ObservationOnly,
        )
        .map_err(|_| ())?;
    let secret = generate_api_secret().map_err(|_| ())?;
    plan.finalize(&secret).map_err(|_| ())
}

fn apply_activation<P: SidecarPort>(
    runtime: &mut SidecarRuntime<P>,
    request: ActivationRequest,
    context: &WorkerContext,
) -> ActivationResult {
    let observations = &context.observations;
    let operation_id = &request.operation_id;
    switch_phase(
        observations,
        operation_id,
        SubscriptionSwitchStatus::Checking,
        None,
    );
    if let Err(error) = runtime.prepare_replacement(request.config) {
        return activation_runtime_failure(runtime, observations, operation_id, error);
    }
    switch_phase(
        observations,
        operation_id,
        SubscriptionSwitchStatus::Prepared,
        None,
    );
    if request.cancel.load(Ordering::Acquire) || context.shutdown_requested.load(Ordering::Acquire)
    {
        if let Err(error) = runtime.cancel_prepared() {
            return activation_runtime_failure(runtime, observations, operation_id, error);
        }
        switch_phase(
            observations,
            operation_id,
            SubscriptionSwitchStatus::Cancelled,
            Some(SubscriptionSwitchErrorCode::Cancelled),
        );
        return ActivationResult::Pending {
            operation_id: operation_id.clone(),
        };
    }
    let persisted = (|| {
        let _access = context
            .state_gate
            .try_lock()
            .map_err(|_| ActivationError::Busy)?;
        let store = JsonStateStore::new(context.app_local_data_root.join("state.json"))
            .map_err(|_| ActivationError::StateUnavailable)?;
        let current = store
            .load()
            .map_err(|_| ActivationError::StateUnavailable)?;
        if current != request.expected {
            return Err(ActivationError::Busy);
        }
        // 获取短门控之后再次检查取消，保存成功之后则必须完成生命周期收敛。
        if request.cancel.load(Ordering::Acquire)
            || context.shutdown_requested.load(Ordering::Acquire)
        {
            return Ok(false);
        }
        if request.candidate != request.expected {
            store
                .save(&request.candidate)
                .map_err(|_| ActivationError::SaveFailed)?;
        }
        Ok(true)
    })();
    match persisted {
        Ok(true) => {}
        other => {
            if let Err(error) = runtime.cancel_prepared() {
                return activation_runtime_failure(runtime, observations, operation_id, error);
            }
            match other {
                Ok(false) => {
                    switch_phase(
                        observations,
                        operation_id,
                        SubscriptionSwitchStatus::Cancelled,
                        Some(SubscriptionSwitchErrorCode::Cancelled),
                    );
                    return ActivationResult::Pending {
                        operation_id: operation_id.clone(),
                    };
                }
                Err(error) => {
                    let code = match error {
                        ActivationError::Busy => SubscriptionSwitchErrorCode::Busy,
                        ActivationError::SaveFailed => SubscriptionSwitchErrorCode::SaveFailed,
                        _ => SubscriptionSwitchErrorCode::StateUnavailable,
                    };
                    observations.record_managed_failure(None, ManagedRuntimeFailure::Configuration);
                    switch_phase(
                        observations,
                        operation_id,
                        SubscriptionSwitchStatus::Failed,
                        Some(code),
                    );
                    return activation_error(operation_id, error);
                }
                Ok(true) => unreachable!(),
            }
        }
    }
    switch_phase(
        observations,
        operation_id,
        SubscriptionSwitchStatus::Persisted,
        None,
    );
    // 无变化的“重新应用”不伪报订阅内容更新。
    if request.candidate != request.expected {
        context.subscriptions.notify_change(
            &request.projection.selected_subscription_id.0,
            crate::application::subscription_management::SubscriptionChange::Activated,
        );
    }
    switch_phase(
        observations,
        operation_id,
        SubscriptionSwitchStatus::Applying,
        None,
    );
    if let Err(error) = runtime.commit_prepared() {
        return activation_runtime_failure(runtime, observations, operation_id, error);
    }
    let generation = request.projection.selected_generation;
    let id = &request.projection.selected_subscription_id.0;
    observations.record_subscription_ready(
        id.clone(),
        generation,
        Some(operation_id.clone()),
        runtime.mixed_port().is_some(),
    );
    if request.reactivation {
        ActivationResult::Reactivated {
            operation_id: operation_id.clone(),
            subscription: request.subscription,
            generation,
        }
    } else {
        ActivationResult::Activated {
            operation_id: operation_id.clone(),
            subscription: request.subscription,
            generation,
        }
    }
}

#[derive(Debug)]
pub(crate) enum RunningConfigurationResult {
    Ok {
        content: String,
        applied_subscription_id: String,
        applied_configuration_generation: u64,
    },
    Error(RunningConfigurationError),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum RunningConfigurationError {
    NotRunning,
    RecoveryRequired,
    ContentTooLarge,
    Busy,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum DocumentApplyError {
    StopFailed,
    StartFailed,
    RecoveryRequired,
    OperationTimedOut,
}

#[derive(Debug)]
pub(crate) enum DocumentApplyResult {
    NotRequired,
    Ready {
        operation_id: String,
        generation: u64,
    },
    Failed {
        operation_id: String,
        generation: u64,
        error: DocumentApplyError,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum DocumentSaveError {
    InvalidInput,
    NotFound,
    DocumentUnavailable,
    DocumentConflict,
    ContentTooLarge,
    FormatFailed,
    ParseFailed,
    UnsupportedNodes,
    UnsupportedClashProviders,
    NormalizationFailed,
    ValidationFailed,
    ConfigurationFailed,
    SaveFailed,
    StateUnavailable,
    IdentityFailed,
    Busy,
    RecoveryRequired,
    OperationTimedOut,
}

#[derive(Debug)]
pub(crate) enum DocumentSaveResult {
    Ok {
        subscription: Box<SubscriptionSummary>,
        document_revision: String,
        apply: DocumentApplyResult,
    },
    Error {
        operation_id: Option<String>,
        error: DocumentSaveError,
    },
}

fn map_document_error(error: DocumentOperationError) -> DocumentSaveError {
    match error {
        DocumentOperationError::InvalidInput => DocumentSaveError::InvalidInput,
        DocumentOperationError::ContentTooLarge => DocumentSaveError::ContentTooLarge,
        DocumentOperationError::FormatFailed => DocumentSaveError::FormatFailed,
        DocumentOperationError::ParseFailed => DocumentSaveError::ParseFailed,
        DocumentOperationError::UnsupportedNodes => DocumentSaveError::UnsupportedNodes,
        DocumentOperationError::UnsupportedClashProviders => {
            DocumentSaveError::UnsupportedClashProviders
        }
        DocumentOperationError::NormalizationFailed => DocumentSaveError::NormalizationFailed,
        DocumentOperationError::ValidationFailed => DocumentSaveError::ValidationFailed,
        DocumentOperationError::DocumentConflict => DocumentSaveError::DocumentConflict,
        DocumentOperationError::DocumentUnavailable => DocumentSaveError::DocumentUnavailable,
        DocumentOperationError::NotFound => DocumentSaveError::NotFound,
        DocumentOperationError::StateUnavailable => DocumentSaveError::StateUnavailable,
        DocumentOperationError::SaveFailed => DocumentSaveError::SaveFailed,
        DocumentOperationError::Busy => DocumentSaveError::Busy,
        DocumentOperationError::OperationTimedOut => DocumentSaveError::OperationTimedOut,
    }
}

fn document_error(operation_id: String, error: DocumentSaveError) -> DocumentSaveResult {
    DocumentSaveResult::Error {
        operation_id: Some(operation_id),
        error,
    }
}

fn document_switch_error_code(error: DocumentSaveError) -> SubscriptionSwitchErrorCode {
    match error {
        DocumentSaveError::DocumentConflict | DocumentSaveError::Busy => {
            SubscriptionSwitchErrorCode::Busy
        }
        DocumentSaveError::SaveFailed => SubscriptionSwitchErrorCode::SaveFailed,
        DocumentSaveError::NotFound
        | DocumentSaveError::DocumentUnavailable
        | DocumentSaveError::StateUnavailable
        | DocumentSaveError::IdentityFailed => SubscriptionSwitchErrorCode::StateUnavailable,
        DocumentSaveError::RecoveryRequired => SubscriptionSwitchErrorCode::RecoveryRequired,
        DocumentSaveError::OperationTimedOut => SubscriptionSwitchErrorCode::OperationTimedOut,
        DocumentSaveError::InvalidInput
        | DocumentSaveError::ContentTooLarge
        | DocumentSaveError::FormatFailed
        | DocumentSaveError::ParseFailed
        | DocumentSaveError::UnsupportedNodes
        | DocumentSaveError::UnsupportedClashProviders
        | DocumentSaveError::NormalizationFailed
        | DocumentSaveError::ValidationFailed
        | DocumentSaveError::ConfigurationFailed => {
            SubscriptionSwitchErrorCode::ConfigurationFailed
        }
    }
}

fn failed_document_switch(
    observations: &InMemoryRuntimeObservations,
    operation_id: String,
    error: DocumentSaveError,
) -> DocumentSaveResult {
    switch_phase(
        observations,
        &operation_id,
        SubscriptionSwitchStatus::Failed,
        Some(document_switch_error_code(error)),
    );
    document_error(operation_id, error)
}

fn document_runtime_error<P: SidecarPort>(
    runtime: &SidecarRuntime<P>,
    observations: &InMemoryRuntimeObservations,
    operation_id: &str,
    generation: u64,
    error: SidecarError,
) -> DocumentApplyResult {
    let lifecycle = observed_lifecycle(runtime.snapshot().lifecycle);
    let (wire, switch_error, log) = if error == SidecarError::ActiveStop {
        (
            DocumentApplyError::StopFailed,
            SubscriptionSwitchErrorCode::StopFailed,
            ManagedRuntimeFailure::Stop,
        )
    } else if lifecycle == ObservedSidecarLifecycle::RecoveryRequired {
        (
            DocumentApplyError::RecoveryRequired,
            SubscriptionSwitchErrorCode::RecoveryRequired,
            ManagedRuntimeFailure::Stop,
        )
    } else {
        (
            DocumentApplyError::StartFailed,
            SubscriptionSwitchErrorCode::StartFailed,
            ManagedRuntimeFailure::Start,
        )
    };
    observations.record_managed_failure(Some(lifecycle), log);
    switch_phase(
        observations,
        operation_id,
        SubscriptionSwitchStatus::Failed,
        Some(switch_error),
    );
    DocumentApplyResult::Failed {
        operation_id: operation_id.to_owned(),
        generation,
        error: wire,
    }
}

fn apply_document_save<P: SidecarPort>(
    runtime: &mut Option<SidecarRuntime<P>>,
    input: DocumentSaveInput,
    operation_id: String,
    deadline: Instant,
    context: &WorkerContext,
    create_runtime: impl FnOnce() -> Result<SidecarRuntime<P>, ()>,
    after_persist: impl FnOnce(),
) -> DocumentSaveResult {
    let target_id = input.id.clone();
    let prepared = match context.subscriptions.prepare_document_save(input) {
        Ok(prepared) => prepared,
        Err(error) => return document_error(operation_id, map_document_error(error)),
    };
    if Instant::now() >= deadline {
        return document_error(operation_id, DocumentSaveError::OperationTimedOut);
    }
    let observed = context.observations.snapshot();
    let selected = prepared
        .next()
        .active_subscription_id
        .as_ref()
        .is_some_and(|id| id.0 == target_id);
    let actual_selected_ready = selected
        && runtime
            .as_ref()
            .is_some_and(|runtime| runtime.snapshot().lifecycle == SidecarLifecycle::Ready)
        && observed.sidecar_lifecycle == ObservedSidecarLifecycle::Ready
        && observed.applied_subscription_id.as_deref() == Some(target_id.as_str())
        && observed.applied_configuration_generation == Some(prepared.generation());
    if !prepared.requires_apply() && (!selected || actual_selected_ready) {
        return match context.subscriptions.commit_prepared_document(&prepared) {
            Ok(commit) => DocumentSaveResult::Ok {
                subscription: Box::new(commit.subscription),
                document_revision: commit.document_revision,
                apply: DocumentApplyResult::NotRequired,
            },
            Err(error) => document_error(operation_id, map_document_error(error)),
        };
    }

    switch_phase(
        &context.observations,
        &operation_id,
        SubscriptionSwitchStatus::Queued,
        None,
    );
    switch_phase(
        &context.observations,
        &operation_id,
        SubscriptionSwitchStatus::Checking,
        None,
    );
    let projection = match project_selected_runtime(prepared.next()) {
        Ok(projection) => projection,
        Err(_) => {
            return failed_document_switch(
                &context.observations,
                operation_id,
                DocumentSaveError::ConfigurationFailed,
            );
        }
    };
    let config = match compile_projection(&projection) {
        Ok(config) => config,
        Err(_) => {
            return failed_document_switch(
                &context.observations,
                operation_id,
                DocumentSaveError::ConfigurationFailed,
            );
        }
    };
    if runtime.is_none() {
        let created = match create_runtime() {
            Ok(runtime) => runtime,
            Err(_) => {
                return failed_document_switch(
                    &context.observations,
                    operation_id,
                    DocumentSaveError::ConfigurationFailed,
                );
            }
        };
        *runtime = Some(created);
    }
    let runtime = runtime.as_mut().expect("runtime initialized");
    if runtime.prepare_replacement(config).is_err() {
        let lifecycle = observed_lifecycle(runtime.snapshot().lifecycle);
        let wire = if lifecycle == ObservedSidecarLifecycle::RecoveryRequired {
            context
                .observations
                .record_managed_failure(Some(lifecycle), ManagedRuntimeFailure::Stop);
            DocumentSaveError::RecoveryRequired
        } else {
            DocumentSaveError::ConfigurationFailed
        };
        return failed_document_switch(&context.observations, operation_id, wire);
    }
    switch_phase(
        &context.observations,
        &operation_id,
        SubscriptionSwitchStatus::Prepared,
        None,
    );
    if Instant::now() >= deadline || context.shutdown_requested.load(Ordering::Acquire) {
        if runtime.cancel_prepared().is_err() {
            context.observations.record_managed_failure(
                Some(ObservedSidecarLifecycle::RecoveryRequired),
                ManagedRuntimeFailure::Stop,
            );
            return failed_document_switch(
                &context.observations,
                operation_id,
                DocumentSaveError::RecoveryRequired,
            );
        }
        let error = if context.shutdown_requested.load(Ordering::Acquire) {
            DocumentSaveError::Busy
        } else {
            DocumentSaveError::OperationTimedOut
        };
        return failed_document_switch(&context.observations, operation_id, error);
    }
    let commit = match context.subscriptions.commit_prepared_document(&prepared) {
        Ok(commit) => commit,
        Err(error) => {
            if runtime.cancel_prepared().is_err() {
                context.observations.record_managed_failure(
                    Some(ObservedSidecarLifecycle::RecoveryRequired),
                    ManagedRuntimeFailure::Stop,
                );
                return failed_document_switch(
                    &context.observations,
                    operation_id,
                    DocumentSaveError::RecoveryRequired,
                );
            }
            return failed_document_switch(
                &context.observations,
                operation_id,
                map_document_error(error),
            );
        }
    };
    switch_phase(
        &context.observations,
        &operation_id,
        SubscriptionSwitchStatus::Persisted,
        None,
    );
    after_persist();
    let generation = prepared.generation();
    if Instant::now() >= deadline {
        let apply_error = if runtime.cancel_prepared().is_err() {
            context.observations.record_managed_failure(
                Some(ObservedSidecarLifecycle::RecoveryRequired),
                ManagedRuntimeFailure::Stop,
            );
            DocumentApplyError::RecoveryRequired
        } else {
            DocumentApplyError::OperationTimedOut
        };
        switch_phase(
            &context.observations,
            &operation_id,
            SubscriptionSwitchStatus::Failed,
            Some(if apply_error == DocumentApplyError::RecoveryRequired {
                SubscriptionSwitchErrorCode::RecoveryRequired
            } else {
                SubscriptionSwitchErrorCode::OperationTimedOut
            }),
        );
        return DocumentSaveResult::Ok {
            subscription: Box::new(commit.subscription),
            document_revision: commit.document_revision,
            apply: DocumentApplyResult::Failed {
                operation_id,
                generation,
                error: apply_error,
            },
        };
    }
    switch_phase(
        &context.observations,
        &operation_id,
        SubscriptionSwitchStatus::Applying,
        None,
    );
    let apply = match runtime.commit_prepared() {
        Ok(()) => {
            let id = projection.selected_subscription_id.0;
            context.observations.record_subscription_ready(
                id,
                generation,
                Some(operation_id.clone()),
                runtime.mixed_port().is_some(),
            );
            DocumentApplyResult::Ready {
                operation_id,
                generation,
            }
        }
        Err(error) => document_runtime_error(
            runtime,
            &context.observations,
            &operation_id,
            generation,
            error,
        ),
    };
    DocumentSaveResult::Ok {
        subscription: Box::new(commit.subscription),
        document_revision: commit.document_revision,
        apply,
    }
}

fn activation_runtime_failure<P: SidecarPort>(
    runtime: &SidecarRuntime<P>,
    observations: &InMemoryRuntimeObservations,
    operation_id: &str,
    error: SidecarError,
) -> ActivationResult {
    let lifecycle = observed_lifecycle(runtime.snapshot().lifecycle);
    let (wire, code, log) = if error == SidecarError::ActiveStop {
        (
            ActivationError::StopFailed,
            SubscriptionSwitchErrorCode::StopFailed,
            ManagedRuntimeFailure::Stop,
        )
    } else if lifecycle == ObservedSidecarLifecycle::RecoveryRequired {
        (
            ActivationError::RecoveryRequired,
            SubscriptionSwitchErrorCode::RecoveryRequired,
            ManagedRuntimeFailure::Stop,
        )
    } else {
        match error {
            SidecarError::CandidateCheck | SidecarError::CandidatePrepare => (
                ActivationError::ConfigurationFailed,
                SubscriptionSwitchErrorCode::ConfigurationFailed,
                ManagedRuntimeFailure::Configuration,
            ),
            SidecarError::ActiveStop => (
                ActivationError::StopFailed,
                SubscriptionSwitchErrorCode::StopFailed,
                ManagedRuntimeFailure::Stop,
            ),
            _ => (
                ActivationError::StartFailed,
                SubscriptionSwitchErrorCode::StartFailed,
                ManagedRuntimeFailure::Start,
            ),
        }
    };
    observations.record_managed_failure(Some(lifecycle), log);
    switch_phase(
        observations,
        operation_id,
        SubscriptionSwitchStatus::Failed,
        Some(code),
    );
    activation_error(operation_id, wire)
}

fn start_stopped_runtime<P: SidecarPort>(
    runtime: &mut SidecarRuntime<P>,
    intent: &ObservationCompilationInput,
    observations: &InMemoryRuntimeObservations,
) -> ManagedRuntimeStartResult {
    let plan = match SingBoxCompiler.compile(
        &intent.runtime_intent,
        &intent.projected_default_target,
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
            observations.record_subscription_ready(
                intent.selected_subscription_id.0.clone(),
                intent.selected_generation,
                None,
                runtime.mixed_port().is_some(),
            );
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
        Ok(Some(observation)) => {
            let memory = runtime
                .with_active_port(|port, child| port.read_owned_core_memory_bytes(child))
                .ok()
                .flatten()
                .filter(|bytes| *bytes <= 9_007_199_254_740_991);
            observations.record_managed_sample(observation, memory);
        }
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
    use crate::domain::RouteTarget;
    use std::{
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };

    use super::*;
    use crate::application::observability::RuntimeObservationPort;

    static NEXT_TEST_ID: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn force_bypasses_only_the_authoritative_already_current_shortcut() {
        let observations = InMemoryRuntimeObservations::new_mock();
        observations.record_subscription_ready("sub".into(), 7, None, false);
        let snapshot = observations.snapshot();
        assert!(should_return_already_current(false, &snapshot, "sub", 7));
        assert!(!should_return_already_current(true, &snapshot, "sub", 7));
        assert!(!should_return_already_current(false, &snapshot, "sub", 8));
    }

    #[test]
    fn routing_apply_rejects_a_changed_structural_tuple_before_worker_dispatch() {
        let root = isolated_test_root();
        std::fs::create_dir_all(&root).expect("create isolated state root");
        let state_file = root.join("state.json");
        JsonStateStore::new(state_file.clone())
            .expect("store")
            .save(&valid_selected_state())
            .expect("state");
        let (requests, receiver) = mpsc::sync_channel(REQUEST_QUEUE_CAPACITY);
        let mut controller = test_controller(requests, InMemoryRuntimeObservations::new_mock());
        controller.state_file = state_file.clone();
        controller.subscriptions = test_manager(state_file.clone());

        let first_operation_id = match controller.apply_proxy_routing("sub".into(), 2) {
            ActivationResult::Error {
                operation_id: Some(operation_id),
                error: ActivationError::StateChanged,
            } => operation_id,
            result => panic!("unexpected first Apply result: {result:?}"),
        };
        let first_terminal = controller.routing_apply_terminal().expect("first terminal");
        assert_eq!(first_terminal.operation_id, first_operation_id);
        assert_eq!(
            first_terminal.active_subscription_id.as_deref(),
            Some("sub")
        );
        assert_eq!(first_terminal.desired_generation, 1);
        assert_eq!(
            first_terminal.failure,
            RoutingApplyTerminalFailure::StateChanged
        );

        let mut next_state = valid_selected_state();
        next_state.active_configuration_generation = 3;
        JsonStateStore::new(state_file.clone())
            .expect("store")
            .save(&next_state)
            .expect("newer state");
        let second_operation_id = match controller.apply_proxy_routing("sub".into(), 2) {
            ActivationResult::Error {
                operation_id: Some(operation_id),
                error: ActivationError::StateChanged,
            } => operation_id,
            result => panic!("unexpected second Apply result: {result:?}"),
        };
        let second_terminal = controller
            .routing_apply_terminal()
            .expect("latest terminal");
        assert_ne!(second_terminal.operation_id, first_operation_id);
        assert_eq!(second_terminal.operation_id, second_operation_id);
        assert_eq!(second_terminal.desired_generation, 3);
        assert_eq!(
            controller
                .runtime_observation_snapshot()
                .subscription_switch
                .expect("authoritative terminal observation")
                .operation_id,
            second_operation_id
        );
        assert!(
            matches!(receiver.try_recv(), Err(mpsc::TryRecvError::Empty)),
            "tuple mismatch cannot enqueue runtime work"
        );

        drop(controller);
        std::fs::remove_dir_all(root).expect("remove isolated state root");
    }

    #[test]
    fn routing_apply_projects_pre_worker_configuration_failure_into_same_operation_terminal() {
        let root = isolated_test_root();
        std::fs::create_dir_all(&root).expect("create isolated state root");
        let state_file = root.join("state.json");
        let mut state = valid_selected_state();
        state.pools[0].id = PoolId("runtime-active-sub".into());
        state.default_target = RouteTarget::Pool(PoolId("runtime-active-sub".into()));
        JsonStateStore::new(state_file.clone())
            .expect("store")
            .save(&state)
            .expect("state");
        let (requests, receiver) = mpsc::sync_channel(REQUEST_QUEUE_CAPACITY);
        let mut controller = test_controller(requests, InMemoryRuntimeObservations::new_mock());
        controller.state_file = state_file.clone();
        controller.subscriptions = test_manager(state_file);

        let operation_id = match controller.apply_proxy_routing("sub".into(), 1) {
            ActivationResult::Error {
                operation_id: Some(operation_id),
                error: ActivationError::SelectionConflict,
            } => operation_id,
            result => panic!("unexpected Apply result: {result:?}"),
        };
        assert_eq!(
            controller.routing_apply_terminal(),
            Some(RoutingApplyTerminal {
                operation_id: operation_id.clone(),
                active_subscription_id: Some("sub".into()),
                desired_generation: 1,
                failure: RoutingApplyTerminalFailure::ConfigurationFailed,
            })
        );
        let observed = controller.runtime_observation_snapshot();
        assert_eq!(
            observed.subscription_switch,
            Some(SubscriptionSwitch {
                operation_id,
                status: SubscriptionSwitchStatus::Failed,
                error_code: Some(SubscriptionSwitchErrorCode::ConfigurationFailed),
            })
        );
        assert!(matches!(
            receiver.try_recv(),
            Err(mpsc::TryRecvError::Empty)
        ));

        drop(controller);
        std::fs::remove_dir_all(root).expect("remove isolated state root");
    }

    #[test]
    fn routing_apply_dispatch_failure_replaces_queued_phase_with_state_changed_terminal() {
        let root = isolated_test_root();
        std::fs::create_dir_all(&root).expect("create isolated state root");
        let state_file = root.join("state.json");
        JsonStateStore::new(state_file.clone())
            .expect("store")
            .save(&valid_selected_state())
            .expect("state");
        let (requests, receiver) = mpsc::sync_channel(REQUEST_QUEUE_CAPACITY);
        let (stop_response, _) = mpsc::sync_channel(RESPONSE_QUEUE_CAPACITY);
        requests
            .try_send(WorkerRequest::Stop {
                response: stop_response,
                _guards: test_guards(),
            })
            .expect("occupy worker queue");
        let mut controller = test_controller(requests, InMemoryRuntimeObservations::new_mock());
        controller.state_file = state_file.clone();
        controller.subscriptions = test_manager(state_file);

        let operation_id = match controller.apply_proxy_routing("sub".into(), 1) {
            ActivationResult::Error {
                operation_id: Some(operation_id),
                error: ActivationError::Busy,
            } => operation_id,
            result => panic!("unexpected Apply result: {result:?}"),
        };
        assert_eq!(
            controller.routing_apply_terminal(),
            Some(RoutingApplyTerminal {
                operation_id: operation_id.clone(),
                active_subscription_id: Some("sub".into()),
                desired_generation: 1,
                failure: RoutingApplyTerminalFailure::StateChanged,
            })
        );
        assert_eq!(
            controller
                .runtime_observation_snapshot()
                .subscription_switch
                .expect("dispatch terminal"),
            SubscriptionSwitch {
                operation_id,
                status: SubscriptionSwitchStatus::Cancelled,
                error_code: Some(SubscriptionSwitchErrorCode::Cancelled),
            }
        );
        assert!(matches!(
            receiver.try_recv(),
            Ok(WorkerRequest::Stop { .. })
        ));

        drop(controller);
        std::fs::remove_dir_all(root).expect("remove isolated state root");
    }

    #[test]
    fn applied_routing_index_contains_only_generation_and_stable_runtime_tag_mapping() {
        let projection = valid_compilation_input();
        let index = AppliedRoutingIndex::from_projection(&projection);
        let pool = index
            .pool_members
            .get(&PoolId("default".into()))
            .expect("applied custom pool");
        let node_id = projection.runtime_intent.pools[0].members[0].clone();
        let tags = pool.get(&node_id).expect("applied member");

        assert_eq!(index.configuration_generation, 1);
        assert_eq!(tags.pool, "pool-default");
        assert_eq!(tags.node, format!("node-{}", node_id.0));
    }

    #[test]
    fn manual_selection_does_no_io_when_stopped_or_missing_from_applied_index() {
        let observations = InMemoryRuntimeObservations::new_mock();
        let selected = Mutex::new(BTreeMap::new());
        assert_eq!(
            reconcile_manual_selection::<FailingPort>(
                None,
                None,
                &observations,
                &selected,
                &PoolId("pool".into()),
                &NodeId("node".into()),
            ),
            ManualSelectionRuntimeResult::SavedOnly(ManualSelectionSavedOnlyReason::RuntimeStopped)
        );

        let (context, mut runtime, _) = activation_fixture("");
        context
            .observations
            .record_subscription_ready("sub".into(), 1, None, false);
        assert_eq!(
            reconcile_manual_selection(
                Some(&mut runtime),
                None,
                &context.observations,
                &selected,
                &PoolId("default".into()),
                &NodeId("node".into()),
            ),
            ManualSelectionRuntimeResult::SavedOnly(
                ManualSelectionSavedOnlyReason::NotInAppliedArtifact
            )
        );
        assert_eq!(
            runtime.into_port().events,
            vec!["check", "prepare", "run", "ready"],
            "no selector transport dispatch"
        );
    }

    #[test]
    fn manual_selection_classifies_only_applied_index_read_back_tags() {
        let mut state = valid_selected_state();
        let mut sibling = state.nodes[0].clone();
        sibling.id = NodeId("runtime-old".into());
        state.nodes.push(sibling);
        let projection = project_selected_runtime(&state).expect("projection");
        let index = AppliedRoutingIndex::from_projection(&projection);
        let pool_id = PoolId("default".into());
        let members = &projection
            .runtime_intent
            .pools
            .iter()
            .find(|pool| pool.id == pool_id)
            .expect("custom pool")
            .members;
        let desired = members[0].clone();
        let old = members[1].clone();

        assert_eq!(
            classify_selector_read_back(
                &index,
                &Mutex::new(BTreeMap::new()),
                &pool_id,
                &desired,
                &format!("node-{}", desired.0),
            ),
            ManualSelectionRuntimeResult::Applied
        );
        assert_eq!(
            classify_selector_read_back(
                &index,
                &Mutex::new(BTreeMap::new()),
                &pool_id,
                &desired,
                &format!("node-{}", old.0),
            ),
            ManualSelectionRuntimeResult::NotApplied {
                runtime_node_id: old
            }
        );
        assert_eq!(
            classify_selector_read_back(
                &index,
                &Mutex::new(BTreeMap::new()),
                &pool_id,
                &desired,
                "node-not-in-applied-artifact",
            ),
            ManualSelectionRuntimeResult::ApplyUnknown(
                ManualSelectionUnknownReason::UnknownRuntimeNode
            )
        );
    }

    #[test]
    fn running_configuration_reads_exact_active_slot_with_applied_pair() {
        let (context, runtime, _request) = activation_fixture("");
        context
            .observations
            .record_subscription_ready("sub".into(), 1, None, false);
        let expected = runtime
            .with_active_config(|config| String::from_utf8(config.as_bytes().to_vec()).unwrap())
            .expect("active config");
        assert!(matches!(
            read_running_configuration(Some(&runtime), &context.observations.snapshot()),
            RunningConfigurationResult::Ok {
                content,
                applied_subscription_id,
                applied_configuration_generation: 1,
            } if content == expected && applied_subscription_id == "sub"
        ));
        context.observations.record_managed_recovery();
        assert!(matches!(
            read_running_configuration(Some(&runtime), &context.observations.snapshot()),
            RunningConfigurationResult::Error(RunningConfigurationError::RecoveryRequired)
        ));
    }

    #[test]
    fn running_configuration_rejects_active_bytes_over_limit() {
        let (context, mut runtime, _request) = activation_fixture("");
        let mut bytes = runtime
            .with_active_config(|config| config.as_bytes().to_vec())
            .expect("active bytes");
        bytes.resize(RUNNING_CONFIGURATION_MAX_BYTES + 1, b' ');
        runtime
            .start_or_replace(GeneratedConfig::from_bytes(bytes))
            .expect("mock ready");
        context
            .observations
            .record_subscription_ready("sub".into(), 1, None, false);
        assert!(matches!(
            read_running_configuration(Some(&runtime), &context.observations.snapshot()),
            RunningConfigurationResult::Error(RunningConfigurationError::ContentTooLarge)
        ));
    }

    #[derive(Default)]
    struct FailingPort {
        fault: &'static str,
        events: Vec<&'static str>,
        pending_cleanup: bool,
    }

    impl SidecarPort for FailingPort {
        fn check(
            &mut self,
            config: &crate::singbox::GeneratedConfig,
        ) -> Result<(), crate::singbox::runtime::SidecarPortError> {
            config.validate_final().expect("valid final candidate");
            self.events.push("check");
            let check_count = self
                .events
                .iter()
                .filter(|event| **event == "check")
                .count();
            if self.fault == "check"
                || (self.fault == "replace-check" && check_count > 1)
                || (self.fault == "replace-check-cleanup" && check_count > 1)
            {
                self.pending_cleanup = self.fault == "replace-check-cleanup";
                Err(crate::singbox::runtime::SidecarPortError)
            } else {
                Ok(())
            }
        }
        fn prepare(
            &mut self,
            _: &crate::singbox::GeneratedConfig,
        ) -> Result<(), crate::singbox::runtime::SidecarPortError> {
            self.events.push("prepare");
            if self.fault == "replace-prepare-cancel"
                && self
                    .events
                    .iter()
                    .filter(|event| **event == "prepare")
                    .count()
                    > 1
            {
                Err(crate::singbox::runtime::SidecarPortError)
            } else {
                Ok(())
            }
        }
        fn run(
            &mut self,
        ) -> Result<
            crate::singbox::runtime::ManagedSidecar,
            crate::singbox::runtime::SidecarPortError,
        > {
            self.events.push("run");
            let run_count = self.events.iter().filter(|event| **event == "run").count();
            if self.fault == "run"
                || (self.fault == "replace-run" && run_count > 1)
                || (self.fault == "replace-run-once" && run_count == 2)
            {
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
            if matches!(self.fault, "stop" | "replace-stop") {
                Err(crate::singbox::runtime::SidecarPortError)
            } else {
                Ok(())
            }
        }
        fn cancel_pending(&mut self) -> Result<(), crate::singbox::runtime::SidecarPortError> {
            self.events.push("cancel");
            if matches!(self.fault, "cancel" | "replace-prepare-cancel") {
                self.pending_cleanup = true;
                Err(crate::singbox::runtime::SidecarPortError)
            } else {
                self.pending_cleanup = false;
                Ok(())
            }
        }
        fn has_pending_cleanup(&self) -> bool {
            self.pending_cleanup
        }
    }

    fn test_manager(state_file: PathBuf) -> Arc<SubscriptionManager> {
        Arc::new(SubscriptionManager::new(state_file, StateAccessGate::default()).expect("manager"))
    }

    fn test_guards() -> RequestGuards {
        RequestGuards {
            _runtime: RuntimeGuard(Arc::new(AtomicBool::new(true))),
            _subscription: None,
        }
    }

    fn test_context(
        resource_root: PathBuf,
        app_local_data_root: PathBuf,
        observations: InMemoryRuntimeObservations,
    ) -> WorkerContext {
        WorkerContext {
            subscriptions: test_manager(app_local_data_root.join("state.json")),
            resource_root,
            app_local_data_root,
            observations,
            state_gate: StateAccessGate::default(),
            shutdown_requested: Arc::new(AtomicBool::new(false)),
            shutdown_complete: Arc::new(AtomicBool::new(false)),
            selector_runtime_nodes: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }

    fn test_controller(
        requests: SyncSender<WorkerRequest>,
        observations: InMemoryRuntimeObservations,
    ) -> ManagedObservationRuntimeController {
        ManagedObservationRuntimeController {
            scheduler: None,
            shutdown_waiting: Arc::new(AtomicBool::new(false)),
            state_file: PathBuf::new(),
            requests,
            observations,
            state_gate: StateAccessGate::default(),
            subscriptions: test_manager(isolated_test_root().join("state.json")),
            busy: Arc::new(AtomicBool::new(false)),
            shutdown_requested: Arc::new(AtomicBool::new(false)),
            shutdown_complete: Arc::new(AtomicBool::new(false)),
            selector_runtime_nodes: Arc::new(Mutex::new(BTreeMap::new())),
            routing_apply_terminal: Arc::new(Mutex::new(None)),
        }
    }

    fn valid_selected_state() -> AppState {
        use crate::domain::*;
        let mut state = AppState::empty();
        state.subscriptions.push(Subscription {
            skipped_unsupported_nodes: 0,
            id: SubscriptionId("sub".into()),
            name: "fixture".into(),
            source: SubscriptionSource::Manual,
            document: None,
            last_success_at_ms: None,
            http_metadata: None,
            description: String::new(),
            last_attempt_at_ms: None,
            remote_request: None,
            update_policy: crate::domain::SubscriptionUpdatePolicy::manual(),
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
        state.active_subscription_id = Some(SubscriptionId("sub".into()));
        state.active_configuration_generation = 1;
        state
    }

    fn valid_compilation_input() -> ObservationCompilationInput {
        project_selected_runtime(&valid_selected_state()).expect("selected projection")
    }

    fn activation_fixture(
        fault: &'static str,
    ) -> (
        WorkerContext,
        SidecarRuntime<FailingPort>,
        ActivationRequest,
    ) {
        use crate::domain::*;
        let root = isolated_test_root();
        let context = test_context(
            root.join("resources"),
            root,
            InMemoryRuntimeObservations::new_mock(),
        );
        let mut expected = valid_selected_state();
        expected.pools.clear();
        expected.default_target = RouteTarget::Unconfigured;
        let mut second = expected.subscriptions[0].clone();
        second.id = SubscriptionId("second".into());
        second.name = "second".into();
        expected.subscriptions.push(second);
        expected.providers.push(Provider {
            id: ProviderId("second-provider".into()),
            subscription_id: SubscriptionId("second".into()),
            name: "second".into(),
        });
        let mut node = expected.nodes[0].clone();
        node.id = NodeId("second-node".into());
        node.provider_id = ProviderId("second-provider".into());
        expected.nodes.push(node);
        JsonStateStore::new(context.app_local_data_root.join("state.json"))
            .expect("store")
            .save(&expected)
            .expect("old state");
        let mut runtime = SidecarRuntime::new_observation_only(FailingPort {
            fault,
            ..Default::default()
        });
        let old_projection = project_selected_runtime(&expected).expect("old projection");
        assert_eq!(
            start_stopped_runtime(&mut runtime, &old_projection, &context.observations),
            ManagedRuntimeStartResult::Started
        );
        let mut candidate = expected.clone();
        candidate.active_subscription_id = Some(SubscriptionId("second".into()));
        candidate.active_configuration_generation += 1;
        let projection = project_selected_runtime(&candidate).expect("new projection");
        let config = compile_projection(&projection).expect("new config");
        let subscription = selected_summary(&candidate, "second").expect("safe summary");
        let (response, _) = mpsc::sync_channel(1);
        let request = ActivationRequest {
            operation_id: "fixture-operation".into(),
            expected,
            candidate,
            projection,
            subscription,
            config,
            reactivation: false,
            cancel: Arc::new(AtomicBool::new(false)),
            _guards: test_guards(),
            response,
        };
        switch_phase(
            &context.observations,
            &request.operation_id,
            SubscriptionSwitchStatus::Queued,
            None,
        );
        (context, runtime, request)
    }

    #[test]
    fn activation_checks_and_saves_before_stop_then_publishes_applied_pair() {
        let (context, mut runtime, request) = activation_fixture("");
        let phases = Arc::new(std::sync::Mutex::new(Vec::new()));
        let captured = Arc::clone(&phases);
        let state_file = context.app_local_data_root.join("state.json");
        context.observations.install_delta_sink(move |delta| {
            if let Some(change) = delta.subscription_switch {
                if change.status == SubscriptionSwitchStatus::Applying {
                    let state = JsonStateStore::new(state_file.clone())
                        .expect("store")
                        .load()
                        .expect("persisted");
                    assert_eq!(
                        state.active_subscription_id,
                        Some(SubscriptionId("second".into()))
                    );
                    assert_eq!(state.active_configuration_generation, 2);
                }
                captured.lock().expect("phases").push(change.status);
            }
        });
        assert!(matches!(
            apply_activation(&mut runtime, request, &context),
            ActivationResult::Activated { generation: 2, .. }
        ));
        let snapshot = context.observations.snapshot();
        assert_eq!(snapshot.sidecar_lifecycle, ObservedSidecarLifecycle::Ready);
        assert_eq!(snapshot.applied_subscription_id.as_deref(), Some("second"));
        assert_eq!(snapshot.applied_configuration_generation, Some(2));
        assert!(!snapshot.managed_proxy_available);
        assert_eq!(
            *phases.lock().expect("phases"),
            vec![
                SubscriptionSwitchStatus::Checking,
                SubscriptionSwitchStatus::Prepared,
                SubscriptionSwitchStatus::Persisted,
                SubscriptionSwitchStatus::Applying,
                SubscriptionSwitchStatus::Ready
            ]
        );
        assert_eq!(
            runtime.into_port().events,
            vec![
                "check", "prepare", "run", "ready", "check", "prepare", "stop", "run", "ready"
            ]
        );
        std::fs::remove_dir_all(context.app_local_data_root).expect("isolated cleanup");
    }

    #[test]
    fn forced_request_reports_reactivated_after_real_commit() {
        let (context, mut runtime, mut request) = activation_fixture("");
        request.reactivation = true;
        assert!(matches!(
            apply_activation(&mut runtime, request, &context),
            ActivationResult::Reactivated { generation: 2, .. }
        ));
        assert_eq!(
            runtime.into_port().events,
            vec![
                "check", "prepare", "run", "ready", "check", "prepare", "stop", "run", "ready"
            ]
        );
    }

    #[test]
    fn inactive_document_save_commits_exact_text_without_runtime_apply() {
        let (context, _runtime, request) = activation_fixture("");
        drop(request);
        let store =
            JsonStateStore::new(context.app_local_data_root.join("state.json")).expect("store");
        let mut state = store.load().expect("state");
        state.active_subscription_id = None;
        state.subscriptions[0].document = Some(crate::domain::SubscriptionDocument {
            format: crate::domain::SubscriptionDocumentFormat::Json,
            content: r#"{"outbounds":[{"type":"socks","tag":"fixture","server":"127.0.0.1","server_port":1080}]}"#.into(),
            local_override: false,
        });
        store.save(&state).expect("seed document");
        let before = context
            .subscriptions
            .read_document("sub")
            .expect("read revision");
        let exact = "{\n  \"outbounds\": [{\"type\":\"socks\",\"tag\":\"edited\",\"server\":\"127.0.0.1\",\"server_port\":1081}]\n}\n";
        let input = DocumentSaveInput {
            id: "sub".into(),
            format: crate::domain::SubscriptionDocumentFormat::Json,
            content: exact.into(),
            expected_revision: before.revision,
        };
        let mut runtime: Option<SidecarRuntime<FailingPort>> = None;
        assert!(matches!(
            apply_document_save(
                &mut runtime,
                input.clone(),
                "expired-save".into(),
                Instant::now(),
                &context,
                || Err(()),
                || {},
            ),
            DocumentSaveResult::Error {
                error: DocumentSaveError::OperationTimedOut,
                ..
            }
        ));
        assert_ne!(
            context.subscriptions.read_document("sub").unwrap().content,
            exact
        );
        let result = apply_document_save(
            &mut runtime,
            input,
            "save-operation".into(),
            Instant::now() + Duration::from_secs(1),
            &context,
            || Err(()),
            || {},
        );
        assert!(matches!(
            result,
            DocumentSaveResult::Ok {
                apply: DocumentApplyResult::NotRequired,
                ..
            }
        ));
        assert_eq!(
            context
                .subscriptions
                .read_document("sub")
                .expect("saved document")
                .content,
            exact
        );
    }

    #[test]
    fn selected_document_save_prepares_persists_then_replaces_runtime() {
        let (context, runtime, request) = activation_fixture("");
        drop(request);
        let store =
            JsonStateStore::new(context.app_local_data_root.join("state.json")).expect("store");
        let mut state = store.load().expect("state");
        state.subscriptions[0].document = Some(crate::domain::SubscriptionDocument {
            format: crate::domain::SubscriptionDocumentFormat::Json,
            content: r#"{"outbounds":[{"type":"socks","tag":"fixture","server":"127.0.0.1","server_port":1080}]}"#.into(),
            local_override: false,
        });
        store.save(&state).expect("seed document");
        let before = context
            .subscriptions
            .read_document("sub")
            .expect("revision");
        let exact = "{\n  \"outbounds\": [{\"type\":\"socks\",\"tag\":\"edited\",\"server\":\"127.0.0.1\",\"server_port\":1081}]\n}\n";
        let result = apply_document_save(
            &mut Some(runtime),
            DocumentSaveInput {
                id: "sub".into(),
                format: crate::domain::SubscriptionDocumentFormat::Json,
                content: exact.into(),
                expected_revision: before.revision,
            },
            "save-active".into(),
            Instant::now() + Duration::from_secs(1),
            &context,
            || Err(()),
            || {},
        );
        assert!(matches!(
            result,
            DocumentSaveResult::Ok {
                apply: DocumentApplyResult::Ready { generation: 2, .. },
                ..
            }
        ));
        assert_eq!(
            context.subscriptions.read_document("sub").unwrap().content,
            exact
        );
    }

    fn seed_editable_document(context: &WorkerContext, selected: bool) -> DocumentSaveInput {
        let store =
            JsonStateStore::new(context.app_local_data_root.join("state.json")).expect("store");
        let mut state = store.load().expect("state");
        if !selected {
            state.active_subscription_id = None;
        }
        state.subscriptions[0].document = Some(crate::domain::SubscriptionDocument {
            format: crate::domain::SubscriptionDocumentFormat::Json,
            content: r#"{"outbounds":[{"type":"socks","tag":"fixture","server":"127.0.0.1","server_port":1080}]}"#.into(),
            local_override: false,
        });
        store.save(&state).expect("seed document");
        let revision = context.subscriptions.read_document("sub").unwrap().revision;
        DocumentSaveInput {
            id: "sub".into(),
            format: crate::domain::SubscriptionDocumentFormat::Json,
            content: r#"{"outbounds":[{"type":"socks","tag":"edited","server":"127.0.0.1","server_port":1081}]}"#.into(),
            expected_revision: revision,
        }
    }

    #[test]
    fn selected_stopped_save_starts_saved_candidate() {
        let (context, _old_runtime, request) = activation_fixture("");
        drop(request);
        let input = seed_editable_document(&context, true);
        let mut runtime = Some(SidecarRuntime::new_observation_only(FailingPort::default()));
        let result = apply_document_save(
            &mut runtime,
            input,
            "stopped-save".into(),
            Instant::now() + Duration::from_secs(1),
            &context,
            || Err(()),
            || {},
        );
        assert!(matches!(
            result,
            DocumentSaveResult::Ok {
                apply: DocumentApplyResult::Ready { generation: 2, .. },
                ..
            }
        ));
        assert_eq!(
            runtime.unwrap().into_port().events,
            vec!["check", "prepare", "run", "ready"]
        );
    }

    #[test]
    fn selected_save_preparation_failure_preserves_state_and_old_child() {
        let (context, runtime, request) = activation_fixture("replace-check");
        drop(request);
        let input = seed_editable_document(&context, true);
        let before = std::fs::read(context.app_local_data_root.join("state.json")).unwrap();
        let result = apply_document_save(
            &mut Some(runtime),
            input,
            "failed-save".into(),
            Instant::now() + Duration::from_secs(1),
            &context,
            || Err(()),
            || {},
        );
        assert!(matches!(
            result,
            DocumentSaveResult::Error {
                error: DocumentSaveError::ConfigurationFailed,
                ..
            }
        ));
        assert_eq!(
            std::fs::read(context.app_local_data_root.join("state.json")).unwrap(),
            before
        );
        assert!(matches!(
            context.observations.snapshot().subscription_switch,
            Some(SubscriptionSwitch {
                status: SubscriptionSwitchStatus::Failed,
                error_code: Some(SubscriptionSwitchErrorCode::ConfigurationFailed),
                ..
            })
        ));
    }

    #[test]
    fn selected_save_unconfirmed_preparation_cleanup_publishes_recovery() {
        for fault in ["replace-check-cleanup", "replace-prepare-cancel"] {
            let (context, runtime, request) = activation_fixture(fault);
            drop(request);
            let input = seed_editable_document(&context, true);
            let state_path = context.app_local_data_root.join("state.json");
            let before = std::fs::read(&state_path).expect("state before failed save");
            let result = apply_document_save(
                &mut Some(runtime),
                input,
                format!("{fault}-operation"),
                Instant::now() + Duration::from_secs(1),
                &context,
                || Err(()),
                || {},
            );
            assert!(matches!(
                result,
                DocumentSaveResult::Error {
                    error: DocumentSaveError::RecoveryRequired,
                    ..
                }
            ));
            assert_eq!(
                std::fs::read(&state_path).expect("state after failed save"),
                before,
                "{fault}: persistence must not precede successful preparation"
            );
            let snapshot = context.observations.snapshot();
            assert_eq!(
                snapshot.sidecar_lifecycle,
                ObservedSidecarLifecycle::RecoveryRequired,
                "{fault}"
            );
            assert!(snapshot.applied_subscription_id.is_none(), "{fault}");
            assert!(
                snapshot.applied_configuration_generation.is_none(),
                "{fault}"
            );
            assert!(matches!(
                snapshot.subscription_switch,
                Some(SubscriptionSwitch {
                    status: SubscriptionSwitchStatus::Failed,
                    error_code: Some(SubscriptionSwitchErrorCode::RecoveryRequired),
                    ..
                })
            ));
        }
    }

    #[test]
    fn selected_save_postpersist_start_failure_keeps_new_revision() {
        let (context, runtime, request) = activation_fixture("replace-run");
        drop(request);
        let input = seed_editable_document(&context, true);
        let expected = input.content.clone();
        let result = apply_document_save(
            &mut Some(runtime),
            input,
            "postpersist-failure".into(),
            Instant::now() + Duration::from_secs(1),
            &context,
            || Err(()),
            || {},
        );
        assert!(matches!(
            result,
            DocumentSaveResult::Ok {
                apply: DocumentApplyResult::Failed {
                    error: DocumentApplyError::StartFailed,
                    generation: 2,
                    ..
                },
                ..
            }
        ));
        assert_eq!(
            context.subscriptions.read_document("sub").unwrap().content,
            expected
        );
    }

    #[test]
    fn selected_exact_save_retries_failed_apply_without_advancing_generation() {
        let (context, runtime, request) = activation_fixture("replace-run-once");
        drop(request);
        let input = seed_editable_document(&context, true);
        let expected = input.content.clone();
        let mut runtime = Some(runtime);
        let first = apply_document_save(
            &mut runtime,
            input,
            "first-apply".into(),
            Instant::now() + Duration::from_secs(1),
            &context,
            || Err(()),
            || {},
        );
        assert!(matches!(
            first,
            DocumentSaveResult::Ok {
                apply: DocumentApplyResult::Failed {
                    error: DocumentApplyError::StartFailed,
                    generation: 2,
                    ..
                },
                ..
            }
        ));
        let saved = context
            .subscriptions
            .read_document("sub")
            .expect("persisted first save");
        assert_eq!(saved.content, expected);

        let retried = apply_document_save(
            &mut runtime,
            DocumentSaveInput {
                id: "sub".into(),
                format: saved.format,
                content: saved.content,
                expected_revision: saved.revision,
            },
            "retry-apply".into(),
            Instant::now() + Duration::from_secs(1),
            &context,
            || Err(()),
            || {},
        );
        assert!(matches!(
            retried,
            DocumentSaveResult::Ok {
                apply: DocumentApplyResult::Ready { generation: 2, .. },
                ..
            }
        ));
        let snapshot = context.observations.snapshot();
        assert_eq!(snapshot.sidecar_lifecycle, ObservedSidecarLifecycle::Ready);
        assert_eq!(snapshot.applied_subscription_id.as_deref(), Some("sub"));
        assert_eq!(snapshot.applied_configuration_generation, Some(2));
        assert!(matches!(
            snapshot.subscription_switch,
            Some(SubscriptionSwitch {
                operation_id,
                status: SubscriptionSwitchStatus::Ready,
                error_code: None,
            }) if operation_id == "retry-apply"
        ));
        assert_eq!(
            JsonStateStore::new(context.app_local_data_root.join("state.json"))
                .expect("store")
                .load()
                .expect("state")
                .active_configuration_generation,
            2,
            "retrying the same persisted document must not advance generation"
        );
    }

    #[test]
    fn selected_save_postpersist_deadline_keeps_new_revision_and_old_child() {
        let (context, runtime, request) = activation_fixture("");
        drop(request);
        let input = seed_editable_document(&context, true);
        let expected = input.content.clone();
        let mut runtime = Some(runtime);
        let result = apply_document_save(
            &mut runtime,
            input,
            "postpersist-timeout".into(),
            Instant::now() + Duration::from_millis(100),
            &context,
            || Err(()),
            || thread::sleep(Duration::from_millis(150)),
        );
        assert!(matches!(
            result,
            DocumentSaveResult::Ok {
                apply: DocumentApplyResult::Failed {
                    error: DocumentApplyError::OperationTimedOut,
                    generation: 2,
                    ..
                },
                ..
            }
        ));
        assert_eq!(
            context.subscriptions.read_document("sub").unwrap().content,
            expected
        );
        assert_eq!(
            runtime.unwrap().snapshot().lifecycle,
            SidecarLifecycle::Ready
        );
        let snapshot = context.observations.snapshot();
        assert_eq!(snapshot.sidecar_lifecycle, ObservedSidecarLifecycle::Ready);
        assert!(matches!(
            snapshot.subscription_switch,
            Some(SubscriptionSwitch {
                status: SubscriptionSwitchStatus::Failed,
                error_code: Some(SubscriptionSwitchErrorCode::OperationTimedOut),
                ..
            })
        ));
    }

    #[test]
    fn dropped_save_receiver_does_not_cancel_worker_commit() {
        let (context, _runtime, request) = activation_fixture("");
        drop(request);
        let input = seed_editable_document(&context, false);
        let expected = input.content.clone();
        let manager = Arc::clone(&context.subscriptions);
        let busy = Arc::new(AtomicBool::new(true));
        let (requests, receiver) = mpsc::sync_channel(1);
        let worker = thread::spawn(move || worker_loop(receiver, context));
        let (response, dropped) = mpsc::sync_channel(1);
        drop(dropped);
        requests
            .send(WorkerRequest::SaveDocument {
                input,
                operation_id: "lost-receiver".into(),
                deadline: Instant::now() + Duration::from_secs(1),
                response,
                _runtime_guard: RuntimeGuard(Arc::clone(&busy)),
            })
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(1);
        while Instant::now() < deadline && busy.load(Ordering::Acquire) {
            thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(manager.read_document("sub").unwrap().content, expected);
        assert!(!busy.load(Ordering::Acquire));
        let (response, result) = mpsc::sync_channel(1);
        requests.send(WorkerRequest::Shutdown { response }).unwrap();
        assert_eq!(result.recv().unwrap(), ShutdownResult::ShutdownComplete);
        worker.join().unwrap();
    }

    #[test]
    fn activation_precommit_failures_and_cancellation_preserve_old_state_and_child() {
        for case in [
            "replace-check",
            "save",
            "mismatch",
            "reload",
            "cancelled",
            "shutdown",
            "cancel",
        ] {
            let fault = if matches!(case, "replace-check" | "cancel") {
                case
            } else {
                ""
            };
            let (context, mut runtime, request) = activation_fixture(fault);
            let expected = request.expected.clone();
            let path = context.app_local_data_root.join("state.json");
            match case {
                "save" => std::fs::create_dir(context.app_local_data_root.join("state.json.tmp"))
                    .expect("block only backup temporary write"),
                "mismatch" => {
                    let mut changed = expected.clone();
                    changed.subscriptions[0].description = "external edit".into();
                    JsonStateStore::new(path.clone())
                        .expect("store")
                        .save(&changed)
                        .expect("concurrent fixture edit");
                }
                "reload" => std::fs::remove_file(&path)
                    .expect("remove isolated current fixture with no backup"),
                "cancelled" | "cancel" => request.cancel.store(true, Ordering::Release),
                "shutdown" => context.shutdown_requested.store(true, Ordering::Release),
                _ => {}
            }
            let result = apply_activation(&mut runtime, request, &context);
            let expected_code = match case {
                "replace-check" => SubscriptionSwitchErrorCode::ConfigurationFailed,
                "save" => SubscriptionSwitchErrorCode::SaveFailed,
                "mismatch" => SubscriptionSwitchErrorCode::Busy,
                "reload" => SubscriptionSwitchErrorCode::StateUnavailable,
                "cancel" => SubscriptionSwitchErrorCode::RecoveryRequired,
                _ => SubscriptionSwitchErrorCode::Cancelled,
            };
            let observation = context.observations.snapshot();
            let change = observation.subscription_switch.expect("terminal operation");
            assert_eq!(change.operation_id, "fixture-operation");
            assert_eq!(change.error_code, Some(expected_code), "{case}: {result:?}");
            assert_eq!(
                change.status,
                if matches!(case, "cancelled" | "shutdown") {
                    SubscriptionSwitchStatus::Cancelled
                } else {
                    SubscriptionSwitchStatus::Failed
                }
            );
            if case == "cancel" {
                assert!(observation.applied_subscription_id.is_none());
                assert!(observation.applied_configuration_generation.is_none());
            } else {
                assert_eq!(observation.applied_subscription_id.as_deref(), Some("sub"));
                assert_eq!(observation.applied_configuration_generation, Some(1));
            }
            let port = runtime.into_port();
            assert!(
                !port.events.contains(&"stop"),
                "{case}: old child never stopped"
            );
            assert_eq!(
                port.events.iter().filter(|event| **event == "run").count(),
                1
            );
            if !matches!(case, "reload" | "mismatch") {
                assert_eq!(
                    JsonStateStore::new(path)
                        .expect("store")
                        .load()
                        .expect("old state"),
                    expected,
                    "{case}"
                );
            }
            std::fs::remove_dir_all(context.app_local_data_root).expect("isolated cleanup");
        }
    }

    #[test]
    fn activation_postcommit_failure_keeps_selection_and_reports_actual_runtime() {
        for (fault, code, lifecycle) in [
            (
                "replace-stop",
                ActivationError::StopFailed,
                ObservedSidecarLifecycle::RecoveryRequired,
            ),
            (
                "replace-run",
                ActivationError::StartFailed,
                ObservedSidecarLifecycle::Stopped,
            ),
        ] {
            let (context, mut runtime, request) = activation_fixture(fault);
            let candidate = request.candidate.clone();
            assert!(
                matches!(apply_activation(&mut runtime, request, &context), ActivationResult::Error { error, .. } if error == code)
            );
            assert_eq!(
                JsonStateStore::new(context.app_local_data_root.join("state.json"))
                    .expect("store")
                    .load()
                    .expect("persisted selection"),
                candidate
            );
            assert_eq!(context.observations.snapshot().sidecar_lifecycle, lifecycle);
            let port = runtime.into_port();
            assert_eq!(
                port.events.iter().filter(|event| **event == "run").count(),
                if fault == "replace-stop" { 1 } else { 2 }
            );
            assert_eq!(
                port.events.iter().filter(|event| **event == "stop").count(),
                1,
                "no automatic rollback"
            );
            std::fs::remove_dir_all(context.app_local_data_root).expect("isolated cleanup");
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
        input.projected_default_target = RouteTarget::Unconfigured;
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
                    _guards: test_guards(),
                    intent: valid_compilation_input(),
                    response: first_response,
                })
                .expect("first start queued");
            let worker = thread::spawn(move || {
                worker_loop(
                    receiver,
                    test_context(resources, worker_data, worker_observations),
                )
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
                            _guards: test_guards(),
                            response: second_stop,
                        })
                        .expect("stop queued behind start");
                } else {
                    let mut invalid_input = valid_compilation_input();
                    invalid_input.projected_default_target = RouteTarget::Unconfigured;
                    requests
                        .try_send(WorkerRequest::Start {
                            _guards: test_guards(),
                            intent: invalid_input,
                            response: second_start,
                        })
                        .expect("repeat start queued");
                }
                let controller = test_controller(requests, observations.clone());
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
                _guards: test_guards(),
                intent: valid_compilation_input(),
                response,
            })
            .expect("start queued");
        let worker_observations = observations.clone();
        let worker_data = app_data.clone();
        let (finished, finished_receiver) = mpsc::sync_channel(1);
        let worker = thread::spawn(move || {
            worker_loop(
                receiver,
                test_context(resources, worker_data, worker_observations),
            );
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
            assert_eq!(recovery.sidecar_lifecycle, ObservedSidecarLifecycle::Ready);
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
            invalid_input.projected_default_target = RouteTarget::Unconfigured;
            let (repeat_response, repeat_receiver) = mpsc::sync_channel(RESPONSE_QUEUE_CAPACITY);
            requests
                .try_send(WorkerRequest::Start {
                    _guards: test_guards(),
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
            let controller = test_controller(requests, observations.clone());
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
        let controller = test_controller(requests, observations.clone());
        let worker_observations = observations.clone();
        let worker_data = app_data.clone();
        let (finished, finished_receiver) = mpsc::sync_channel(1);
        let worker = thread::spawn(move || {
            let _installation = stream_test_probe::install(probe);
            worker_loop(
                receiver,
                test_context(resources, worker_data, worker_observations),
            );
            let _ = finished.try_send(());
        });
        let cleanup_data = app_data.clone();
        let assertions = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
            let start = || {
                let (response, receiver) = mpsc::sync_channel(RESPONSE_QUEUE_CAPACITY);
                controller
                    .requests
                    .try_send(WorkerRequest::Start {
                        _guards: test_guards(),
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
                .try_send(WorkerRequest::Stop {
                    response,
                    _guards: test_guards(),
                })
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
            worker_loop(
                receiver,
                test_context(resources, worker_data, worker_observations),
            )
        });
        let assertions = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
            let (response, start_receiver) = mpsc::sync_channel(RESPONSE_QUEUE_CAPACITY);
            requests
                .try_send(WorkerRequest::Start {
                    _guards: test_guards(),
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
                .try_send(WorkerRequest::Stop {
                    response,
                    _guards: test_guards(),
                })
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
            StateAccessGate::default(),
            test_manager(root.join("app-data/state.json")),
            None,
        );

        assert_eq!(
            controller.start(),
            ManagedRuntimeStartResult::StateUnavailable
        );
        assert!(controller.shutdown());
    }

    #[test]
    fn start_state_load_fails_fast_while_a_state_commit_holds_the_gate() {
        let gate = StateAccessGate::default();
        let _commit = gate.try_lock().expect("hold state commit gate");

        assert!(matches!(
            load_runtime_intent(Path::new("unused-state.json"), &gate),
            Err(LoadRuntimeIntentError::Busy)
        ));
    }

    #[test]
    fn stop_without_a_managed_child_is_closed_and_idempotent() {
        let root = isolated_test_root();
        let controller = ManagedObservationRuntimeController::new(
            root.join("resources"),
            root.join("app-data"),
            InMemoryRuntimeObservations::new_mock(),
            StateAccessGate::default(),
            test_manager(root.join("app-data/state.json")),
            None,
        );

        assert_eq!(controller.stop(), ManagedRuntimeStopResult::AlreadyStopped);
        assert!(controller.shutdown());
    }

    #[test]
    fn lost_response_reports_failure_without_overwriting_the_owner_lifecycle() {
        for owner_finishes_before_response_loss in [false, true] {
            let observations = InMemoryRuntimeObservations::new_mock();
            observations.record_managed_stopped();
            let initial_revision = observations.snapshot().revision;
            let (requests, receiver) = mpsc::sync_channel(REQUEST_QUEUE_CAPACITY);
            let worker_observations = observations.clone();
            let worker = thread::spawn(move || {
                let WorkerRequest::Stop { response, .. } = receiver.recv().expect("queued stop")
                else {
                    panic!("expected stop");
                };
                if owner_finishes_before_response_loss {
                    worker_observations.record_managed_stopped();
                }
                drop(response);
            });
            let controller = test_controller(requests, observations.clone());
            assert_eq!(controller.stop(), ManagedRuntimeStopResult::StopFailed);
            worker.join().expect("fake worker exits");
            let snapshot = observations.snapshot();
            assert!(snapshot.revision > initial_revision);
            assert_eq!(
                snapshot.sidecar_lifecycle,
                ObservedSidecarLifecycle::Stopped
            );
            // 丢失响应只返回错误；IPC 等待者不能伪造 owner 的恢复状态。
            observations.record_managed_stopped();
            assert_eq!(
                observations.snapshot().sidecar_lifecycle,
                ObservedSidecarLifecycle::Stopped
            );
        }
    }

    #[test]
    fn lost_shutdown_response_preserves_authoritative_completion_for_retry() {
        let (requests, receiver) = mpsc::sync_channel(REQUEST_QUEUE_CAPACITY);
        let observations = InMemoryRuntimeObservations::new_mock();
        let context = test_context(
            PathBuf::from("missing-resources"),
            isolated_test_root(),
            observations.clone(),
        );
        let completion = Arc::clone(&context.shutdown_complete);
        let mut controller = test_controller(requests, observations.clone());
        controller.shutdown_complete = Arc::clone(&completion);
        let worker = thread::spawn(move || worker_loop(receiver, context));
        let (response, lost_receiver) = mpsc::sync_channel(RESPONSE_QUEUE_CAPACITY);
        drop(lost_receiver);
        controller
            .requests
            .send(WorkerRequest::Shutdown { response })
            .expect("shutdown queued");
        worker
            .join()
            .expect("worker exits after confirmed cleanup despite lost receiver");
        assert!(completion.load(Ordering::Acquire));
        assert!(
            controller.shutdown(),
            "retry reads authoritative completion without requiring an exited worker"
        );
        assert_eq!(
            observations.snapshot().sidecar_lifecycle,
            ObservedSidecarLifecycle::Stopped
        );
    }
    #[test]
    fn stored_default_target_is_carried_exactly_into_compilation() {
        use crate::domain::*;
        let root = isolated_test_root().join("default-target");
        let store = JsonStateStore::new(root.join("state.json")).expect("store");
        let mut state = AppState::empty();
        state.subscriptions.push(Subscription {
            skipped_unsupported_nodes: 0,
            id: SubscriptionId("sub".into()),
            name: "fixture".into(),
            source: SubscriptionSource::Manual,
            document: None,
            last_success_at_ms: None,
            http_metadata: None,
            description: String::new(),
            last_attempt_at_ms: None,
            remote_request: None,
            update_policy: crate::domain::SubscriptionUpdatePolicy::manual(),
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
        state.active_subscription_id = Some(SubscriptionId("sub".into()));
        state.active_configuration_generation = 1;
        for target in [
            RouteTarget::Pool(PoolId("first".into())),
            RouteTarget::Unconfigured,
            RouteTarget::Direct,
            RouteTarget::Block,
        ] {
            state.default_target = target.clone();
            store.save(&state).expect("save exact target");
            let loaded = load_runtime_intent(&root.join("state.json"), &StateAccessGate::default());
            let loaded = loaded.expect("paired selected projection");
            if !matches!(target, RouteTarget::Unconfigured) {
                assert_eq!(loaded.projected_default_target, target);
            } else {
                assert!(
                    matches!(loaded.projected_default_target, RouteTarget::Pool(_)),
                    "Unconfigured gets a runtime-only selected pool"
                );
            }
            let config = compile_projection(&loaded).expect("paired compile");
            let value: serde_json::Value = serde_json::from_slice(config.as_bytes()).expect("JSON");
            let expected_final = match target {
                RouteTarget::Pool(_) => "pool-first",
                RouteTarget::Unconfigured => "pool-runtime-active-sub",
                RouteTarget::Direct => "direct",
                RouteTarget::Block => "block",
            };
            assert_eq!(value["route"]["final"], expected_final);
            assert_eq!(
                store
                    .load()
                    .expect("unchanged persistent target")
                    .default_target,
                target
            );
        }
        std::fs::remove_dir_all(root).expect("remove isolated state fixture");
    }
    #[test]
    fn activation_cancellation_after_commit_still_finishes_the_selected_runtime() {
        let (context, mut runtime, request) = activation_fixture("");
        let cancel = Arc::clone(&request.cancel);
        context.observations.install_delta_sink(move |delta| {
            if delta
                .subscription_switch
                .as_ref()
                .is_some_and(|change| change.status == SubscriptionSwitchStatus::Persisted)
            {
                cancel.store(true, Ordering::Release);
            }
        });
        assert!(matches!(
            apply_activation(&mut runtime, request, &context),
            ActivationResult::Activated { generation: 2, .. }
        ));
        assert_eq!(
            context
                .observations
                .snapshot()
                .subscription_switch
                .expect("terminal")
                .status,
            SubscriptionSwitchStatus::Ready
        );
        std::fs::remove_dir_all(context.app_local_data_root).expect("isolated cleanup");
    }

    #[test]
    fn selected_start_requires_selection_and_already_current_compares_applied_generation() {
        let root = isolated_test_root();
        let store = JsonStateStore::new(root.join("state.json")).expect("store");
        store.save(&AppState::empty()).expect("empty V5");
        let (requests, receiver) = mpsc::sync_channel(1);
        let observations = InMemoryRuntimeObservations::new_mock();
        let mut controller = test_controller(requests, observations.clone());
        controller.state_file = root.join("state.json");
        assert_eq!(
            controller.start(),
            ManagedRuntimeStartResult::SubscriptionSelectionRequired
        );
        assert!(matches!(
            receiver.try_recv(),
            Err(mpsc::TryRecvError::Empty)
        ));
        let state = valid_selected_state();
        store.save(&state).expect("selection");
        observations.record_subscription_ready("sub".into(), 1, None, false);
        assert!(matches!(
            controller.activate("sub".into(), false),
            ActivationResult::AlreadyCurrent { generation: 1, .. }
        ));
        assert!(
            matches!(receiver.try_recv(), Err(mpsc::TryRecvError::Empty)),
            "alreadyCurrent does not queue or replace"
        );
        let _writer = controller
            .subscriptions
            .begin_write_owned()
            .expect("concurrent HTTP writer");
        assert!(matches!(
            controller.activate("sub".into(), false),
            ActivationResult::Error {
                error: ActivationError::Busy,
                ..
            }
        ));
        assert_eq!(controller.start(), ManagedRuntimeStartResult::Busy);
        assert_eq!(controller.stop(), ManagedRuntimeStopResult::Busy);
        assert_eq!(store.load().expect("unchanged"), state);
        std::fs::remove_dir_all(root).expect("isolated cleanup");
    }
    #[test]
    fn activation_timeout_retains_both_owned_guards_until_cancelled_worker_finishes() {
        let root = isolated_test_root();
        let store = JsonStateStore::new(root.join("state.json")).expect("store");
        let selected = valid_selected_state();
        store.save(&selected).expect("selected state");
        let (requests, receiver) = mpsc::sync_channel(1);
        let observations = InMemoryRuntimeObservations::new_mock();
        let mut controller = test_controller(requests, observations.clone());
        controller.state_file = root.join("state.json");
        let controller = Arc::new(controller);
        let caller = Arc::clone(&controller);
        let waiting = thread::spawn(move || caller.activate("sub".into(), false));
        let WorkerRequest::Activate(request) = receiver
            .recv_timeout(Duration::from_secs(2))
            .expect("queued request")
        else {
            panic!("activate only");
        };
        let operation_id = request.operation_id.clone();
        assert_eq!(controller.stop(), ManagedRuntimeStopResult::Busy);
        assert!(controller.subscriptions.begin_write_owned().is_err());
        let result = waiting.join().expect("15 second bounded IPC wait");
        assert!(
            matches!(result, ActivationResult::Pending { operation_id: id } if id == operation_id)
        );
        assert!(request.cancel.load(Ordering::Acquire));
        assert_eq!(controller.start(), ManagedRuntimeStartResult::Busy);
        assert_eq!(controller.stop(), ManagedRuntimeStopResult::Busy);
        assert!(
            controller.subscriptions.begin_write_owned().is_err(),
            "waiting caller no longer owns the guard"
        );
        let context = test_context(root.join("resources"), root.clone(), observations.clone());
        let mut runtime = SidecarRuntime::new_observation_only(FailingPort::default());
        switch_phase(
            &observations,
            &operation_id,
            SubscriptionSwitchStatus::Queued,
            None,
        );
        assert!(matches!(
            apply_activation(&mut runtime, *request, &context),
            ActivationResult::Pending { .. }
        ));
        assert_eq!(
            observations
                .snapshot()
                .subscription_switch
                .expect("late terminal")
                .status,
            SubscriptionSwitchStatus::Cancelled
        );
        assert!(!controller.busy.load(Ordering::Acquire));
        assert!(controller.subscriptions.begin_write_owned().is_ok());
        assert_eq!(store.load().expect("unchanged state"), selected);
        assert!(!runtime.into_port().events.contains(&"run"));
        std::fs::remove_dir_all(root).expect("isolated cleanup");
    }
    #[test]
    fn shutdown_closes_subscription_coordinator_before_core_and_allows_cleanup_retry() {
        let (requests, receiver) = mpsc::sync_channel(1);
        let observations = InMemoryRuntimeObservations::new_mock();
        let mut controller = test_controller(requests, observations.clone());
        controller.scheduler = Some(Arc::new(SubscriptionScheduler::start(Arc::clone(
            &controller.subscriptions,
        ))));
        let subscriptions = Arc::clone(&controller.subscriptions);
        let complete = Arc::clone(&controller.shutdown_complete);
        let worker = thread::spawn(move || {
            let WorkerRequest::Shutdown { response } = receiver
                .recv_timeout(Duration::from_secs(3))
                .expect("first shutdown")
            else {
                panic!("shutdown expected");
            };
            assert!(
                subscriptions.is_closing(),
                "coordinator stopped before core request"
            );
            assert!(
                subscriptions.begin_write_owned().is_err(),
                "no new subscription write after closing"
            );
            response
                .send(ShutdownResult::ShutdownFailed)
                .expect("failed cleanup");
            let WorkerRequest::Stop { response, .. } = receiver
                .recv_timeout(Duration::from_secs(3))
                .expect("manual cleanup")
            else {
                panic!("Stop expected");
            };
            observations.record_managed_stopped();
            response
                .send(ManagedRuntimeStopResult::Stopped)
                .expect("manual cleanup response");
            let WorkerRequest::Shutdown { response } = receiver
                .recv_timeout(Duration::from_secs(3))
                .expect("retry quit")
            else {
                panic!("retry shutdown expected");
            };
            complete.store(true, Ordering::Release);
            response
                .send(ShutdownResult::ShutdownComplete)
                .expect("confirmed completion");
        });
        assert!(
            !controller.shutdown(),
            "unconfirmed core cleanup cannot exit app"
        );
        assert_eq!(controller.start(), ManagedRuntimeStartResult::Busy);
        assert_eq!(
            controller.stop(),
            ManagedRuntimeStopResult::Stopped,
            "closing cannot block explicit resource cleanup"
        );
        assert!(controller.shutdown());
        worker.join().expect("worker joined");
        assert!(
            controller.shutdown(),
            "authoritative completion is idempotent"
        );
    }
}
