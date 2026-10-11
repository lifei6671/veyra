//! 最小代理页：目录只读 Core 投影，选择只有 Runtime owner，测速只写独立历史。
mod history;
mod motion;
mod settings;
mod testing;
use super::{
    components::{Notice, button, icon_button, icon_button_content, notify},
    i18n::tr,
    tokens::{self as t, proxies as pt},
};
use gpui_kit::{
    component::{
        ActiveTheme, Disableable, Sizable,
        input::{InputEvent, InputState},
        scroll::ScrollableElement,
    },
    prelude::*,
    *,
};
use std::{
    collections::{BTreeSet, HashSet},
    sync::{Arc, Weak},
};
use veyra_core::{
    application::{
        applied_proxy_facts::AppliedPoolObservation,
        manual_runtime::{ManualRuntimeSnapshot, SelectionError},
        network::NetworkError,
        proxies::{ProxyEntry, ProxyError, ProxySnapshot, TestCancellation, TestDisposition},
    },
    domain::{
        GroupSelectionMode, NodeId, NodeSort, OutboundAvailability, OutboundId, OutboundKind,
        PoolId,
    },
    storage::latency::{LatencyOutcome, LatencyRecord},
};

pub enum OpenPreferences {
    EditSubscription(String),
    Updated(Box<veyra_core::domain::AppState>),
}
impl EventEmitter<OpenPreferences> for ProxiesView {}

/// 穿透沿正式 current 展示每层的已应用成员；未知、叶子与重复身份都终止，不猜保存默认值。
fn penetration_chain<'a>(snapshot: &'a ProxySnapshot, policy: &ProxyEntry) -> Vec<&'a ProxyEntry> {
    let mut chain = Vec::new();
    let mut visited = BTreeSet::from([policy.outbound.id.clone()]);
    let mut next = policy.current.as_ref();
    while let Some(id) = next {
        if !visited.insert(id.clone()) {
            break;
        }
        let Some(group) = snapshot
            .entries
            .iter()
            .find(|entry| &entry.outbound.id == id)
            .filter(|entry| {
                entry
                    .applied_members
                    .as_ref()
                    .is_some_and(|members| !members.is_empty())
            })
        else {
            break;
        };
        chain.push(group);
        next = group.current.as_ref();
    }
    chain
}

/// 身份变化先撤掉旧画面事实，再异步读取；读取 Busy/失败也不能继续显示旧实例出口。
fn discard_stale_snapshot(
    snapshot: &mut Option<ProxySnapshot>,
    runtime: Option<&ManualRuntimeSnapshot>,
) {
    if snapshot.as_ref().is_some_and(|snapshot| {
        runtime.is_none_or(|runtime| {
            snapshot.runtime.runtime.status != runtime.runtime.status
                || snapshot.runtime.runtime.instance_id != runtime.runtime.instance_id
                || snapshot.runtime.runtime.applied_version != runtime.runtime.applied_version
                || snapshot.version.config != runtime.runtime.saved_version
        })
    }) {
        *snapshot = None;
    }
}

/// 仅显示偏好允许原位换列；真实配置、选择或 epoch 变化仍需重读正式目录。
fn only_proxy_view_changed(
    before: &veyra_core::domain::AppState,
    after: &veyra_core::domain::AppState,
) -> bool {
    let mut compared = after.clone();
    compared.config_revision = before.config_revision;
    compared.app_config.behavior.proxy_view = before.app_config.behavior.proxy_view.clone();
    compared == *before
}

/// Saved 版本由正式 Runtime 回读后才推进；保留同一实例的 current/成员/历史，避免清空再加载。
fn rebase_proxy_view_snapshot(
    snapshot: &mut ProxySnapshot,
    runtime: &ManualRuntimeSnapshot,
    view_config: &veyra_core::domain::ConfigVersion,
) -> bool {
    if runtime.runtime.saved_version != *view_config
        || snapshot.version.selection != runtime.saved_selection_version
        || snapshot.runtime.runtime.status != runtime.runtime.status
        || snapshot.runtime.runtime.instance_id != runtime.runtime.instance_id
        || snapshot.runtime.runtime.applied_version != runtime.runtime.applied_version
        || snapshot.version.config.0.epoch != view_config.0.epoch
    {
        return false;
    }
    snapshot.version.config = view_config.clone();
    snapshot.runtime = runtime.clone();
    true
}

fn pool_observation_identity_matches(
    snapshot: &ProxySnapshot,
    runtime: &ManualRuntimeSnapshot,
) -> bool {
    use veyra_core::application::runtime_snapshot::RuntimeStatus;
    runtime.runtime.status == RuntimeStatus::Ready
        && runtime.runtime.instance_id.is_some()
        && snapshot.runtime.runtime.instance_id == runtime.runtime.instance_id
        && snapshot.runtime.runtime.applied_version == runtime.runtime.applied_version
        && runtime
            .runtime
            .applied_version
            .as_ref()
            .is_some_and(|applied| applied.0.epoch == snapshot.version.config.0.epoch)
        && runtime.runtime.saved_version == snapshot.version.config
}

/// 只更新当前读模型。Pool/Direct 保留领域类型；缺少真实 now 必须清旧值。
fn project_pool_observation(
    snapshot: &mut ProxySnapshot,
    runtime: &ManualRuntimeSnapshot,
    pool: &PoolId,
    observation: Result<Option<AppliedPoolObservation>, ProxyError>,
) -> bool {
    if !pool_observation_identity_matches(snapshot, runtime) {
        return false;
    }
    if let Ok(Some(observation)) = &observation
        && (Some(&observation.instance_id) != runtime.runtime.instance_id.as_ref()
            || Some(&observation.config) != runtime.runtime.applied_version.as_ref()
            || &observation.pool_id != pool)
    {
        return false;
    }
    let Some(entry) = snapshot
        .entries
        .iter_mut()
        .find(|entry| entry.outbound.id == OutboundId::Pool(pool.clone()))
    else {
        return false;
    };
    match observation {
        Ok(Some(observation)) => entry.current = observation.current,
        Ok(None) => {
            // 外层 None 表示 applied 能力失效；不能沿用旧实例的成员事实。
            entry.current = None;
            entry.applied_members = None;
            entry.applied = None;
        }
        // 一次读取失败不撤销已确认成员；后续已有 Runtime 节拍仍可读取真实 now。
        Err(_) => entry.current = None,
    }
    true
}

