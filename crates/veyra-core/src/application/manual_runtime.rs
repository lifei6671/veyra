//! 单一 Runtime owner：应用、选择、cache writer 和恢复记录均在串行入口内执行。
use super::{
    runtime_recovery::*,
    runtime_snapshot::{
        InstanceId, LastSuccessfulVersion, RuntimeFacts, RuntimeSnapshot, RuntimeState,
    },
    selected_subscription::project_selected_runtime,
    state_service::{SelectionService, SnapshotService},
};
use crate::{
    domain::{
        AppState, ConfigVersion, NodeId, OutboundId, PoolId, SelectionPolicy, SelectionVersion,
    },
    singbox::{
        LoopbackListener, ManagedCacheFile, ProductCompileRequest, ProductRuntimeResources,
        RECOVERY_FORMAT, SingBoxCompiler, SingBoxPlan,
        runtime::{ManagedRuntimeEndpoints, SidecarLifecycle, SidecarPort, SidecarRuntime},
        secret::generate_api_secret,
    },
};
use std::{collections::BTreeMap, path::PathBuf, sync::Arc, time::Instant};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeCommand {
    Start,
    Stop,
    Restart,
    Refresh,
    ApplySaved,
    RestoreLastSuccessful,
    /// 显式读回待确认选择；不重发 pending。
    ReconcileSelection,
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
pub enum CandidateFailure {
    RunOrReady,
    SelectionReconcile,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeError {
    KernelUnavailable,
    StateUnavailable,
    CompileFailed,
    CandidateFailed,
    StopFailed,
    UnexpectedExit,
    RecoveryUnavailable(RecoveryError),
    CacheSnapshotFailed,
    SelectionReconcileFailed,
    SelectionPending,
    /// child 真实 Ready、applied 已更新；last_successful 保留旧记录。
    RecoveryRecordFailed,
    RecoveryPreparationFailed,
    RollbackRecordFailed(CandidateFailure),
    CandidateRolledBack(CandidateFailure),
    RollbackFailed(CandidateFailure),
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManualSelectionRequest {
    pub instance: InstanceId,
    pub expected: SelectionVersion,
    pub pool: PoolId,
    pub node: NodeId,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SelectionError {
    StaleInstance,
    StaleVersion,
    ConfigChanged,
    InvalidMember,
    Pending,
    ControllerWrite,
    ControllerReadBack,
    /// pending 保存失败，controller 尚未写入。
    PendingSaveFailed,
    /// 同实例已读回 requested，但业务确认保存失败，磁盘 pending 仍待核对。
    ConfirmationSaveFailed,
    /// 实际旧值已读回，CAS 清 pending 保存失败，仍保留待核对意图。
    PendingClearFailed,
    RecoveryRecordFailed,
    StateUnavailable,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingSelection {
    pub pool: PoolId,
    pub requested: NodeId,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManualRuntimeSnapshot {
    pub runtime: RuntimeSnapshot,
    pub endpoints: Option<ManagedRuntimeEndpoints>,
    pub uptime_seconds: Option<u64>,
    pub kernel_available: bool,
    pub recovery: RecoveryAvailability,
    pub confirmed_selection_version: Option<SelectionVersion>,
    pub pending_selection: Option<PendingSelection>,
    pub pending_selections: Vec<PendingSelection>,
    pub selection_fallbacks: Vec<PoolId>,
    pub confirmed_manual_selections: BTreeMap<PoolId, NodeId>,
}
struct ActivePlan {
    plan: SingBoxPlan,
    selection: ConfirmedSelection,
}
pub struct ManualRuntime<P> {
    sidecar: SidecarRuntime<P>,
    snapshots: Arc<SnapshotService>,
    records: Result<RecoveryStore, RecoveryError>,
    facts: RuntimeFacts,
    active: Option<ActivePlan>,
    started: Option<Instant>,
    kernel_available: bool,
    nonce: String,
    fallbacks: Vec<PoolId>,
    observation: super::observability::controller::ObservationService,
    // 同步 Runtime worker 的观测执行器；不拥有业务事实或另一个内核实例。
    observation_executor: Option<tokio::runtime::Runtime>,
}
impl<P: SidecarPort> ManualRuntime<P> {
    pub fn new(
        port: P,
        snapshots: Arc<SnapshotService>,
        root: PathBuf,
        kernel_available: bool,
    ) -> Self {
        let records = RecoveryStore::new(&root);
        let last_successful = records
            .as_ref()
            .ok()
            .and_then(|r| r.read_manifest().ok())
            .map(|m| LastSuccessfulVersion {
                config: m.config,
                selection: m.confirmed_selection.version,
            });
        let mut random = [0u8; 16];
        getrandom::fill(&mut random).expect("OS randomness for Runtime owner");
        Self {
            sidecar: SidecarRuntime::new_dynamic(port),
            snapshots,
            records,
            facts: RuntimeFacts {
                current: RuntimeState::Stopped,
                last_successful,
            },
            active: None,
            started: None,
            kernel_available,
            nonce: digest(&random),
            fallbacks: vec![],
            observation: super::observability::controller::ObservationService::new(),
            observation_executor: None,
        }
    }
    /// 只读订阅入口；绑定和停止始终由当前 Runtime owner 管理。
    pub fn observation(&self) -> &super::observability::controller::ObservationService {
        &self.observation
    }
    fn store(&self) -> Result<&RecoveryStore, RuntimeError> {
        self.records
            .as_ref()
            .map_err(|e| RuntimeError::RecoveryUnavailable(*e))
    }
    fn resources(
        &self,
        epoch: &crate::domain::StateEpoch,
    ) -> Result<ProductRuntimeResources, RuntimeError> {
        let generation = cache_generation(epoch);
        let path = self
            .store()?
            .cache_path(&generation)
            .map_err(RuntimeError::RecoveryUnavailable)?;
        Ok(ProductRuntimeResources {
            mixed: LoopbackListener::new(([127, 0, 0, 1], 0).into())
                .map_err(|_| RuntimeError::CompileFailed)?,
            controller: LoopbackListener::new(([127, 0, 0, 1], 0).into())
                .map_err(|_| RuntimeError::CompileFailed)?,
            cache: ManagedCacheFile::new(
                path.parent().expect("managed cache parent"),
                path.clone(),
                generation,
                false,
                false,
            )
            .map_err(|_| RuntimeError::CompileFailed)?,
        })
    }
    fn load_recovery(
        &self,
        state: &AppState,
    ) -> Result<(LastAppliedManifest, SingBoxPlan), RuntimeError> {
        let (m, bytes) = self
            .store()?
            .load(&state.state_epoch)
            .map_err(RuntimeError::RecoveryUnavailable)?;
        let plan = SingBoxPlan::recover(&bytes, &self.resources(&state.state_epoch)?)
            .map_err(|_| RuntimeError::RecoveryUnavailable(RecoveryError::Corrupt))?;
        let index = plan
            .artifact_index()
            .ok_or(RuntimeError::RecoveryUnavailable(RecoveryError::Corrupt))?;
        if index.config != m.config
            || index.selection != m.plan_selection
            || m.confirmed_selection.nodes.iter().any(|(id, node)| {
                index
                    .pools
                    .get(id)
                    .is_none_or(|p| !p.members.contains_key(node))
            })
            || index.pools.len() != m.confirmed_selection.nodes.len()
        {
            return Err(RuntimeError::RecoveryUnavailable(RecoveryError::Corrupt));
        }
        Ok((m, plan))
    }
    pub fn snapshot(&self) -> Result<ManualRuntimeSnapshot, RuntimeError> {
        let state = self
            .snapshots
            .snapshot()
            .map_err(|_| RuntimeError::StateUnavailable)?;
        let recovery = match self.load_recovery(&state) {
            Ok(_) => RecoveryAvailability::Available,
            Err(RuntimeError::RecoveryUnavailable(reason)) => {
                RecoveryAvailability::Unavailable(reason)
            }
            Err(_) => RecoveryAvailability::Unavailable(RecoveryError::Corrupt),
        };
        let ready = matches!(self.facts.current, RuntimeState::Ready { .. });
        let pending = Self::pending_in(&state);
        Ok(ManualRuntimeSnapshot {
            runtime: RuntimeSnapshot::from_owner(state.config_version(), &self.facts),
            endpoints: ready.then(|| self.sidecar.endpoints()).flatten(),
            uptime_seconds: self.started.map(|s| s.elapsed().as_secs()),
            kernel_available: self.kernel_available,
            recovery,
            confirmed_selection_version: pending
                .is_empty()
                .then(|| self.active.as_ref().map(|a| a.selection.version.clone()))
                .flatten(),
            pending_selection: pending.first().cloned(),
            pending_selections: pending.clone(),
            selection_fallbacks: self.fallbacks.clone(),
            confirmed_manual_selections: pending
                .is_empty()
                .then_some(&self.active)
                .and_then(|a| a.as_ref())
                .map(|a| {
                    a.selection
                        .nodes
                        .iter()
                        .filter(|(id, _)| state.pools.iter().any(|p| p.id == **id))
                        .map(|(id, node)| (id.clone(), node.clone()))
                        .collect()
                })
                .unwrap_or_default(),
        })
    }
    pub fn execute(
        &mut self,
        command: RuntimeCommand,
        mut publish: impl FnMut(ManualRuntimeSnapshot),
    ) -> Result<RuntimeResult, RuntimeError> {
        if !matches!(command, RuntimeCommand::Stop | RuntimeCommand::Refresh)
            && !self
                .store()?
                .local_owner_available(&self.nonce)
                .map_err(RuntimeError::RecoveryUnavailable)?
        {
            return Err(RuntimeError::RecoveryUnavailable(RecoveryError::UnsafePath));
        }
        let result = self.execute_inner(command, &mut publish);
        if result.is_err()
            && self.sidecar.active_identity().is_none()
            && self.sidecar.snapshot().has_candidate_config
        {
            let _ = self.sidecar.cancel_prepared();
        }
        if self.sidecar.snapshot().lifecycle == SidecarLifecycle::RecoveryRequired {
            self.facts.current = RuntimeState::Recovering { instance: None };
            self.started = None;
        }
        if result.is_err()
            && !matches!(
                self.facts.current,
                RuntimeState::Ready { .. } | RuntimeState::Recovering { .. }
            )
            && !(command == RuntimeCommand::Stop
                && self.sidecar.snapshot().lifecycle == SidecarLifecycle::Stopped)
        {
            self.facts.current =
                if self.sidecar.snapshot().lifecycle == SidecarLifecycle::RecoveryRequired {
                    RuntimeState::Recovering { instance: None }
                } else {
                    RuntimeState::Failed { instance_id: None }
                };
            self.started = None;
            self.active = None;
        }
        if !matches!(self.facts.current, RuntimeState::Ready { .. }) {
            self.observation.stop();
        }
        result
    }
    fn execute_inner(
        &mut self,
        command: RuntimeCommand,
        publish: &mut impl FnMut(ManualRuntimeSnapshot),
    ) -> Result<RuntimeResult, RuntimeError> {
        if command == RuntimeCommand::Stop {
            self.observation.stop();
            self.sidecar.stop().map_err(|_| {
                self.facts.current = RuntimeState::Recovering { instance: None };
                self.started = None;
                RuntimeError::StopFailed
            })?;
            self.facts.current = RuntimeState::Stopped;
            self.started = None;
            let active = self.active.take();
            if let Some(active) = active {
                // 缓存更新也是 last-successful owner 唯一写路径；新运行未提交 manifest 时不覆盖旧配置的快照。
                let index = active.plan.artifact_index().expect("product index");
                let store = self.store()?;
                if let Ok(mut m) = store.read_manifest()
                    && m.config == index.config
                {
                    m.confirmed_selection = active.selection.clone();
                    m.cache = Some(
                        store
                            .snapshot_cache(&m.cache_generation)
                            .map_err(|_| RuntimeError::CacheSnapshotFailed)?,
                    );
                    store
                        .commit(&m)
                        .map_err(|_| RuntimeError::RecoveryPreparationFailed)?;
                    store.prune_after_success(&m);
                    self.facts.last_successful = Some(LastSuccessfulVersion {
                        config: m.config,
                        selection: m.confirmed_selection.version,
                    });
                }
            }
            return Ok(RuntimeResult::Stopped);
        }
        let previously_ready = matches!(self.facts.current, RuntimeState::Ready { .. });
        if previously_ready {
            match self.sidecar.refresh_alive() {
                Ok(true) => {}
                Ok(false) => {
                    self.observation.stop();
                    self.active = None;
                    self.started = None;
                    self.facts.current = RuntimeState::Failed { instance_id: None };
                    if command == RuntimeCommand::Refresh {
                        return Err(RuntimeError::UnexpectedExit);
                    }
                }
                Err(_) => {
                    self.observation.stop();
                    self.facts.current = RuntimeState::Recovering { instance: None };
                    self.started = None;
                    return Err(RuntimeError::StopFailed);
                }
            }
        }
        if command == RuntimeCommand::ReconcileSelection {
            let active = self.active.as_ref().ok_or(RuntimeError::SelectionPending)?;
            let plan = active.plan.clone();
            let prior = active.selection.clone();
            let state = self
                .snapshots
                .snapshot()
                .map_err(|_| RuntimeError::StateUnavailable)?;
            self.reconcile(&plan, &prior, &state, true)?;
            self.commit_confirmed_selection()
                .map_err(|_| RuntimeError::RecoveryRecordFailed)?;
            return Ok(RuntimeResult::Refreshed);
        }
        if command == RuntimeCommand::Refresh {
            return Ok(RuntimeResult::Refreshed);
        }
        if command == RuntimeCommand::Start
            && matches!(self.facts.current, RuntimeState::Ready { .. })
        {
            return Ok(RuntimeResult::AlreadyRunning);
        }
        if !self.kernel_available {
            return Err(RuntimeError::KernelUnavailable);
        }
        let state = self
            .snapshots
            .snapshot()
            .map_err(|_| RuntimeError::StateUnavailable)?;
        let restore = command == RuntimeCommand::RestoreLastSuccessful;
        let loaded = self.load_recovery(&state);
        if !restore
            && matches!(
                loaded,
                Err(RuntimeError::RecoveryUnavailable(
                    RecoveryError::Corrupt
                        | RecoveryError::DigestMismatch
                        | RecoveryError::ResourceMissing
                ))
            )
        {
            self.store()?
                .preserve_unavailable_record()
                .map_err(|_| RuntimeError::RecoveryPreparationFailed)?;
        }
        let mut recovery = loaded.ok();
        let (plan, selection) = if restore {
            let (m, p) = self.load_recovery(&state)?;
            (p, m.confirmed_selection.clone())
        } else {
            let projection =
                project_selected_runtime(&state).map_err(|_| RuntimeError::CompileFailed)?;
            let outbound = OutboundId::from_route_target(&projection.projected_default_target)
                .map_err(|_| RuntimeError::CompileFailed)?;
            let plan = SingBoxCompiler
                .compile_product(ProductCompileRequest {
                    state: &state,
                    runtime_intent: &projection.runtime_intent,
                    default_outbound: &outbound,
                    resources: &self.resources(&state.state_epoch)?,
                })
                .map_err(|_| RuntimeError::CompileFailed)?;
            let selection = Self::selection_for(&plan, &state, None).0;
            (plan, selection)
        };
        // 全部业务 pending 必须由 candidate 的受控 index 解释；旧 plan 缺组时
        // 在 check/prepare/停止旧 writer 之前拒绝，保留现有实例与磁盘意图。
        Self::require_pending_coverage(&plan, &state)?;
        // Restore命令不授权以另一plan/cache消除pending。active同plan的关闭cache可信；
        // 重建owner仅接受与manifest引用相同的live字节，不能把较旧cache覆盖上去再推断actual。
        let restore_cache_trusted = if restore {
            self.restore_cache_trusted(&recovery.as_ref().expect("loaded restore").0)?
        } else {
            true
        };
        if !restore_cache_trusted && !Self::pending_in(&state).is_empty() {
            return Err(RuntimeError::SelectionPending);
        }
        let generated = plan
            .finalize(&generate_api_secret().map_err(|_| RuntimeError::CandidateFailed)?)
            .map_err(|_| RuntimeError::CompileFailed)?;
        let config_digest = digest(generated.as_bytes());
        self.store()?
            .ensure_cache(&cache_generation(&state.state_epoch))
            .map_err(RuntimeError::RecoveryUnavailable)?;
        // check/prepare 失败旧 child 不动；恢复 artifact 写失败同样发生在停止旧 writer 前。
        self.sidecar
            .prepare_replacement(generated)
            .map_err(|_| RuntimeError::CandidateFailed)?;
        let plan_ref = match self.store()?.save_plan(
            &plan
                .recovery_bytes()
                .map_err(|_| RuntimeError::CompileFailed)?,
        ) {
            Ok(r) => r,
            Err(_) => {
                let _ = self.sidecar.cancel_prepared();
                return Err(RuntimeError::RecoveryPreparationFailed);
            }
        };
        // prepare 期间业务快照可能变化；停止旧 writer 前再次检查，取消候选而不动旧 child。
        let coverage = self
            .snapshots
            .snapshot()
            .map_err(|_| RuntimeError::StateUnavailable)
            .and_then(|latest| {
                Self::require_pending_coverage(&plan, &latest)?;
                if !restore_cache_trusted && !Self::pending_in(&latest).is_empty() {
                    return Err(RuntimeError::SelectionPending);
                }
                Ok(())
            });
        if let Err(error) = coverage {
            let _ = self.sidecar.cancel_prepared();
            return Err(error);
        }
        let old_index = self
            .active
            .as_ref()
            .map(|a| a.plan.artifact_index().expect("product index").clone());
        self.observation.stop();
        self.sidecar.stop_old_writer().map_err(|_| {
            self.facts.current = RuntimeState::Recovering { instance: None };
            self.started = None;
            RuntimeError::StopFailed
        })?;
        self.facts.current = RuntimeState::Starting {
            instance_id: InstanceId("pending".into()),
        };
        self.started = None;
        self.active = None;
        if let Ok(s) = self.snapshot() {
            publish(s);
        }
        let cache = if let Some(old) = old_index {
            let store = self.store()?;
            let closed = match store.snapshot_cache(&cache_generation(&old.config.0.epoch)) {
                Ok(r) => r,
                Err(_) => {
                    let _ = self.sidecar.cancel_prepared();
                    return Err(RuntimeError::CacheSnapshotFailed);
                }
            };
            if let Some((m, _)) = &mut recovery
                && m.config == old.config
            {
                m.cache = Some(closed.clone());
                // 上一个成功版的关闭缓存先持久化，失败阻止 candidate 运行并保留证据。
                if store.commit(m).is_err() {
                    let _ = self.sidecar.cancel_prepared();
                    return Err(RuntimeError::RecoveryPreparationFailed);
                }
            }
            (old.config.0.epoch == state.state_epoch).then_some(closed)
        } else {
            let generation = cache_generation(&state.state_epoch);
            if self
                .store()?
                .cache_path(&generation)
                .map_err(RuntimeError::RecoveryUnavailable)?
                .exists()
            {
                Some(
                    self.store()?
                        .snapshot_cache(&generation)
                        .map_err(|_| RuntimeError::CacheSnapshotFailed)?,
                )
            } else {
                None
            }
        };
        let candidate_cache = if restore {
            let m = &recovery
                .as_ref()
                .ok_or(RuntimeError::RecoveryUnavailable(RecoveryError::Missing))?
                .0;
            if self.store()?.restore_cache(m).is_err() {
                let _ = self.sidecar.cancel_prepared();
                return Err(RuntimeError::CacheSnapshotFailed);
            }
            self.store()?
                .ensure_cache(&m.cache_generation)
                .map_err(RuntimeError::RecoveryUnavailable)?;
            m.cache.clone()
        } else {
            cache
        };
        let failure = if self.sidecar.run_prepared().is_err() {
            Some(CandidateFailure::RunOrReady)
        } else {
            match self.reconcile(&plan, &selection, &state, restore_cache_trusted) {
                Ok(()) => None,
                // 不能借 rollback 的另一个 cache 实例猜测/消除尚未解释的请求。
                Err(RuntimeError::SelectionPending) => {
                    self.sidecar.stop().map_err(|_| RuntimeError::StopFailed)?;
                    return Err(RuntimeError::SelectionPending);
                }
                Err(_) => Some(CandidateFailure::SelectionReconcile),
            }
        };
        if let Some(failure) = failure {
            if self.sidecar.stop().is_err() {
                self.facts.current = RuntimeState::Recovering { instance: None };
                return Err(RuntimeError::RollbackFailed(failure));
            }
            // candidate失败不构成pending的实际值证据。清理候选后保留live cache及磁盘意图，
            // 禁止从旧manifest/cache再起一个child，把其actual=a误当成原实例未切到b。
            let latest = self
                .snapshots
                .snapshot()
                .map_err(|_| RuntimeError::StateUnavailable)?;
            if !Self::pending_in(&latest).is_empty() {
                return Err(RuntimeError::SelectionPending);
            }
            if let Some((mut m, previous)) = recovery {
                // 只回退一次；新 secret、动态端口与 child，重新 check/Ready/reconcile。
                let result = (|| {
                    self.store()?
                        .restore_cache(&m)
                        .map_err(|_| RuntimeError::CacheSnapshotFailed)?;
                    self.store()?
                        .ensure_cache(&m.cache_generation)
                        .map_err(RuntimeError::RecoveryUnavailable)?;
                    let generated = previous
                        .finalize(
                            &generate_api_secret().map_err(|_| RuntimeError::CandidateFailed)?,
                        )
                        .map_err(|_| RuntimeError::CompileFailed)?;
                    m.config_digest = digest(generated.as_bytes());
                    self.sidecar
                        .prepare_replacement(generated)
                        .map_err(|_| RuntimeError::CandidateFailed)?;
                    self.sidecar
                        .commit_prepared()
                        .map_err(|_| RuntimeError::CandidateFailed)?;
                    // 即使pending在prepare/run期间才出现，rollback cache也没有核对它的来源。
                    self.reconcile(&previous, &m.confirmed_selection, &state, false)?;
                    self.mark_ready(previous);
                    Ok::<_, RuntimeError>(())
                })();
                if result.is_ok() {
                    m.confirmed_selection = self
                        .active
                        .as_ref()
                        .expect("rollback ready")
                        .selection
                        .clone();
                    m.selection_at_apply = m.confirmed_selection.version.clone();
                    if self.store()?.commit(&m).is_err() {
                        return Err(RuntimeError::RollbackRecordFailed(failure));
                    }
                    self.store()?.prune_after_success(&m);
                    self.facts.last_successful = Some(LastSuccessfulVersion {
                        config: m.config,
                        selection: m.confirmed_selection.version,
                    });
                    return Err(RuntimeError::CandidateRolledBack(failure));
                }
                let stopped = self.sidecar.stop();
                if matches!(result, Err(RuntimeError::SelectionPending)) && stopped.is_ok() {
                    return Err(RuntimeError::SelectionPending);
                }
                return Err(RuntimeError::RollbackFailed(failure));
            }
            return Err(if failure == CandidateFailure::SelectionReconcile {
                RuntimeError::SelectionReconcileFailed
            } else {
                RuntimeError::CandidateFailed
            });
        }
        self.mark_ready(plan);
        let active = self.active.as_ref().expect("ready plan");
        let index = active.plan.artifact_index().expect("product index");
        let m = LastAppliedManifest {
            schema: 1,
            state_epoch: index.config.0.epoch.clone(),
            config: index.config.clone(),
            selection_at_apply: active.selection.version.clone(),
            plan_selection: index.selection.clone(),
            confirmed_selection: active.selection.clone(),
            kernel_version: KERNEL_VERSION.into(),
            kernel_digest: KERNEL_DIGEST.into(),
            compiler_format: RECOVERY_FORMAT,
            recovery_format: RECOVERY_FORMAT,
            plan: plan_ref,
            config_digest,
            cache_generation: cache_generation(&index.config.0.epoch),
            cache: candidate_cache,
            resources: vec![],
        };
        self.store()?
            .commit(&m)
            .map_err(|_| RuntimeError::RecoveryRecordFailed)?;
        self.store()?.prune_after_success(&m);
        self.facts.last_successful = Some(LastSuccessfulVersion {
            config: m.config,
            selection: m.confirmed_selection.version,
        });
        Ok(if command == RuntimeCommand::Restart {
            RuntimeResult::Restarted
        } else {
            RuntimeResult::Started
        })
    }
    fn selection_for(
        plan: &SingBoxPlan,
        state: &AppState,
        prior: Option<&ConfirmedSelection>,
    ) -> (ConfirmedSelection, Vec<PoolId>) {
        let index = plan.artifact_index().expect("product index");
        let mut nodes = BTreeMap::new();
        let mut fallbacks = vec![];
        let use_business = state.state_epoch == index.config.0.epoch
            && prior.is_none_or(|p| state.selection_revision >= p.version.0.revision);
        for (id, pool) in &index.pools {
            let business = use_business
                .then(|| state.pools.iter().find(|p| p.id == *id))
                .flatten();
            let target = if let Some(p) = business {
                match &p.selection {
                    SelectionPolicy::Manual {
                        selected_node_id, ..
                    } => selected_node_id.clone(),
                    _ => None,
                }
            } else {
                prior
                    .and_then(|p| p.nodes.get(id).cloned())
                    .or(pool.selected_node.clone())
            };
            let node = match target {
                Some(node) if pool.members.contains_key(&node) => node,
                Some(_) => {
                    fallbacks.push(id.clone());
                    pool.default_node.clone()
                }
                None => pool.default_node.clone(),
            };
            nodes.insert(id.clone(), node);
        }
        let version = if use_business {
            state.selection_version()
        } else {
            prior.expect("historical selection").version.clone()
        };
        (ConfirmedSelection { version, nodes }, fallbacks)
    }
    fn pending_in(state: &AppState) -> Vec<PendingSelection> {
        state
            .pools
            .iter()
            .filter_map(|pool| match &pool.selection {
                SelectionPolicy::Manual {
                    pending_node_id: Some(requested),
                    ..
                } => Some(PendingSelection {
                    pool: pool.id.clone(),
                    requested: requested.clone(),
                }),
                _ => None,
            })
            .collect()
    }
    fn restore_cache_trusted(&self, manifest: &LastAppliedManifest) -> Result<bool, RuntimeError> {
        if let Some(active) = &self.active {
            return active
                .plan
                .recovery_bytes()
                .map(|bytes| digest(&bytes) == manifest.plan.digest)
                .map_err(|_| RuntimeError::CompileFailed);
        }
        let live = self
            .store()?
            .cache_path(&manifest.cache_generation)
            .map_err(RuntimeError::RecoveryUnavailable)?;
        let Some(cache) = &manifest.cache else {
            return Ok(false);
        };
        if !live.exists() {
            return Ok(false);
        }
        // 这里只核验本owner私有cache，不复制、不修改manifest。不同字节不能借恢复猜选项。
        std::fs::read(live)
            .map(|bytes| digest(&bytes) == cache.digest)
            .map_err(|_| RuntimeError::CacheSnapshotFailed)
    }
    fn require_pending_coverage(plan: &SingBoxPlan, state: &AppState) -> Result<(), RuntimeError> {
        let index = plan.artifact_index().expect("product index");
        if Self::pending_in(state).iter().any(|pending| {
            index
                .pools
                .get(&pending.pool)
                .is_none_or(|pool| !pool.members.contains_key(&pending.requested))
        }) {
            return Err(RuntimeError::SelectionPending);
        }
        Ok(())
    }
    fn reconcile(
        &mut self,
        plan: &SingBoxPlan,
        prior: &ConfirmedSelection,
        state: &AppState,
        pending_cache_trusted: bool,
    ) -> Result<(), RuntimeError> {
        let index = plan.artifact_index().expect("product index");
        if state.state_epoch != index.config.0.epoch {
            return Err(RuntimeError::SelectionReconcileFailed);
        }
        let mut current = self
            .snapshots
            .snapshot()
            .map_err(|_| RuntimeError::StateUnavailable)?;
        if current.state_epoch != state.state_epoch {
            return Err(RuntimeError::SelectionReconcileFailed);
        }
        // 不只检查将要遍历的组：active/recovery plan 缺任一 pending 时不得 GET/PUT/CAS。
        Self::require_pending_coverage(plan, &current)?;
        if !pending_cache_trusted && !Self::pending_in(&current).is_empty() {
            return Err(RuntimeError::SelectionPending);
        }
        let service = SelectionService::new((*self.snapshots).clone());
        let (mut selection, mut fallbacks) = Self::selection_for(plan, &current, Some(prior));
        for (id, node) in &mut selection.nodes {
            let pool = &index.pools[id];
            let pending =
                current
                    .pools
                    .iter()
                    .find(|p| p.id == *id)
                    .and_then(|p| match &p.selection {
                        SelectionPolicy::Manual {
                            pending_node_id, ..
                        } => pending_node_id.clone(),
                        _ => None,
                    });
            // 每个 selector 先 GET。pending 只根据实际值解决，永远不自动 PUT 请求值。
            let actual = self
                .sidecar
                .with_active_port(|p, c| p.read_selector(c, &pool.runtime_tag))
                .map_err(|_| {
                    if pending.is_some() {
                        RuntimeError::SelectionPending
                    } else {
                        RuntimeError::SelectionReconcileFailed
                    }
                })?
                .ok_or(RuntimeError::SelectionReconcileFailed)?;
            if let Some(requested) = pending {
                let requested_tag = pool
                    .members
                    .get(&requested)
                    .ok_or(RuntimeError::SelectionPending)?;
                // 旧 confirmed 必须由业务 stable ID 唯一映射；仅 None 才采用 compiler 默认。
                // Restore 旧 plan 不含当前已选新节点时，不能用 fallback 猜旧值并清 pending。
                let old_node = current
                    .pools
                    .iter()
                    .find(|p| p.id == *id)
                    .and_then(|p| match &p.selection {
                        SelectionPolicy::Manual {
                            selected_node_id, ..
                        } => selected_node_id.clone(),
                        _ => None,
                    })
                    .unwrap_or_else(|| pool.default_node.clone());
                let old_tag = pool.members.get(&old_node);
                let saved = if actual == *requested_tag {
                    let saved = service
                        .confirm_manual_pending(
                            current.selection_version(),
                            id.clone(),
                            requested.clone(),
                        )
                        .map_err(|_| RuntimeError::SelectionPending)?;
                    *node = requested;
                    saved
                } else if old_tag.is_some_and(|tag| actual == *tag) {
                    let saved = service
                        .clear_manual_pending(current.selection_version(), id.clone(), requested)
                        .map_err(|_| RuntimeError::SelectionPending)?;
                    *node = old_node;
                    saved
                } else {
                    return Err(RuntimeError::SelectionPending);
                };
                // 自己的 staged commit 产生的新版本必须进入 active/manifest；并发写不冒充确认。
                current = self
                    .snapshots
                    .snapshot()
                    .map_err(|_| RuntimeError::StateUnavailable)?;
                if current.selection_version() != saved.version {
                    return Err(RuntimeError::SelectionPending);
                }
                selection.version = saved.version;
                fallbacks.retain(|pool| pool != id);
            } else if actual != pool.members[node] {
                self.controller_confirm(
                    &pool.runtime_tag,
                    &pool.members[node],
                    &current.selection_version(),
                )
                .map_err(|error| {
                    if error == SelectionError::StaleVersion {
                        RuntimeError::SelectionPending
                    } else {
                        RuntimeError::SelectionReconcileFailed
                    }
                })?;
            }
        }
        let latest = self
            .snapshots
            .snapshot()
            .map_err(|_| RuntimeError::StateUnavailable)?;
        // 最终 Ready gate：即使循环期间出现新 pending，也不能漏过全量业务事实。
        if !Self::pending_in(&latest).is_empty() {
            return Err(RuntimeError::SelectionPending);
        }
        if latest.selection_version() != current.selection_version()
            || latest.state_epoch != state.state_epoch
        {
            return Err(RuntimeError::SelectionReconcileFailed);
        }
        self.fallbacks = fallbacks;
        self.active = Some(ActivePlan {
            plan: plan.clone(),
            selection,
        });
        Ok(())
    }
    fn mark_ready(&mut self, plan: SingBoxPlan) {
        let config: ConfigVersion = plan.artifact_index().expect("product index").config.clone();
        self.facts.current = RuntimeState::Ready {
            instance_id: InstanceId(format!(
                "manual-{}-{}",
                self.nonce,
                self.sidecar.active_identity().expect("ready owner")
            )),
            applied_version: config,
        };
        self.started = Some(Instant::now());
        // 必须等 check/run、鉴权 Ready 和选择核对全部完成后绑定。
        let endpoint = self
            .sidecar
            .with_active_port(|port, child| Ok(port.observation_endpoint(child)))
            .ok()
            .flatten()
            .flatten();
        self.observation.stop();
        if let Some(endpoint) = endpoint {
            if self.observation_executor.is_none() {
                self.observation_executor = tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(1)
                    .thread_name("veyra-observation")
                    .enable_all()
                    .build()
                    .ok();
            }
            if let Some(executor) = &self.observation_executor {
                let _entered = executor.enter();
                let RuntimeState::Ready { instance_id, .. } = &self.facts.current else {
                    unreachable!()
                };
                self.observation.bind(instance_id.clone(), endpoint);
            } else {
                tracing::warn!("observation executor unavailable; metrics remain unknown");
            }
        }
    }
    fn controller_confirm(
        &mut self,
        pool: &str,
        node: &str,
        expected: &SelectionVersion,
    ) -> Result<(), SelectionError> {
        let snapshots = self.snapshots.clone();
        snapshots
            .with_selection_write(expected, || {
                self.sidecar
                    .with_active_port(|p, c| p.write_selector(c, pool, node))
                    .map_err(|_| SelectionError::ControllerWrite)?
                    .ok_or(SelectionError::StaleInstance)?;
                let actual = self
                    .sidecar
                    .with_active_port(|p, c| p.read_selector(c, pool))
                    .map_err(|_| SelectionError::ControllerReadBack)?
                    .ok_or(SelectionError::StaleInstance)?;
                if actual != node {
                    return Err(SelectionError::ControllerReadBack);
                }
                Ok(())
            })
            .map_err(|error| {
                if error.code() == crate::domain::AppErrorCode::RevisionConflict {
                    SelectionError::StaleVersion
                } else {
                    SelectionError::StateUnavailable
                }
            })?
    }
    /// 当前 Manual 的唯一业务入口；未来编排器同样经 owner 调用，UI 不接收 tag。
    pub fn select_manual(
        &mut self,
        request: ManualSelectionRequest,
    ) -> Result<SelectionVersion, SelectionError> {
        if !self
            .records
            .as_ref()
            .map_err(|_| SelectionError::StateUnavailable)?
            .local_owner_available(&self.nonce)
            .map_err(|_| SelectionError::StateUnavailable)?
        {
            return Err(SelectionError::Pending);
        }
        let RuntimeState::Ready {
            instance_id,
            applied_version,
        } = &self.facts.current
        else {
            return Err(SelectionError::StaleInstance);
        };
        if *instance_id != request.instance {
            return Err(SelectionError::StaleInstance);
        }
        let applied_version = applied_version.clone();
        match self.sidecar.refresh_alive() {
            Ok(true) => {}
            Ok(false) => {
                self.observation.stop();
                self.facts.current = RuntimeState::Failed { instance_id: None };
                self.active = None;
                self.started = None;
                return Err(SelectionError::StaleInstance);
            }
            Err(_) => {
                self.observation.stop();
                self.facts.current = RuntimeState::Recovering { instance: None };
                self.started = None;
                return Err(SelectionError::StaleInstance);
            }
        }
        let state = self
            .snapshots
            .snapshot()
            .map_err(|_| SelectionError::StateUnavailable)?;
        if !Self::pending_in(&state).is_empty() {
            return Err(SelectionError::Pending);
        }
        if state.selection_version() != request.expected
            || self
                .active
                .as_ref()
                .is_none_or(|a| a.selection.version != request.expected)
        {
            return Err(SelectionError::StaleVersion);
        }
        if state.state_epoch != applied_version.0.epoch {
            return Err(SelectionError::ConfigChanged);
        }
        let active = self.active.as_ref().expect("ready plan");
        let pool = active
            .plan
            .artifact_index()
            .expect("product index")
            .pools
            .get(&request.pool)
            .ok_or(SelectionError::InvalidMember)?
            .clone();
        let node_tag = pool
            .members
            .get(&request.node)
            .ok_or(SelectionError::InvalidMember)?
            .clone();
        // Synthetic projection 不能保存到业务快照；当前 API 只允许真实 Manual pool。
        if !state
            .pools
            .iter()
            .any(|p| p.id == request.pool && matches!(p.selection, SelectionPolicy::Manual { .. }))
        {
            return Err(SelectionError::InvalidMember);
        }
        let old = pool.members[&active.selection.nodes[&request.pool]].clone();
        let service = SelectionService::new((*self.snapshots).clone());
        let pending = service
            .begin_manual_pending(request.expected, request.pool.clone(), request.node.clone())
            .map_err(|e| {
                if e.code() == crate::domain::AppErrorCode::RevisionConflict {
                    SelectionError::StaleVersion
                } else {
                    SelectionError::PendingSaveFailed
                }
            })?;
        if let Err(error) = self.controller_confirm(&pool.runtime_tag, &node_tag, &pending.version)
        {
            if error == SelectionError::ControllerWrite {
                // PUT 的错误响应本身不能证明没有切换；额外 GET 确认旧值才可清 pending。
                let actual = self
                    .sidecar
                    .with_active_port(|p, c| p.read_selector(c, &pool.runtime_tag));
                if matches!(actual, Ok(Some(ref tag)) if *tag == old) {
                    let cleared = service
                        .clear_manual_pending(pending.version, request.pool, request.node)
                        .map_err(|_| SelectionError::PendingClearFailed)?;
                    self.active
                        .as_mut()
                        .expect("same serial instance")
                        .selection
                        .version = cleared.version;
                    self.commit_confirmed_selection()?;
                }
            }
            return Err(error);
        }
        // read-back 已确认切换，commit 失败保留磁盘 pending，不补偿、不宣称旧值仍在运行。
        let saved = service
            .confirm_manual_pending(pending.version, request.pool.clone(), request.node.clone())
            .map_err(|_| SelectionError::ConfirmationSaveFailed)?;
        let active = self.active.as_mut().expect("same serial instance");
        active.selection.nodes.insert(request.pool, request.node);
        active.selection.version = saved.version.clone();
        self.commit_confirmed_selection()?;
        Ok(saved.version)
    }
    /// 仅轻量更新确认 section，配置 artifact/version 不变。
    fn commit_confirmed_selection(&mut self) -> Result<(), SelectionError> {
        let active = self.active.as_ref().expect("ready plan");
        let config = active
            .plan
            .artifact_index()
            .expect("product index")
            .config
            .clone();
        let selection = active.selection.clone();
        let store = self
            .records
            .as_ref()
            .map_err(|_| SelectionError::RecoveryRecordFailed)?;
        let mut m = store
            .read_manifest()
            .map_err(|_| SelectionError::RecoveryRecordFailed)?;
        if m.config != config {
            return Err(SelectionError::RecoveryRecordFailed);
        }
        m.confirmed_selection = selection;
        store
            .commit(&m)
            .map_err(|_| SelectionError::RecoveryRecordFailed)?;
        self.facts.last_successful = Some(LastSuccessfulVersion {
            config: m.config,
            selection: m.confirmed_selection.version,
        });
        Ok(())
    }
}
#[cfg(test)]
#[path = "manual_runtime/tests.rs"]
mod tests;

mod handoff;

mod remote_selection;

impl<P> Drop for ManualRuntime<P> {
    fn drop(&mut self) {
        self.observation.stop();
        // 允许 owner 在 async 测试/宿主中释放；不能阻塞另一个 Tokio executor。
        if let Some(executor) = self.observation_executor.take() {
            executor.shutdown_background();
        }
    }
}
