//! P2-03 手动代理应用服务；唯一 owner 串行调用，不包含 last-applied/rollback。
use super::{
    runtime_snapshot::{InstanceId, RuntimeFacts, RuntimeSnapshot, RuntimeState},
    selected_subscription::project_selected_runtime,
    state_service::SnapshotService,
};
use crate::{
    domain::OutboundId,
    singbox::{
        LoopbackListener, ManagedCacheFile, ProductCompileRequest, ProductRuntimeResources,
        SingBoxCompiler,
        runtime::{ManagedRuntimeEndpoints, SidecarLifecycle, SidecarPort, SidecarRuntime},
        secret::generate_api_secret,
    },
};
use std::{path::PathBuf, sync::Arc, time::Instant};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeCommand {
    Start,
    Stop,
    Restart,
    Refresh,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeResult {
    Started,
    Stopped,
    Restarted,
    AlreadyRunning,
    Refreshed,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeError {
    KernelUnavailable,
    StateUnavailable,
    CompileFailed,
    CandidateFailed,
    StopFailed,
    UnexpectedExit,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManualRuntimeSnapshot {
    pub runtime: RuntimeSnapshot,
    pub endpoints: Option<ManagedRuntimeEndpoints>,
    pub uptime_seconds: Option<u64>,
    pub kernel_available: bool,
}
pub struct ManualRuntime<P> {
    sidecar: SidecarRuntime<P>,
    snapshots: Arc<SnapshotService>,
    root: PathBuf,
    facts: RuntimeFacts,
    started: Option<Instant>,
    kernel_available: bool,
}
impl<P: SidecarPort> ManualRuntime<P> {
    pub fn new(
        port: P,
        snapshots: Arc<SnapshotService>,
        root: PathBuf,
        kernel_available: bool,
    ) -> Self {
        Self {
            sidecar: SidecarRuntime::new_dynamic(port),
            snapshots,
            root,
            facts: RuntimeFacts {
                current: RuntimeState::Stopped,
                last_successful: None,
            },
            started: None,
            kernel_available,
        }
    }
    pub fn snapshot(&self) -> Result<ManualRuntimeSnapshot, RuntimeError> {
        let saved = self
            .snapshots
            .snapshot()
            .map_err(|_| RuntimeError::StateUnavailable)?
            .config_version();
        Ok(ManualRuntimeSnapshot {
            runtime: RuntimeSnapshot::from_owner(saved, &self.facts),
            endpoints: self.sidecar.endpoints(),
            uptime_seconds: self.started.map(|s| s.elapsed().as_secs()),
            kernel_available: self.kernel_available,
        })
    }
    pub fn execute(
        &mut self,
        command: RuntimeCommand,
        mut publish: impl FnMut(ManualRuntimeSnapshot),
    ) -> Result<RuntimeResult, RuntimeError> {
        let result = self.execute_inner(command, &mut publish);
        if result.is_err()
            && !matches!(
                self.facts.current,
                RuntimeState::Ready { .. } | RuntimeState::Recovering { .. }
            )
        {
            self.facts.current = RuntimeState::Failed { instance_id: None };
            self.started = None;
        }
        result
    }
    fn execute_inner(
        &mut self,
        command: RuntimeCommand,
        publish: &mut impl FnMut(ManualRuntimeSnapshot),
    ) -> Result<RuntimeResult, RuntimeError> {
        if command == RuntimeCommand::Stop {
            if self.sidecar.stop().is_err() {
                self.facts.current = RuntimeState::Recovering { instance: None };
                self.started = None;
                return Err(RuntimeError::StopFailed);
            }
            self.facts.current = RuntimeState::Stopped;
            self.started = None;
            return Ok(RuntimeResult::Stopped);
        }
        let previously_ready = matches!(self.facts.current, RuntimeState::Ready { .. });
        let alive = if previously_ready {
            match self.sidecar.refresh_alive() {
                Ok(alive) => alive,
                Err(_) => {
                    self.facts.current = RuntimeState::Recovering { instance: None };
                    self.started = None;
                    return Err(RuntimeError::StopFailed);
                }
            }
        } else {
            false
        };
        if previously_ready && !alive {
            self.facts.current = RuntimeState::Failed { instance_id: None };
            self.started = None;
            if command == RuntimeCommand::Refresh {
                return Err(RuntimeError::UnexpectedExit);
            }
        }
        if command == RuntimeCommand::Refresh {
            return Ok(RuntimeResult::Refreshed);
        }
        if command == RuntimeCommand::Start
            && matches!(self.facts.current, RuntimeState::Ready { .. })
        {
            return Ok(RuntimeResult::AlreadyRunning);
        }
        if !matches!(self.facts.current, RuntimeState::Ready { .. }) {
            self.facts.current = RuntimeState::Starting {
                instance_id: InstanceId("pending".into()),
            };
            if let Ok(snapshot) = self.snapshot() {
                publish(snapshot);
            }
        }
        if !self.kernel_available {
            return Err(RuntimeError::KernelUnavailable);
        }
        let state = self
            .snapshots
            .snapshot()
            .map_err(|_| RuntimeError::StateUnavailable)?;
        let projection =
            project_selected_runtime(&state).map_err(|_| RuntimeError::CompileFailed)?;
        let default_outbound = OutboundId::from_route_target(&projection.projected_default_target)
            .map_err(|_| RuntimeError::CompileFailed)?;
        // 同 epoch/内核代际稳定，普通 revision/instance 不改变 cache 路径；停止旧 writer 后再 run。
        let epoch: Vec<u8> = serde_json::from_value(
            serde_json::to_value(&state.state_epoch).map_err(|_| RuntimeError::CompileFailed)?,
        )
        .map_err(|_| RuntimeError::CompileFailed)?;
        let generation = format!(
            "v1.14.0-{}",
            epoch.iter().map(|v| format!("{v:02x}")).collect::<String>()
        );
        let cache_root = self.root.join("kernel-cache");
        std::fs::create_dir_all(&cache_root).map_err(|_| RuntimeError::CandidateFailed)?;
        let resources = ProductRuntimeResources {
            mixed: LoopbackListener::new(([127, 0, 0, 1], 0).into())
                .map_err(|_| RuntimeError::CompileFailed)?,
            controller: LoopbackListener::new(([127, 0, 0, 1], 0).into())
                .map_err(|_| RuntimeError::CompileFailed)?,
            cache: ManagedCacheFile::new(
                &cache_root,
                cache_root.join(format!("{generation}.db")),
                generation,
                false,
                false,
            )
            .map_err(|_| RuntimeError::CompileFailed)?,
        };
        let plan = SingBoxCompiler
            .compile_product(ProductCompileRequest {
                state: &state,
                runtime_intent: &projection.runtime_intent,
                default_outbound: &default_outbound,
                resources: &resources,
            })
            .map_err(|_| RuntimeError::CompileFailed)?;
        let secret = generate_api_secret().map_err(|_| RuntimeError::CandidateFailed)?;
        let candidate = plan
            .finalize(&secret)
            .map_err(|_| RuntimeError::CompileFailed)?;
        // 准备失败不修改旧 facts/applied version，也不停止旧 child。
        self.sidecar
            .prepare_replacement(candidate)
            .map_err(|_| RuntimeError::CandidateFailed)?;
        self.facts.current = RuntimeState::Starting {
            instance_id: InstanceId("pending".into()),
        };
        self.started = None;
        if let Ok(snapshot) = self.snapshot() {
            publish(snapshot);
        }
        if self.sidecar.commit_prepared().is_err() {
            self.facts.current =
                if self.sidecar.snapshot().lifecycle == SidecarLifecycle::RecoveryRequired {
                    RuntimeState::Recovering { instance: None }
                } else {
                    RuntimeState::Failed { instance_id: None }
                };
            self.started = None;
            return Err(RuntimeError::CandidateFailed);
        }
        if self.sidecar.endpoints().is_none() {
            let _ = self.sidecar.stop();
            self.facts.current = RuntimeState::Failed { instance_id: None };
            self.started = None;
            return Err(RuntimeError::CandidateFailed);
        }
        self.facts.current = RuntimeState::Ready {
            instance_id: InstanceId(format!(
                "manual-{}",
                self.sidecar.active_identity().expect("ready owner")
            )),
            applied_version: state.config_version(),
        };
        self.started = Some(Instant::now());
        Ok(if command == RuntimeCommand::Restart {
            RuntimeResult::Restarted
        } else {
            RuntimeResult::Started
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        application::state_access::StateAccessGate,
        domain::{AppState, RouteTarget},
        singbox::{
            GeneratedConfig,
            runtime::{ManagedSidecar, SidecarPortError},
        },
        storage::{JsonStateStore, StateStore},
    };
    use std::sync::Mutex;
    #[derive(Default)]
    struct Trace {
        next: u64,
        active: Option<u64>,
        checks: usize,
        stops: usize,
        fail_check: bool,
        fail_ready: bool,
        fail_stop: bool,
        crash: bool,
        config: Option<serde_json::Value>,
    }
    struct Mock(Arc<Mutex<Trace>>);
    impl SidecarPort for Mock {
        fn check(&mut self, c: &GeneratedConfig) -> Result<(), SidecarPortError> {
            let mut t = self.0.lock().unwrap();
            t.checks += 1;
            t.config = Some(serde_json::from_slice(c.as_bytes()).unwrap());
            if t.fail_check {
                Err(SidecarPortError)
            } else {
                Ok(())
            }
        }
        fn prepare(&mut self, _: &GeneratedConfig) -> Result<(), SidecarPortError> {
            Ok(())
        }
        fn run(&mut self) -> Result<ManagedSidecar, SidecarPortError> {
            let mut t = self.0.lock().unwrap();
            assert!(t.active.is_none());
            t.next += 1;
            t.active = Some(t.next);
            Ok(ManagedSidecar::from_port_identity(t.next))
        }
        fn ready(&mut self, _: &ManagedSidecar) -> Result<(), SidecarPortError> {
            if self.0.lock().unwrap().fail_ready {
                Err(SidecarPortError)
            } else {
                Ok(())
            }
        }
        fn endpoints(&self, i: &ManagedSidecar) -> Option<ManagedRuntimeEndpoints> {
            let t = self.0.lock().unwrap();
            (t.active == Some(i.identity())).then_some(ManagedRuntimeEndpoints {
                mixed: ([127, 0, 0, 1], 12345).into(),
                controller: ([127, 0, 0, 1], 54321).into(),
            })
        }
        fn is_alive(&mut self, i: &ManagedSidecar) -> Result<bool, SidecarPortError> {
            let t = self.0.lock().unwrap();
            Ok(t.active == Some(i.identity()) && !t.crash)
        }
        fn stop(&mut self, i: &ManagedSidecar) -> Result<(), SidecarPortError> {
            let mut t = self.0.lock().unwrap();
            if t.fail_stop {
                return Err(SidecarPortError);
            }
            assert_eq!(t.active, Some(i.identity()));
            t.active = None;
            t.stops += 1;
            Ok(())
        }
        fn cancel_pending(&mut self) -> Result<(), SidecarPortError> {
            Ok(())
        }
        fn has_pending_cleanup(&self) -> bool {
            false
        }
    }
    fn fixture() -> (
        ManualRuntime<Mock>,
        Arc<Mutex<Trace>>,
        JsonStateStore,
        PathBuf,
    ) {
        let mut value = serde_json::to_value(AppState::empty()).unwrap();
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/compiler/p2-02b-selected.json"
        ))
        .unwrap();
        for (k, v) in fixture.as_object().unwrap() {
            value[k] = v.clone();
        }
        let state: AppState = serde_json::from_value(value).unwrap();
        assert_eq!(state.default_target, RouteTarget::Unconfigured);
        let root = std::env::temp_dir().join(format!("veyra-p203-core-{:?}", state.state_epoch));
        let store = JsonStateStore::new(root.join("state.json")).unwrap();
        store.save(&state).unwrap();
        let snapshots = Arc::new(SnapshotService::new(
            store.clone(),
            StateAccessGate::default(),
        ));
        let trace = Arc::new(Mutex::new(Trace::default()));
        (
            ManualRuntime::new(Mock(trace.clone()), snapshots, root.clone(), true),
            trace,
            store,
            root,
        )
    }
    // 保护首次导入 Unconfigured 真实正式服务路径、版本区别与失败保留旧进程。
    #[test]
    fn first_import_product_duplicate_check_failure_and_saved_applied() {
        let (mut owner, trace, store, root) = fixture();
        assert_eq!(
            owner.execute(RuntimeCommand::Start, |s| assert_eq!(
                s.runtime.status,
                super::super::runtime_snapshot::RuntimeStatus::Starting
            )),
            Ok(RuntimeResult::Started)
        );
        let before = owner.snapshot().unwrap();
        let config = trace.lock().unwrap().config.clone().unwrap();
        assert_eq!(config["inbounds"][0]["listen_port"], 0);
        assert_eq!(
            config["experimental"]["clash_api"]["external_controller"],
            "127.0.0.1:0"
        );
        assert!(
            config["route"]["final"]
                .as_str()
                .unwrap()
                .contains("runtime-active")
        );
        assert_eq!(
            owner.execute(RuntimeCommand::Start, |_| {}),
            Ok(RuntimeResult::AlreadyRunning)
        );
        assert_eq!(trace.lock().unwrap().checks, 1);
        let mut state = store.load().unwrap();
        state.profile.reject_quic = true;
        store.commit(&state).unwrap();
        trace.lock().unwrap().fail_check = true;
        assert_eq!(
            owner.execute(RuntimeCommand::Restart, |_| {}),
            Err(RuntimeError::CandidateFailed)
        );
        let after = owner.snapshot().unwrap();
        assert_eq!(after.runtime.instance_id, before.runtime.instance_id);
        assert_eq!(
            after.runtime.applied_version,
            before.runtime.applied_version
        );
        assert_ne!(after.runtime.saved_version, before.runtime.saved_version);
        assert_eq!(trace.lock().unwrap().stops, 0);
        owner.execute(RuntimeCommand::Stop, |_| {}).unwrap();
        let stopped = owner.snapshot().unwrap();
        assert!(stopped.runtime.applied_version.is_none());
        assert!(stopped.endpoints.is_none());
        std::fs::remove_dir_all(root).unwrap();
    }
    // 保护 ready 失败清理、异常退出撤销 endpoint 及新实例恢复。
    #[test]
    fn timeout_cleanup_crash_refresh_and_restart_identity() {
        let (mut owner, trace, _, root) = fixture();
        trace.lock().unwrap().fail_ready = true;
        assert_eq!(
            owner.execute(RuntimeCommand::Start, |_| {}),
            Err(RuntimeError::CandidateFailed)
        );
        assert!(trace.lock().unwrap().active.is_none());
        assert!(owner.snapshot().unwrap().endpoints.is_none());
        trace.lock().unwrap().fail_ready = false;
        owner.execute(RuntimeCommand::Start, |_| {}).unwrap();
        let before = owner.snapshot().unwrap();
        trace.lock().unwrap().crash = true;
        assert_eq!(
            owner.execute(RuntimeCommand::Refresh, |_| {}),
            Err(RuntimeError::UnexpectedExit)
        );
        let failed = owner.snapshot().unwrap();
        assert!(failed.endpoints.is_none());
        assert!(failed.runtime.applied_version.is_none());
        trace.lock().unwrap().crash = false;
        owner.execute(RuntimeCommand::Start, |_| {}).unwrap();
        assert_ne!(
            owner.snapshot().unwrap().runtime.instance_id,
            before.runtime.instance_id
        );
        owner.execute(RuntimeCommand::Stop, |_| {}).unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn stop_and_crash_cleanup_failure_never_retains_ready() {
        let (mut owner, trace, _, root) = fixture();
        owner.execute(RuntimeCommand::Start, |_| {}).unwrap();
        trace.lock().unwrap().fail_stop = true;
        assert_eq!(
            owner.execute(RuntimeCommand::Stop, |_| {}),
            Err(RuntimeError::StopFailed)
        );
        let s = owner.snapshot().unwrap();
        assert_eq!(
            s.runtime.status,
            super::super::runtime_snapshot::RuntimeStatus::Recovering
        );
        assert!(s.runtime.applied_version.is_none());
        assert!(s.uptime_seconds.is_none());
        assert!(s.endpoints.is_none());
        trace.lock().unwrap().fail_stop = false;
        owner.execute(RuntimeCommand::Stop, |_| {}).unwrap();
        owner.execute(RuntimeCommand::Start, |_| {}).unwrap();
        {
            let mut t = trace.lock().unwrap();
            t.crash = true;
            t.fail_stop = true;
        }
        assert_eq!(
            owner.execute(RuntimeCommand::Refresh, |_| {}),
            Err(RuntimeError::StopFailed)
        );
        assert!(owner.snapshot().unwrap().runtime.applied_version.is_none());
        trace.lock().unwrap().fail_stop = false;
        owner.execute(RuntimeCommand::Stop, |_| {}).unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }
}