// 视觉 busy 依据真实操作来源，不能从当前剩余节点数推断。
#[derive(Clone, Debug)]
enum TestSource {
    Node,
    Group,
    All,
    Subscription(String),
}
pub struct ProxiesView {
    services: Weak<crate::services::AppServices>,
    runtime: Option<ManualRuntimeSnapshot>,
    snapshot: Option<ProxySnapshot>,
    visible: bool,
    loading: bool,
    load_generation: u64,
    observing_pools: bool,
    pool_observation_task: Option<Task<()>>,
    error: Option<&'static str>,
    selection_error: Option<SelectionError>,
    selecting: Option<PoolId>,
    selection_generation: u64,
    test_generation: u64,
    test_source: Option<TestSource>,
    cancellations: Vec<TestCancellation>,
    testing: HashSet<NodeId>,
    testing_scopes: BTreeSet<OutboundId>,
    progress: Option<(usize, usize, usize)>,
    test_errors: Vec<&'static str>,
    query: Entity<InputState>,
    tab: usize,
    policy_order: Vec<PoolId>,
    node_order: Vec<PoolId>,
    collapsed: BTreeSet<OutboundId>,
    penetration_expanded: BTreeSet<OutboundId>,
    width: f32,
    content_scroll: ScrollHandle,
    subscription_model: crate::subscriptions::Model,
    subscription_collapsed: BTreeSet<String>,
    subscription_errors:
        Vec<veyra_core::application::subscription_management::SubscriptionOperationError>,
    _query_events: Subscription,
    settings: Entity<settings::ProxySettings>,
    _settings_events: Subscription,
    projected_state: Option<veyra_core::domain::AppState>,
    view_only_config: Option<veyra_core::domain::ConfigVersion>,
}
impl ProxiesView {
    pub fn new(
        services: &Arc<crate::services::AppServices>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let query = cx.new(|cx| InputState::new(window, cx));
        let query_events = cx.subscribe(&query, |_, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                cx.notify();
            }
        });
        let settings = cx.new(|cx| settings::ProxySettings::new(services, window, cx));
        let settings_events = cx.subscribe(&settings, |view, _, event, cx| {
            match event {
                settings::SettingsEvent::Changed(preferences) => {
                    if let Some(snapshot) = &mut view.snapshot {
                        snapshot.view_preferences = preferences.clone();
                    }
                }
                settings::SettingsEvent::Saved(state) => {
                    cx.emit(OpenPreferences::Updated(state.clone()))
                }
            }
            cx.notify();
        });
        Self {
            services: Arc::downgrade(services),
            runtime: None,
            snapshot: None,
            visible: false,
            loading: false,
            load_generation: 0,
            observing_pools: false,
            pool_observation_task: None,
            error: None,
            selection_error: None,
            selecting: None,
            selection_generation: 0,
            test_generation: 0,
            test_source: None,
            cancellations: vec![],
            testing: HashSet::new(),
            testing_scopes: BTreeSet::new(),
            progress: None,
            test_errors: vec![],
            query,
            tab: 0,
            policy_order: vec![],
            node_order: vec![],
            collapsed: BTreeSet::new(),
            penetration_expanded: BTreeSet::new(),
            content_scroll: ScrollHandle::new(),
            width: 800.,
            subscription_model: crate::subscriptions::Model::default(),
            subscription_collapsed: BTreeSet::new(),
            subscription_errors: Vec::new(),
            _query_events: query_events,
            settings,
            _settings_events: settings_events,
            projected_state: None,
            view_only_config: None,
        }
    }
    /// 订阅摘要来自同一权威 AppState，不另建目录或持久状态。
    pub fn project_state(&mut self, state: &veyra_core::domain::AppState, cx: &mut Context<Self>) {
        if let Some(before) = &self.projected_state
            && before != state
        {
            // 连续输入的多次保存可合并，但不能跨过一次真实出站/选择变更。
            self.view_only_config = if only_proxy_view_changed(before, state)
                && self.snapshot.as_ref().is_some_and(|snapshot| {
                    snapshot.version.selection == before.selection_version()
                        && (snapshot.version.config == before.config_version()
                            || self.view_only_config.as_ref() == Some(&before.config_version()))
                }) {
                Some(state.config_version())
            } else {
                None
            };
        }
        self.projected_state = Some(state.clone());
        self.subscription_model.project(state);
        self.node_order = state.groups.iter().map(|g| g.id.clone()).collect();
        self.policy_order = state
            .profile
            .routing
            .selectors()
            .map(|p| p.id.clone())
            .collect();
        self.settings
            .update(cx, |settings, cx| settings.project(state, cx));
        cx.notify();
    }
    fn refresh_subscriptions(&mut self, ids: Vec<String>, cx: &mut Context<Self>) {
        if self.subscription_model.busy() || !self.testing.is_empty() || self.selecting.is_some() {
            return;
        }
        let Some(services) = self.services.upgrade() else {
            return;
        };
        let request = self.subscription_model.begin();
        self.subscription_errors.clear();
        let receiver = services.proxy_refresh_subscriptions(request.expected.clone(), ids);
        cx.spawn(async move |view, cx| {
            let result = receiver.await;
            let _ = view.update(cx, |view, cx| {
                let result = result.unwrap_or(Err(veyra_core::application::subscription_management::SubscriptionOperationError::StateUnavailable));
                if !view.subscription_model.accepts(&request) { return; }
                let result = result.map(|(completion, failures)| {view.subscription_errors = failures; completion});
                if let Some(state) = view.subscription_model.complete(&request, result) {
                    cx.emit(OpenPreferences::Updated(state));
                    view.load(cx);
                }
                let errors = view.subscription_errors.iter().copied().chain(
                    if let crate::subscriptions::Status::Error(error) = view.subscription_model.status { Some(error) } else { None }
                ).map(|error| tr(cx,crate::subscriptions::error_key(error)).to_owned()).collect::<Vec<_>>();
                if !errors.is_empty() { super::components::notice::notify_app(Notice::Error,errors.join(" · "),cx); }
                cx.notify();
            });
        }).detach();
        cx.notify();
    }
    fn visible_group(entry: &ProxyEntry, tab: usize) -> bool {
        match tab {
            0 => entry.routing_policy.as_ref().is_some_and(|p| p.enabled),
            1 => entry
                .configured_group
                .as_ref()
                .is_some_and(|g| !g.builtin() && g.enabled),
            _ => false,
        }
    }

    pub fn project_runtime(
        &mut self,
        runtime: Option<ManualRuntimeSnapshot>,
        cx: &mut Context<Self>,
    ) {
        // Shell 投递命令时也会重投影旧快照。相同事实/仅 uptime 变化不重读 Store，
        // 否则本页后台读取会抢占 fail-fast gate，令同一次 Start/选择错误地报 Busy。
        let mut directory_runtime = runtime.clone();
        if let Some(snapshot) = &mut directory_runtime {
            snapshot.uptime_seconds = self.runtime.as_ref().and_then(|old| old.uptime_seconds);
        }
        let directory_changed = self.runtime != directory_runtime;
        let uptime_changed = self.runtime.as_ref().and_then(|r| r.uptime_seconds)
            != runtime.as_ref().and_then(|r| r.uptime_seconds);
        if self
            .runtime
            .as_ref()
            .and_then(|r| r.runtime.instance_id.as_ref())
            != runtime
                .as_ref()
                .and_then(|r| r.runtime.instance_id.as_ref())
        {
            if self.selecting.take().is_some() {
                self.selection_generation += 1;
                self.selection_error = Some(SelectionError::StaleInstance);
            }
            self.cancel_tests();
        }
        let view_rebased = match (&mut self.snapshot, &runtime, &self.view_only_config) {
            (Some(snapshot), Some(runtime), Some(version)) => {
                rebase_proxy_view_snapshot(snapshot, runtime, version)
            }
            _ => false,
        };
        if view_rebased {
            #[cfg(debug_assertions)]
            if let Some(snapshot) = &self.snapshot {
                eprintln!(
                    "proxy-view-rebased config={} selection={} entries={}",
                    snapshot.version.config.0.revision,
                    snapshot.version.selection.0.revision,
                    snapshot.entries.len()
                );
            }
            self.view_only_config = None;
            // 旧异步目录结果不能回滚即时布局；下一次真实配置变化仍走原失效路径。
            self.load_generation += 1;
            self.loading = false;
        }
        discard_stale_snapshot(&mut self.snapshot, runtime.as_ref());
        self.runtime = runtime;
        if self.visible && directory_changed && !view_rebased {
            self.load(cx);
        } else if self.visible && uptime_changed {
            // 沿用 Shell 已有 Runtime 刷新节拍，只读 now，不为自动组新建轮询任务。
            self.observe_pools(cx);
        }
    }
    pub fn set_visible(&mut self, visible: bool, cx: &mut Context<Self>) {
        self.visible = visible;
        self.load_generation += 1;
        self.loading = false;
        self.observing_pools = false;
        self.pool_observation_task = None;
        if visible {
            self.load(cx);
        } else {
            self.cancel_tests();
        }
        cx.notify();
    }
    pub fn cancel_tests(&mut self) {
        for cancel in self.cancellations.drain(..) {
            cancel.cancel();
        }
        self.test_generation += 1;
        self.testing.clear();
        self.testing_scopes.clear();
        self.progress = None;
        self.test_source = None;
    }
    fn load(&mut self, cx: &mut Context<Self>) {
        self.pool_observation_task = None;
        let (Some(services), Some(runtime)) = (self.services.upgrade(), self.runtime.clone())
        else {
            return;
        };
        self.load_generation += 1;
        self.observing_pools = false;
        let generation = self.load_generation;
        self.loading = self.snapshot.is_none();
        let receiver = services.proxy_snapshot(runtime);
        cx.spawn(async move |view, cx| {
            let result = receiver.await;
            let _ = view.update(cx, |view, cx| {
                if view.load_generation != generation || !view.visible {
                    return;
                }
                view.loading = false;
                match result {
                    Ok(Ok(snapshot)) => {
                        view.snapshot = Some(snapshot);
                        view.error = None;
                        view.observe_pools(cx);
                    }
                    Ok(Err(error)) => {
                        // 只记录实际失败，便于定位正式读取竞争；不读取额外状态或改变错误语义。
                        #[cfg(debug_assertions)]
                        eprintln!("proxy-load-failed generation={generation} error={error:?}");
                        let message = proxy_error(&error);
                        view.error = Some(message);
                        super::components::notice::notify_app(Notice::Error, tr(cx, message), cx);
                    }
                    Err(_) => {
                        view.error = Some("代理服务已关闭");
                        super::components::notice::notify_app(
                            Notice::Error,
                            tr(cx, "代理服务已关闭"),
                            cx,
                        );
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }
    fn observe_pools(&mut self, cx: &mut Context<Self>) {
        if self.observing_pools
            || !self.visible
            || !self.testing.is_empty()
            || self.selecting.is_some()
        {
            return;
        }
        let (Some(services), Some(runtime), Some(snapshot)) = (
            self.services.upgrade(),
            self.runtime.clone(),
            self.snapshot.as_mut(),
        ) else {
            return;
        };
        if !pool_observation_identity_matches(snapshot, &runtime) {
            // Saved 与 Applied revision 独立；只在实例、epoch 或已应用身份失配时清旧。
            for entry in &mut snapshot.entries {
                entry.udp = None;
                if matches!(entry.outbound.id, OutboundId::Pool(_)) {
                    entry.current = None;
                    entry.applied_members = None;
                    if runtime.runtime.status
                        == veyra_core::application::runtime_snapshot::RuntimeStatus::Ready
                    {
                        entry.applied = None;
                    }
                }
            }
            return;
        }
        let pools = snapshot
            .entries
            .iter()
            // 手动组继续使用 Runtime owner 的确认事实；这里只补自动组只读 now。
            .filter(|entry| entry.outbound.kind == OutboundKind::UrlTest)
            .filter(|entry| entry.applied_members.is_some())
            .filter_map(|entry| match &entry.outbound.id {
                OutboundId::Pool(pool) => Some(pool.clone()),
                _ => None,
            })
            .collect();
        let outbounds = snapshot
            .entries
            .iter()
            .map(|e| e.outbound.id.clone())
            .collect();
        let generation = self.load_generation;
        self.observing_pools = true;
        let mut receiver = services.proxy_pool_observations(runtime, pools, outbounds);
        self.pool_observation_task = Some(cx.spawn(async move |view, cx| {
            while let Some(observation) = receiver.recv().await {
                let _ = view.update(cx, |view, cx| {
                    if view.load_generation != generation || !view.visible {
                        return;
                    }
                    if let (Some(snapshot), Some(runtime)) = (&mut view.snapshot, &view.runtime) {
                        match observation {
                            crate::services::ProxyObservation::Pool(pool, result) => {
                                project_pool_observation(snapshot, runtime, &pool, result);
                            }
                            crate::services::ProxyObservation::Metadata(outbound, result) => {
                                if pool_observation_identity_matches(snapshot, runtime) {
                                    let metadata = result.ok().flatten().filter(|m| {
                                        Some(&m.instance_id) == runtime.runtime.instance_id.as_ref()
                                            && Some(&m.config)
                                                == runtime.runtime.applied_version.as_ref()
                                            && m.outbound_id == outbound
                                    });
                                    if let Some(entry) = snapshot
                                        .entries
                                        .iter_mut()
                                        .find(|e| e.outbound.id == outbound)
                                    {
                                        entry.udp = metadata.and_then(|m| m.udp);
                                    }
                                }
                            }
                        }
                    }
                    cx.notify();
                });
            }
            let _ = view.update(cx, |view, _| {
                if view.load_generation == generation {
                    view.observing_pools = false;
                }
            });
        }));
    }
    fn select_node(
        &mut self,
        pool: PoolId,
        member: OutboundId,
        mode: GroupSelectionMode,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.selecting.is_some() || self.subscription_model.busy() || !self.testing.is_empty() {
            return;
        }
        self.pool_observation_task = None;
        self.observing_pools = false;
        let (Some(services), Some(runtime)) = (self.services.upgrade(), self.runtime.as_ref())
        else {
            return;
        };
        let request = match services
            .proxies
            .selection_request(runtime, pool.clone(), member, mode)
        {
            Ok(request) => request,
            Err(error) => {
                #[cfg(debug_assertions)]
                if std::env::var_os("VEYRA_DESKTOP_EVIDENCE_LOG").is_some() {
                    eprintln!(
                        "proxy-select request-failed error={error:?} saved={:?} selection={:?}",
                        runtime.runtime.saved_version, runtime.saved_selection_version
                    );
                }
                self.error = Some(proxy_error(&error));
                cx.notify();
                return;
            }
        };
        #[cfg(debug_assertions)]
        if std::env::var_os("VEYRA_DESKTOP_EVIDENCE_LOG").is_some() {
            eprintln!(
                "proxy-select request config={:?} selection={:?}",
                request.config, request.expected
            );
        }
        let identity = request.instance.clone();
        let receiver = services.manual_runtime.select_manual(request);
        self.selection_generation += 1;
        let generation = self.selection_generation;
        self.selecting = Some(pool);
        self.selection_error = None;
        cx.spawn_in(window, async move |view, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    receiver
                        .recv()
                        .unwrap_or(Err(SelectionError::StateUnavailable))
                })
                .await;
            let _ = view.update_in(cx, |view, window, cx| {
                if view.selection_generation != generation {
                    return;
                }
                view.selecting = None;
                let result = if view
                    .runtime
                    .as_ref()
                    .and_then(|r| r.runtime.instance_id.as_ref())
                    != Some(&identity)
                {
                    Err(SelectionError::StaleInstance)
                } else {
                    result
                };
                view.selection_error = result.as_ref().err().copied();
                notify(
                    if result.is_ok() {
                        Notice::Success
                    } else {
                        Notice::Error
                    },
                    result
                        .map(|_| "选择已确认")
                        .unwrap_or_else(super::groups::selection_error_text),
                    window,
                    cx,
                );
                // owner 在选择回复后投递 RuntimeEvent，由 project_runtime 读回唯一确认快照。
                // 此处再 load 会与该投影竞争短 Store gate，成功选择也可能被后到 Busy 覆盖。
                // 卡片 selected 不作乐观覆盖；失败仍保留真实 selection_error。
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }
    fn test(&mut self, scopes: Vec<OutboundId>, source: TestSource, cx: &mut Context<Self>) {
        if !self.testing.is_empty() || self.subscription_model.busy() || self.selecting.is_some() {
            return;
        }
        // 取消只读批次及其网络 future，避免额外 Snapshot 读取争抢测速落盘 gate。
        self.pool_observation_task = None;
        self.observing_pools = false;
        let (Some(services), Some(runtime)) = (self.services.upgrade(), self.runtime.as_ref())
        else {
            return;
        };
        self.error = None;
        self.test_errors.clear();
        self.test_generation += 1;
        let generation = self.test_generation;
        let mut requests = vec![];
        self.test_source = Some(source);
        self.testing_scopes = scopes.iter().cloned().collect();
        for scope in scopes {
            #[cfg(debug_assertions)]
            let diagnostic_scope = scope.clone();
            match services.proxies.begin_test(runtime, scope) {
                Ok(request) => {
                    self.testing.extend(request.nodes().iter().cloned());
                    self.cancellations.push(request.cancellation());
                    if !request.untestable.is_empty() {
                        self.test_errors.push("直连与拒绝不进行节点测速");
                    }
                    requests.push(request);
                }
                Err(error) => {
                    // 原始 typed error 用于区分 Store Busy 与目录失配，不吞错或重试。
                    #[cfg(debug_assertions)]
                    eprintln!(
                        "proxy-test-begin-failed generation={generation} source={:?} scope={diagnostic_scope:?} error={error:?}",
                        self.test_source
                    );
                    let message = proxy_error(&error);
                    self.test_errors.push(message);
                    super::components::notice::notify_app(Notice::Error, tr(cx, message), cx);
                }
            }
        }
        self.progress = Some((0, self.testing.len(), 0));
        let mut receiver = services.proxy_tests(requests);
        cx.spawn(async move |view, cx| {
            while let Some(result) = receiver.recv().await {
                let _ = view.update(cx, |view, cx| {
                    if view.test_generation != generation {
                        return;
                    }
                    view.testing.remove(&result.node);
                    let failure = match &result.disposition {
                        TestDisposition::Recorded => match &result.result {
                            Ok(latency) if latency.milliseconds.is_some_and(|ms| ms > 0) => None,
                            Ok(latency) if latency.milliseconds == Some(0) => {
                                Some("零值，未确认成功")
                            }
                            Ok(_) => Some("未知"),
                            Err(error) => Some(network_error(error)),
                        },
                        TestDisposition::Cancelled => Some("测速已取消"),
                        TestDisposition::Superseded => Some("新测速已替代此结果"),
                        TestDisposition::StateChanged => Some("配置已变化，结果已丢弃"),
                        TestDisposition::StateFailed(_) => Some("测速结果未保存，保留旧历史"),
                        TestDisposition::InstanceChanged => Some("实例已更换，结果已丢弃"),
                        TestDisposition::HistoryFailed(_) => Some("测速历史保存失败，保留旧历史"),
                    };
                    if let Some((done, _, failed)) = &mut view.progress {
                        *done += 1;
                        *failed += usize::from(failure.is_some());
                    }
                    if let Some(error) = failure
                        && !view.test_errors.contains(&error)
                    {
                        view.test_errors.push(error);
                    }
                    cx.notify();
                });
            }
            let _ = view.update(cx, |view, cx| {
                if view.test_generation == generation {
                    view.cancellations.clear();
                    view.testing.clear();
                    view.testing_scopes.clear();
                    if !view.test_errors.is_empty() {
                        let message = view
                            .test_errors
                            .iter()
                            .map(|error| tr(cx, error))
                            .collect::<Vec<_>>()
                            .join(" · ");
                        super::components::notice::notify_app(Notice::Error, message, cx);
                    }
                    // 完成后撤掉进度行，恢复 React 原有卡片位置；错误反馈仍保留。
                    view.progress = None;
                    view.test_source = None;
                    view.load(cx);
                    cx.notify();
                }
            });
        })
        .detach();
        cx.notify();
    }
    fn name(entry: &ProxyEntry, cx: &App) -> String {
        entry.outbound.display_name.clone().unwrap_or_else(|| {
            tr(
                cx,
                match entry.outbound.id {
                    OutboundId::Direct => "直连",
                    OutboundId::Block => "拒绝",
                    _ => "未命名",
                },
            )
            .into()
        })
    }
    fn node_card(
        &self,
        entry: &ProxyEntry,
        parent: Option<&ProxyEntry>,
        width: f32,
        presentation: (bool, bool),
        subscription_testing: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let id = entry.outbound.id.clone();
        let name = Self::name(entry, cx);
        let selected = parent.and_then(|parent| parent.current.as_ref()) == Some(&id);
        let selectable =
            parent.is_some_and(|parent| parent.outbound.kind == OutboundKind::Selector);
        let (penetration, inert) = presentation;
        // 单节点测速只淡化该目标；保持 Runtime 的单次请求互斥，但不让其它卡片闪烁。
        let testing_target = matches!(&id, OutboundId::Node(node) if self.testing.contains(node))
            || self.testing_scopes.contains(&id);
        let visibly_busy = self.selecting.is_some()
            || self.subscription_model.busy()
            || !self.testing.is_empty()
                && (!matches!(self.test_source, Some(TestSource::Node)) || testing_target);
        let nodes_variant = self.tab == 1 && !penetration;
        let mut subtitle = entry
            .protocol
            .map(|protocol| {
                if protocol == veyra_core::domain::ProxyProtocol::Hysteria2 {
                    "hy2".into()
                } else {
                    format!("{protocol:?}").to_lowercase()
                }
            })
            .unwrap_or_else(|| match entry.outbound.kind {
                OutboundKind::Direct => "direct".into(),
                OutboundKind::Block => "reject".into(),
                OutboundKind::Selector => "selector".into(),
                OutboundKind::UrlTest => "urltest".into(),
                _ => String::new(),
            });
        if entry.udp == Some(true) {
            subtitle.push_str(" / udp");
        }
        let content = div()
            .w(px(width - pt::NODE_BORDER * 2.))
            .h(px(pt::NODE_HEIGHT - pt::NODE_BORDER * 2.))
            .flex()
            .flex_col()
            .py(px(pt::NODE_PY))
            .pl(px(t::GAP))
            .pr(px(if nodes_variant {
                t::GAP
            } else {
                pt::NODE_RIGHT
            }))
            .child(
                div()
                    .h(px(pt::NODE_TITLE))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .gap(px(t::ROW_GAP))
                    .when(
                        !nodes_variant
                            && (!penetration || entry.outbound.kind != OutboundKind::Node),
                        |title| {
                            title.child(configured_group_icon(
                                entry,
                                if penetration {
                                    pt::CURRENT_ICON
                                } else {
                                    pt::OPTION_ICON
                                },
                                self.snapshot.as_ref().expect("node snapshot"),
                                cx,
                            ))
                        },
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_size(px(t::BODY))
                            .line_height(px(t::BODY_LINE))
                            .font_weight(pt::FONT_MEDIUM)
                            .text_color(if cx.theme().mode.is_dark() {
                                cx.theme().muted_foreground
                            } else {
                                rgba(if selected {
                                    pt::NODE_SELECTED_TEXT
                                } else {
                                    pt::NODE_TEXT
                                })
                                .into()
                            })
                            .truncate()
                            .child(name.clone()),
                    ),
            )
            .child(
                div()
                    .h(px(pt::LATENCY_HEIGHT))
                    .flex_shrink_0()
                    .when(nodes_variant, |subtitle| {
                        subtitle.pr(px(pt::NODE_SUBTITLE_RIGHT))
                    })
                    .text_size(px(t::SMALL))
                    .line_height(px(t::BODY_LINE))
                    .text_color(if cx.theme().mode.is_dark() {
                        cx.theme().muted_foreground
                    } else {
                        rgba(if selected {
                            pt::NODE_SELECTED_TEXT
                        } else {
                            pt::NODE_MUTED_TEXT
                        })
                        .into()
                    })
                    .font(t::openbox_font())
                    .truncate()
                    .child(subtitle),
            );
        let main = if selectable {
            let pool = match &parent.expect("selector parent").outbound.id {
                OutboundId::Pool(pool) => pool.clone(),
                _ => unreachable!("selector pool"),
            };
            let mode = parent.expect("selector parent").selection_mode;
            let member = id.clone();
            button(
                SharedString::from(format!("proxy-select-{pool:?}-{id:?}")),
                "",
            )
            .accessibility_label(format!("{} {name}", tr(cx, "选择节点")))
            .w(px(width - pt::NODE_BORDER * 2.))
            .h(px(pt::NODE_HEIGHT - pt::NODE_BORDER * 2.))
            .p_0()
            .rounded(px(pt::NODE_RADIUS - pt::NODE_BORDER))
            .reference_hover(
                rgba(0).into(),
                rgba(if cx.theme().mode.is_dark() {
                    pt::NODE_HOVER
                } else {
                    0
                })
                .into(),
                cx.theme().foreground,
            )
            .disabled(
                inert
                    || self.selecting.is_some()
                    || !self.testing.is_empty()
                    || self.subscription_model.busy(),
            )
            .child(content)
            .on_click(cx.listener(move |view, _, window, cx| {
                view.select_node(pool.clone(), member.clone(), mode, window, cx)
            }))
            .into_any_element()
        } else {
            // URLTest 和订阅节点主区为静态展示；只有独立延迟按钮执行测速。
            content.into_any_element()
        };
        // React 全部测速标记所有卡片；订阅只标记该订阅的卡片，单节点只标记目标。
        let testing = self.progress.is_some()
            && (matches!(self.test_source, Some(TestSource::All))
                || subscription_testing
                || matches!(self.test_source, Some(TestSource::Node)) && testing_target);
        let (foreground, background) = if testing {
            (
                cx.theme().muted_foreground,
                rgba(pt::TESTING_BACKGROUND).into(),
            )
        } else {
            latency_colors(entry, self.snapshot.as_ref(), cx)
        };
        let scale = if testing { pt::TESTING_SCALE } else { 1. };
        let mut hover: Rgba = background.into();
        hover.r *= 0.97;
        hover.g *= 0.97;
        hover.b *= 0.97;
        let target = id.clone();
        let latency = button(
            SharedString::from(format!(
                "proxy-latency-{:?}-{id:?}",
                parent.map(|p| &p.outbound.id)
            )),
            "",
        )
        .accessibility_label(format!("{} {name}", tr(cx, "测速")))
        .min_w(px(pt::LATENCY_MIN_WIDTH * scale))
        .h(px(pt::LATENCY_HEIGHT * scale))
        .px(px(pt::LATENCY_PX * scale))
        .py_0()
        .rounded(px(t::RADIUS * scale))
        .reference_hover(background, hover.into(), foreground)
        .text_size(px(t::SMALL))
        .line_height(px(pt::LATENCY_HEIGHT))
        .font_weight(pt::FONT_MEDIUM)
        .disabled(
            inert
                || !self.testing.is_empty()
                || self.subscription_model.busy()
                || self.selecting.is_some(),
        )
        .tooltip(format!("{} {name}", tr(cx, "测速")))
        .child(if testing {
            testing_dots(cx).into_any_element()
        } else if entry
            .history
            .iter()
            .find_map(|r| r.outcome.milliseconds())
            .is_none()
        {
            // 与 React latestLatency 一致；typed 失败/超时仍留在历史及错误反馈中。
            super::icons::icon("Bolt", pt::LATENCY_ICON).into_any_element()
        } else {
            div()
                .text_size(px(t::SMALL))
                .line_height(px(pt::LATENCY_HEIGHT))
                .font_weight(pt::FONT_MEDIUM)
                .child(
                    entry
                        .history
                        .iter()
                        .find_map(|record| record.outcome.milliseconds())
                        .expect("measured latency")
                        .to_string(),
                )
                .into_any_element()
        })
        .on_click(cx.listener(move |view, _, _, cx| {
            cx.stop_propagation();
            view.test(vec![target.clone()], TestSource::Node, cx);
        }));
        // CSS 测速胶囊 scale(.96) 保留36×20占位，绘制和点击子区域居中缩至34.56×19.2。
        let latency = if testing {
            div()
                .w(px(pt::LATENCY_MIN_WIDTH))
                .h(px(pt::LATENCY_HEIGHT))
                .flex()
                .items_center()
                .justify_center()
                .child(latency)
                .into_any_element()
        } else {
            latency.into_any_element()
        };
        div()
            .id(SharedString::from(format!(
                "proxy-option-{:?}-{id:?}",
                parent.map(|p| &p.outbound.id)
            )))
            .w(px(width))
            .h(px(pt::NODE_HEIGHT))
            .relative()
            .rounded(px(pt::NODE_RADIUS))
            .border(px(pt::NODE_BORDER))
            .border_color(if selected {
                rgba(if cx.theme().mode.is_dark() {
                    pt::NODE_SELECTED_BORDER
                } else {
                    pt::NODE_SELECTED_BORDER_LIGHT
                })
                .into()
            } else {
                if cx.theme().mode.is_dark() {
                    cx.theme().border
                } else {
                    rgba(pt::NODE_BORDER_COLOR).into()
                }
            })
            .when(visibly_busy, |card| {
                // CSS opacity 先合成整卡再淡化；逐 primitive 乘 alpha 会让胶囊透绿。
                // 仅 busy 节点卡 opt-in，普通卡与其它页面保持原绘制路径。
                card.group_opacity(pt::NODE_DISABLED_OPACITY)
            })
            .bg(rgba(if cx.theme().mode.is_dark() {
                if selected {
                    pt::NODE_SELECTED
                } else {
                    pt::NODE_BG
                }
            } else if selected {
                pt::NODE_SELECTED_LIGHT
            } else {
                pt::NODE_BG_LIGHT
            }))
            .when(!cx.theme().mode.is_dark() && !visibly_busy, |card| {
                // LAN hover由整卡拥有：selected仍为/.9，不能内外两层叠加alpha。
                card.hover(move |style| {
                    style
                        .bg(rgba(if selected {
                            pt::NODE_SELECTED_LIGHT
                        } else {
                            pt::NODE_HOVER_LIGHT
                        }))
                        .border_color(rgba(if selected {
                            pt::NODE_SELECTED_HOVER_BORDER
                        } else {
                            pt::NODE_HOVER_BORDER
                        }))
                })
            })
            .child(main)
            .child(
                div()
                    .absolute()
                    .right(px(t::GAP))
                    .bottom(px(pt::NODE_PY))
                    .child(latency),
            )
            .into_any_element()
    }
    fn subscription_card(
        &self,
        subscription: &veyra_core::application::subscription_management::SubscriptionSummary,
        width: f32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(snapshot) = &self.snapshot else {
            return div().into_any_element();
        };
        let closed = self.subscription_collapsed.contains(&subscription.id);
        // 稳定 SubscriptionId 来自统一目录，名称前缀不参与归属判断。
        let mut options = snapshot
            .entries
            .iter()
            .filter(|entry| {
                matches!(entry.outbound.id, OutboundId::Node(_))
                    && entry
                        .outbound
                        .subscription_ids
                        .iter()
                        .any(|id| id.0 == subscription.id)
            })
            .filter(|entry| {
                !snapshot.view_preferences.hide_unavailable
                    || entry.outbound.availability == OutboundAvailability::Available
            })
            .collect::<Vec<_>>();
        sort_nodes(&mut options, snapshot.view_preferences.node_sort);
        let all_scopes = options
            .iter()
            .map(|entry| entry.outbound.id.clone())
            .collect::<Vec<_>>();
        let total = options.len();
        let query = self.query.read(cx).value().to_lowercase();
        let dots = options.clone();
        options.retain(|entry| {
            matches_node_query(
                &Self::name(entry, cx),
                entry.protocol,
                entry.outbound.kind,
                entry.udp,
                &query,
            )
        });
        let tested = options
            .iter()
            .filter(|entry| {
                entry
                    .history
                    .iter()
                    .any(|r| r.outcome.milliseconds().is_some())
            })
            .count();
        let toggle_id = subscription.id.clone();
        let refresh_id = subscription.id.clone();
        let edit_id = subscription.id.clone();
        let subtitle = subscription.last_success_at_ms.map_or_else(
            || tr(cx, "尚未更新").to_owned(),
            |ms| {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("system clock")
                    .as_millis() as u64;
                let hours = now.saturating_sub(ms) / 3_600_000;
                if hours == 0 {
                    tr(cx, "刚刚").into()
                } else {
                    tr(cx, "{hours} 小时前").replace("{hours}", &hours.to_string())
                }
            },
        );
        let title = div()
            .w_full()
            .line_height(px(t::BODY_LINE))
            .flex()
            .flex_col()
            .child(
                div()
                    .h(px(pt::SUBSCRIPTION_HEADER
                        - t::BODY_LINE
                        - pt::SUBSCRIPTION_UPDATED_GAP))
                    .flex()
                    .items_center()
                    .gap(px(pt::SUBSCRIPTION_COUNT_GAP))
                    .child(
                        div()
                            .text_size(px(pt::GROUP_TITLE))
                            .font_weight(pt::FONT_MEDIUM)
                            .child(subscription.name.clone()),
                    )
                    .child(
                        div()
                            .text_size(px(t::SMALL))
                            .text_color(cx.theme().muted_foreground)
                            .child(format!("({tested}/{total})")),
                    ),
            )
            .child(
                div()
                    .mt(px(pt::SUBSCRIPTION_UPDATED_GAP))
                    .text_size(px(t::SMALL))
                    .text_color(cx.theme().muted_foreground)
                    .child(format!("{} {subtitle}", tr(cx, "更新于"))),
            );
        let title = title.child(motion::CollapseBody {
            id: SharedString::from(format!("subscription-dots-{}", subscription.id)),
            closed,
            height: 28.,
            provider: false,
            dots: true,
            content: div()
                .mt(px(t::GAP))
                .min_h(px(pt::DOT_LINE))
                .flex()
                .items_center()
                .flex_wrap()
                .gap(px(t::ROW_GAP))
                .on_children_prepainted(|bounds, _, _| {
                    #[cfg(debug_assertions)]
                    super::components::log_layout("proxy-subscription-dots", &bounds);
                })
                .children(dots.iter().map(|entry| {
                    div()
                        // header span 的后置级联也作用于圆点，不能只迁移 .node-health-dots。
                        .ml(px(pt::SUBSCRIPTION_COUNT_GAP))
                        .size(px(pt::DOT))
                        .rounded_full()
                        .bg(dot_color(entry, snapshot))
                }))
                .into_any_element(),
        });
        let toggle = button(
            SharedString::from(format!("subscription-proxy-toggle-{}", subscription.id)),
            "",
        )
        .p_0()
        .w_full()
        // header 的自然高度跟随圆点 grid 的同一过渡，避免收起瞬间跳高。
        .h(px(gpui_kit::base::motion::transition(
            (
                SharedString::from(format!("subscription-head-{}", subscription.id)),
                "height",
            ),
            pt::SUBSCRIPTION_HEADER + if closed { 28. } else { 0. },
            gpui_kit::base::motion::Transition::new(std::time::Duration::from_millis(240))
                .ease(gpui_kit::component::animation::cubic_bezier(
                    0.4, 0., 0.2, 1.,
                ))
                .delay(std::time::Duration::from_millis(if closed {
                    40
                } else {
                    0
                })),
            window,
            cx,
        )))
        .justify_start()
        .reference_hover(rgba(0).into(), rgba(0).into(), cx.theme().foreground)
        .reference_focus(rgb(t::ACCENT).opacity(0.72).into())
        .child(title)
        .accessibility_label(format!(
            "{} {}",
            tr(cx, if closed { "展开" } else { "收起" }),
            subscription.name
        ))
        .on_click(cx.listener(move |view, _, _, cx| {
            if !view.subscription_collapsed.remove(&toggle_id) {
                view.subscription_collapsed.insert(toggle_id.clone());
            }
            cx.notify();
        }));
        let subscription_testing = self.progress.is_some()
            && match &self.test_source {
                Some(TestSource::All) => true,
                Some(TestSource::Subscription(id)) => id == &subscription.id,
                _ => false,
            };
        let test_subscription_id = subscription.id.clone();
        let mut card = div()
            .w(px(width))
            .min_h(px(motion::minimum(
                SharedString::from(format!("subscription-min-{}", subscription.id)),
                closed,
                pt::SUBSCRIPTION_MIN_HEIGHT,
                false,
                window,
                cx,
            )))
            .p(px(t::SECTION_PADDING))
            .rounded(cx.theme().radius_lg)
            .bg(rgba(if cx.theme().mode.is_dark() {
                pt::SURFACE_DARK
            } else {
                pt::SURFACE_LIGHT
            }))
            .flex()
            .flex_col()
            .child(
                div()
                    .flex()
                    .items_start()
                    .min_h(px(pt::PROVIDER_HEAD))
                    .gap(px(t::SECTION_PADDING))
                    .child(div().flex_1().child(toggle))
                    .child(
                        div()
                            .flex()
                            .gap(px(t::ROW_GAP))
                            .child(
                                div()
                                    .ml(px(pt::TOOL_TOOLTIP_MARGIN))
                                    .child(testing::control(
                                        SharedString::from(format!(
                                            "subscription-testing-layer-{}",
                                            subscription.id
                                        )),
                                        icon_button_content(
                                            SharedString::from(format!(
                                                "subscription-proxy-test-{}",
                                                subscription.id
                                            )),
                                            tr(cx, "测试这条订阅的节点延迟"),
                                            t::CONTROL,
                                            if subscription_testing {
                                                div().size(px(t::ICON_SMALL)).into_any_element()
                                            } else {
                                                super::icons::icon("Bolt", t::ICON_SMALL)
                                                    .into_any_element()
                                            },
                                        )
                                        .anchored_tooltip(
                                            tr(cx, "测试这条订阅的节点延迟"),
                                            false,
                                            pt::TOOL_TOOLTIP_MARGIN,
                                        )
                                        .disabled(
                                            !self.testing.is_empty()
                                                || self.subscription_model.busy()
                                                || self.selecting.is_some()
                                                || all_scopes.is_empty(),
                                        )
                                        .when(subscription_testing, |control| {
                                            control.retain_disabled_appearance()
                                        })
                                        .on_click(cx.listener(move |view, _, _, cx| {
                                            view.test(
                                                all_scopes.clone(),
                                                TestSource::Subscription(
                                                    test_subscription_id.clone(),
                                                ),
                                                cx,
                                            )
                                        }))
                                        .into_any_element(),
                                        subscription_testing,
                                        (t::CONTROL, t::CONTROL, t::ICON_SMALL),
                                        cx.theme().muted_foreground,
                                        cx,
                                    )),
                            )
                            .child(
                                div().ml(px(pt::TOOL_TOOLTIP_MARGIN)).child(
                                    icon_button(
                                        SharedString::from(format!(
                                            "subscription-proxy-refresh-{}",
                                            subscription.id
                                        )),
                                        tr(cx, "刷新"),
                                        "ArrowPath",
                                        t::CONTROL,
                                    )
                                    .anchored_tooltip(
                                        tr(cx, "刷新"),
                                        false,
                                        pt::TOOL_TOOLTIP_MARGIN,
                                    )
                                    .disabled(
                                        self.subscription_model.busy()
                                            || !self.testing.is_empty()
                                            || self.selecting.is_some(),
                                    )
                                    .on_click(cx.listener(
                                        move |view, _, _, cx| {
                                            view.refresh_subscriptions(vec![refresh_id.clone()], cx)
                                        },
                                    )),
                                ),
                            )
                            .child(
                                div().ml(px(pt::TOOL_TOOLTIP_MARGIN)).child(
                                    icon_button(
                                        SharedString::from(format!(
                                            "subscription-proxy-edit-{}",
                                            subscription.id
                                        )),
                                        tr(cx, "修改订阅"),
                                        "PencilSquare",
                                        t::CONTROL,
                                    )
                                    .anchored_tooltip(tr(cx, "修改订阅"), true, 0.)
                                    .on_click(cx.listener(
                                        move |_, _, _, cx| {
                                            cx.emit(OpenPreferences::EditSubscription(
                                                edit_id.clone(),
                                            ))
                                        },
                                    )),
                                ),
                            ),
                    ),
            );
        {
            let inner = width - t::SECTION_PADDING * 2.;
            let columns = ((inner + t::GAP)
                / (snapshot.view_preferences.node_card_min_width as f32 + t::GAP))
                .floor()
                .max(1.) as usize;
            let node_width = (inner - t::GAP * (columns - 1) as f32) / columns as f32;
            let rows = options.len().div_ceil(columns);
            let height = t::SECTION_PADDING
                + if rows == 0 {
                    0.
                } else {
                    pt::NODE_HEIGHT * rows as f32 + t::GAP * (rows - 1) as f32
                };
            let content = div()
                .mt(px(t::SECTION_PADDING))
                .flex()
                .flex_wrap()
                .gap(px(t::GAP))
                .children(options.iter().map(|entry| {
                    self.node_card(
                        entry,
                        None,
                        node_width,
                        (false, closed),
                        subscription_testing,
                        cx,
                    )
                }));
            card = card.child(motion::CollapseBody {
                id: SharedString::from(format!("subscription-body-{}", subscription.id)),
                closed,
                height,
                provider: false,
                dots: false,
                content: content.into_any_element(),
            });
        }
        card.into_any_element()
    }
    fn group_card(
        &self,
        entry: &ProxyEntry,
        width: f32,
        presentation: (bool, bool),
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let id = entry.outbound.id.clone();
        let (embedded, ancestor_closed) = presentation;
        let closed = !embedded && self.collapsed.contains(&id);
        let name = Self::name(entry, cx);
        let snapshot = self.snapshot.as_ref().expect("group snapshot");
        let current = entry
            .current
            .as_ref()
            .and_then(|id| snapshot.entries.iter().find(|e| &e.outbound.id == id))
            .map(|entry| Self::name(entry, cx))
            // React ProxyCards 的缺失 now 文案；事实仍为 None，不补默认/保存出口。
            .unwrap_or_else(|| tr(cx, "未选择").into());
        let min_width = snapshot.view_preferences.node_card_min_width as f32;
        let available_width = (width
            - if embedded {
                0.
            } else {
                t::SECTION_PADDING * 2.
            })
        .max(1.);
        let columns = ((available_width + t::GAP) / (min_width + t::GAP))
            .floor()
            .max(1.) as usize;
        let node_width = (available_width - t::GAP * (columns - 1) as f32) / columns as f32;
        let query = self.query.read(cx).value().to_lowercase();
        // 已应用成员决定真实点击范围；默认出口投影不应改变目标分流的显示顺序。
        let members = if entry.applied == Some(false) {
            entry.outbound.references.as_slice()
        } else {
            entry.applied_members.as_deref().unwrap_or(&[])
        };
        let mut options = members
            .iter()
            .filter_map(|id| snapshot.entries.iter().find(|e| &e.outbound.id == id))
            .filter(|e| {
                !snapshot.view_preferences.hide_unavailable
                    || e.outbound.availability == OutboundAvailability::Available
            })
            .collect::<Vec<_>>();
        if entry.routing_policy.is_some()
            && let Some(state) = &self.projected_state
        {
            let display_order = state.target_routing_members();
            options.sort_by_key(|option| {
                display_order
                    .iter()
                    .position(|id| id == &option.outbound.id)
                    .unwrap_or(usize::MAX)
            });
        }
        sort_nodes(&mut options, snapshot.view_preferences.node_sort);
        let tested = options
            .iter()
            .filter(|e| e.history.iter().any(|r| r.outcome.milliseconds().is_some()))
            .count();
        let total = options.len();
        let dots = options.clone();
        options.retain(|e| {
            matches_node_query(
                &Self::name(e, cx),
                e.protocol,
                e.outbound.kind,
                e.udp,
                &query,
            )
        });
        // 与 Tab 相同：显式使用原件文本 advance，避免各内在宽度向上取整累计。
        // React h2 / badge / small 为相邻 inline-flex；不使用固定近似标签宽度。
        let text_width = |text: &str, size: f32, weight: FontWeight| {
            window
                .text_system()
                .shape_line(
                    text.to_owned().into(),
                    px(size),
                    &[TextRun {
                        len: text.len(),
                        font: Font {
                            weight,
                            ..t::openbox_font()
                        },
                        color: cx.theme().foreground,
                        background_color: None,
                        underline: None,
                        strikethrough: None,
                    }],
                    None,
                )
                .width
        };
        let title_width = text_width(&name, t::ICON_SMALL, pt::FONT_MEDIUM);
        let badge_width = text_width(tr(cx, "域名穿透"), pt::POLICY_BADGE_FONT, pt::FONT_MEDIUM)
            + px(pt::POLICY_BADGE_PX * 2.);
        let metadata = format!(
            "{} ({tested}/{total})",
            if embedded {
                tr(
                    cx,
                    if entry
                        .configured_group
                        .as_ref()
                        .is_some_and(|g| g.rule == veyra_core::domain::GroupRule::Failover)
                    {
                        "故障转移"
                    } else if entry.outbound.kind == OutboundKind::Selector {
                        "手动选择"
                    } else {
                        "自动择优"
                    },
                )
            } else if entry.outbound.kind == OutboundKind::Selector {
                "Selector"
            } else {
                "URLTest"
            }
        );
        // 原版类型和计数是两个text-xs节点，之间gap4，不能由普通空格代替。
        let (kind_text, count_text) = metadata.rsplit_once(' ').expect("group metadata count");
        let kind_width = text_width(kind_text, pt::POLICY_METADATA_FONT, pt::FONT_REGULAR);
        let count_width = text_width(count_text, pt::POLICY_METADATA_FONT, pt::FONT_REGULAR);
        let metadata_width = kind_width + count_width + px(pt::POLICY_TITLE_GAP);
        #[cfg(debug_assertions)]
        let metadata_measurement = format!("policy-metadata:{name}");
        let toggle = id.clone();
        #[cfg(debug_assertions)]
        let measurement = format!("policy-head:{name}");
        let title_row = div()
            .flex()
            .items_center()
            .gap(px(if embedded {
                pt::PENETRATION_TITLE_GAP
            } else {
                pt::POLICY_TITLE_GAP
            }))
            .child(
                div()
                    .w(title_width)
                    .min_w_0()
                    .text_size(px(t::ICON_SMALL))
                    .font_weight(pt::FONT_MEDIUM)
                    .truncate()
                    .line_height(px(pt::GROUP_TITLE_LINE))
                    .child(name.clone()),
            )
            .when(self.tab == 0 && !embedded, |row| {
                row.child(
                    div()
                        .w(badge_width)
                        .flex_shrink_0()
                        .px(px(pt::POLICY_BADGE_PX))
                        .py(px(pt::POLICY_BADGE_PY))
                        .rounded(px(pt::POLICY_BADGE_RADIUS))
                        .line_height(px(t::BODY_LINE))
                        .bg(rgba(if cx.theme().mode.is_dark() {
                            pt::POLICY_BADGE_BG
                        } else {
                            pt::POLICY_BADGE_BG_LIGHT
                        }))
                        .text_size(px(pt::POLICY_BADGE_FONT))
                        .font_weight(pt::FONT_MEDIUM)
                        .text_color(if cx.theme().mode.is_dark() {
                            cx.theme().muted_foreground
                        } else {
                            rgba(pt::POLICY_BADGE_TEXT).into()
                        })
                        .child(tr(cx, "域名穿透")),
                )
            })
            .child(
                div()
                    .w(metadata_width)
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .gap(px(pt::POLICY_TITLE_GAP))
                    // 原版 small 始终单行；精确 advance 的小数宽度不能触发内部换行。
                    .whitespace_nowrap()
                    .text_size(px(pt::POLICY_METADATA_FONT))
                    .font_weight(pt::FONT_REGULAR)
                    .text_color(if cx.theme().mode.is_dark() {
                        cx.theme().muted_foreground
                    } else {
                        rgba(pt::POLICY_METADATA_TEXT).into()
                    })
                    .font(t::openbox_font())
                    .line_height(px(pt::GROUP_METADATA_LINE))
                    // GPUI裸文本布局会ceil宽度；固定真实advance避免父容器精确宽度下收缩。
                    .child(
                        div()
                            .w(kind_width)
                            .flex_shrink_0()
                            .child(kind_text.to_owned()),
                    )
                    .child(
                        div()
                            .w(count_width)
                            .flex_shrink_0()
                            .child(count_text.to_owned()),
                    )
                    .on_children_prepainted(move |_bounds, _, _| {
                        #[cfg(debug_assertions)]
                        super::components::log_layout(&metadata_measurement, &_bounds);
                    }),
            );
        // 与 selected_latency 同样只跟随已确认 current；不把保存默认值当成实际出口。
        // 深层 Selector 逐段展示，避免只追一跳就把另一个组误标成终端节点。
        let mut current_tail = Vec::new();
        if !embedded {
            let mut next = entry
                .current
                .as_ref()
                .and_then(|id| snapshot.entries.iter().find(|e| &e.outbound.id == id))
                .and_then(|selected| selected.current.as_ref());
            let mut visited = std::collections::BTreeSet::from([&entry.outbound.id]);
            while let Some(id) = next {
                if !visited.insert(id) {
                    break;
                }
                let Some(selected) = snapshot.entries.iter().find(|e| &e.outbound.id == id) else {
                    break;
                };
                current_tail.push(selected);
                next = selected.current.as_ref();
            }
        }
        let current_font = if embedded {
            pt::PENETRATION_CURRENT_FONT
        } else {
            pt::CURRENT_FONT
        };
        let current_width = text_width(&current, current_font, pt::FONT_REGULAR);
        let current_row = div()
            .mt(px(if embedded {
                pt::PENETRATION_CURRENT_TOP
            } else {
                pt::CURRENT_TOP
            }))
            .h(px(t::BODY_LINE))
            .flex()
            .items_center()
            .gap(px(if embedded {
                pt::PENETRATION_CURRENT_GAP
            } else {
                pt::CURRENT_GAP
            }))
            .when(self.tab == 0, |row| {
                if let Some(current) = entry
                    .current
                    .as_ref()
                    .and_then(|id| snapshot.entries.iter().find(|e| &e.outbound.id == id))
                {
                    row.when(current.outbound.kind != OutboundKind::Node, |row| {
                        row.child(configured_group_icon(
                            current,
                            pt::CURRENT_ICON,
                            snapshot,
                            cx,
                        ))
                    })
                } else {
                    row.child(proxy_group_icon("其他", pt::CURRENT_ICON))
                }
            })
            .text_size(px(current_font))
            .text_color(cx.theme().muted_foreground)
            .truncate()
            // 与标题行同样使用原字体 advance，避免每跳内在宽度取整后累积错位。
            .child(div().w(current_width).flex_shrink_0().child(current))
            .children(current_tail.iter().map(|selected| {
                let name = Self::name(selected, cx);
                let width = text_width(&name, current_font, pt::FONT_REGULAR);
                div()
                    .flex()
                    .items_center()
                    .gap(px(pt::CURRENT_GAP))
                    .child(super::icons::icon("ArrowRightCircle", pt::CURRENT_ICON))
                    .when(selected.outbound.kind != OutboundKind::Node, |hop| {
                        hop.child(configured_group_icon(
                            selected,
                            pt::CURRENT_ICON,
                            snapshot,
                            cx,
                        ))
                    })
                    .child(div().w(width).flex_shrink_0().child(name))
            }));
        // 外层与穿透层均将当前出口放在独立下一行。
        let head_height = if embedded {
            pt::PENETRATION_HEAD
        } else {
            pt::HEAD
        };
        let header_content = div()
            .flex()
            .flex_col()
            .child(
                div()
                    .h(px(pt::GROUP_ICON))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .gap(px(pt::POLICY_HEAD_GAP))
                    .child(configured_group_icon(entry, pt::GROUP_ICON, snapshot, cx))
                    .child(div().flex_1().min_w_0().child(title_row)),
            )
            .child(current_row);
        let header_content = header_content
            .w_full()
            .h(px(head_height))
            .font_weight(pt::FONT_REGULAR)
            .line_height(px(t::BODY_LINE))
            .on_children_prepainted(move |_bounds, _, _| {
                #[cfg(debug_assertions)]
                super::components::log_layout(&measurement, &_bounds);
            });
        let header = if embedded {
            header_content.into_any_element()
        } else {
            button(SharedString::from(format!("proxy-toggle-{id:?}")), "")
                .accessibility_label(format!(
                    "{} {name}",
                    tr(cx, if closed { "展开" } else { "收起" })
                ))
                .flex_1()
                .min_w_0()
                .h(px(pt::HEAD))
                .p_0()
                .items_start()
                .justify_start()
                .reference_hover(rgba(0).into(), rgba(0).into(), cx.theme().foreground)
                .reference_focus(rgb(t::ACCENT).opacity(0.72).into())
                .text_color(cx.theme().foreground)
                .child(header_content)
                .on_click(cx.listener(move |view, _, _, cx| {
                    if !view.collapsed.remove(&toggle) {
                        view.collapsed.insert(toggle.clone());
                    }
                    cx.notify();
                }))
                .into_any_element()
        };
        let scope = id.clone();
        let selected_delay = snapshot.selected_latency(&id).or_else(|| {
            // ProxyCards::selectedProxyDelay 在已选组引用尚无 now 时展示该引用的历史。
            // 仅为卡片显示：不把历史样本节点推断成当前出口，不改变 Core 的叶子测速语义。
            let current = entry.current.as_ref()?;
            let mut target = snapshot
                .entries
                .iter()
                .find(|e| &e.outbound.id == current)?;
            while let Some(current) = &target.current {
                target = snapshot
                    .entries
                    .iter()
                    .find(|e| &e.outbound.id == current)?;
            }
            matches!(
                target.outbound.kind,
                OutboundKind::Selector | OutboundKind::UrlTest
            )
            .then(|| target.history.iter().find_map(|r| r.outcome.milliseconds()))
            .flatten()
        });
        let delay_color = selected_delay
            .map(|ms| {
                if ms > u64::from(snapshot.latency_preferences.medium_ms) {
                    rgb(pt::DANGER)
                } else if ms > u64::from(snapshot.latency_preferences.low_ms) {
                    rgb(pt::WARNING)
                } else {
                    rgb(pt::LATENCY_SUCCESS)
                }
            })
            .map(Into::into)
            .unwrap_or(cx.theme().muted_foreground);
        let group_testing = self.progress.is_some()
            && (matches!(self.test_source, Some(TestSource::All))
                || matches!(self.test_source, Some(TestSource::Group))
                    && self.testing_scopes.contains(&id));
        // 有历史时保留真实时间线；没有历史的闪电按钮仍须说明其测速操作。
        let no_history = entry.history.is_empty()
            && (embedded
                || entry
                    .current
                    .as_ref()
                    .and_then(|id| snapshot.entries.iter().find(|e| &e.outbound.id == id))
                    .is_none_or(|current| current.history.is_empty()));
        let group_test = button(SharedString::from(format!("proxy-group-test-{id:?}")), "")
            .accessibility_label(format!("{} {}", tr(cx, "测速"), Self::name(entry, cx)))
            .when(no_history, |b| {
                b.tooltip(format!("{} {}", tr(cx, "测速"), Self::name(entry, cx)))
            })
            .h(px(pt::LATENCY_HEIGHT))
            .py_0()
            .px(px(6.))
            .child(if group_testing {
                div().size(px(pt::LATENCY_ICON)).into_any_element()
            } else {
                selected_delay
                    .map(|ms| {
                        div()
                            .text_size(px(t::SMALL))
                            .line_height(px(pt::GROUP_LATENCY_LINE))
                            .font_weight(pt::FONT_REGULAR)
                            .font(t::openbox_font())
                            .child(ms.to_string())
                            .into_any_element()
                    })
                    .unwrap_or_else(|| {
                        super::icons::icon("Bolt", pt::LATENCY_ICON).into_any_element()
                    })
            })
            .min_w(px(40.))
            .rounded_full()
            .reference_hover(
                rgba(0xffffffc2).into(),
                rgba(0xffffffc2).into(),
                delay_color,
            )
            .disabled(
                ancestor_closed
                    || !self.testing.is_empty()
                    || self.subscription_model.busy()
                    || self.selecting.is_some(),
            )
            .opacity(
                // active Bolt 已由 testing::control 整层淡化，避免叠加为 .25。
                if !group_testing
                    && (!self.testing.is_empty()
                        && !matches!(self.test_source, Some(TestSource::Node))
                        || self.selecting.is_some()
                        || self.subscription_model.busy())
                {
                    pt::CONTROL_DISABLED_OPACITY
                } else {
                    1.
                },
            )
            .on_click(cx.listener(move |view, _, _, cx| {
                view.test(vec![scope.clone()], TestSource::Group, cx)
            }));
        let group_test = testing::control(
            SharedString::from(format!("proxy-group-testing-layer-{id:?}")),
            group_test.into_any_element(),
            group_testing,
            (40., pt::LATENCY_HEIGHT, pt::LATENCY_ICON),
            delay_color,
            cx,
        );
        let group_test = history::HistoryTrigger {
            id: SharedString::from(format!("proxy-group-history-{id:?}")),
            trigger: group_test.into_any_element(),
            // React PolicyLatency 在组自身历史为空时展示当前出口的真实历史。
            records: if !embedded && entry.history.is_empty() {
                entry
                    .current
                    .as_ref()
                    .and_then(|id| snapshot.entries.iter().find(|e| &e.outbound.id == id))
                    .map(|e| e.history.as_slice())
                    .unwrap_or(&[])
            } else {
                entry.history.as_slice()
            }
            .iter()
            .map(|record| {
                let name = snapshot
                    .entries
                    .iter()
                    .find(|entry| entry.outbound.id == OutboundId::Node(record.node.clone()))
                    .map(|entry| Self::name(entry, cx))
                    .unwrap_or_else(|| record.node.0.clone());
                (name, record.clone())
            })
            .collect(),
        };
        let mut card = div()
            .w(px(width))
            .when(!embedded, |card| {
                card.min_h(px(motion::minimum(
                    SharedString::from(format!("proxy-min-{id:?}")),
                    closed,
                    pt::CARD_MIN_HEIGHT,
                    self.tab == 1,
                    window,
                    cx,
                )))
                .p(px(t::SECTION_PADDING))
                .rounded(cx.theme().radius_lg)
                .bg(rgba(if cx.theme().mode.is_dark() {
                    pt::SURFACE_DARK
                } else {
                    pt::SURFACE_LIGHT
                }))
            })
            .flex()
            .flex_col()
            .child(
                div()
                    .w_full()
                    .h(px(head_height))
                    .relative()
                    .flex()
                    .items_start()
                    .gap(px(pt::HEAD_GAP))
                    .when(
                        self.tab == 1
                            && entry.outbound.display_name.as_deref() == Some("所有-自动"),
                        |head| {
                            head.child(
                                div()
                                    .absolute()
                                    .right_0()
                                    .bottom_0()
                                    .text_size(px(t::SMALL))
                                    .font_weight(pt::FONT_REGULAR)
                                    .text_color(cx.theme().muted_foreground)
                                    .child(tr(cx, "检测间隔 300 秒 · 容差 100 毫秒")),
                            )
                        },
                    )
                    .child(div().flex_1().min_w_0().child(header))
                    // 测速固定在该层右上，不能挤窄当前出口行。
                    .child(div().absolute().right_0().top_0().child(group_test)),
            );
        let dot_content = div()
            .mt(px(t::GAP))
            .min_h(px(pt::LATENCY_HEIGHT))
            .items_center()
            .flex()
            .flex_wrap()
            .gap(px(t::ROW_GAP))
            .children(dots.iter().enumerate().map(|(i, node)| {
                let selected = entry.current.as_ref() == Some(&node.outbound.id);
                let history = history_label(&node.history, cx);
                super::components::HoverSurface::new(SharedString::from(format!(
                    "proxy-dot-{id:?}-{i}"
                )))
                .with_tooltip(format!("{} · {history}", Self::name(node, cx)))
                .scale(pt::DOT)
                .size(px(pt::DOT))
                .rounded_full()
                .border(if selected { px(3.) } else { px(0.) })
                .border_color(dot_color(node, snapshot))
                .bg(if selected {
                    rgba(0).into()
                } else {
                    dot_color(node, snapshot)
                })
                .into_any_element()
            }));
        let dot_columns = ((available_width + t::ROW_GAP) / (pt::DOT + t::ROW_GAP))
            .floor()
            .max(1.) as usize;
        let dot_rows = dots.len().div_ceil(dot_columns).max(1);
        let dots_height = t::GAP
            + (pt::DOT * dot_rows as f32 + t::ROW_GAP * (dot_rows - 1) as f32)
                .max(pt::LATENCY_HEIGHT);
        if !embedded {
            card = card.child(motion::CollapseBody {
                id: SharedString::from(format!("proxy-dots-{id:?}")),
                closed,
                height: dots_height,
                provider: false,
                dots: true,
                content: dot_content.into_any_element(),
            });
        }
        let mut body = div().w_full().flex().flex_col();
        if !embedded && self.tab == 1 && snapshot.view_preferences.group_by_provider {
            let scope = id.clone();
            body = body.child(
                div()
                    .w_full()
                    .mt(px(if embedded { 0. } else { t::GAP }))
                    .h(px(pt::PROVIDER_HEAD))
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_size(px(t::BODY))
                            .font_weight(pt::FONT_MEDIUM)
                            .child(tr(cx, "订阅")),
                    )
                    .child(testing::control(
                        SharedString::from(format!("proxy-provider-testing-layer-{id:?}")),
                        icon_button_content(
                            SharedString::from(format!("proxy-provider-test-{id:?}")),
                            tr(cx, "组测速"),
                            pt::LATENCY_HEIGHT,
                            if group_testing {
                                div().size(px(pt::LATENCY_ICON)).into_any_element()
                            } else {
                                super::icons::icon("Bolt", pt::LATENCY_ICON).into_any_element()
                            },
                        )
                        .w(px(40.))
                        .bg(rgba(0xffffffc2))
                        .text_color(cx.theme().muted_foreground)
                        .rounded_full()
                        .disabled(
                            closed
                                || ancestor_closed
                                || !self.testing.is_empty()
                                || self.subscription_model.busy()
                                || self.selecting.is_some(),
                        )
                        .when(
                            group_testing || matches!(self.test_source, Some(TestSource::Node)),
                            |control| control.retain_disabled_appearance(),
                        )
                        .on_click(cx.listener(move |view, _, _, cx| {
                            view.test(vec![scope.clone()], TestSource::Group, cx)
                        }))
                        .into_any_element(),
                        group_testing,
                        (40., pt::LATENCY_HEIGHT, pt::LATENCY_ICON),
                        cx.theme().muted_foreground,
                        cx,
                    )),
            );
        }
        body = body.child(
            div()
                .mt(px(if embedded {
                    pt::PENETRATION_GRID_TOP
                } else if self.tab == 1 {
                    if snapshot.view_preferences.group_by_provider {
                        0.
                    } else {
                        t::GAP
                    }
                } else {
                    t::SECTION_PADDING
                }))
                .flex()
                .flex_wrap()
                .gap(px(t::GAP))
                .children(options.iter().map(|node| {
                    self.node_card(
                        node,
                        Some(entry),
                        node_width,
                        (embedded, closed || ancestor_closed),
                        false,
                        cx,
                    )
                })),
        );
        let rows = options.len().div_ceil(columns);
        let grid_height = if rows == 0 {
            0.
        } else {
            pt::NODE_HEIGHT * rows as f32 + t::GAP * (rows - 1) as f32
        };
        let provider = !embedded && self.tab == 1 && snapshot.view_preferences.group_by_provider;
        let mut body_height = grid_height
            + if embedded {
                pt::PENETRATION_GRID_TOP
            } else if provider {
                t::GAP + pt::PROVIDER_HEAD
            } else if self.tab == 1 {
                t::GAP
            } else {
                t::SECTION_PADDING
            };
        // 同一次正式读模型沿当前链逐层展示；每层独立消费自己的成员。
        let penetration = penetration_chain(snapshot, entry);
        if self.tab == 0 && !embedded && !penetration.is_empty() {
            let open = self.penetration_expanded.contains(&id);
            let toggle = id.clone();
            body = body.child(
                div()
                    .mt(px(pt::PENETRATION_FOOTER_TOP))
                    .flex()
                    .items_start()
                    .child(
                        button(SharedString::from(format!("proxy-penetration-{id:?}")), "")
                            .child(
                                // Kit Button 多 child 内部固定 gap8；单个 flex 子层保留 CSS gap4。
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(t::ROW_GAP))
                                    .child(
                                        div()
                                            .text_size(px(t::BODY))
                                            .line_height(px(t::BODY_LINE))
                                            .font_weight(pt::FONT_SEMIBOLD)
                                            .child(tr(
                                                cx,
                                                if open { "收起穿透" } else { "策略穿透" },
                                            )),
                                    )
                                    .child(super::icons::icon(
                                        if open { "ChevronUp" } else { "ChevronDown" },
                                        pt::LATENCY_ICON,
                                    )),
                            )
                            .accessibility_label(tr(cx, if open { "收起穿透" } else { "策略穿透" }))
                            .h(px(pt::PENETRATION_FOOTER_HEIGHT))
                            .px(px(if open { pt::PENETRATION_FOOTER_PX } else { 0. }))
                            .py_0()
                            .rounded(px(pt::PENETRATION_FOOTER_RADIUS))
                            .reference_hover(
                                rgba(if open { pt::PENETRATION_FOOTER_BG } else { 0 }).into(),
                                rgba(if open { pt::PENETRATION_FOOTER_BG } else { 0 }).into(),
                                if open {
                                    rgb(0xffffff).into()
                                } else {
                                    cx.theme().foreground
                                },
                            )
                            .disabled(closed || ancestor_closed)
                            .on_click(cx.listener(move |view, _, _, cx| {
                                if !view.penetration_expanded.remove(&toggle) {
                                    view.penetration_expanded.insert(toggle.clone());
                                }
                                cx.notify();
                            })),
                    ),
            );
            body_height += pt::PENETRATION_FOOTER_TOP + pt::PENETRATION_FOOTER_HEIGHT;
            if open {
                let count = penetration.len();
                let mut layers = div().w_full();
                body_height += pt::PENETRATION_TOP + 1.;
                for (index, nested) in penetration.into_iter().enumerate() {
                    let members = nested
                        .applied_members
                        .as_ref()
                        .expect("confirmed nested members");
                    let nested_count = members
                        .iter()
                        .filter_map(|id| snapshot.entries.iter().find(|e| &e.outbound.id == id))
                        .filter(|e| {
                            !snapshot.view_preferences.hide_unavailable
                                || e.outbound.availability == OutboundAvailability::Available
                        })
                        .filter(|e| {
                            matches_node_query(
                                &Self::name(e, cx),
                                e.protocol,
                                e.outbound.kind,
                                e.udp,
                                &query,
                            )
                        })
                        .count();
                    let rows = nested_count.div_ceil(columns);
                    let last = index + 1 == count;
                    body_height += pt::PENETRATION_PADDING_TOP
                        + pt::PENETRATION_INNER_TOP
                        + pt::PENETRATION_HEAD
                        + pt::PENETRATION_GRID_TOP
                        + if rows == 0 {
                            0.
                        } else {
                            pt::NODE_HEIGHT * rows as f32 + t::GAP * (rows - 1) as f32
                        }
                        + if last { 0. } else { t::SECTION_PADDING + 1. };
                    layers = layers.child(
                        div()
                            .pt(px(pt::PENETRATION_PADDING_TOP + pt::PENETRATION_INNER_TOP))
                            .when(!last, |layer| {
                                layer
                                    .pb(px(t::SECTION_PADDING))
                                    .border_b_1()
                                    .border_color(cx.theme().border)
                            })
                            .child(self.group_card(
                                nested,
                                available_width,
                                (true, closed || ancestor_closed),
                                window,
                                cx,
                            )),
                    );
                }
                body = body.child(
                    div()
                        .mt(px(pt::PENETRATION_TOP))
                        .border_t_1()
                        .border_color(cx.theme().border)
                        .child(layers),
                );
            }
        }
        card = card.child(motion::CollapseBody {
            id: SharedString::from(format!("proxy-body-{id:?}")),
            closed,
            height: body_height,
            provider,
            dots: false,
            content: body.into_any_element(),
        });
        card.into_any_element()
    }
}
impl Render for ProxiesView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let surface = rgba(if cx.theme().mode.is_dark() {
            pt::SURFACE_DARK
        } else {
            0xffffffe6
        });
        let weak = cx.entity().downgrade();
        // 内容内边距属于列表内容，不能放在 scrollbar 的满高容器上，否则空余页面也能滚动8px。
        let mut root = div()
            .w_full()
            .flex_shrink_0()
            .p(px(t::GAP))
            .on_children_prepainted(|bounds, _, _| {
                #[cfg(debug_assertions)]
                super::components::log_layout("proxy-scroll-body", &bounds);
            })
            .font(t::openbox_font())
            .font_weight(pt::FONT_REGULAR)
            .flex()
            .flex_col();
        self.query.update(cx, |input, cx| {
            input.set_placeholder(tr(cx, "搜索 | 多个关键词用空格分隔"), window, cx)
        });
        let groups = self
            .snapshot
            .as_ref()
            .map(|s| {
                s.entries
                    .iter()
                    .filter(|e| Self::visible_group(e, 0))
                    .count()
            })
            .unwrap_or(0);
        let nodes = self
            .snapshot
            .as_ref()
            .map(|s| {
                s.entries
                    .iter()
                    .filter(|e| Self::visible_group(e, 1))
                    .count()
            })
            .unwrap_or(0);
        let mut tabs = div()
            .h(px(pt::TOOLBAR))
            .p(px(t::ROW_GAP))
            .rounded(px(pt::CONTROL_RADIUS))
            .bg(surface)
            .flex()
            .items_center();
        for (tab, label, count) in [
            (0, "策略", groups),
            (1, "节点", nodes),
            (2, "订阅", self.subscription_model.list.len()),
        ] {
            let label = format!("{} ({count})", tr(cx, label));
            // 浏览器使用小数逻辑宽度；显式文本测量避免 GPUI 内在宽度逐项向上取整。
            let font = Font {
                weight: pt::FONT_REGULAR,
                ..t::openbox_font()
            };
            let label_width = window
                .text_system()
                .shape_line(
                    label.clone().into(),
                    px(t::BODY),
                    &[TextRun {
                        len: label.len(),
                        font,
                        color: cx.theme().foreground,
                        background_color: None,
                        underline: None,
                        strikethrough: None,
                    }],
                    None,
                )
                .width;
            tabs = tabs.child(
                button(SharedString::from(format!("proxy-tab-{tab}")), "")
                    .accessibility_label(label.clone())
                    .w(label_width + px(pt::TAB_PX * 2.))
                    .min_w_0()
                    .flex_shrink_0()
                    .child(
                        div()
                            .text_size(px(t::BODY))
                            .line_height(px(pt::CONTROL_LINE))
                            .font_weight(pt::FONT_REGULAR)
                            .child(label),
                    )
                    .h(px(pt::TAB))
                    .font_weight(pt::FONT_REGULAR)
                    .px(px(pt::TAB_PX))
                    .rounded(px(pt::TAB_RADIUS))
                    .reference_hover(
                        if self.tab == tab {
                            rgba(pt::TAB_SELECTED).into()
                        } else {
                            rgba(0).into()
                        },
                        if self.tab == tab {
                            rgba(pt::TAB_SELECTED).into()
                        } else {
                            rgba(pt::NODE_HOVER).into()
                        },
                        if self.tab == tab {
                            rgb(pt::TAB_SELECTED_TEXT).into()
                        } else {
                            cx.theme().muted_foreground
                        },
                    )
                    .on_click(cx.listener(move |view, _, _, cx| {
                        view.tab = tab;
                        view.pool_observation_task = None;
                        view.observing_pools = false;
                        view.observe_pools(cx);
                        cx.notify();
                    })),
            );
        }
        let all_scopes = self
            .snapshot
            .as_ref()
            .map(|s| {
                s.visible_nodes()
                    .iter()
                    .map(|e| e.outbound.id.clone())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let testing = !self.testing.is_empty();
        let testing_all =
            self.progress.is_some() && matches!(self.test_source, Some(TestSource::All));
        let all_closed = if self.tab == 2 {
            !self.subscription_model.list.is_empty()
                && self
                    .subscription_model
                    .list
                    .iter()
                    .all(|s| self.subscription_collapsed.contains(&s.id))
        } else {
            self.snapshot.as_ref().is_some_and(|s| {
                let groups = s
                    .entries
                    .iter()
                    .filter(|e| Self::visible_group(e, self.tab))
                    .collect::<Vec<_>>();
                !groups.is_empty()
                    && groups
                        .iter()
                        .all(|e| self.collapsed.contains(&e.outbound.id))
            })
        };
        let controls = div()
            .on_children_prepainted(|_bounds, _, _| {
                #[cfg(debug_assertions)]
                super::components::log_layout("proxy-controls", &_bounds);
            })
            .h(px(pt::TOOLBAR))
            .flex_shrink_0()
            .w_full()
            .flex()
            .items_center()
            .gap(px(t::GAP))
            .child(tabs)
            .child(super::components::search_input::SearchInput {
                state: self.query.clone(),
                input: gpui_kit::component::input::Input::new(&self.query)
                    // Kit has_suffix 的最终 pr 来自 Size：Small 对应 React 的 8px。
                    .small()
                    // 当前运行原版input-sm没有前缀图标；保留真实编辑器与独立清除按钮。
                    .suffix(
                        icon_button("proxy-search-clear", tr(cx, "清空搜索"), "XMark", 16.)
                            .w(px(12.))
                            .h(px(16.))
                            .bg(rgba(0))
                            .on_click(cx.listener(|view, _, window, cx| {
                                view.query
                                    .update(cx, |input, cx| input.set_value("", window, cx));
                            })),
                    )
                    .w(px(pt::SEARCH_WIDTH))
                    // Input 的固有尺寸 API 会覆盖 Styled::h，明确指定渲染布局。
                    .map(|input| Styled::h(input, px(pt::TOOLBAR)))
                    .px(px(12.))
                    .py_0()
                    .text_size(px(t::BODY))
                    .font_weight(pt::FONT_REGULAR)
                    .gap_0()
                    .line_height(px(pt::CONTROL_LINE))
                    .bg(surface)
                    .rounded(px(pt::CONTROL_RADIUS)),
            })
            .child(div().flex_1())
            .child(
                div()
                    .flex()
                    .gap(px(t::ROW_GAP))
                    .child(
                        icon_button(
                            "proxy-settings",
                            tr(cx, "策略设置"),
                            "WrenchScrewdriver",
                            t::CONTROL,
                        )
                        .bg(surface)
                        .disabled(self.subscription_model.busy() || self.selecting.is_some())
                        .anchored_tooltip(tr(cx, "策略设置"), false, 0.)
                        .on_click(cx.listener(|view, _, window, cx| {
                            view.settings
                                .update(cx, |settings, cx| settings.show(window, cx));
                        })),
                    )
                    .child(
                        icon_button(
                            "proxy-collapse",
                            tr(
                                cx,
                                if all_closed {
                                    "展开全部卡片"
                                } else {
                                    "收起全部卡片"
                                },
                            ),
                            if all_closed {
                                "ChevronDown"
                            } else {
                                "ChevronUp"
                            },
                            t::CONTROL,
                        )
                        .bg(surface)
                        .disabled(self.subscription_model.busy() || self.selecting.is_some())
                        .anchored_tooltip(
                            tr(
                                cx,
                                if all_closed {
                                    "展开全部卡片"
                                } else {
                                    "收起全部卡片"
                                },
                            ),
                            false,
                            0.,
                        )
                        .on_click(cx.listener(|view, _, _, cx| {
                            if view.tab == 2 {
                                if view
                                    .subscription_model
                                    .list
                                    .iter()
                                    .all(|s| view.subscription_collapsed.contains(&s.id))
                                {
                                    view.subscription_collapsed.clear();
                                } else {
                                    view.subscription_collapsed.extend(
                                        view.subscription_model.list.iter().map(|s| s.id.clone()),
                                    );
                                }
                            } else if let Some(s) = &view.snapshot {
                                let ids = s
                                    .entries
                                    .iter()
                                    .filter(|entry| Self::visible_group(entry, view.tab))
                                    .map(|entry| entry.outbound.id.clone())
                                    .collect::<Vec<_>>();
                                let all_closed = ids.iter().all(|id| view.collapsed.contains(id));
                                for id in ids {
                                    if all_closed {
                                        view.collapsed.remove(&id);
                                    } else {
                                        view.collapsed.insert(id);
                                    }
                                }
                            }
                            cx.notify();
                        })),
                    )
                    .child(testing::control(
                        "proxy-all-testing-layer".into(),
                        icon_button_content(
                            "proxy-test-all",
                            tr(
                                cx,
                                if testing {
                                    "取消测速"
                                } else {
                                    "全部测速"
                                },
                            ),
                            t::CONTROL,
                            if testing_all {
                                div().size(px(t::ICON_SMALL)).into_any_element()
                            } else {
                                super::icons::icon("Bolt", t::ICON_SMALL).into_any_element()
                            },
                        )
                        .bg(surface)
                        .disabled(self.subscription_model.busy() || self.selecting.is_some())
                        .anchored_tooltip(
                            tr(
                                cx,
                                if testing {
                                    "取消测速"
                                } else {
                                    "全部测速"
                                },
                            ),
                            self.tab != 2,
                            0.,
                        )
                        .on_click(cx.listener(move |view, _, _, cx| {
                            if testing {
                                view.cancel_tests();
                                view.test_errors.push("测速已取消");
                                cx.notify();
                            } else {
                                view.test(all_scopes.clone(), TestSource::All, cx);
                                if let Some(s) = &view.snapshot {
                                    view.testing_scopes.extend(
                                        s.entries
                                            .iter()
                                            .filter(|e| {
                                                matches!(e.outbound.id, OutboundId::Pool(_))
                                            })
                                            .map(|e| e.outbound.id.clone()),
                                    );
                                }
                            }
                        }))
                        .into_any_element(),
                        testing_all,
                        (t::CONTROL, t::CONTROL, t::ICON_SMALL),
                        cx.theme().foreground,
                        cx,
                    ))
                    .when(self.tab == 2, |controls| {
                        let ids = self
                            .subscription_model
                            .list
                            .iter()
                            .map(|s| s.id.clone())
                            .collect::<Vec<_>>();
                        controls.child(
                            icon_button(
                                "proxy-refresh-subscriptions",
                                tr(cx, "刷新所有订阅"),
                                "ArrowPath",
                                t::CONTROL,
                            )
                            .bg(surface)
                            .anchored_tooltip(tr(cx, "刷新所有订阅"), true, 0.)
                            .disabled(
                                self.subscription_model.busy()
                                    || !self.testing.is_empty()
                                    || self.selecting.is_some()
                                    || ids.is_empty(),
                            )
                            .on_click(cx.listener(
                                move |view, _, _, cx| view.refresh_subscriptions(ids.clone(), cx),
                            )),
                        )
                    }),
            );

        if self.loading || self.runtime.is_none() {
            root = root.child(div().py(px(t::PAD)).child(tr(cx, "加载中")));
        }
        // 加载失败通过全局 Toast 反馈；错误不能插入工具栏下方挤动既有卡片。
        if let Some(error) = self.selection_error {
            root = root.child(
                div()
                    .text_color(rgb(pt::DANGER))
                    .child(tr(cx, super::groups::selection_error_text(error))),
            );
        }
        // 原版用通知反馈测速结果，进度/错误不能插入卡片之前改变整页几何。
        if let Some(snapshot) = &self.snapshot {
            {
                let mut groups = snapshot
                    .entries
                    .iter()
                    .filter(|e| Self::visible_group(e, self.tab))
                    .collect::<Vec<_>>();
                if self.tab == 1 {
                    groups.sort_by_key(|entry| {
                        entry
                            .configured_group
                            .as_ref()
                            .and_then(|group| self.node_order.iter().position(|id| id == &group.id))
                            .unwrap_or(usize::MAX)
                    });
                }
                if self.tab == 0 {
                    groups.sort_by_key(|entry| {
                        snapshot
                            .view_preferences
                            .strategy_order
                            .iter()
                            .position(|id| {
                                entry.routing_policy.as_ref().is_some_and(|p| p.id.0 == *id)
                            })
                            .unwrap_or_else(|| {
                                self.policy_order
                                    .iter()
                                    .position(|id| {
                                        entry.routing_policy.as_ref().is_some_and(|p| &p.id == id)
                                    })
                                    .map(|i| snapshot.view_preferences.strategy_order.len() + i)
                                    .unwrap_or(usize::MAX)
                            })
                    });
                }
                let columns = usize::from(snapshot.view_preferences.group_columns).max(1);
                let width = (self.width - t::GAP * (columns - 1) as f32) / columns as f32;
                if groups.is_empty() && self.tab != 2 {
                    root = root.child(div().py(px(t::PAD)).child(tr(cx, "暂无代理")));
                }
                root = root.child(
                    div()
                        .w_full()
                        .flex_shrink_0()
                        .flex()
                        .flex_wrap()
                        .gap(px(t::GAP))
                        .on_children_prepainted(|bounds, _, _| {
                            #[cfg(debug_assertions)]
                            super::components::log_layout("proxy-card-bounds", &bounds);
                        })
                        .children(groups.into_iter().map(|group| {
                            self.group_card(group, width, (false, false), window, cx)
                        })),
                );
            }
        }
        if self.tab == 2 {
            let subscriptions = self.subscription_model.list.clone();
            // React 订阅列表不应用策略组列数偏好，始终占满内容宽度。
            let width = self.width;
            root = root.child(
                div()
                    .w_full()
                    .flex_shrink_0()
                    .flex()
                    .flex_wrap()
                    .gap(px(t::GAP))
                    .children(subscriptions.iter().map(|subscription| {
                        self.subscription_card(subscription, width, window, cx)
                    })),
            );
        }
        super::components::page_shell::viewport()
            // 顶部与正文共同继承原版字体，Tab 测量与实际绘制使用同一家族。
            .font(t::openbox_font())
            .font_weight(pt::FONT_REGULAR)
            .on_children_prepainted(move |bounds, _, cx| {
                #[cfg(debug_assertions)]
                super::components::log_layout("proxy-viewport", &bounds);
                if let Some(bounds) = bounds.get(1) {
                    // 从共享视口的正文 bounds 测量，避免卡片自身宽度形成循环依赖。
                    let width = f32::from(bounds.size.width) - 2. * t::GAP;
                    let weak = weak.clone();
                    cx.defer(move |cx| {
                        let _ = weak.update(cx, |view, cx| {
                            if (view.width - width).abs() > 0.5 {
                                view.width = width;
                                cx.notify();
                            }
                        });
                    });
                }
            })
            .child(super::components::page_shell::toolbar(cx).child(controls))
            .child(
                div()
                    .w_full()
                    .flex_1()
                    .min_h_0()
                    .on_children_prepainted(|bounds, _, _| {
                        #[cfg(debug_assertions)]
                        super::components::log_layout("proxy-scroll-content", &bounds);
                    })
                    .id("proxy-content-scroll")
                    .track_scroll(&self.content_scroll)
                    .overflow_y_scroll()
                    .vertical_scrollbar(&self.content_scroll)
                    .child(root),
            )
            .when(self.settings.read(cx).open, |viewport| {
                viewport.child(self.settings.clone())
            })
    }
}
/// ProxyCards .latency-testing-dots：真实测速期间的三点动画，尊重系统减少动态效果。
fn testing_dots(cx: &App) -> Div {
    let color = cx.theme().muted_foreground;
    div()
        .h(px(pt::TESTING_DOTS_HEIGHT * pt::TESTING_SCALE))
        .flex()
        .items_center()
        .gap(px(pt::TESTING_DOT_GAP * pt::TESTING_SCALE))
        .children((0usize..3).map(move |index| {
            let dot = div()
                .size(px(pt::TESTING_DOT * pt::TESTING_SCALE))
                .rounded_full()
                .bg(color);
            if cx.reduce_motion() {
                dot.into_any_element()
            } else {
                let mut animations = Vec::new();
                if index > 0 {
                    animations.push(Animation::new(std::time::Duration::from_millis(
                        120 * index as u64,
                    )));
                }
                animations.push(
                    Animation::new(std::time::Duration::from_millis(pt::TESTING_PERIOD_MS))
                        .repeat()
                        .with_easing(|v| v),
                );
                dot.with_animations(
                    ("proxy-testing-dot", index),
                    animations,
                    move |dot, animation, progress| {
                        // CSS 正向 animation-delay 的等待阶段没有 fill，保留原始圆点。
                        if index > 0 && animation == 0 {
                            return dot;
                        }
                        let phase = progress;
                        let pulse =
                            gpui_kit::component::animation::cubic_bezier(0.42, 0., 0.58, 1.)(
                                (1. - (phase * 2. - 1.).abs()).clamp(0., 1.),
                            );
                        dot.opacity(0.35 + 0.65 * pulse)
                            .relative()
                            .top(px((1. - 3. * pulse) * pt::TESTING_SCALE))
                    },
                )
                .into_any_element()
            }
        }))
}
fn proxy_error(error: &ProxyError) -> &'static str {
    match error {
        ProxyError::NotReady => "请先启动并应用当前配置",
        ProxyError::ConfigNotApplied => "配置已变化，请重新应用",
        ProxyError::GroupApplicationUnknown => "组的已应用成员与当前出口未知，未执行测速",
        ProxyError::InvalidSelection => "线路或成员已变化",
        ProxyError::SnapshotChanged => "代理数据已变化，请重新加载",
        ProxyError::History(_) => "测速历史读取失败",
        ProxyError::UnsupportedTarget => "该出口不支持节点测速",
        ProxyError::State(_) | ProxyError::Directory(_) => "代理数据加载失败",
    }
}
pub(super) fn network_error(error: &NetworkError) -> &'static str {
    match error {
        NetworkError::UnsupportedNodeTestUrl => {
            "节点测速仅支持 HTTPS，请到设置中修改网址；原设置未改"
        }
        NetworkError::Closed => "测速已取消",
        NetworkError::Controller(veyra_core::singbox::clash_api::ClashApiError::Request(
            veyra_core::subscription::FetchError::Timeout,
        ))
        | NetworkError::Request(veyra_core::subscription::outbound::OutboundError::Fetch(
            veyra_core::subscription::FetchError::Timeout,
        )) => "测速超时",
        NetworkError::ProxyUnavailable => "当前运行实例不可用",
        NetworkError::NodeNotApplied => "节点未应用，未执行测速",
        _ => "测速失败",
    }
}
fn matches_query(name: &str, query: &str) -> bool {
    let name = name.to_lowercase();
    query.split_whitespace().all(|word| name.contains(word))
}
// 与 React 的名称/协议/协议别名搜索一致；没有 UDP owner 事实时不推测。
fn matches_node_query(
    name: &str,
    protocol: Option<veyra_core::domain::ProxyProtocol>,
    kind: OutboundKind,
    udp: Option<bool>,
    query: &str,
) -> bool {
    let protocol = protocol
        .map(|p| format!("{p:?}"))
        .unwrap_or_else(|| match kind {
            OutboundKind::Direct => "direct".into(),
            OutboundKind::Block => "reject".into(),
            OutboundKind::Selector => "selector".into(),
            OutboundKind::UrlTest => "urltest".into(),
            _ => String::new(),
        });
    let alias = if protocol == "Hysteria2" { "hy2" } else { "" };
    matches_query(
        &format!(
            "{name} {protocol} {alias} {}",
            if udp == Some(true) { "udp" } else { "" }
        ),
        query,
    )
}
fn latency_label(outcome: Option<&LatencyOutcome>, cx: &App) -> String {
    match outcome {
        Some(LatencyOutcome::Success(ms)) => format!("{ms}"),
        Some(LatencyOutcome::Zero) => tr(cx, "零值").into(),
        Some(LatencyOutcome::Unknown) => tr(cx, "未知").into(),
        Some(LatencyOutcome::Timeout) => tr(cx, "超时").into(),
        Some(LatencyOutcome::Failed) => tr(cx, "失败").into(),
        None => tr(cx, "未测速").into(),
    }
}
fn history_label(records: &[LatencyRecord], cx: &App) -> String {
    if records.is_empty() {
        return tr(cx, "未测速").into();
    }
    records
        .iter()
        .map(|r| {
            format!(
                "{} · {}{}",
                history_time(r.tested_at_ms),
                latency_label(Some(&r.outcome), cx),
                if matches!(r.outcome, LatencyOutcome::Success(_)) {
                    " ms"
                } else {
                    ""
                }
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}
#[cfg(target_os = "macos")]
fn history_time(milliseconds: i64) -> String {
    use objc2::AllocAnyThread;
    use objc2_foundation::{NSDate, NSDateFormatter, NSString};
    let formatter = NSDateFormatter::init(NSDateFormatter::alloc());
    formatter.setDateFormat(Some(&NSString::from_str("yyyy-MM-dd HH:mm:ss")));
    formatter
        .stringFromDate(&NSDate::dateWithTimeIntervalSince1970(
            milliseconds as f64 / 1000.,
        ))
        .to_string()
}
#[cfg(not(target_os = "macos"))]
fn history_time(milliseconds: i64) -> String {
    milliseconds.to_string()
}
fn latency_colors(entry: &ProxyEntry, snapshot: Option<&ProxySnapshot>, cx: &App) -> (Hsla, Hsla) {
    match entry.history.iter().find_map(|r| r.outcome.milliseconds()) {
        Some(ms) => {
            let p = snapshot.expect("latency snapshot");
            if ms > u64::from(p.latency_preferences.medium_ms) {
                (rgb(pt::DANGER).into(), rgba(0xffe8e8f5).into())
            } else if ms > u64::from(p.latency_preferences.low_ms) {
                (rgb(pt::WARNING).into(), rgba(0xfff4d9f5).into())
            } else {
                (rgb(pt::LATENCY_SUCCESS).into(), rgba(0xe8faf0f5).into())
            }
        }
        None => (cx.theme().foreground, rgba(0xffffffd1).into()),
    }
}
fn dot_color(entry: &ProxyEntry, snapshot: &ProxySnapshot) -> Hsla {
    let ms = entry.history.iter().find_map(|r| r.outcome.milliseconds());
    rgb(match ms {
        Some(ms) if ms > u64::from(snapshot.latency_preferences.medium_ms) => 0xe86b78,
        Some(ms) if ms > u64::from(snapshot.latency_preferences.low_ms) => 0xdaa447,
        Some(_) => pt::SUCCESS,
        None => pt::UNKNOWN,
    })
    .into()
}
fn sort_nodes(nodes: &mut Vec<&ProxyEntry>, mode: NodeSort) {
    if matches!(mode, NodeSort::NameAsc | NodeSort::NameDesc) {
        nodes.sort_by(|a, b| {
            // 原版按实际Controller名称排序；typed Direct/Block没有display_name，不能当空串。
            let name = |entry: &ProxyEntry| match entry.outbound.id {
                OutboundId::Direct => "直连".to_owned(),
                OutboundId::Block => "拒绝".to_owned(),
                _ => entry.outbound.display_name.clone().unwrap_or_default(),
            };
            let order = chinese_compare(&name(a), &name(b));
            if mode == NodeSort::NameDesc {
                order.reverse()
            } else {
                order
            }
        });
    } else if matches!(mode, NodeSort::LatencyAsc | NodeSort::LatencyDesc) {
        nodes.sort_by(|a, b| {
            let delay = |e: &ProxyEntry| e.history.iter().find_map(|r| r.outcome.milliseconds());
            match (delay(a), delay(b)) {
                (Some(a), Some(b)) => {
                    if mode == NodeSort::LatencyAsc {
                        a.cmp(&b)
                    } else {
                        b.cmp(&a)
                    }
                }
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                _ => std::cmp::Ordering::Equal,
            }
        });
    }
}
/// macOS Foundation 的显式 zh-CN locale；与 UI 语言无关，不新增排序库或锁文件。
#[cfg(target_os = "macos")]
fn chinese_compare(a: &str, b: &str) -> std::cmp::Ordering {
    use objc2_foundation::{NSLocale, NSRange, NSString, NSStringCompareOptions};
    let left = NSString::from_str(a);
    let right = NSString::from_str(b);
    let locale = NSLocale::localeWithLocaleIdentifier(&NSString::from_str("zh_CN"));
    // 两个 NSString 和 NSLocale 都是当前调用持有的有效 Foundation 对象；range为UTF-16长度。
    let result = unsafe {
        left.compare_options_range_locale(
            &right,
            NSStringCompareOptions::empty(),
            NSRange::new(0, left.length()),
            Some(locale.as_ref()),
        )
    };
    (result as isize).cmp(&0)
}
#[cfg(not(target_os = "macos"))]
fn chinese_compare(a: &str, b: &str) -> std::cmp::Ordering {
    a.cmp(b)
}
#[derive(IntoElement)]
struct ProxyIcon {
    name: String,
    extent: f32,
}
// 图标事实来自正式目标分流或节点组；名称不能用作图标映射键。
fn configured_group_icon(
    entry: &ProxyEntry,
    base: f32,
    snapshot: &ProxySnapshot,
    cx: &App,
) -> AnyElement {
    if let Some(policy) = &entry.routing_policy {
        return super::components::icon_picker::scaled_group_icon(
            &policy.icon,
            base,
            policy.icon_scale,
            cx,
        )
        .into_any_element();
    }
    if let Some(group) = &entry.configured_group {
        return super::components::icon_picker::scaled_group_icon(
            &group.icon,
            base,
            group.icon_scale,
            cx,
        )
        .into_any_element();
    }
    // 线路图标从配置的稳定 pool_id 对应关系读取，不解析名称或虚构元数据。
    if let Some((group, lane)) = snapshot
        .entries
        .iter()
        .filter_map(|entry| entry.configured_group.as_ref())
        .find_map(|group| {
            group
                .lanes
                .iter()
                .find(|lane| OutboundId::Pool(lane.pool_id(&group.id)) == entry.outbound.id)
                .map(|lane| (group, lane))
        })
    {
        let (icon, scale) = if lane.icon.is_empty() {
            (&group.icon, group.icon_scale)
        } else {
            (&lane.icon, 0)
        };
        return super::components::icon_picker::scaled_group_icon(icon, base, scale, cx)
            .into_any_element();
    }
    proxy_group_icon(
        match entry.outbound.id {
            OutboundId::Direct => "直连",
            OutboundId::Block => "拒绝",
            _ => "其他",
        },
        base,
    )
    .into_any_element()
}
fn proxy_group_icon(name: &str, extent: f32) -> ProxyIcon {
    ProxyIcon {
        name: name.into(),
        extent,
    }
}
impl RenderOnce for ProxyIcon {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let name = self.name.as_str();
        let bytes: &[u8] = match name {
            "直连" => include_bytes!("../../../../src/openbox/assets/option-direct.svg"),
            "拒绝" => include_bytes!("../../../../src/openbox/assets/option-reject.svg"),
            "所有-手动" => include_bytes!("../../../../src/openbox/assets/option-manual.svg"),
            "所有-自动" => include_bytes!("../../../../src/openbox/assets/option-auto.svg"),
            "Speed" => include_bytes!("../../../../src/openbox/assets/group-speed.svg"),
            "AI" => include_bytes!("../../../../src/openbox/assets/group-ai.svg"),
            "Youtube" => include_bytes!("../../../../src/openbox/assets/group-youtube.svg"),
            "TikTok" => include_bytes!("../../../../src/openbox/assets/group-tiktok.svg"),
            "Netflix" => include_bytes!("../../../../src/openbox/assets/group-netflix.svg"),
            "Github" => include_bytes!("../../../../src/openbox/assets/group-github.svg"),
            "Google" => include_bytes!("../../../../src/openbox/assets/group-google.svg"),
            "Microsoft" => include_bytes!("../../../../src/openbox/assets/group-microsoft.svg"),
            "Apple" => include_bytes!("../../../../src/openbox/assets/group-apple.svg"),
            "Games" => include_bytes!("../../../../src/openbox/assets/group-games.svg"),
            "国外" => include_bytes!("../../../../src/openbox/assets/group-global.svg"),
            "国内" => include_bytes!("../../../../src/openbox/assets/group-china.svg"),
            "其他" => include_bytes!("../../../../src/openbox/assets/group-other.svg"),
            _ => include_bytes!("../../../../src/openbox/assets/group-other.svg"),
        };
        // GPUI ImageFormat::Svg 默认只按 SVG intrinsic size 栅格化；24px 源图
        // 放大至 46px@2x 会模糊。使用同一资源按最终设备像素栅格化并保留全彩。
        let scale = window.scale_factor();
        let key = SharedString::from(format!("proxy-svg-{name}-{}-{scale}", self.extent));
        let image = window
            .use_keyed_state(key, cx, |_, cx| {
                let intrinsic = cx
                    .svg_renderer()
                    .render_single_frame(bytes, 1.)
                    .expect("bundled proxy SVG");
                let size = intrinsic.size(0);
                let source_extent = size.width.0.max(size.height.0) as f32;
                cx.svg_renderer()
                    .render_single_frame(bytes, self.extent * scale / source_extent)
                    .expect("bundled proxy SVG")
            })
            .read(cx)
            .clone();
        img(image)
            .size(px(self.extent))
            .object_fit(ObjectFit::Contain)
    }
}
#[cfg(test)]
mod tests {
    use super::{
        AppliedPoolObservation, ManualRuntimeSnapshot, ProxySnapshot, project_pool_observation,
    };
    use super::{chinese_compare, matches_node_query, matches_query};
    use veyra_core::{
        application::{
            proxies::ProxyEntry,
            runtime_recovery::{RecoveryAvailability, RecoveryError},
            runtime_snapshot::{InstanceId, RuntimeSnapshot, RuntimeStatus},
        },
        domain::*,
    };

    fn applied_pool() -> (ProxySnapshot, ManualRuntimeSnapshot, PoolId) {
        let state = AppState::empty();
        let pool = PoolId("auto".into());
        let runtime = ManualRuntimeSnapshot {
            runtime: RuntimeSnapshot {
                status: RuntimeStatus::Ready,
                instance_id: Some(InstanceId("owned-ui".into())),
                saved_version: state.config_version(),
                applied_version: Some(state.config_version()),
                last_successful_version: None,
                last_successful_selection_version: None,
            },
            endpoints: None,
            uptime_seconds: None,
            kernel_available: true,
            recovery: RecoveryAvailability::Unavailable(RecoveryError::Missing),
            saved_selection_version: state.selection_version(),
            confirmed_selection_version: Some(state.selection_version()),
            pending_selection: None,
            pending_selections: vec![],
            selection_fallbacks: vec![],
            confirmed_manual_selections: Default::default(),
            group_selections: Default::default(),
            confirmed_group_selections: Default::default(),
            failover_status: Default::default(),
        };
        let snapshot = ProxySnapshot {
            version: state.version(),
            runtime: runtime.clone(),
            latency_preferences: state.app_config.behavior.latency,
            view_preferences: state.app_config.behavior.proxy_view,
            entries: vec![ProxyEntry {
                routing_policy: None,
                routing_fallback: false,
                configured_group: None,
                outbound: OutboundEntry {
                    id: OutboundId::Pool(pool.clone()),
                    kind: OutboundKind::UrlTest,
                    display_name: Some("Auto".into()),
                    pool_kind: None,
                    provider_ids: vec![],
                    subscription_ids: vec![],
                    references: vec![],
                    selection: None,
                    availability: OutboundAvailability::Available,
                },
                protocol: None,
                applied: Some(true),
                udp: None,
                applied_members: Some(vec![]),
                selection_mode: GroupSelectionMode::Auto,
                current: None,
                history: vec![],
                band: None,
            }],
        };
        (snapshot, runtime, pool)
    }

    // 保护用户的目标分流→故障转移→主用组链；当前未知不能用保存选择补出子层。
    #[test]
    fn penetration_tracks_confirmed_layers_and_stops_at_unknown_or_leaf() {
        let (mut snapshot, _, _) = applied_pool();
        let ids = ["policy", "failover", "primary"].map(|id| OutboundId::Pool(PoolId(id.into())));
        let leaf = OutboundId::Node(NodeId("subscription-node".into()));
        let template = snapshot.entries[0].clone();
        snapshot.entries = ids
            .iter()
            .enumerate()
            .map(|(index, id)| {
                let mut entry = template.clone();
                entry.outbound.id = id.clone();
                let member = ids.get(index + 1).unwrap_or(&leaf).clone();
                entry.applied_members = Some(vec![member.clone()]);
                entry.current = Some(member);
                entry
            })
            .collect();
        let mut terminal = template;
        terminal.outbound.id = leaf.clone();
        terminal.outbound.kind = OutboundKind::Node;
        terminal.applied_members = None;
        snapshot.entries.push(terminal);
        let chain_ids = |snapshot: &ProxySnapshot| {
            super::penetration_chain(snapshot, &snapshot.entries[0])
                .iter()
                .map(|entry| entry.outbound.id.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(chain_ids(&snapshot), ids[1..]);
        snapshot.entries[1].current = None;
        assert_eq!(chain_ids(&snapshot), vec![ids[1].clone()]);
        snapshot.entries[0].current = Some(leaf);
        assert!(chain_ids(&snapshot).is_empty());
        snapshot.entries[0].current = None;
        snapshot.entries[0].outbound.references = vec![ids[1].clone()];
        assert!(chain_ids(&snapshot).is_empty());
        snapshot.entries[0].current = Some(ids[1].clone());
        snapshot.entries[1].current = Some(ids[0].clone());
        assert_eq!(chain_ids(&snapshot), vec![ids[1].clone()]);
        snapshot.entries[1].applied_members = None;
        assert!(chain_ids(&snapshot).is_empty());
    }

    // 保护 UI 不猜自动组叶子；Controller 的 Node/Pool/Direct 都按 typed now 显示，未知清旧。
    #[test]
    fn applied_pool_projection_preserves_types_and_clears_unknown() {
        let (mut snapshot, runtime, pool) = applied_pool();
        for current in [
            OutboundId::Node(NodeId("node".into())),
            OutboundId::Pool(PoolId("nested".into())),
            OutboundId::Direct,
        ] {
            let observation = AppliedPoolObservation {
                instance_id: runtime.runtime.instance_id.clone().unwrap(),
                config: snapshot.version.config.clone(),
                pool_id: pool.clone(),
                current: Some(current.clone()),
            };
            assert!(project_pool_observation(
                &mut snapshot,
                &runtime,
                &pool,
                Ok(Some(observation))
            ));
            assert_eq!(snapshot.entries[0].current, Some(current));
        }
        let members = snapshot.entries[0].applied_members.clone();
        assert!(project_pool_observation(
            &mut snapshot,
            &runtime,
            &pool,
            Err(veyra_core::application::proxies::ProxyError::GroupApplicationUnknown)
        ));
        assert_eq!(snapshot.entries[0].current, None);
        assert_eq!(snapshot.entries[0].applied, Some(true));
        assert_eq!(snapshot.entries[0].applied_members, members);

        let unknown = AppliedPoolObservation {
            instance_id: runtime.runtime.instance_id.clone().unwrap(),
            config: snapshot.version.config.clone(),
            pool_id: pool.clone(),
            current: None,
        };
        assert!(project_pool_observation(
            &mut snapshot,
            &runtime,
            &pool,
            Ok(Some(unknown))
        ));
        assert_eq!(snapshot.entries[0].current, None);
        assert_eq!(snapshot.entries[0].applied, Some(true));
        assert_eq!(snapshot.entries[0].applied_members, members);
        assert!(project_pool_observation(
            &mut snapshot,
            &runtime,
            &pool,
            Ok(None)
        ));
        assert_eq!(snapshot.entries[0].current, None);
        assert_eq!(snapshot.entries[0].applied_members, None);
        assert_eq!(snapshot.entries[0].applied, None);
    }

    // 保护新页面/实例不接收旧完整身份，也不把 Saved≠Applied 的响应并入当前配置。
    #[test]
    fn applied_pool_projection_rejects_instance_config_epoch_and_pool_mismatch() {
        let (mut snapshot, runtime, pool) = applied_pool();
        let good = AppliedPoolObservation {
            instance_id: runtime.runtime.instance_id.clone().unwrap(),
            config: snapshot.version.config.clone(),
            pool_id: pool.clone(),
            current: Some(OutboundId::Direct),
        };
        let mut bad = good.clone();
        bad.instance_id = InstanceId("old".into());
        assert!(!project_pool_observation(
            &mut snapshot,
            &runtime,
            &pool,
            Ok(Some(bad))
        ));
        let mut bad = good.clone();
        bad.config.0.revision += 1;
        assert!(!project_pool_observation(
            &mut snapshot,
            &runtime,
            &pool,
            Ok(Some(bad))
        ));
        let mut bad = good.clone();
        bad.config.0.epoch = StateEpoch::fresh().unwrap();
        assert!(!project_pool_observation(
            &mut snapshot,
            &runtime,
            &pool,
            Ok(Some(bad))
        ));
        let mut bad = good.clone();
        bad.pool_id = PoolId("other".into());
        assert!(!project_pool_observation(
            &mut snapshot,
            &runtime,
            &pool,
            Ok(Some(bad))
        ));
        let mut changed = runtime.clone();
        changed.runtime.saved_version.0.revision += 1;
        assert!(!project_pool_observation(
            &mut snapshot,
            &changed,
            &pool,
            Ok(Some(good))
        ));
        assert_eq!(snapshot.entries[0].current, None);
    }

    // 纯桌面偏好保存推进 Saved，不得撤掉同一实例的正式 Applied 出口/成员。
    #[test]
    fn saved_preferences_preserve_current_applied_projection() {
        let (mut snapshot, mut runtime, pool) = applied_pool();
        let applied = runtime.runtime.applied_version.clone().unwrap();
        runtime.runtime.saved_version.0.revision += 1;
        snapshot.version.config = runtime.runtime.saved_version.clone();
        snapshot.runtime = runtime.clone();
        let members = snapshot.entries[0].applied_members.clone();
        let observation = AppliedPoolObservation {
            instance_id: runtime.runtime.instance_id.clone().unwrap(),
            config: applied,
            pool_id: pool.clone(),
            current: Some(OutboundId::Block),
        };
        assert!(project_pool_observation(
            &mut snapshot,
            &runtime,
            &pool,
            Ok(Some(observation.clone()))
        ));
        assert_eq!(snapshot.entries[0].current, Some(OutboundId::Block));
        assert_eq!(snapshot.entries[0].applied_members, members);
        let mut wrong = observation;
        wrong.config = snapshot.version.config.clone();
        assert!(!project_pool_observation(
            &mut snapshot,
            &runtime,
            &pool,
            Ok(Some(wrong))
        ));
    }

    // 宽度/排序保存只重排当前卡片；不能使当前出口消失，也不能放行真实配置或实例替换。
    #[test]
    fn proxy_view_save_keeps_cards_without_accepting_other_state_changes() {
        let (mut snapshot, mut runtime, _) = applied_pool();
        snapshot.entries[0].current = Some(OutboundId::Block);
        let members = snapshot.entries[0].applied_members.clone();
        let history = snapshot.entries[0].history.clone();
        let mut before = veyra_core::domain::AppState::empty();
        before.state_epoch = snapshot.version.config.0.epoch.clone();
        before.config_revision = snapshot.version.config.0.revision;
        let mut saved = before.clone();
        saved.config_revision += 1;
        saved.app_config.behavior.proxy_view.node_card_min_width = 300;
        assert!(super::only_proxy_view_changed(&before, &saved));
        let mut other = saved.clone();
        other.app_config.behavior.latency.timeout_ms += 1;
        assert!(!super::only_proxy_view_changed(&before, &other));
        other = saved.clone();
        other.selection_revision += 1;
        assert!(!super::only_proxy_view_changed(&before, &other));
        other = saved.clone();
        other.state_epoch = StateEpoch::fresh().unwrap();
        assert!(!super::only_proxy_view_changed(&before, &other));
        // Runtime 还没回读新 Saved 时不能提前改其事实。
        assert!(!super::rebase_proxy_view_snapshot(
            &mut snapshot,
            &runtime,
            &saved.config_version()
        ));
        let old_config = snapshot.version.config.clone();
        runtime.runtime.saved_version = saved.config_version();
        // 选择读回仍在途时，宽度保存不能取消它并留下旧高亮/current。
        let mut selected = runtime.clone();
        selected.saved_selection_version.0.revision += 1;
        assert!(!super::rebase_proxy_view_snapshot(
            &mut snapshot,
            &selected,
            &saved.config_version()
        ));
        let mut replaced = runtime.clone();
        replaced.runtime.instance_id = Some(InstanceId("replacement".into()));
        assert!(!super::rebase_proxy_view_snapshot(
            &mut snapshot,
            &replaced,
            &saved.config_version()
        ));
        assert_eq!(snapshot.version.config, old_config);
        assert!(super::rebase_proxy_view_snapshot(
            &mut snapshot,
            &runtime,
            &saved.config_version()
        ));
        let mut displayed = Some(snapshot);
        super::discard_stale_snapshot(&mut displayed, Some(&runtime));
        let kept = displayed.unwrap();
        assert_eq!(kept.entries[0].current, Some(OutboundId::Block));
        assert_eq!(kept.entries[0].applied_members, members);
        assert_eq!(kept.entries[0].history, history);
        assert_eq!(kept.version.config, saved.config_version());
    }

    // 保护停止/替换/保存后的读取失败：异步读之前已撤掉旧 current、selected 与 applied 成员。
    #[test]
    fn identity_change_discards_display_before_a_failed_reload() {
        let (mut snapshot, runtime, _) = applied_pool();
        snapshot.entries[0].current = Some(OutboundId::Direct);
        for change in 0..4 {
            let mut changed = runtime.clone();
            match change {
                0 => changed.runtime.status = RuntimeStatus::Stopped,
                1 => changed.runtime.instance_id = Some(InstanceId("replacement".into())),
                2 => changed.runtime.saved_version.0.revision += 1,
                _ => changed.runtime.applied_version = None,
            }
            let mut displayed = Some(snapshot.clone());
            super::discard_stale_snapshot(&mut displayed, Some(&changed));
            // load 失败只报告 error；旧快照在发请求前就必须不存在。
            assert!(displayed.is_none());
        }
        let mut displayed = Some(snapshot);
        super::discard_stale_snapshot(&mut displayed, Some(&runtime));
        assert!(displayed.is_some());
    }
    // React的zh-CN名称排序，与三种UI语言无关；保护中文混合节点排序。
    #[test]
    #[cfg(target_os = "macos")]
    fn chinese_names_use_explicit_zh_cn_collation() {
        assert_eq!(chinese_compare("北京", "上海"), std::cmp::Ordering::Less);
        assert_eq!(chinese_compare("北京", "北京"), std::cmp::Ordering::Equal);
    }
    // typed内置出口无display_name；不能让名称排序把可见“直连/拒绝”当空字符串。
    #[test]
    #[cfg(target_os = "macos")]
    fn node_name_sort_includes_builtin_controller_names() {
        let (snapshot, _, _) = applied_pool();
        let template = snapshot.entries[0].clone();
        let entries = [
            (OutboundId::Direct, None),
            (OutboundId::Block, None),
            (OutboundId::Pool(PoolId("auto".into())), Some("所有-自动")),
            (OutboundId::Pool(PoolId("manual".into())), Some("嵌套-手动")),
            (OutboundId::Pool(PoolId("mixed".into())), Some("混合-自动")),
        ]
        .map(|(id, name)| {
            let mut entry = template.clone();
            entry.outbound.id = id;
            entry.outbound.display_name = name.map(str::to_owned);
            entry
        });
        let mut sorted = entries.iter().collect();
        super::sort_nodes(&mut sorted, NodeSort::NameDesc);
        assert_eq!(
            sorted
                .iter()
                .map(|e| e.outbound.id.clone())
                .collect::<Vec<_>>(),
            [0, 2, 3, 1, 4].map(|i| entries[i].outbound.id.clone())
        );
    }
    // 旧 HTTP 拒绝后用同一 i18n source 提醒手动改设置，不声称已经更改 URL。
    #[test]
    fn unsupported_http_error_points_to_settings_in_all_languages() {
        use veyra_core::{application::network::NetworkError, domain::DesktopLanguage};
        let key = super::network_error(&NetworkError::UnsupportedNodeTestUrl);
        for (language, expected) in [
            (
                DesktopLanguage::SimplifiedChinese,
                "节点测速仅支持 HTTPS，请到设置中修改网址；原设置未改",
            ),
            (
                DesktopLanguage::TraditionalChinese,
                "節點測速僅支援 HTTPS，請到設定中修改網址；原設定未改",
            ),
            (
                DesktopLanguage::English,
                "Node latency tests require HTTPS. Change the URL in Settings; your setting is unchanged",
            ),
        ] {
            assert_eq!(crate::ui::i18n::translate(language, key), expected);
        }
    }
    #[test]
    fn search_requires_all_keywords() {
        assert!(matches_query("上海 Example 01", "example 上海"));
        assert!(!matches_query("上海 Example 01", "example 北京"));
    }
    // 节点按可见协议检索不能变成空态；不猜测未知 UDP 能力。
    #[test]
    fn search_matches_protocol_and_reference_alias() {
        use veyra_core::domain::{OutboundKind, ProxyProtocol};
        assert!(matches_node_query(
            "node-a",
            Some(ProxyProtocol::Http),
            OutboundKind::Node,
            None,
            "http a"
        ));
        assert!(matches_node_query(
            "node-b",
            Some(ProxyProtocol::Hysteria2),
            OutboundKind::Node,
            None,
            "hy2"
        ));
        assert!(matches_node_query(
            "node-a",
            Some(ProxyProtocol::Http),
            OutboundKind::Node,
            Some(true),
            "a http udp"
        ));
        assert!(matches_node_query(
            "直连",
            None,
            OutboundKind::Direct,
            Some(true),
            "direct udp"
        ));
        assert!(!matches_node_query(
            "node-a",
            Some(ProxyProtocol::Http),
            OutboundKind::Node,
            Some(false),
            "udp"
        ));
        assert!(!matches_node_query(
            "node-a",
            Some(ProxyProtocol::Http),
            OutboundKind::Node,
            None,
            "udp"
        ));
    }
}

#[cfg(all(test, target_os = "macos"))]
mod native_tests;
