//! P2-08 正式代理读模型与测速。目录只读查询 seam，选择仍交唯一 Runtime owner。
use super::{
    applied_proxy_facts::AppliedPoolObservation,
    manual_runtime::{GroupChoice, ManualRuntimeSnapshot, ManualSelectionRequest, SelectionChoice},
    network::{NetworkError, NetworkService},
    outbound_query::OutboundQuerySnapshot,
    runtime_snapshot::RuntimeStatus,
    state_service::SnapshotService,
};
use crate::{
    domain::{
        AppError, AppState, GroupSelectionMode, NodeId, NodeSort, OutboundAvailability,
        OutboundEntry, OutboundId, OutboundKind, OutboundResolveError, PoolId,
        ProxyViewPreferences, SnapshotVersion, UiLatencyPreferences,
    },
    storage::latency::{LatencyOutcome, LatencyRecord, LatencyStore, LatencyStoreError},
    subscription::outbound::{ProxySource, RunningProxy},
};
use futures_util::{StreamExt, stream};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::sync::{Semaphore, watch};

pub const MAX_CONCURRENT_TESTS: usize = 4;

#[derive(Debug, thiserror::Error)]
pub enum ProxyError {
    #[error("proxy state unavailable")]
    State(AppError),
    #[error("outbound query failed")]
    Directory(OutboundResolveError),
    #[error("latency history failed")]
    History(LatencyStoreError),
    #[error("proxy snapshot changed")]
    SnapshotChanged,
    #[error("runtime not ready")]
    NotReady,
    #[error("group configuration not applied")]
    ConfigNotApplied,
    #[error("group application unknown")]
    GroupApplicationUnknown,
    #[error("invalid proxy selection")]
    InvalidSelection,
    #[error("unsupported latency target")]
    UnsupportedTarget,
}
impl From<AppError> for ProxyError {
    fn from(value: AppError) -> Self {
        Self::State(value)
    }
}
impl From<OutboundResolveError> for ProxyError {
    fn from(value: OutboundResolveError) -> Self {
        Self::Directory(value)
    }
}
impl From<LatencyStoreError> for ProxyError {
    fn from(value: LatencyStoreError) -> Self {
        Self::History(value)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LatencyBand {
    Low,
    Medium,
    High,
}

#[derive(Clone, Debug)]
pub struct ProxyEntry {
    pub outbound: OutboundEntry,
    /// 仅目标分流的正式策略元数据；普通 NodeGroup 永远为 None。
    pub routing_policy: Option<crate::domain::TargetRoutingPolicy>,
    pub routing_fallback: bool,
    /// 普通组配置按同快照稳定出口 ID 连接；lane 内部选择器不冒充顶层组。
    pub configured_group: Option<crate::domain::NodeGroup>,
    /// 协议来自同一配置快照的 NodeId；不以显示名称连接两套目录。
    pub protocol: Option<crate::domain::ProxyProtocol>,
    /// None 表示当前能力缺少完整应用事实，不从 Saved 目录推断已应用。
    pub applied: Option<bool>,
    /// 来自 owner 固定鉴权观测；Unknown 不按协议推断。
    pub udp: Option<bool>,
    /// 组的已应用直接成员，保留 Compiler 次序；与 outbound.references 的保存意图分开。
    pub applied_members: Option<Vec<OutboundId>>,
    /// 既有主备 lane 内部选择保持 Auto；UI不根据ID字符串猜所属组。
    pub selection_mode: GroupSelectionMode,
    pub current: Option<OutboundId>,
    pub history: Vec<LatencyRecord>,
    pub band: Option<LatencyBand>,
}
#[derive(Clone, Debug)]
pub struct ProxySnapshot {
    pub version: SnapshotVersion,
    pub runtime: ManualRuntimeSnapshot,
    pub latency_preferences: UiLatencyPreferences,
    pub view_preferences: ProxyViewPreferences,
    pub entries: Vec<ProxyEntry>,
}
impl ProxySnapshot {
    /// 组卡片的延迟来自实际当前选择链；组历史中的任意成员不能充当当前出口。
    pub fn selected_latency(&self, id: &OutboundId) -> Option<u64> {
        let mut id = id;
        let mut visited = BTreeSet::new();
        loop {
            if !visited.insert(id) {
                return None;
            }
            let entry = self.entries.iter().find(|entry| &entry.outbound.id == id)?;
            if entry.outbound.kind == OutboundKind::Node {
                return entry.history.iter().find_map(|h| h.outcome.milliseconds());
            }
            id = entry.current.as_ref()?;
        }
    }
    /// 排序、隐藏只是读模型；只隐藏目录明确不可用项，测速失败不推导 alive=false。
    pub fn visible_nodes(&self) -> Vec<&ProxyEntry> {
        let mut nodes = self
            .entries
            .iter()
            .filter(|entry| {
                entry.outbound.kind == OutboundKind::Node
                    && (!self.view_preferences.hide_unavailable
                        || entry.outbound.availability == OutboundAvailability::Available)
            })
            .collect::<Vec<_>>();
        let delay =
            |entry: &ProxyEntry| entry.history.iter().find_map(|h| h.outcome.milliseconds());
        nodes.sort_by(|a, b| match self.view_preferences.node_sort {
            NodeSort::Default => std::cmp::Ordering::Equal,
            NodeSort::NameAsc => a.outbound.display_name.cmp(&b.outbound.display_name),
            NodeSort::NameDesc => b.outbound.display_name.cmp(&a.outbound.display_name),
            NodeSort::LatencyAsc | NodeSort::LatencyDesc => match (delay(a), delay(b)) {
                (Some(a), Some(b)) => {
                    if self.view_preferences.node_sort == NodeSort::LatencyAsc {
                        a.cmp(&b)
                    } else {
                        b.cmp(&a)
                    }
                }
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            },
        });
        nodes
    }
}

/// 取消句柄不持有 Runtime，也不触发选择；丢弃 run future 同样丢弃所有网络请求。
#[derive(Clone)]
pub struct TestCancellation(watch::Sender<bool>);
impl TestCancellation {
    pub fn new() -> Self {
        Self(watch::channel(false).0)
    }
    pub fn cancel(&self) {
        self.0.send_replace(true);
    }
    pub(crate) fn receiver(&self) -> watch::Receiver<bool> {
        self.0.subscribe()
    }
}
impl Default for TestCancellation {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ProxyTestRequest {
    generation: u64,
    version: SnapshotVersion,
    scope: OutboundId,
    nodes: Vec<NodeId>,
    proxy: RunningProxy,
    preferences: UiLatencyPreferences,
    cancellation: TestCancellation,
    /// Direct/Block 不走节点测速，不能由组的成功数推断直连成功。
    pub untestable: Vec<OutboundId>,
}
impl ProxyTestRequest {
    pub fn generation(&self) -> u64 {
        self.generation
    }
    pub fn nodes(&self) -> &[NodeId] {
        &self.nodes
    }
    pub fn cancellation(&self) -> TestCancellation {
        self.cancellation.clone()
    }
}
#[derive(Debug)]
pub enum TestDisposition {
    Recorded,
    Superseded,
    Cancelled,
    StateChanged,
    StateFailed(ProxyError),
    InstanceChanged,
    HistoryFailed(LatencyStoreError),
}
#[derive(Debug)]
pub struct NodeTestResult {
    pub node: NodeId,
    pub generation: u64,
    pub result: Result<super::network::Latency, NetworkError>,
    pub disposition: TestDisposition,
}

#[derive(Default)]
struct Requests {
    next: u64,
    nodes: BTreeMap<NodeId, u64>,
    groups: BTreeMap<PoolId, u64>,
}

pub struct ProxyService {
    snapshots: SnapshotService,
    source: ProxySource,
    network: NetworkService,
    history: Mutex<LatencyStore>,
    requests: Mutex<Requests>,
    permits: Arc<Semaphore>,
}
impl ProxyService {
    pub fn shutdown(&self) {
        self.permits.close();
        self.network.shutdown();
    }
    pub fn open(
        snapshots: SnapshotService,
        source: ProxySource,
        history_path: &Path,
    ) -> Result<Self, ProxyError> {
        Ok(Self {
            snapshots,
            network: NetworkService::new(source.clone()),
            source,
            history: Mutex::new(LatencyStore::open(history_path)?),
            requests: Mutex::new(Requests::default()),
            permits: Arc::new(Semaphore::new(MAX_CONCURRENT_TESTS)),
        })
    }
    fn facts(&self) -> Result<(AppState, OutboundQuerySnapshot), ProxyError> {
        let state = self.snapshots.snapshot()?;
        // 目录与页面字段派生自同次正式读取，避免第二个 read gate 与跨快照版本漂移。
        // 首读 Busy 原样传播；后续选择及测速写回仍由既有版本/实例 CAS 核对。
        let directory = OutboundQuerySnapshot::from_state(&state);
        Ok((state, directory))
    }
    fn running(&self, runtime: &ManualRuntimeSnapshot) -> Result<RunningProxy, ProxyError> {
        let proxy = self
            .source
            .current()
            .filter(|p| p.is_valid())
            .ok_or(ProxyError::NotReady)?;
        if runtime.runtime.status != RuntimeStatus::Ready
            || runtime.runtime.instance_id.as_ref().map(|id| id.0.as_str())
                != Some(proxy.instance_id())
        {
            return Err(ProxyError::NotReady);
        }
        Ok(proxy)
    }
    pub fn snapshot(&self, runtime: &ManualRuntimeSnapshot) -> Result<ProxySnapshot, ProxyError> {
        let (state, directory) = self.facts()?;
        let running = self.running(runtime).ok();
        let applied = running.as_ref().and_then(|proxy| {
            proxy.applied_proxy_facts(
                runtime.runtime.instance_id.as_ref()?,
                runtime.runtime.applied_version.as_ref()?,
            )
        });
        let history = self.history.lock().expect("latency history");
        let entries = directory
            .list()
            .iter()
            .map(|outbound| {
                let records = history.history(&state.state_epoch, &outbound.id)?;
                let applied_members = match &outbound.id {
                    OutboundId::Pool(id) => applied.as_ref().and_then(|f| f.pools.get(id)).cloned(),
                    _ => None,
                };
                let current = applied
                    .as_ref()
                    .and_then(|_| match &outbound.id {
                        OutboundId::Pool(id) => runtime
                            .confirmed_group_selections
                            .get(id)
                            .cloned()
                            .or_else(|| {
                                runtime
                                    .confirmed_manual_selections
                                    .get(id)
                                    .cloned()
                                    .map(OutboundId::Node)
                            }),
                        _ => None,
                    })
                    .filter(|id| {
                        applied_members
                            .as_ref()
                            .is_some_and(|members| members.contains(id))
                    });
                let applied = if running.is_none() {
                    Some(false)
                } else {
                    applied.as_ref().map(|facts| match &outbound.id {
                        OutboundId::Node(node) => running
                            .as_ref()
                            .expect("applied capability")
                            .nodes
                            .contains_key(node),
                        OutboundId::Pool(id) => facts.pools.contains_key(id),
                        _ => false,
                    })
                };
                let band = (outbound.kind == OutboundKind::Node)
                    .then(|| records.first().and_then(|h| h.outcome.milliseconds()))
                    .flatten()
                    .map(|ms| {
                        if ms > u64::from(state.app_config.behavior.latency.medium_ms) {
                            LatencyBand::High
                        } else if ms > u64::from(state.app_config.behavior.latency.low_ms) {
                            LatencyBand::Medium
                        } else {
                            LatencyBand::Low
                        }
                    });
                Ok(ProxyEntry {
                    configured_group: state
                        .groups
                        .iter()
                        .find(|group| group.outbound_id() == outbound.id)
                        .cloned(),
                    routing_policy: match &outbound.id {
                        OutboundId::Pool(id) => state.profile.routing.selector(id).cloned(),
                        _ => None,
                    },
                    routing_fallback: state
                        .profile
                        .routing
                        .fallback
                        .as_ref()
                        .is_some_and(|p| outbound.id == OutboundId::Pool(p.id.clone())),
                    outbound: outbound.clone(),
                    protocol: match &outbound.id {
                        OutboundId::Node(id) => state
                            .nodes
                            .iter()
                            .find(|node| &node.id == id)
                            .map(|node| node.protocol),
                        _ => None,
                    },
                    applied,
                    udp: None,
                    applied_members,
                    selection_mode: if match &outbound.id {
                        OutboundId::Pool(id) => state.profile.routing.selector(id).is_some(),
                        _ => false,
                    } || state.groups.iter().any(|group| {
                        (OutboundId::Pool(group.id.clone()) == outbound.id
                            && group.rule != crate::domain::GroupRule::Failover)
                            || group.lanes.iter().any(|lane| {
                                OutboundId::Pool(lane.pool_id(&group.id)) == outbound.id
                            })
                    }) {
                        GroupSelectionMode::Auto
                    } else {
                        GroupSelectionMode::ManualPin
                    },
                    current,
                    history: records,
                    band,
                })
            })
            .collect::<Result<Vec<_>, LatencyStoreError>>()?;
        Ok(ProxySnapshot {
            version: directory.version,
            runtime: runtime.clone(),
            entries,
            latency_preferences: state.app_config.behavior.latency,
            view_preferences: state.app_config.behavior.proxy_view,
        })
    }
    /// 仅构造既有 owner 请求。Desktop 必须直接调用 RuntimeService::select_manual，
    /// 原样消费 Result<SelectionVersion, SelectionError>；此服务绝不写选择或 Controller。
    pub fn selection_request(
        &self,
        runtime: &ManualRuntimeSnapshot,
        pool: PoolId,
        member: OutboundId,
        mode: GroupSelectionMode,
    ) -> Result<ManualSelectionRequest<SelectionChoice>, ProxyError> {
        let (state, directory) = self.facts()?;
        self.running(runtime)?;
        if runtime.runtime.saved_version != state.config_version()
            || runtime.saved_selection_version != state.selection_version()
        {
            return Err(ProxyError::SnapshotChanged);
        }
        let entry = directory.get(&OutboundId::Pool(pool.clone()))?;
        if entry.kind != OutboundKind::Selector
            || entry.availability != OutboundAvailability::Available
            || !entry.references.contains(&member)
        {
            return Err(ProxyError::InvalidSelection);
        }
        let node = if state.pools.iter().any(|p| p.id == pool) {
            let OutboundId::Node(node) = member else {
                return Err(ProxyError::InvalidSelection);
            };
            if mode != GroupSelectionMode::ManualPin {
                return Err(ProxyError::InvalidSelection);
            }
            SelectionChoice::Node(node)
        } else if let Some(parent) = state.groups.iter().find(|group| {
            group
                .lanes
                .iter()
                .any(|lane| lane.pool_id(&group.id) == pool)
        }) {
            if mode != GroupSelectionMode::Auto {
                return Err(ProxyError::InvalidSelection);
            }
            SelectionChoice::Group(GroupChoice {
                group: parent.id.clone(),
                member,
                mode,
            })
        } else {
            SelectionChoice::Group(GroupChoice {
                group: pool.clone(),
                member,
                mode,
            })
        };
        Ok(ManualSelectionRequest {
            instance: runtime.runtime.instance_id.clone().expect("ready instance"),
            expected: state.selection_version(),
            config: state.config_version(),
            pool,
            node,
        })
    }
    /// 单次只读观测：不写选择/缓存，不猜嵌套 Pool 的叶子。UI 按 DTO 身份及页面代次接收。
    pub async fn observe_applied_pool(
        &self,
        runtime: &ManualRuntimeSnapshot,
        pool: &PoolId,
    ) -> Result<Option<AppliedPoolObservation>, ProxyError> {
        let proxy = self.running(runtime)?;
        let instance = runtime
            .runtime
            .instance_id
            .as_ref()
            .expect("ready instance");
        let Some(config) = runtime.runtime.applied_version.as_ref() else {
            return Ok(None);
        };
        let requested_config = self.snapshots.snapshot()?.config_version();
        let observation = proxy.observe_applied_pool(instance, config, pool).await;
        // GET 返回后核对当前 source 和原能力，Saved 编辑也不能把旧页面响应写入新快照。
        let current = self.running(runtime).ok();
        if !proxy.is_valid()
            || current
                .as_ref()
                .and_then(|p| p.applied_proxy_facts(instance, config))
                .is_none()
            || self.snapshots.snapshot()?.config_version() != requested_config
        {
            return Ok(None);
        }
        Ok(observation)
    }
    /// UDP 显示事实与选择事实共用同一 owner 能力；I/O 后再次核对 Saved 配置。
    pub async fn observe_applied_proxy_metadata(
        &self,
        runtime: &ManualRuntimeSnapshot,
        outbound: &OutboundId,
    ) -> Result<Option<super::applied_proxy_facts::AppliedProxyMetadata>, ProxyError> {
        let proxy = self.running(runtime)?;
        let instance = runtime
            .runtime
            .instance_id
            .as_ref()
            .expect("ready instance");
        let Some(config) = runtime.runtime.applied_version.as_ref() else {
            return Ok(None);
        };
        let requested_config = self.snapshots.snapshot()?.config_version();
        let observation = proxy
            .observe_applied_proxy_metadata(instance, config, outbound)
            .await;
        let current = self.running(runtime).ok();
        if !proxy.is_valid()
            || current
                .as_ref()
                .and_then(|p| p.applied_proxy_facts(instance, config))
                .is_none()
            || self.snapshots.snapshot()?.config_version() != requested_config
        {
            return Ok(None);
        }
        Ok(observation)
    }
    pub fn begin_test(
        &self,
        runtime: &ManualRuntimeSnapshot,
        scope: OutboundId,
    ) -> Result<ProxyTestRequest, ProxyError> {
        if self.permits.is_closed() {
            return Err(ProxyError::NotReady);
        }
        let (state, directory) = self.facts()?;
        let proxy = self.running(runtime)?;
        let entry = directory.get(&scope)?;
        if entry.availability != OutboundAvailability::Available {
            return Err(ProxyError::UnsupportedTarget);
        }
        // 测速绑定实际已应用图；保存目录仅用于展示与 scope 校验，不能补造成员。
        if runtime.runtime.applied_version.as_ref() != Some(&state.config_version()) {
            return Err(ProxyError::ConfigNotApplied);
        }
        let applied = proxy
            .applied_proxy_facts(
                runtime
                    .runtime
                    .instance_id
                    .as_ref()
                    .expect("ready instance"),
                &state.config_version(),
            )
            .ok_or(ProxyError::GroupApplicationUnknown)?;
        let mut nodes = Vec::new();
        let mut untestable = Vec::new();
        let mut visited = BTreeSet::new();
        let mut pending = vec![scope.clone()];
        while let Some(id) = pending.pop() {
            if !visited.insert(id.clone()) {
                continue;
            }
            match id {
                OutboundId::Node(node) => {
                    if !proxy.nodes.contains_key(&node) {
                        return Err(ProxyError::UnsupportedTarget);
                    }
                    nodes.push(node);
                }
                OutboundId::Pool(id) => {
                    let members = applied
                        .pools
                        .get(&id)
                        .ok_or(ProxyError::GroupApplicationUnknown)?;
                    // 栈反向入栈，输出沿应用成员的原顺序；visited 同时去重和阻断环。
                    pending.extend(members.iter().rev().cloned());
                }
                _ => untestable.push(id),
            }
        }
        if nodes.is_empty() {
            return Err(ProxyError::UnsupportedTarget);
        }
        let mut requests = self.requests.lock().expect("latency generations");
        requests.next = requests
            .next
            .checked_add(1)
            .expect("latency generation overflow");
        let generation = requests.next;
        for node in &nodes {
            requests.nodes.insert(node.clone(), generation);
        }
        if let OutboundId::Pool(id) = &scope {
            requests.groups.insert(id.clone(), generation);
        }
        Ok(ProxyTestRequest {
            generation,
            version: directory.version,
            scope,
            nodes,
            proxy,
            preferences: state.app_config.behavior.latency,
            cancellation: TestCancellation::new(),
            untestable,
        })
    }
    pub async fn run_test(&self, request: ProxyTestRequest) -> Vec<NodeTestResult> {
        let mut results = Vec::new();
        self.run_test_with_progress(request, |result| results.push(result))
            .await;
        results
    }
    /// 只在真实节点操作完成后报告进度；UI 不用计时器伪造完成数。
    pub async fn run_test_with_progress(
        &self,
        request: ProxyTestRequest,
        mut complete: impl FnMut(NodeTestResult),
    ) {
        let cancellation = request.cancellation.clone();
        let mut results = stream::iter(request.nodes.iter().cloned()).map(|node| {
            let cancellation = cancellation.clone();
            let request = &request;
            async move {
                let mut cancelled = cancellation.receiver();
                let result = async {
                    let _permit = self.permits.acquire().await.map_err(|_| NetworkError::Closed)?;
                    if !request.proxy.is_valid() { return Err(NetworkError::ProxyUnavailable); }
                    // 绑定计划中的实例；等待并发槽期间替换实例也不能给新实例发送旧请求。
                    self.network.node_latency_for(Some(request.proxy.clone()), &node, &request.preferences.test_url.0, Duration::from_millis(u64::from(request.preferences.timeout_ms))).await
                };
                let result = if *cancelled.borrow() { Err(NetworkError::Closed) } else {
                    tokio::select! { biased; _ = cancelled.wait_for(|v| *v) => Err(NetworkError::Closed), result = result => result }
                };
                let disposition = self.record_result(request, &node, &result);
                NodeTestResult { node:node.clone(), generation: request.generation, result, disposition }
            }
        }).buffer_unordered(MAX_CONCURRENT_TESTS);
        while let Some(result) = results.next().await {
            complete(result);
        }
    }
    fn record_result(
        &self,
        request: &ProxyTestRequest,
        node: &NodeId,
        result: &Result<super::network::Latency, NetworkError>,
    ) -> TestDisposition {
        let requests = self.requests.lock().expect("latency generations");
        if *request.cancellation.receiver().borrow() || matches!(result, Err(NetworkError::Closed))
        {
            return TestDisposition::Cancelled;
        }
        if requests.nodes.get(node) != Some(&request.generation)
            || matches!(&request.scope, OutboundId::Pool(id) if requests.groups.get(id) != Some(&request.generation))
        {
            return TestDisposition::Superseded;
        }
        if !request.proxy.is_valid() {
            return TestDisposition::InstanceChanged;
        }
        let (state, directory) = match self.facts() {
            Ok(facts) => facts,
            Err(ProxyError::SnapshotChanged) => return TestDisposition::StateChanged,
            // 读取失败并不证明配置变化；保留真实错误与旧历史，不自动重试。
            Err(error) => return TestDisposition::StateFailed(error),
        };
        // config/epoch 改变使本次结果过期；selection revision 独立变化不抹掉实测。
        if state.config_version() != request.version.config
            || directory.get(&OutboundId::Node(node.clone())).is_err()
        {
            return TestDisposition::StateChanged;
        }
        let record = LatencyRecord {
            epoch: state.state_epoch.clone(),
            node: node.clone(),
            tested_at_ms: i64::try_from(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("system clock")
                    .as_millis(),
            )
            .expect("timestamp"),
            generation: request.generation,
            instance_id: request.proxy.instance_id().into(),
            outcome: LatencyOutcome::from_result(result),
        };
        let group = matches!(request.scope, OutboundId::Pool(_)).then_some(&request.scope);
        match self.snapshots.with_runtime_version(&state.version(), || {
            self.history
                .lock()
                .expect("latency history")
                .append(group, &record)
        }) {
            Ok(Ok(())) => TestDisposition::Recorded,
            Ok(Err(error)) => TestDisposition::HistoryFailed(error),
            Err(error) if error.code() == crate::domain::AppErrorCode::RevisionConflict => {
                TestDisposition::StateChanged
            }
            Err(error) => TestDisposition::StateFailed(ProxyError::State(error)),
        }
    }
}

#[cfg(test)]
mod tests;
