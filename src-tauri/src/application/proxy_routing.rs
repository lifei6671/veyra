use std::{
    collections::{BTreeMap, HashSet},
    path::PathBuf,
    sync::{Arc, Mutex},
};

use serde::{Deserialize, Serialize};

use crate::{
    application::{
        managed_observation_runtime::{
            ActivationError, ActivationResult, ManagedObservationRuntimeController,
            ManualSelectionRuntimeResult, ManualSelectionSavedOnlyReason,
            ManualSelectionUnknownReason, RoutingApplyTerminal, RoutingApplyTerminalFailure,
        },
        observability::{
            ObservedSidecarLifecycle, RuntimeObservationSnapshot, SubscriptionSwitchErrorCode,
            SubscriptionSwitchStatus,
        },
        state_access::{StateAccessError, StateAccessGate},
    },
    domain::{
        AppState, MAX_SAFE_INTEGER, NetworkProtocol, NodeFilter, NodeId, NodePool, PoolId,
        PoolKind, PoolSource, ProviderId, ProxyProtocol, RoutePolicy, RoutePolicyId, RouteTarget,
        SelectionPolicy, StateValidationError, TrafficMatcher,
    },
    storage::{JsonStateStore, StateStore},
};

const MAX_POOLS: usize = 100;
const MAX_ROUTES: usize = 500;
const MAX_SOURCES: usize = 100;
const MAX_FILTER_TEXTS: usize = 64;
const MAX_FILTER_NODE_IDS: usize = 5_000;
const MAX_MATCHER_VALUES: usize = 500;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct MutateProxyRoutingRequest {
    pub(crate) expected_revision: u64,
    pub(crate) mutation: ProxyRoutingMutation,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub(crate) enum ProxyRoutingMutation {
    CreateCustomPool {
        name: String,
        enabled: bool,
        sources: Vec<PoolSourceDto>,
        selection: SelectionDto,
    },
    UpdateCustomPool {
        id: String,
        name: String,
        enabled: bool,
        sources: Vec<PoolSourceDto>,
        selection: SelectionDto,
    },
    DeleteCustomPool {
        id: String,
    },
    SetManualSelection {
        pool_id: String,
        node_id: String,
    },
    SetDefaultTarget {
        target: DefaultTargetDto,
    },
    CreateRoute {
        name: String,
        enabled: bool,
        matcher: MatcherDto,
        target: RouteTargetDto,
        insert_at: usize,
    },
    UpdateRoute {
        id: String,
        name: String,
        enabled: bool,
        matcher: MatcherDto,
        target: RouteTargetDto,
    },
    DeleteRoute {
        id: String,
    },
    ReorderRoutes {
        route_ids: Vec<String>,
    },
    ApplyConfiguration,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PoolSourceDto {
    pub(crate) provider_id: String,
    pub(crate) filter: NodeFilterDto,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct NodeFilterDto {
    pub(crate) regions: Vec<String>,
    pub(crate) protocols: Vec<ProxyProtocolDto>,
    pub(crate) include_keywords: Vec<String>,
    pub(crate) exclude_keywords: Vec<String>,
    pub(crate) include_node_ids: Vec<String>,
    pub(crate) exclude_node_ids: Vec<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum ProxyProtocolDto {
    Socks,
    Http,
    Shadowsocks,
    Vmess,
    Vless,
    Trojan,
    WireGuard,
    Hysteria,
    Hysteria2,
    Tuic,
    ShadowTls,
    Ssh,
    Naive,
    AnyTls,
    Snell,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub(crate) enum SelectionDto {
    Manual {
        selected_node_id: Option<String>,
    },
    UrlTest {
        probe_url: String,
        interval_seconds: u64,
        tolerance_ms: u32,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    tag = "type",
    content = "values",
    rename_all = "camelCase",
    deny_unknown_fields
)]
pub(crate) enum MatcherDto {
    Domain(Vec<String>),
    DomainSuffix(Vec<String>),
    Application(Vec<String>),
    IpCidr(Vec<String>),
    Port(Vec<u16>),
    Protocol(Vec<NetworkProtocolDto>),
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum NetworkProtocolDto {
    Tcp,
    Udp,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub(crate) enum DefaultTargetDto {
    FollowActiveSubscription,
    Pool { pool_id: String },
    Direct,
    Block,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub(crate) enum RouteTargetDto {
    Pool { pool_id: String },
    Direct,
    Block,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum QueryError {
    Busy,
    StateUnavailable,
    RuntimeUnavailable,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub(crate) enum SnapshotResult {
    Ok { snapshot: Box<ProxyRoutingSnapshot> },
    Error { error: QueryError },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ProxyRoutingSnapshot {
    pub(crate) revision: u64,
    pub(crate) desired_generation: u64,
    pub(crate) applied_generation: Option<u64>,
    pub(crate) runtime_state: RuntimeState,
    pub(crate) apply_state: ApplyState,
    pub(crate) active_subscription_id: Option<String>,
    pub(crate) applied_subscription_id: Option<String>,
    pub(crate) providers: Vec<ProviderSnapshot>,
    pub(crate) nodes: Vec<NodeSnapshot>,
    pub(crate) default_target: DefaultTargetDto,
    pub(crate) pools: Vec<PoolSnapshot>,
    pub(crate) routes: Vec<RouteSnapshot>,
    pub(crate) selectors: Vec<SelectorSnapshot>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum RuntimeState {
    Stopped,
    Ready,
    Transitioning,
    RecoveryRequired,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub(crate) enum ApplyState {
    Applied,
    SavedPendingApply,
    Applying {
        operation_id: String,
    },
    SavedApplyFailed {
        operation_id: String,
        error: ApplyFailure,
    },
    ApplyUnknown {
        operation_id: String,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum ApplyFailure {
    ConfigurationFailed,
    StateChanged,
    StopFailed,
    StartFailed,
    RecoveryRequired,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ProviderSnapshot {
    pub(crate) id: String,
    pub(crate) subscription_id: String,
    pub(crate) name: String,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct NodeSnapshot {
    pub(crate) id: String,
    pub(crate) provider_id: String,
    pub(crate) name: String,
    pub(crate) protocol: ProxyProtocolDto,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PoolSnapshot {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) kind: PoolKindDto,
    pub(crate) enabled: bool,
    pub(crate) sources: Vec<PoolSourceDto>,
    pub(crate) selection: SelectionDto,
    pub(crate) resolved_node_ids: Vec<String>,
}
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum PoolKindDto {
    ImplicitProvider,
    Custom,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RouteSnapshot {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) enabled: bool,
    pub(crate) priority: i32,
    pub(crate) matcher: MatcherDto,
    pub(crate) target: RouteTargetDto,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SelectorSnapshot {
    pub(crate) pool_id: String,
    pub(crate) desired_node_id: Option<String>,
    pub(crate) runtime_node_id: Option<String>,
    pub(crate) state: SelectorState,
}
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum SelectorState {
    InSync,
    SavedOnly,
    NotApplied,
    Unknown,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub(crate) enum MutationResult {
    Ok {
        outcome: MutationOutcome,
        snapshot: Box<ProxyRoutingSnapshot>,
    },
    Error {
        error: MutationError,
        revision: Option<u64>,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub(crate) enum MutationOutcome {
    Saved,
    SelectorApplied {
        pool_id: String,
        node_id: String,
    },
    SelectorNotApplied {
        pool_id: String,
        node_id: String,
        runtime_node_id: String,
    },
    SelectorSavedOnly {
        pool_id: String,
        node_id: String,
        reason: SelectorSavedOnlyReason,
    },
    SelectorApplyUnknown {
        pool_id: String,
        node_id: String,
        reason: SelectorApplyUnknownReason,
    },
    ApplyStarted {
        operation_id: String,
    },
    ApplyCompleted {
        operation_id: String,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum SelectorSavedOnlyReason {
    RuntimeStopped,
    RuntimeNotReady,
    NotInAppliedArtifact,
    DispatchUnavailable,
}
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum SelectorApplyUnknownReason {
    ReadBackUnavailable,
    UnknownRuntimeNode,
    InstanceChanged,
    Superseded,
}
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum MutationError {
    InvalidInput,
    Busy,
    Conflict,
    NotFound,
    ReferenceConflict,
    ValidationFailed,
    SaveFailed,
    StateUnavailable,
    RuntimeUnavailable,
    ConfigurationFailed,
    StateChanged,
    StopFailed,
    StartFailed,
    RecoveryRequired,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CasMaterial {
    active_subscription_id: Option<String>,
    desired_generation: u64,
    providers: Vec<(String, String, String)>,
    nodes: Vec<(String, String, String, ProxyProtocolDto)>,
    pools: Vec<NodePool>,
    default_target: RouteTarget,
    routes: Vec<RoutePolicy>,
}
impl From<&AppState> for CasMaterial {
    fn from(state: &AppState) -> Self {
        Self {
            active_subscription_id: state.active_subscription_id.as_ref().map(|v| v.0.clone()),
            desired_generation: state.active_configuration_generation,
            providers: state
                .providers
                .iter()
                .map(|v| (v.id.0.clone(), v.subscription_id.0.clone(), v.name.clone()))
                .collect(),
            nodes: state
                .nodes
                .iter()
                .map(|v| {
                    (
                        v.id.0.clone(),
                        v.provider_id.0.clone(),
                        v.name.clone(),
                        v.protocol.into(),
                    )
                })
                .collect(),
            pools: state.pools.clone(),
            default_target: state.default_target.clone(),
            routes: state.routes.clone(),
        }
    }
}

#[derive(Default)]
struct RevisionState {
    revision: u64,
    material: Option<CasMaterial>,
}
impl RevisionState {
    fn observe(&mut self, material: CasMaterial) -> Result<u64, QueryError> {
        if self.material.as_ref() != Some(&material) {
            self.revision = self
                .revision
                .checked_add(1)
                .filter(|v| *v <= MAX_SAFE_INTEGER)
                .ok_or(QueryError::StateUnavailable)?;
            self.material = Some(material);
        }
        Ok(self.revision)
    }
    fn commit(&mut self, material: CasMaterial) -> Result<u64, MutationError> {
        self.observe(material)
            .map_err(|_| MutationError::StateUnavailable)
    }
}

pub(crate) struct ProxyRoutingManager {
    store: JsonStateStore,
    gate: StateAccessGate,
    revision: Mutex<RevisionState>,
    apply_tuple: Mutex<Option<(Option<String>, u64)>>,
    runtime: Arc<ManagedObservationRuntimeController>,
}

impl ProxyRoutingManager {
    pub(crate) fn new(
        state_file: PathBuf,
        gate: StateAccessGate,
        runtime: Arc<ManagedObservationRuntimeController>,
    ) -> Result<Self, QueryError> {
        Ok(Self {
            store: JsonStateStore::new(state_file).map_err(|_| QueryError::StateUnavailable)?,
            gate,
            revision: Mutex::new(RevisionState::default()),
            apply_tuple: Mutex::new(None),
            runtime,
        })
    }

    pub(crate) fn snapshot(&self) -> SnapshotResult {
        match self
            .load_observed()
            .and_then(|(state, revision)| self.build_snapshot(&state, revision))
        {
            Ok(snapshot) => SnapshotResult::Ok {
                snapshot: Box::new(snapshot),
            },
            Err(error) => SnapshotResult::Error { error },
        }
    }

    /// spawn_blocking join failure is outside the mutation transaction. Preserve the closed
    /// response contract and include the last revision this manager can authoritatively name.
    pub(crate) fn execution_unavailable(&self) -> MutationResult {
        MutationResult::Error {
            error: MutationError::RuntimeUnavailable,
            revision: self.known_revision(),
        }
    }

    pub(crate) fn mutate(&self, request: MutateProxyRoutingRequest) -> MutationResult {
        let expected = request.expected_revision;
        if !(1..=MAX_SAFE_INTEGER).contains(&expected) {
            return MutationResult::Error {
                error: MutationError::InvalidInput,
                revision: self.known_revision(),
            };
        }
        let access = match self.gate.try_lock() {
            Ok(v) => v,
            Err(e) => return self.mutation_access_error(e),
        };
        let mut state = match self.load_state() {
            Ok(v) => v,
            Err(e) => {
                return MutationResult::Error {
                    error: e,
                    revision: self.known_revision(),
                };
            }
        };
        let current = match self.observe(&state) {
            Ok(v) => v,
            Err(e) => {
                return MutationResult::Error {
                    error: e,
                    revision: self.known_revision(),
                };
            }
        };
        if current != expected {
            return MutationResult::Error {
                error: MutationError::Conflict,
                revision: Some(current),
            };
        }
        if matches!(request.mutation, ProxyRoutingMutation::ApplyConfiguration) {
            let Some(active_id) = state.active_subscription_id.as_ref().map(|id| id.0.clone())
            else {
                return MutationResult::Error {
                    error: MutationError::ValidationFailed,
                    revision: Some(current),
                };
            };
            drop(access);
            return self.apply_configuration(
                active_id,
                state.active_configuration_generation,
                current,
            );
        }
        let manual_selection = match &request.mutation {
            ProxyRoutingMutation::SetManualSelection { pool_id, node_id } => {
                Some((pool_id.clone(), node_id.clone()))
            }
            _ => None,
        };
        let manual = matches!(
            request.mutation,
            ProxyRoutingMutation::SetManualSelection { .. }
        );
        let outcome = match apply_mutation(&mut state, request.mutation) {
            Ok(v) => v,
            Err(e) => {
                return MutationResult::Error {
                    error: e,
                    revision: Some(current),
                };
            }
        };
        let changed = CasMaterial::from(&state) != self.current_material();
        if !changed {
            drop(access);
            return if let Some((pool_id, node_id)) = manual_selection {
                self.finish_manual_selection(pool_id, node_id, state, current)
            } else {
                match self.build_snapshot(&state, current) {
                    Ok(snapshot) => MutationResult::Ok {
                        outcome,
                        snapshot: Box::new(snapshot),
                    },
                    Err(e) => MutationResult::Error {
                        error: map_query_error(e),
                        revision: Some(current),
                    },
                }
            };
        }
        if !manual {
            state.active_configuration_generation = match state
                .active_configuration_generation
                .checked_add(1)
                .filter(|v| *v <= MAX_SAFE_INTEGER)
            {
                Some(v) => v,
                None => {
                    return MutationResult::Error {
                        error: MutationError::StateUnavailable,
                        revision: Some(current),
                    };
                }
            };
        }
        if current == MAX_SAFE_INTEGER {
            return MutationResult::Error {
                error: MutationError::StateUnavailable,
                revision: Some(current),
            };
        }
        if let Err(error) = state.validate() {
            return MutationResult::Error {
                error: map_state_validation(error),
                revision: Some(current),
            };
        }
        if self.store.save(&state).is_err() {
            return MutationResult::Error {
                error: MutationError::SaveFailed,
                revision: Some(current),
            };
        }
        let revision = match self
            .revision
            .lock()
            .map_err(|_| MutationError::StateUnavailable)
            .and_then(|mut v| v.commit(CasMaterial::from(&state)))
        {
            Ok(v) => v,
            Err(e) => {
                return MutationResult::Error {
                    error: e,
                    revision: Some(current),
                };
            }
        };
        if !manual && let Ok(mut apply_tuple) = self.apply_tuple.lock() {
            *apply_tuple = None;
        }
        drop(access);
        if let Some((pool_id, node_id)) = manual_selection {
            return self.finish_manual_selection(pool_id, node_id, state, revision);
        }
        match self.build_snapshot(&state, revision) {
            Ok(snapshot) => MutationResult::Ok {
                outcome,
                snapshot: Box::new(snapshot),
            },
            Err(e) => MutationResult::Error {
                error: map_query_error(e),
                revision: Some(revision),
            },
        }
    }

    fn apply_configuration(
        &self,
        active_id: String,
        desired_generation: u64,
        revision: u64,
    ) -> MutationResult {
        let Ok(mut tracked_tuple) = self.apply_tuple.lock() else {
            return MutationResult::Error {
                error: MutationError::RuntimeUnavailable,
                revision: Some(revision),
            };
        };
        *tracked_tuple = Some((Some(active_id.clone()), desired_generation));
        drop(tracked_tuple);
        let result = self
            .runtime
            .apply_proxy_routing(active_id, desired_generation);
        let outcome = match result {
            ActivationResult::Activated { operation_id, .. }
            | ActivationResult::Reactivated { operation_id, .. }
            | ActivationResult::AlreadyCurrent { operation_id, .. } => {
                MutationOutcome::ApplyCompleted { operation_id }
            }
            ActivationResult::Pending { operation_id } => {
                MutationOutcome::ApplyStarted { operation_id }
            }
            ActivationResult::Error { error, .. } => {
                return MutationResult::Error {
                    error: map_activation_error(error),
                    revision: Some(revision),
                };
            }
        };
        match self
            .load_observed()
            .and_then(|(state, latest)| self.build_snapshot(&state, latest))
        {
            Ok(snapshot) => MutationResult::Ok {
                outcome,
                snapshot: Box::new(snapshot),
            },
            Err(error) => MutationResult::Error {
                error: map_query_error(error),
                revision: self.known_revision(),
            },
        }
    }

    fn finish_manual_selection(
        &self,
        pool_id: String,
        node_id: String,
        committed_state: AppState,
        committed_revision: u64,
    ) -> MutationResult {
        let runtime_result = self
            .runtime
            .reconcile_manual_selection(PoolId(pool_id.clone()), NodeId(node_id.clone()));
        // Post-save runtime coordination cannot turn the already committed mutation into an
        // error. A successful reload may reveal a newer superseding mutation; otherwise the
        // exact committed state remains the last authoritative state this operation can prove.
        let (state, revision, reload_proved_latest) = match self.load_observed() {
            Ok((state, revision)) => (state, revision, true),
            Err(_) => (committed_state, committed_revision, false),
        };
        let outcome = settle_manual_outcome(
            pool_id,
            node_id,
            runtime_result,
            &state,
            reload_proved_latest,
        );
        match self.build_snapshot(&state, revision) {
            Ok(snapshot) => MutationResult::Ok {
                outcome,
                snapshot: Box::new(snapshot),
            },
            Err(error) => MutationResult::Error {
                error: map_query_error(error),
                revision: Some(revision),
            },
        }
    }

    fn load_observed(&self) -> Result<(AppState, u64), QueryError> {
        let _access = self.gate.try_lock().map_err(map_query_access)?;
        let state = self
            .load_state()
            .map_err(|_| QueryError::StateUnavailable)?;
        let revision = self
            .revision
            .lock()
            .map_err(|_| QueryError::StateUnavailable)?
            .observe(CasMaterial::from(&state))?;
        Ok((state, revision))
    }
    fn load_state(&self) -> Result<AppState, MutationError> {
        if !self
            .store
            .has_snapshot_or_backup()
            .map_err(|_| MutationError::StateUnavailable)?
        {
            return Ok(AppState::empty());
        }
        self.store
            .load()
            .map_err(|_| MutationError::StateUnavailable)
    }
    fn observe(&self, state: &AppState) -> Result<u64, MutationError> {
        self.revision
            .lock()
            .map_err(|_| MutationError::StateUnavailable)?
            .observe(CasMaterial::from(state))
            .map_err(map_query_error)
    }
    fn current_material(&self) -> CasMaterial {
        self.revision
            .lock()
            .expect("routing revision mutex")
            .material
            .clone()
            .expect("observed before mutation")
    }
    fn known_revision(&self) -> Option<u64> {
        self.revision
            .lock()
            .ok()
            .and_then(|v| (v.revision > 0).then_some(v.revision))
    }
    fn mutation_access_error(&self, error: StateAccessError) -> MutationResult {
        MutationResult::Error {
            error: match error {
                StateAccessError::Busy => MutationError::Busy,
                StateAccessError::Unavailable => MutationError::StateUnavailable,
            },
            revision: self.known_revision(),
        }
    }
    fn build_snapshot(
        &self,
        state: &AppState,
        revision: u64,
    ) -> Result<ProxyRoutingSnapshot, QueryError> {
        let observation = self.runtime.runtime_observation_snapshot();
        let observed_runtime_state = match observation.sidecar_lifecycle {
            ObservedSidecarLifecycle::Ready => RuntimeState::Ready,
            ObservedSidecarLifecycle::RecoveryRequired => RuntimeState::RecoveryRequired,
            ObservedSidecarLifecycle::Stopped | ObservedSidecarLifecycle::NotObserved => {
                RuntimeState::Stopped
            }
        };
        let applied_generation = observation.applied_configuration_generation;
        let applied_subscription_id = observation.applied_subscription_id.clone();
        let desired_subscription_id = state.active_subscription_id.as_ref().map(|v| v.0.clone());
        let desired_tuple = (
            desired_subscription_id.clone(),
            state.active_configuration_generation,
        );
        let apply_attempt_is_current = self
            .apply_tuple
            .lock()
            .map(|apply_tuple| apply_tuple.as_ref() == Some(&desired_tuple))
            .unwrap_or(false);
        let routing_terminal = self.runtime.routing_apply_terminal();
        let (runtime_state, apply_state) = canonical_runtime_apply_state(
            &observation,
            observed_runtime_state,
            state,
            &applied_subscription_id,
            apply_attempt_is_current,
            routing_terminal.as_ref(),
        );
        let runtime_nodes = self.runtime.selector_runtime_nodes();
        Ok(ProxyRoutingSnapshot {
            revision,
            desired_generation: state.active_configuration_generation,
            applied_generation,
            runtime_state,
            apply_state,
            active_subscription_id: desired_subscription_id,
            applied_subscription_id,
            providers: state
                .providers
                .iter()
                .map(|v| ProviderSnapshot {
                    id: v.id.0.clone(),
                    subscription_id: v.subscription_id.0.clone(),
                    name: v.name.clone(),
                })
                .collect(),
            nodes: state
                .nodes
                .iter()
                .map(|v| NodeSnapshot {
                    id: v.id.0.clone(),
                    provider_id: v.provider_id.0.clone(),
                    name: v.name.clone(),
                    protocol: v.protocol.into(),
                })
                .collect(),
            default_target: (&state.default_target).into(),
            pools: state
                .pools
                .iter()
                .map(|v| PoolSnapshot {
                    id: v.id.0.clone(),
                    name: v.name.clone(),
                    kind: v.kind.into(),
                    enabled: v.enabled,
                    sources: v.sources.iter().map(Into::into).collect(),
                    selection: (&v.selection).into(),
                    resolved_node_ids: state
                        .resolve_pool_members(v)
                        .into_iter()
                        .map(|id| id.0)
                        .collect(),
                })
                .collect(),
            routes: state.routes.iter().map(Into::into).collect(),
            selectors: selectors(state, &runtime_nodes, runtime_state),
        })
    }
}

fn map_query_access(error: StateAccessError) -> QueryError {
    match error {
        StateAccessError::Busy => QueryError::Busy,
        StateAccessError::Unavailable => QueryError::StateUnavailable,
    }
}
fn map_query_error(error: QueryError) -> MutationError {
    match error {
        QueryError::Busy => MutationError::Busy,
        QueryError::StateUnavailable => MutationError::StateUnavailable,
        QueryError::RuntimeUnavailable => MutationError::RuntimeUnavailable,
    }
}

fn map_state_validation(error: StateValidationError) -> MutationError {
    match error {
        StateValidationError::MissingSubscription
        | StateValidationError::MissingProvider
        | StateValidationError::MissingPool
        | StateValidationError::EmptyPoolSources
        | StateValidationError::InvalidFilter
        | StateValidationError::InvalidSelection
        | StateValidationError::EmptyPoolMembership
        | StateValidationError::InactivePoolTarget => MutationError::ReferenceConflict,
        StateValidationError::InvalidIdentifier
        | StateValidationError::DuplicateIdentifier
        | StateValidationError::InvalidRoute
        | StateValidationError::InvalidProtocolOptions
        | StateValidationError::InvalidSubscription
        | StateValidationError::UnsupportedSchemaVersion => MutationError::ValidationFailed,
    }
}

fn apply_mutation(
    state: &mut AppState,
    mutation: ProxyRoutingMutation,
) -> Result<MutationOutcome, MutationError> {
    match mutation {
        ProxyRoutingMutation::CreateCustomPool {
            name,
            enabled,
            sources,
            selection,
        } => {
            if state.pools.len() >= MAX_POOLS {
                return Err(MutationError::ValidationFailed);
            }
            validate_name(&name)?;
            validate_sources(&sources)?;
            validate_selection(&selection)?;
            let id = new_id("pool")?;
            if state.pools.iter().any(|pool| pool.id.0 == id) {
                return Err(MutationError::StateUnavailable);
            }
            state.pools.push(NodePool {
                id: PoolId(id),
                name: name.trim().into(),
                kind: PoolKind::Custom,
                sources: sources.into_iter().map(Into::into).collect(),
                selection: selection.into(),
                enabled,
            });
        }
        ProxyRoutingMutation::UpdateCustomPool {
            id,
            name,
            enabled,
            sources,
            selection,
        } => {
            validate_reference(&id)?;
            validate_name(&name)?;
            validate_sources(&sources)?;
            validate_selection(&selection)?;
            let pool = state
                .pools
                .iter_mut()
                .find(|v| v.id.0 == id)
                .ok_or(MutationError::NotFound)?;
            if pool.kind != PoolKind::Custom {
                return Err(MutationError::ReferenceConflict);
            }
            pool.name = name.trim().into();
            pool.enabled = enabled;
            pool.sources = sources.into_iter().map(Into::into).collect();
            pool.selection = selection.into();
        }
        ProxyRoutingMutation::DeleteCustomPool { id } => {
            validate_reference(&id)?;
            let index = state
                .pools
                .iter()
                .position(|v| v.id.0 == id)
                .ok_or(MutationError::NotFound)?;
            if state.pools[index].kind != PoolKind::Custom {
                return Err(MutationError::ReferenceConflict);
            }
            if matches!(&state.default_target, RouteTarget::Pool(v) if v.0 == id)
                || state
                    .routes
                    .iter()
                    .any(|v| matches!(&v.target, RouteTarget::Pool(p) if p.0 == id))
            {
                return Err(MutationError::ReferenceConflict);
            }
            state.pools.remove(index);
        }
        ProxyRoutingMutation::SetManualSelection { pool_id, node_id } => {
            validate_reference(&pool_id)?;
            validate_reference(&node_id)?;
            let pool = state
                .pools
                .iter_mut()
                .find(|v| v.id.0 == pool_id)
                .ok_or(MutationError::NotFound)?;
            match &mut pool.selection {
                SelectionPolicy::Manual { selected_node_id } => {
                    *selected_node_id = Some(NodeId(node_id.clone()))
                }
                _ => return Err(MutationError::ReferenceConflict),
            }
            return Ok(MutationOutcome::SelectorSavedOnly {
                pool_id,
                node_id,
                reason: SelectorSavedOnlyReason::RuntimeNotReady,
            });
        }
        ProxyRoutingMutation::SetDefaultTarget { target } => {
            validate_default_target(&target)?;
            state.default_target = target.into();
        }
        ProxyRoutingMutation::CreateRoute {
            name,
            enabled,
            matcher,
            target,
            insert_at,
        } => {
            if state.routes.len() >= MAX_ROUTES || insert_at > state.routes.len() {
                return Err(MutationError::ValidationFailed);
            }
            validate_name(&name)?;
            validate_matcher(&matcher)?;
            validate_route_target(&target)?;
            let id = new_id("route")?;
            if state.routes.iter().any(|route| route.id.0 == id) {
                return Err(MutationError::StateUnavailable);
            }
            state.routes.insert(
                insert_at,
                RoutePolicy {
                    id: RoutePolicyId(id),
                    name: name.trim().into(),
                    enabled,
                    priority: 0,
                    matcher: matcher.into(),
                    target: target.into(),
                },
            );
            normalize_priorities(&mut state.routes)?;
        }
        ProxyRoutingMutation::UpdateRoute {
            id,
            name,
            enabled,
            matcher,
            target,
        } => {
            validate_reference(&id)?;
            validate_name(&name)?;
            validate_matcher(&matcher)?;
            validate_route_target(&target)?;
            let route = state
                .routes
                .iter_mut()
                .find(|v| v.id.0 == id)
                .ok_or(MutationError::NotFound)?;
            route.name = name.trim().into();
            route.enabled = enabled;
            route.matcher = matcher.into();
            route.target = target.into();
        }
        ProxyRoutingMutation::DeleteRoute { id } => {
            validate_reference(&id)?;
            let index = state
                .routes
                .iter()
                .position(|v| v.id.0 == id)
                .ok_or(MutationError::NotFound)?;
            state.routes.remove(index);
            normalize_priorities(&mut state.routes)?;
        }
        ProxyRoutingMutation::ReorderRoutes { route_ids } => {
            if route_ids.len() != state.routes.len()
                || route_ids.iter().collect::<HashSet<_>>().len() != route_ids.len()
                || !route_ids
                    .iter()
                    .all(|id| state.routes.iter().any(|v| &v.id.0 == id))
            {
                return Err(MutationError::ValidationFailed);
            }
            let mut ordered = Vec::with_capacity(state.routes.len());
            for id in route_ids {
                let index = state
                    .routes
                    .iter()
                    .position(|v| v.id.0 == id)
                    .expect("validated route id");
                ordered.push(state.routes.remove(index));
            }
            state.routes = ordered;
            normalize_priorities(&mut state.routes)?;
        }
        ProxyRoutingMutation::ApplyConfiguration => unreachable!(),
    }
    Ok(MutationOutcome::Saved)
}

fn validate_name(value: &str) -> Result<(), MutationError> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > 80 || value.chars().any(char::is_control) {
        Err(MutationError::InvalidInput)
    } else {
        Ok(())
    }
}
fn validate_reference(value: &str) -> Result<(), MutationError> {
    if value.is_empty() || value.trim() != value || value.chars().any(char::is_control) {
        Err(MutationError::InvalidInput)
    } else {
        Ok(())
    }
}
fn validate_texts(values: &[String], max: usize) -> Result<(), MutationError> {
    if values.len() > max
        || values.iter().any(|v| {
            let t = v.trim();
            t.is_empty() || t.chars().count() > 255 || t.chars().any(char::is_control)
        })
        || values
            .iter()
            .map(|value| value.trim())
            .collect::<HashSet<_>>()
            .len()
            != values.len()
    {
        Err(MutationError::InvalidInput)
    } else {
        Ok(())
    }
}
fn validate_references(values: &[String], max: usize) -> Result<(), MutationError> {
    if values.len() > max || values.iter().collect::<HashSet<_>>().len() != values.len() {
        return Err(MutationError::InvalidInput);
    }
    values
        .iter()
        .try_for_each(|value| validate_reference(value))
}
fn validate_sources(values: &[PoolSourceDto]) -> Result<(), MutationError> {
    if values.is_empty()
        || values.len() > MAX_SOURCES
        || values
            .iter()
            .map(|v| &v.provider_id)
            .collect::<HashSet<_>>()
            .len()
            != values.len()
    {
        return Err(MutationError::InvalidInput);
    }
    for v in values {
        validate_reference(&v.provider_id)?;
        validate_filter(&v.filter)?;
    }
    Ok(())
}
fn validate_filter(v: &NodeFilterDto) -> Result<(), MutationError> {
    validate_texts(&v.regions, MAX_FILTER_TEXTS)?;
    validate_texts(&v.include_keywords, MAX_FILTER_TEXTS)?;
    validate_texts(&v.exclude_keywords, MAX_FILTER_TEXTS)?;
    validate_references(&v.include_node_ids, MAX_FILTER_NODE_IDS)?;
    validate_references(&v.exclude_node_ids, MAX_FILTER_NODE_IDS)?;
    if v.protocols.len() > 15
        || v.protocols.iter().collect::<HashSet<_>>().len() != v.protocols.len()
        || v.include_node_ids
            .iter()
            .any(|id| v.exclude_node_ids.contains(id))
    {
        Err(MutationError::InvalidInput)
    } else {
        Ok(())
    }
}
fn validate_selection(v: &SelectionDto) -> Result<(), MutationError> {
    match v {
        SelectionDto::Manual { selected_node_id } => {
            if let Some(id) = selected_node_id {
                validate_reference(id)?;
            }
            Ok(())
        }
        SelectionDto::UrlTest {
            probe_url,
            interval_seconds,
            tolerance_ms,
        } => {
            if !(1..=86400).contains(interval_seconds)
                || *tolerance_ms > 60000
                || probe_url.len() > 2048
                || !valid_probe_url(probe_url)
            {
                Err(MutationError::InvalidInput)
            } else {
                Ok(())
            }
        }
    }
}
fn valid_probe_url(value: &str) -> bool {
    let Ok(url) = reqwest::Url::parse(value) else {
        return false;
    };
    if url.scheme() == "https" {
        return true;
    }
    url.scheme() == "http"
        && url.username().is_empty()
        && url.password().is_none()
        && url.query().is_none()
        && url.fragment().is_none()
        && matches!(url.host_str(), Some("127.0.0.1" | "::1"))
}
fn validate_default_target(v: &DefaultTargetDto) -> Result<(), MutationError> {
    if let DefaultTargetDto::Pool { pool_id } = v {
        validate_reference(pool_id)?;
    }
    Ok(())
}
fn validate_route_target(v: &RouteTargetDto) -> Result<(), MutationError> {
    if let RouteTargetDto::Pool { pool_id } = v {
        validate_reference(pool_id)?;
    }
    Ok(())
}
fn validate_matcher(v: &MatcherDto) -> Result<(), MutationError> {
    match v {
        MatcherDto::Domain(x)
        | MatcherDto::DomainSuffix(x)
        | MatcherDto::Application(x)
        | MatcherDto::IpCidr(x) => {
            if x.is_empty() || x.len() > MAX_MATCHER_VALUES {
                Err(MutationError::InvalidInput)
            } else {
                validate_texts(x, MAX_MATCHER_VALUES)
            }
        }
        MatcherDto::Port(x) => {
            if x.is_empty()
                || x.len() > MAX_MATCHER_VALUES
                || x.contains(&0)
                || x.iter().collect::<HashSet<_>>().len() != x.len()
            {
                Err(MutationError::InvalidInput)
            } else {
                Ok(())
            }
        }
        MatcherDto::Protocol(x) => {
            if x.is_empty()
                || x.len() > MAX_MATCHER_VALUES
                || x.iter().collect::<HashSet<_>>().len() != x.len()
            {
                Err(MutationError::InvalidInput)
            } else {
                Ok(())
            }
        }
    }
}
fn normalize_priorities(routes: &mut [RoutePolicy]) -> Result<(), MutationError> {
    for (index, route) in routes.iter_mut().enumerate() {
        route.priority = i32::try_from(index).map_err(|_| MutationError::ValidationFailed)?;
    }
    Ok(())
}
fn new_id(prefix: &str) -> Result<String, MutationError> {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).map_err(|_| MutationError::StateUnavailable)?;
    Ok(format!(
        "{prefix}-{}",
        bytes.iter().map(|v| format!("{v:02x}")).collect::<String>()
    ))
}

fn map_activation_error(error: ActivationError) -> MutationError {
    match error {
        ActivationError::InvalidInput | ActivationError::IdentityFailed => {
            MutationError::InvalidInput
        }
        ActivationError::Busy => MutationError::Busy,
        ActivationError::NotFound => MutationError::NotFound,
        ActivationError::StateUnavailable => MutationError::StateUnavailable,
        ActivationError::StateChanged => MutationError::StateChanged,
        ActivationError::SelectionConflict => MutationError::Conflict,
        ActivationError::ConfigurationFailed => MutationError::ConfigurationFailed,
        ActivationError::SaveFailed => MutationError::SaveFailed,
        ActivationError::StopFailed => MutationError::StopFailed,
        ActivationError::StartFailed => MutationError::StartFailed,
        ActivationError::RecoveryRequired => MutationError::RecoveryRequired,
    }
}

fn map_manual_result(
    pool_id: String,
    node_id: String,
    result: ManualSelectionRuntimeResult,
) -> MutationOutcome {
    match result {
        ManualSelectionRuntimeResult::Applied => {
            MutationOutcome::SelectorApplied { pool_id, node_id }
        }
        ManualSelectionRuntimeResult::NotApplied { runtime_node_id } => {
            MutationOutcome::SelectorNotApplied {
                pool_id,
                node_id,
                runtime_node_id: runtime_node_id.0,
            }
        }
        ManualSelectionRuntimeResult::SavedOnly(reason) => MutationOutcome::SelectorSavedOnly {
            pool_id,
            node_id,
            reason: match reason {
                ManualSelectionSavedOnlyReason::RuntimeStopped => {
                    SelectorSavedOnlyReason::RuntimeStopped
                }
                ManualSelectionSavedOnlyReason::RuntimeNotReady => {
                    SelectorSavedOnlyReason::RuntimeNotReady
                }
                ManualSelectionSavedOnlyReason::NotInAppliedArtifact => {
                    SelectorSavedOnlyReason::NotInAppliedArtifact
                }
                ManualSelectionSavedOnlyReason::DispatchUnavailable => {
                    SelectorSavedOnlyReason::DispatchUnavailable
                }
            },
        },
        ManualSelectionRuntimeResult::ApplyUnknown(reason) => {
            MutationOutcome::SelectorApplyUnknown {
                pool_id,
                node_id,
                reason: match reason {
                    ManualSelectionUnknownReason::ReadBackUnavailable => {
                        SelectorApplyUnknownReason::ReadBackUnavailable
                    }
                    ManualSelectionUnknownReason::UnknownRuntimeNode => {
                        SelectorApplyUnknownReason::UnknownRuntimeNode
                    }
                    ManualSelectionUnknownReason::InstanceChanged => {
                        SelectorApplyUnknownReason::InstanceChanged
                    }
                },
            }
        }
    }
}

fn settle_manual_outcome(
    pool_id: String,
    node_id: String,
    runtime_result: ManualSelectionRuntimeResult,
    state: &AppState,
    reload_proved_latest: bool,
) -> MutationOutcome {
    let still_desired = state.pools.iter().any(|pool| {
        pool.id.0 == pool_id
            && matches!(&pool.selection, SelectionPolicy::Manual { selected_node_id: Some(id) } if id.0 == node_id)
    });
    if still_desired || !reload_proved_latest {
        map_manual_result(pool_id, node_id, runtime_result)
    } else {
        MutationOutcome::SelectorApplyUnknown {
            pool_id,
            node_id,
            reason: SelectorApplyUnknownReason::Superseded,
        }
    }
}

fn canonical_runtime_apply_state(
    observation: &RuntimeObservationSnapshot,
    observed_runtime: RuntimeState,
    state: &AppState,
    applied_subscription_id: &Option<String>,
    apply_attempt_is_current: bool,
    routing_terminal: Option<&RoutingApplyTerminal>,
) -> (RuntimeState, ApplyState) {
    let desired_subscription_id = state
        .active_subscription_id
        .as_ref()
        .map(|id| id.0.as_str());
    if let Some(terminal) = routing_terminal.filter(|terminal| {
        terminal.active_subscription_id.as_deref() == desired_subscription_id
            && terminal.desired_generation == state.active_configuration_generation
    }) {
        let error = match terminal.failure {
            RoutingApplyTerminalFailure::ConfigurationFailed => ApplyFailure::ConfigurationFailed,
            RoutingApplyTerminalFailure::StateChanged => ApplyFailure::StateChanged,
        };
        return (
            observed_runtime,
            ApplyState::SavedApplyFailed {
                operation_id: terminal.operation_id.clone(),
                error,
            },
        );
    }
    if routing_terminal.is_some_and(|terminal| {
        observation
            .subscription_switch
            .as_ref()
            .is_some_and(|operation| operation.operation_id == terminal.operation_id)
    }) {
        return (observed_runtime, ApplyState::SavedPendingApply);
    }
    if let Some(operation) = &observation.subscription_switch {
        match operation.status {
            SubscriptionSwitchStatus::Queued
            | SubscriptionSwitchStatus::Checking
            | SubscriptionSwitchStatus::Prepared
            | SubscriptionSwitchStatus::Persisted
            | SubscriptionSwitchStatus::Applying => {
                if !apply_attempt_is_current {
                    return (observed_runtime, ApplyState::SavedPendingApply);
                }
                return (
                    RuntimeState::Transitioning,
                    ApplyState::Applying {
                        operation_id: operation.operation_id.clone(),
                    },
                );
            }
            SubscriptionSwitchStatus::Cancelled => {
                if !apply_attempt_is_current {
                    return (observed_runtime, ApplyState::SavedPendingApply);
                }
                return (
                    observed_runtime,
                    ApplyState::SavedApplyFailed {
                        operation_id: operation.operation_id.clone(),
                        error: ApplyFailure::StateChanged,
                    },
                );
            }
            SubscriptionSwitchStatus::Failed => {
                if !apply_attempt_is_current {
                    return (observed_runtime, ApplyState::SavedPendingApply);
                }
                let (runtime, error) = match operation.error_code {
                    Some(
                        SubscriptionSwitchErrorCode::ConfigurationFailed
                        | SubscriptionSwitchErrorCode::SaveFailed,
                    ) => (observed_runtime, ApplyFailure::ConfigurationFailed),
                    Some(SubscriptionSwitchErrorCode::StopFailed) => {
                        (RuntimeState::RecoveryRequired, ApplyFailure::StopFailed)
                    }
                    Some(SubscriptionSwitchErrorCode::StartFailed) => {
                        (RuntimeState::Stopped, ApplyFailure::StartFailed)
                    }
                    Some(SubscriptionSwitchErrorCode::RecoveryRequired) => (
                        RuntimeState::RecoveryRequired,
                        ApplyFailure::RecoveryRequired,
                    ),
                    Some(SubscriptionSwitchErrorCode::OperationTimedOut) => {
                        return (
                            RuntimeState::Transitioning,
                            ApplyState::ApplyUnknown {
                                operation_id: operation.operation_id.clone(),
                            },
                        );
                    }
                    Some(
                        SubscriptionSwitchErrorCode::Cancelled
                        | SubscriptionSwitchErrorCode::Busy
                        | SubscriptionSwitchErrorCode::StateUnavailable,
                    )
                    | None => (observed_runtime, ApplyFailure::StateChanged),
                };
                return (
                    runtime,
                    ApplyState::SavedApplyFailed {
                        operation_id: operation.operation_id.clone(),
                        error,
                    },
                );
            }
            SubscriptionSwitchStatus::Ready => {}
        }
    }
    if observed_runtime == RuntimeState::Ready
        && observation.applied_configuration_generation
            == Some(state.active_configuration_generation)
        && applied_subscription_id.as_deref() == desired_subscription_id
    {
        (RuntimeState::Ready, ApplyState::Applied)
    } else {
        (observed_runtime, ApplyState::SavedPendingApply)
    }
}

fn selectors(
    state: &AppState,
    runtime_nodes: &BTreeMap<PoolId, NodeId>,
    runtime_state: RuntimeState,
) -> Vec<SelectorSnapshot> {
    state
        .pools
        .iter()
        .filter_map(|p| match &p.selection {
            SelectionPolicy::Manual { selected_node_id } => {
                let runtime_node_id = if runtime_state == RuntimeState::Ready {
                    runtime_nodes.get(&p.id)
                } else {
                    None
                };
                let selector_state = match (selected_node_id.as_ref(), runtime_node_id) {
                    (Some(desired), Some(runtime)) if desired == runtime => SelectorState::InSync,
                    (Some(_), Some(_)) => SelectorState::NotApplied,
                    (_, None) if runtime_state == RuntimeState::Ready => SelectorState::Unknown,
                    _ => SelectorState::SavedOnly,
                };
                Some(SelectorSnapshot {
                    pool_id: p.id.0.clone(),
                    desired_node_id: selected_node_id.as_ref().map(|v| v.0.clone()),
                    runtime_node_id: runtime_node_id.map(|v| v.0.clone()),
                    state: selector_state,
                })
            }
            SelectionPolicy::UrlTest { .. } => None,
        })
        .collect()
}

impl From<ProxyProtocol> for ProxyProtocolDto {
    fn from(v: ProxyProtocol) -> Self {
        match v {
            ProxyProtocol::Socks => Self::Socks,
            ProxyProtocol::Http => Self::Http,
            ProxyProtocol::Shadowsocks => Self::Shadowsocks,
            ProxyProtocol::Vmess => Self::Vmess,
            ProxyProtocol::Vless => Self::Vless,
            ProxyProtocol::Trojan => Self::Trojan,
            ProxyProtocol::WireGuard => Self::WireGuard,
            ProxyProtocol::Hysteria => Self::Hysteria,
            ProxyProtocol::Hysteria2 => Self::Hysteria2,
            ProxyProtocol::Tuic => Self::Tuic,
            ProxyProtocol::ShadowTls => Self::ShadowTls,
            ProxyProtocol::Ssh => Self::Ssh,
            ProxyProtocol::Naive => Self::Naive,
            ProxyProtocol::AnyTls => Self::AnyTls,
            ProxyProtocol::Snell => Self::Snell,
        }
    }
}
impl From<ProxyProtocolDto> for ProxyProtocol {
    fn from(v: ProxyProtocolDto) -> Self {
        match v {
            ProxyProtocolDto::Socks => Self::Socks,
            ProxyProtocolDto::Http => Self::Http,
            ProxyProtocolDto::Shadowsocks => Self::Shadowsocks,
            ProxyProtocolDto::Vmess => Self::Vmess,
            ProxyProtocolDto::Vless => Self::Vless,
            ProxyProtocolDto::Trojan => Self::Trojan,
            ProxyProtocolDto::WireGuard => Self::WireGuard,
            ProxyProtocolDto::Hysteria => Self::Hysteria,
            ProxyProtocolDto::Hysteria2 => Self::Hysteria2,
            ProxyProtocolDto::Tuic => Self::Tuic,
            ProxyProtocolDto::ShadowTls => Self::ShadowTls,
            ProxyProtocolDto::Ssh => Self::Ssh,
            ProxyProtocolDto::Naive => Self::Naive,
            ProxyProtocolDto::AnyTls => Self::AnyTls,
            ProxyProtocolDto::Snell => Self::Snell,
        }
    }
}
impl From<&NodeFilter> for NodeFilterDto {
    fn from(v: &NodeFilter) -> Self {
        Self {
            regions: v.regions.clone(),
            protocols: v.protocols.iter().copied().map(Into::into).collect(),
            include_keywords: v.include_keywords.clone(),
            exclude_keywords: v.exclude_keywords.clone(),
            include_node_ids: v.include_node_ids.iter().map(|x| x.0.clone()).collect(),
            exclude_node_ids: v.exclude_node_ids.iter().map(|x| x.0.clone()).collect(),
        }
    }
}
impl From<NodeFilterDto> for NodeFilter {
    fn from(v: NodeFilterDto) -> Self {
        Self {
            regions: trim_values(v.regions),
            protocols: v.protocols.into_iter().map(Into::into).collect(),
            include_keywords: trim_values(v.include_keywords),
            exclude_keywords: trim_values(v.exclude_keywords),
            include_node_ids: v.include_node_ids.into_iter().map(NodeId).collect(),
            exclude_node_ids: v.exclude_node_ids.into_iter().map(NodeId).collect(),
        }
    }
}
impl From<&PoolSource> for PoolSourceDto {
    fn from(v: &PoolSource) -> Self {
        Self {
            provider_id: v.provider_id.0.clone(),
            filter: (&v.filter).into(),
        }
    }
}
impl From<PoolSourceDto> for PoolSource {
    fn from(v: PoolSourceDto) -> Self {
        Self {
            provider_id: ProviderId(v.provider_id),
            filter: v.filter.into(),
        }
    }
}
impl From<&SelectionPolicy> for SelectionDto {
    fn from(v: &SelectionPolicy) -> Self {
        match v {
            SelectionPolicy::Manual { selected_node_id } => Self::Manual {
                selected_node_id: selected_node_id.as_ref().map(|x| x.0.clone()),
            },
            SelectionPolicy::UrlTest {
                probe_url,
                interval_secs,
                tolerance_ms,
            } => Self::UrlTest {
                probe_url: probe_url.clone(),
                interval_seconds: *interval_secs,
                tolerance_ms: *tolerance_ms,
            },
        }
    }
}
impl From<SelectionDto> for SelectionPolicy {
    fn from(v: SelectionDto) -> Self {
        match v {
            SelectionDto::Manual { selected_node_id } => Self::Manual {
                selected_node_id: selected_node_id.map(NodeId),
            },
            SelectionDto::UrlTest {
                probe_url,
                interval_seconds,
                tolerance_ms,
            } => Self::UrlTest {
                probe_url,
                interval_secs: interval_seconds,
                tolerance_ms,
            },
        }
    }
}
impl From<PoolKind> for PoolKindDto {
    fn from(v: PoolKind) -> Self {
        match v {
            PoolKind::ImplicitProvider => Self::ImplicitProvider,
            PoolKind::Custom => Self::Custom,
        }
    }
}
impl From<&RouteTarget> for DefaultTargetDto {
    fn from(v: &RouteTarget) -> Self {
        match v {
            RouteTarget::Unconfigured => Self::FollowActiveSubscription,
            RouteTarget::Pool(id) => Self::Pool {
                pool_id: id.0.clone(),
            },
            RouteTarget::Direct => Self::Direct,
            RouteTarget::Block => Self::Block,
        }
    }
}
impl From<DefaultTargetDto> for RouteTarget {
    fn from(v: DefaultTargetDto) -> Self {
        match v {
            DefaultTargetDto::FollowActiveSubscription => Self::Unconfigured,
            DefaultTargetDto::Pool { pool_id } => Self::Pool(PoolId(pool_id)),
            DefaultTargetDto::Direct => Self::Direct,
            DefaultTargetDto::Block => Self::Block,
        }
    }
}
impl From<&RouteTarget> for RouteTargetDto {
    fn from(v: &RouteTarget) -> Self {
        match v {
            RouteTarget::Pool(id) => Self::Pool {
                pool_id: id.0.clone(),
            },
            RouteTarget::Direct => Self::Direct,
            RouteTarget::Block => Self::Block,
            RouteTarget::Unconfigured => unreachable!("validated routes cannot be unconfigured"),
        }
    }
}
impl From<RouteTargetDto> for RouteTarget {
    fn from(v: RouteTargetDto) -> Self {
        match v {
            RouteTargetDto::Pool { pool_id } => Self::Pool(PoolId(pool_id)),
            RouteTargetDto::Direct => Self::Direct,
            RouteTargetDto::Block => Self::Block,
        }
    }
}
impl From<&TrafficMatcher> for MatcherDto {
    fn from(v: &TrafficMatcher) -> Self {
        match v {
            TrafficMatcher::Domain(x) => Self::Domain(x.clone()),
            TrafficMatcher::DomainSuffix(x) => Self::DomainSuffix(x.clone()),
            TrafficMatcher::Application(x) => Self::Application(x.clone()),
            TrafficMatcher::IpCidr(x) => Self::IpCidr(x.clone()),
            TrafficMatcher::Port(x) => Self::Port(x.clone()),
            TrafficMatcher::Protocol(x) => Self::Protocol(
                x.iter()
                    .map(|v| match v {
                        NetworkProtocol::Tcp => NetworkProtocolDto::Tcp,
                        NetworkProtocol::Udp => NetworkProtocolDto::Udp,
                    })
                    .collect(),
            ),
        }
    }
}
impl From<MatcherDto> for TrafficMatcher {
    fn from(v: MatcherDto) -> Self {
        match v {
            MatcherDto::Domain(x) => Self::Domain(trim_values(x)),
            MatcherDto::DomainSuffix(x) => Self::DomainSuffix(trim_values(x)),
            MatcherDto::Application(x) => Self::Application(trim_values(x)),
            MatcherDto::IpCidr(x) => Self::IpCidr(trim_values(x)),
            MatcherDto::Port(x) => Self::Port(x),
            MatcherDto::Protocol(x) => Self::Protocol(
                x.into_iter()
                    .map(|v| match v {
                        NetworkProtocolDto::Tcp => NetworkProtocol::Tcp,
                        NetworkProtocolDto::Udp => NetworkProtocol::Udp,
                    })
                    .collect(),
            ),
        }
    }
}
fn trim_values(values: Vec<String>) -> Vec<String> {
    values
        .into_iter()
        .map(|value| value.trim().to_owned())
        .collect()
}
impl From<&RoutePolicy> for RouteSnapshot {
    fn from(v: &RoutePolicy) -> Self {
        Self {
            id: v.id.0.clone(),
            name: v.name.clone(),
            enabled: v.enabled,
            priority: v.priority,
            matcher: (&v.matcher).into(),
            target: (&v.target).into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::observability::{
        InMemoryRuntimeObservations, RuntimeObservationPort, SubscriptionSwitch,
    };
    use crate::application::subscription_management::SubscriptionManager;
    use crate::domain::{
        Provider, Subscription, SubscriptionId, SubscriptionSource, SubscriptionUpdatePolicy,
    };
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn manager_fixture(name: &str, state: &AppState) -> (PathBuf, ProxyRoutingManager) {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("veyra-routing-{name}-{nonce}"));
        fs::create_dir_all(&root).unwrap();
        let state_file = root.join("state.json");
        JsonStateStore::new(state_file.clone())
            .unwrap()
            .save(state)
            .unwrap();
        let gate = StateAccessGate::default();
        let observations = InMemoryRuntimeObservations::new_mock();
        let subscriptions =
            Arc::new(SubscriptionManager::new(state_file.clone(), gate.clone()).unwrap());
        let runtime = Arc::new(ManagedObservationRuntimeController::new(
            root.join("resources"),
            root.clone(),
            observations,
            gate.clone(),
            subscriptions,
            None,
        ));
        let manager = ProxyRoutingManager::new(state_file, gate, runtime).unwrap();
        (root, manager)
    }

    fn state_with_manual_pool() -> (AppState, String) {
        let mut state = AppState::empty();
        state.subscriptions.push(Subscription {
            id: SubscriptionId("subscription".into()),
            name: "Fixture".into(),
            description: String::new(),
            source: SubscriptionSource::Manual,
            document: None,
            skipped_unsupported_nodes: 0,
            last_success_at_ms: None,
            last_attempt_at_ms: None,
            http_metadata: None,
            remote_request: None,
            update_policy: SubscriptionUpdatePolicy::manual(),
        });
        state.providers.push(Provider {
            id: ProviderId("provider".into()),
            subscription_id: SubscriptionId("subscription".into()),
            name: "Provider".into(),
        });
        state.nodes = crate::subscription::normalize_nodes(
            ProviderId("provider".into()),
            crate::subscription::parse_subscription(
                r#"{"outbounds":[{"type":"socks","tag":"node","server":"127.0.0.1","server_port":1080}]}"#,
            ).unwrap().nodes,
        ).unwrap();
        let node_id = state.nodes[0].id.0.clone();
        state.pools.push(NodePool {
            id: PoolId("pool".into()),
            name: "Pool".into(),
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
        state.validate().unwrap();
        (state, node_id)
    }
    #[test]
    fn request_is_closed_and_exact() {
        let valid =
            serde_json::json!({"expectedRevision":1,"mutation":{"type":"applyConfiguration"}});
        assert!(serde_json::from_value::<MutateProxyRoutingRequest>(valid.clone()).is_ok());
        let mut extra = valid;
        extra
            .as_object_mut()
            .unwrap()
            .insert("payload".into(), serde_json::json!({}));
        assert!(serde_json::from_value::<MutateProxyRoutingRequest>(extra).is_err());
        assert!(serde_json::from_value::<MutateProxyRoutingRequest>(serde_json::json!({"expectedRevision":1,"mutation":{"type":"anything","payload":{}}})).is_err());
    }
    #[test]
    fn response_has_frozen_exact_snapshot_keys() {
        let snapshot = ProxyRoutingSnapshot {
            revision: 1,
            desired_generation: 0,
            applied_generation: None,
            runtime_state: RuntimeState::Stopped,
            apply_state: ApplyState::SavedPendingApply,
            active_subscription_id: None,
            applied_subscription_id: None,
            providers: vec![],
            nodes: vec![],
            default_target: DefaultTargetDto::FollowActiveSubscription,
            pools: vec![],
            routes: vec![],
            selectors: vec![],
        };
        let value = serde_json::to_value(&snapshot).unwrap();
        let keys = value
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect::<HashSet<_>>();
        assert_eq!(
            keys,
            [
                "revision",
                "desiredGeneration",
                "appliedGeneration",
                "runtimeState",
                "applyState",
                "activeSubscriptionId",
                "appliedSubscriptionId",
                "providers",
                "nodes",
                "defaultTarget",
                "pools",
                "routes",
                "selectors"
            ]
            .map(String::from)
            .into_iter()
            .collect()
        );
        assert_eq!(
            serde_json::to_value(MutationResult::Ok {
                outcome: MutationOutcome::Saved,
                snapshot: Box::new(snapshot),
            })
            .unwrap(),
            serde_json::json!({
                "status":"ok",
                "outcome":{"type":"saved"},
                "snapshot":value,
            })
        );
        assert_eq!(
            serde_json::to_value(MutationResult::Error {
                error: MutationError::Conflict,
                revision: Some(9),
            })
            .unwrap(),
            serde_json::json!({"status":"error","error":"conflict","revision":9})
        );
        assert_eq!(
            serde_json::to_value(MutationOutcome::SelectorSavedOnly {
                pool_id: "pool".into(),
                node_id: "node".into(),
                reason: SelectorSavedOnlyReason::DispatchUnavailable,
            })
            .unwrap(),
            serde_json::json!({
                "type":"selectorSavedOnly",
                "poolId":"pool",
                "nodeId":"node",
                "reason":"dispatchUnavailable"
            })
        );
        assert_eq!(
            serde_json::to_value(SnapshotResult::Error {
                error: QueryError::RuntimeUnavailable,
            })
            .unwrap(),
            serde_json::json!({"status":"error","error":"runtimeUnavailable"})
        );
    }

    #[test]
    fn frozen_validation_boundaries_accept_limits_and_reject_plus_or_minus_one() {
        assert_eq!(validate_name(&"n".repeat(80)), Ok(()));
        assert_eq!(
            validate_name(&"n".repeat(81)),
            Err(MutationError::InvalidInput)
        );
        assert_eq!(validate_texts(&["x".repeat(255)], 1), Ok(()));
        assert_eq!(
            validate_texts(&["x".repeat(256)], 1),
            Err(MutationError::InvalidInput)
        );

        for (interval_seconds, tolerance_ms, expected) in [
            (0, 0, Err(MutationError::InvalidInput)),
            (1, 0, Ok(())),
            (86_400, 60_000, Ok(())),
            (86_401, 60_000, Err(MutationError::InvalidInput)),
            (1, 60_001, Err(MutationError::InvalidInput)),
        ] {
            assert_eq!(
                validate_selection(&SelectionDto::UrlTest {
                    probe_url: "https://example.com/generate_204".into(),
                    interval_seconds,
                    tolerance_ms,
                }),
                expected
            );
        }

        assert_eq!(validate_matcher(&MatcherDto::Port(vec![1])), Ok(()));
        assert_eq!(
            validate_matcher(&MatcherDto::Port(vec![0])),
            Err(MutationError::InvalidInput)
        );
        assert_eq!(validate_matcher(&MatcherDto::Port(vec![u16::MAX])), Ok(()));

        let references = (0..=MAX_FILTER_NODE_IDS)
            .map(|index| format!("node-{index}"))
            .collect::<Vec<_>>();
        assert_eq!(
            validate_references(&references[..MAX_FILTER_NODE_IDS], MAX_FILTER_NODE_IDS),
            Ok(())
        );
        assert_eq!(
            validate_references(&references, MAX_FILTER_NODE_IDS),
            Err(MutationError::InvalidInput)
        );

        let matcher_values = (0..=MAX_MATCHER_VALUES)
            .map(|index| format!("domain-{index}.example"))
            .collect::<Vec<_>>();
        assert_eq!(
            validate_matcher(&MatcherDto::Domain(
                matcher_values[..MAX_MATCHER_VALUES].to_vec()
            )),
            Ok(())
        );
        assert_eq!(
            validate_matcher(&MatcherDto::Domain(matcher_values)),
            Err(MutationError::InvalidInput)
        );

        let filter_texts = (0..=MAX_FILTER_TEXTS)
            .map(|index| format!("region-{index}"))
            .collect::<Vec<_>>();
        assert_eq!(
            validate_texts(&filter_texts[..MAX_FILTER_TEXTS], MAX_FILTER_TEXTS),
            Ok(())
        );
        assert_eq!(
            validate_texts(&filter_texts, MAX_FILTER_TEXTS),
            Err(MutationError::InvalidInput)
        );

        let sources = (0..=MAX_SOURCES)
            .map(|index| PoolSourceDto {
                provider_id: format!("provider-{index}"),
                filter: NodeFilterDto {
                    regions: vec![],
                    protocols: vec![],
                    include_keywords: vec![],
                    exclude_keywords: vec![],
                    include_node_ids: vec![],
                    exclude_node_ids: vec![],
                },
            })
            .collect::<Vec<_>>();
        assert_eq!(validate_sources(&sources[..MAX_SOURCES]), Ok(()));
        assert_eq!(validate_sources(&sources), Err(MutationError::InvalidInput));
    }
    #[test]
    fn manual_selection_only_changes_revision_material_not_generation() {
        let mut state = AppState::empty();
        state.pools.push(NodePool {
            id: PoolId("pool".into()),
            name: "Pool".into(),
            kind: PoolKind::Custom,
            sources: vec![],
            selection: SelectionPolicy::Manual {
                selected_node_id: None,
            },
            enabled: false,
        });
        let generation = state.active_configuration_generation;
        let result = apply_mutation(
            &mut state,
            ProxyRoutingMutation::SetManualSelection {
                pool_id: "pool".into(),
                node_id: "node".into(),
            },
        )
        .unwrap();
        assert_eq!(state.active_configuration_generation, generation);
        assert!(matches!(result, MutationOutcome::SelectorSavedOnly { .. }));
    }
    #[test]
    fn duplicate_or_incomplete_reorder_is_rejected() {
        let mut state = AppState::empty();
        state.routes = ["a", "b"]
            .into_iter()
            .enumerate()
            .map(|(i, id)| RoutePolicy {
                id: RoutePolicyId(id.into()),
                name: id.into(),
                enabled: false,
                priority: i as i32,
                matcher: TrafficMatcher::Domain(vec!["example.com".into()]),
                target: RouteTarget::Direct,
            })
            .collect();
        assert_eq!(
            apply_mutation(
                &mut state,
                ProxyRoutingMutation::ReorderRoutes {
                    route_ids: vec!["a".into(), "a".into()]
                }
            ),
            Err(MutationError::ValidationFailed)
        );
    }

    #[test]
    fn revision_observe_bumps_only_when_cas_material_changes() {
        let mut revision = RevisionState::default();
        let mut state = AppState::empty();
        assert_eq!(revision.observe(CasMaterial::from(&state)), Ok(1));
        assert_eq!(revision.observe(CasMaterial::from(&state)), Ok(1));
        state.active_configuration_generation = 1;
        assert_eq!(revision.observe(CasMaterial::from(&state)), Ok(2));
    }

    #[test]
    fn apply_tuple_drift_maps_to_the_frozen_state_changed_error() {
        assert_eq!(
            map_activation_error(ActivationError::StateChanged),
            MutationError::StateChanged
        );
    }

    #[test]
    fn canonical_apply_mapping_owns_in_progress_and_terminal_runtime_truth() {
        let observations = InMemoryRuntimeObservations::new_mock();
        let mut snapshot = observations.snapshot();
        let mut state = AppState::empty();
        state.active_configuration_generation = 2;
        state.active_subscription_id = Some(SubscriptionId("sub".into()));

        snapshot.applied_subscription_id = Some("sub".into());
        snapshot.applied_configuration_generation = Some(2);
        assert_eq!(
            canonical_runtime_apply_state(
                &snapshot,
                RuntimeState::Ready,
                &state,
                &Some("sub".into()),
                false,
                None,
            ),
            (RuntimeState::Ready, ApplyState::Applied)
        );
        snapshot.applied_configuration_generation = Some(1);
        assert_eq!(
            canonical_runtime_apply_state(
                &snapshot,
                RuntimeState::Ready,
                &state,
                &Some("sub".into()),
                false,
                None,
            ),
            (RuntimeState::Ready, ApplyState::SavedPendingApply)
        );
        assert_eq!(
            canonical_runtime_apply_state(
                &snapshot,
                RuntimeState::Stopped,
                &state,
                &Some("sub".into()),
                false,
                None,
            ),
            (RuntimeState::Stopped, ApplyState::SavedPendingApply)
        );

        snapshot.subscription_switch = Some(SubscriptionSwitch {
            operation_id: "op".into(),
            status: SubscriptionSwitchStatus::Checking,
            error_code: None,
        });
        assert_eq!(
            canonical_runtime_apply_state(
                &snapshot,
                RuntimeState::Ready,
                &state,
                &Some("sub".into()),
                true,
                None,
            ),
            (
                RuntimeState::Transitioning,
                ApplyState::Applying {
                    operation_id: "op".into()
                }
            )
        );

        for (error_code, expected_runtime, expected_failure) in [
            (
                SubscriptionSwitchErrorCode::StopFailed,
                RuntimeState::RecoveryRequired,
                ApplyFailure::StopFailed,
            ),
            (
                SubscriptionSwitchErrorCode::StartFailed,
                RuntimeState::Stopped,
                ApplyFailure::StartFailed,
            ),
            (
                SubscriptionSwitchErrorCode::RecoveryRequired,
                RuntimeState::RecoveryRequired,
                ApplyFailure::RecoveryRequired,
            ),
        ] {
            snapshot.subscription_switch = Some(SubscriptionSwitch {
                operation_id: "op".into(),
                status: SubscriptionSwitchStatus::Failed,
                error_code: Some(error_code),
            });
            assert_eq!(
                canonical_runtime_apply_state(
                    &snapshot,
                    RuntimeState::Ready,
                    &state,
                    &Some("sub".into()),
                    true,
                    None,
                ),
                (
                    expected_runtime,
                    ApplyState::SavedApplyFailed {
                        operation_id: "op".into(),
                        error: expected_failure,
                    }
                )
            );
        }

        snapshot.subscription_switch = Some(SubscriptionSwitch {
            operation_id: "op".into(),
            status: SubscriptionSwitchStatus::Failed,
            error_code: Some(SubscriptionSwitchErrorCode::OperationTimedOut),
        });
        assert_eq!(
            canonical_runtime_apply_state(
                &snapshot,
                RuntimeState::Ready,
                &state,
                &Some("sub".into()),
                true,
                None,
            ),
            (
                RuntimeState::Transitioning,
                ApplyState::ApplyUnknown {
                    operation_id: "op".into()
                }
            )
        );

        for (failure, expected) in [
            (
                RoutingApplyTerminalFailure::ConfigurationFailed,
                ApplyFailure::ConfigurationFailed,
            ),
            (
                RoutingApplyTerminalFailure::StateChanged,
                ApplyFailure::StateChanged,
            ),
        ] {
            let terminal = RoutingApplyTerminal {
                operation_id: "terminal".into(),
                active_subscription_id: Some("sub".into()),
                desired_generation: 2,
                failure,
            };
            assert_eq!(
                canonical_runtime_apply_state(
                    &snapshot,
                    RuntimeState::Ready,
                    &state,
                    &Some("sub".into()),
                    false,
                    Some(&terminal),
                ),
                (
                    RuntimeState::Ready,
                    ApplyState::SavedApplyFailed {
                        operation_id: "terminal".into(),
                        error: expected,
                    }
                )
            );
        }

        let stale_terminal = RoutingApplyTerminal {
            operation_id: "op".into(),
            active_subscription_id: Some("sub".into()),
            desired_generation: 1,
            failure: RoutingApplyTerminalFailure::ConfigurationFailed,
        };
        assert_eq!(
            canonical_runtime_apply_state(
                &snapshot,
                RuntimeState::Ready,
                &state,
                &Some("sub".into()),
                true,
                Some(&stale_terminal),
            ),
            (RuntimeState::Ready, ApplyState::SavedPendingApply)
        );

        assert_eq!(
            canonical_runtime_apply_state(
                &snapshot,
                RuntimeState::Ready,
                &state,
                &Some("sub".into()),
                false,
                None,
            ),
            (RuntimeState::Ready, ApplyState::SavedPendingApply)
        );
    }

    #[test]
    fn nested_mutation_objects_reject_unknown_and_null_fields() {
        assert!(serde_json::from_value::<MutateProxyRoutingRequest>(serde_json::json!({
            "expectedRevision": 1,
            "mutation": {
                "type": "createCustomPool", "name": "pool", "enabled": true,
                "sources": [{"providerId":"provider","filter":{"regions":[],"protocols":[],"includeKeywords":[],"excludeKeywords":[],"includeNodeIds":[],"excludeNodeIds":[],"extra":false}}],
                "selection": {"type":"manual","selectedNodeId":null}
            }
        })).is_err());
        assert!(
            serde_json::from_value::<MutateProxyRoutingRequest>(serde_json::json!({
                "expectedRevision": 1,
                "mutation": {"type":"setManualSelection","poolId":"pool","nodeId":null}
            }))
            .is_err()
        );
    }

    #[test]
    fn post_save_manual_result_only_uses_superseded_when_a_reload_proves_drift() {
        let mut state = AppState::empty();
        state.pools.push(NodePool {
            id: PoolId("pool".into()),
            name: "Pool".into(),
            kind: PoolKind::Custom,
            sources: vec![],
            selection: SelectionPolicy::Manual {
                selected_node_id: Some(NodeId("node".into())),
            },
            enabled: false,
        });
        let fallback = settle_manual_outcome(
            "pool".into(),
            "node".into(),
            ManualSelectionRuntimeResult::SavedOnly(
                ManualSelectionSavedOnlyReason::DispatchUnavailable,
            ),
            &state,
            false,
        );
        assert!(matches!(
            fallback,
            MutationOutcome::SelectorSavedOnly {
                reason: SelectorSavedOnlyReason::DispatchUnavailable,
                ..
            }
        ));

        if let SelectionPolicy::Manual { selected_node_id } = &mut state.pools[0].selection {
            *selected_node_id = Some(NodeId("newer".into()));
        }
        assert!(matches!(
            settle_manual_outcome(
                "pool".into(),
                "node".into(),
                ManualSelectionRuntimeResult::Applied,
                &state,
                true,
            ),
            MutationOutcome::SelectorApplyUnknown {
                reason: SelectorApplyUnknownReason::Superseded,
                ..
            }
        ));
    }

    #[test]
    fn manager_structural_save_and_noop_obey_generation_revision_and_cas() {
        let (root, manager) = manager_fixture("structural", &AppState::empty());
        let SnapshotResult::Ok { snapshot } = manager.snapshot() else {
            panic!("snapshot")
        };
        assert_eq!(snapshot.revision, 1);
        let saved = manager.mutate(MutateProxyRoutingRequest {
            expected_revision: 1,
            mutation: ProxyRoutingMutation::SetDefaultTarget {
                target: DefaultTargetDto::Direct,
            },
        });
        let MutationResult::Ok { snapshot, .. } = saved else {
            panic!("saved")
        };
        assert_eq!((snapshot.revision, snapshot.desired_generation), (2, 1));
        let noop = manager.mutate(MutateProxyRoutingRequest {
            expected_revision: 2,
            mutation: ProxyRoutingMutation::SetDefaultTarget {
                target: DefaultTargetDto::Direct,
            },
        });
        let MutationResult::Ok { snapshot, .. } = noop else {
            panic!("noop")
        };
        assert_eq!((snapshot.revision, snapshot.desired_generation), (2, 1));
        assert!(matches!(
            manager.mutate(MutateProxyRoutingRequest {
                expected_revision: 1,
                mutation: ProxyRoutingMutation::SetDefaultTarget {
                    target: DefaultTargetDto::Block
                },
            }),
            MutationResult::Error {
                error: MutationError::Conflict,
                revision: Some(2)
            }
        ));
        drop(manager);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn manager_manual_save_advances_only_revision_and_returns_authoritative_snapshot() {
        let (state, node_id) = state_with_manual_pool();
        let (root, manager) = manager_fixture("manual", &state);
        let _ = manager.snapshot();
        let result = manager.mutate(MutateProxyRoutingRequest {
            expected_revision: 1,
            mutation: ProxyRoutingMutation::SetManualSelection {
                pool_id: "pool".into(),
                node_id: node_id.clone(),
            },
        });
        let MutationResult::Ok { outcome, snapshot } = result else {
            panic!("manual saved")
        };
        assert_eq!((snapshot.revision, snapshot.desired_generation), (2, 0));
        assert!(matches!(
            outcome,
            MutationOutcome::SelectorSavedOnly {
                reason: SelectorSavedOnlyReason::RuntimeStopped,
                ..
            }
        ));
        assert_eq!(
            snapshot.selectors[0].desired_node_id.as_deref(),
            Some(node_id.as_str())
        );
        drop(manager);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn manager_rejects_boundary_values_without_advancing_revision() {
        let (root, manager) = manager_fixture("boundaries", &AppState::empty());
        let _ = manager.snapshot();
        for expected_revision in [0, MAX_SAFE_INTEGER + 1] {
            assert!(matches!(
                manager.mutate(MutateProxyRoutingRequest {
                    expected_revision,
                    mutation: ProxyRoutingMutation::SetDefaultTarget {
                        target: DefaultTargetDto::Direct
                    },
                }),
                MutationResult::Error {
                    error: MutationError::InvalidInput,
                    revision: Some(1)
                }
            ));
        }
        assert!(matches!(
            manager.mutate(MutateProxyRoutingRequest {
                expected_revision: 1,
                mutation: ProxyRoutingMutation::CreateRoute {
                    name: "Route".into(),
                    enabled: true,
                    matcher: MatcherDto::Domain(vec!["example.com".into()]),
                    target: RouteTargetDto::Direct,
                    insert_at: 1,
                },
            }),
            MutationResult::Error {
                error: MutationError::ValidationFailed,
                revision: Some(1)
            }
        ));
        drop(manager);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn manager_covers_closed_structural_mutations_with_persisted_snapshots() {
        let (state, node_id) = state_with_manual_pool();
        let (root, manager) = manager_fixture("all-mutations", &state);
        let _ = manager.snapshot();
        let source = PoolSourceDto {
            provider_id: "provider".into(),
            filter: NodeFilterDto {
                regions: vec![],
                protocols: vec![],
                include_keywords: vec![],
                exclude_keywords: vec![],
                include_node_ids: vec![],
                exclude_node_ids: vec![],
            },
        };
        let saved = manager.mutate(MutateProxyRoutingRequest {
            expected_revision: 1,
            mutation: ProxyRoutingMutation::CreateCustomPool {
                name: "Second".into(),
                enabled: false,
                sources: vec![source.clone()],
                selection: SelectionDto::Manual {
                    selected_node_id: None,
                },
            },
        });
        let MutationResult::Ok { snapshot, .. } = saved else {
            panic!("create pool")
        };
        let second_pool = snapshot
            .pools
            .iter()
            .find(|pool| pool.name == "Second")
            .unwrap()
            .id
            .clone();
        assert_eq!((snapshot.revision, snapshot.desired_generation), (2, 1));

        let updated = manager.mutate(MutateProxyRoutingRequest {
            expected_revision: 2,
            mutation: ProxyRoutingMutation::UpdateCustomPool {
                id: second_pool.clone(),
                name: "Updated".into(),
                enabled: false,
                sources: vec![source],
                selection: SelectionDto::UrlTest {
                    probe_url: "https://example.com/generate_204".into(),
                    interval_seconds: 60,
                    tolerance_ms: 0,
                },
            },
        });
        assert!(
            matches!(updated, MutationResult::Ok { ref snapshot, .. } if (snapshot.revision, snapshot.desired_generation) == (3, 2))
        );
        assert!(matches!(manager.mutate(MutateProxyRoutingRequest {
            expected_revision: 3,
            mutation: ProxyRoutingMutation::DeleteCustomPool { id: second_pool },
        }), MutationResult::Ok { ref snapshot, .. } if (snapshot.revision, snapshot.desired_generation) == (4, 3)));

        let manual = manager.mutate(MutateProxyRoutingRequest {
            expected_revision: 4,
            mutation: ProxyRoutingMutation::SetManualSelection {
                pool_id: "pool".into(),
                node_id,
            },
        });
        assert!(
            matches!(manual, MutationResult::Ok { ref snapshot, .. } if (snapshot.revision, snapshot.desired_generation) == (5, 3))
        );
        assert!(matches!(manager.mutate(MutateProxyRoutingRequest {
            expected_revision: 5,
            mutation: ProxyRoutingMutation::SetDefaultTarget { target: DefaultTargetDto::Direct },
        }), MutationResult::Ok { ref snapshot, .. } if (snapshot.revision, snapshot.desired_generation) == (6, 4)));

        let first = manager.mutate(MutateProxyRoutingRequest {
            expected_revision: 6,
            mutation: ProxyRoutingMutation::CreateRoute {
                name: "First".into(),
                enabled: true,
                matcher: MatcherDto::Domain(vec!["example.com".into()]),
                target: RouteTargetDto::Direct,
                insert_at: 0,
            },
        });
        let MutationResult::Ok { snapshot, .. } = first else {
            panic!("create route")
        };
        let first_id = snapshot.routes[0].id.clone();
        assert_eq!((snapshot.revision, snapshot.desired_generation), (7, 5));
        assert!(matches!(manager.mutate(MutateProxyRoutingRequest {
            expected_revision: 7,
            mutation: ProxyRoutingMutation::UpdateRoute {
                id: first_id.clone(), name: "First updated".into(), enabled: false,
                matcher: MatcherDto::Port(vec![443]), target: RouteTargetDto::Block,
            },
        }), MutationResult::Ok { ref snapshot, .. } if (snapshot.revision, snapshot.desired_generation) == (8, 6)));
        let second = manager.mutate(MutateProxyRoutingRequest {
            expected_revision: 8,
            mutation: ProxyRoutingMutation::CreateRoute {
                name: "Second".into(),
                enabled: true,
                matcher: MatcherDto::Protocol(vec![NetworkProtocolDto::Tcp]),
                target: RouteTargetDto::Direct,
                insert_at: 1,
            },
        });
        let MutationResult::Ok { snapshot, .. } = second else {
            panic!("second route")
        };
        let second_id = snapshot.routes[1].id.clone();
        assert!(matches!(manager.mutate(MutateProxyRoutingRequest {
            expected_revision: 9,
            mutation: ProxyRoutingMutation::ReorderRoutes { route_ids: vec![second_id, first_id.clone()] },
        }), MutationResult::Ok { ref snapshot, .. } if snapshot.routes[0].name == "Second" && (snapshot.revision, snapshot.desired_generation) == (10, 8)));
        assert!(matches!(manager.mutate(MutateProxyRoutingRequest {
            expected_revision: 10,
            mutation: ProxyRoutingMutation::DeleteRoute { id: first_id },
        }), MutationResult::Ok { ref snapshot, .. } if snapshot.routes.len() == 1 && (snapshot.revision, snapshot.desired_generation) == (11, 9)));
        let persisted = JsonStateStore::new(root.join("state.json"))
            .unwrap()
            .load()
            .unwrap();
        assert_eq!(persisted.active_configuration_generation, 9);
        assert_eq!(persisted.routes.len(), 1);
        drop(manager);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn manager_save_failure_preserves_bytes_generation_and_revision() {
        let (root, manager) = manager_fixture("save-failure", &AppState::empty());
        let state_file = root.join("state.json");
        let before = fs::read(&state_file).unwrap();
        let _ = manager.snapshot();
        fs::create_dir(root.join("state.json.bak")).unwrap();
        assert!(matches!(
            manager.mutate(MutateProxyRoutingRequest {
                expected_revision: 1,
                mutation: ProxyRoutingMutation::SetDefaultTarget {
                    target: DefaultTargetDto::Direct
                },
            }),
            MutationResult::Error {
                error: MutationError::SaveFailed,
                revision: Some(1)
            }
        ));
        assert_eq!(fs::read(&state_file).unwrap(), before);
        assert!(
            matches!(manager.snapshot(), SnapshotResult::Ok { ref snapshot } if (snapshot.revision, snapshot.desired_generation) == (1, 0))
        );
        drop(manager);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn apply_without_active_subscription_fails_before_runtime_dispatch() {
        let (root, manager) = manager_fixture("apply-preflight", &AppState::empty());
        let _ = manager.snapshot();
        assert!(matches!(
            manager.mutate(MutateProxyRoutingRequest {
                expected_revision: 1,
                mutation: ProxyRoutingMutation::ApplyConfiguration,
            }),
            MutationResult::Error {
                error: MutationError::ValidationFailed,
                revision: Some(1)
            }
        ));
        drop(manager);
        fs::remove_dir_all(root).unwrap();
    }
}
