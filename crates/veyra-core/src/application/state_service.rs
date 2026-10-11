use super::state_access::StateAccessGate;
use crate::{
    domain::{
        AppError, AppErrorCode, AppState, BehaviorField, ConfigVersion, ErrorDetail, FieldPath,
        NodeId, PoolId, Profile, ProfilePatch, Provider, ProxyNode, SelectionPolicy,
        SelectionVersion, SnapshotVersion, StateEpoch, Subscription,
    },
    storage::{JsonStateStore, StateStore},
};
use serde::Serialize;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum ApplyEffect {
    /// Persisted and statically validated; no current runtime application is proven.
    SavedOnly,
    /// A runtime owner confirmed application to the current instance.
    Applied,
    /// A runtime owner confirmed that this saved change needs a restart.
    RestartRequired,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SaveOutcome<T, V> {
    pub value: T,
    pub version: V,
    pub effect: ApplyEffect,
}

/// Already normalized subscription facts. Import parsing/unknown-field reports and collision
/// remapping belong to the later backup importer; duplicate stable IDs are rejected here.
#[derive(Clone, Debug, Default)]
pub struct SubscriptionAppend {
    pub subscriptions: Vec<Subscription>,
    pub providers: Vec<Provider>,
    pub nodes: Vec<ProxyNode>,
}

/// Sole snapshot transaction boundary; shares the legacy gate and existing JSON store.
#[derive(Clone)]
pub struct SnapshotService {
    store: JsonStateStore,
    gate: StateAccessGate,
}
impl SnapshotService {
    pub fn new(store: JsonStateStore, gate: StateAccessGate) -> Self {
        Self { store, gate }
    }
    fn lock(&self) -> Result<std::sync::MutexGuard<'_, ()>, AppError> {
        self.gate.try_lock().map_err(|_error| {
            // 仅诊断原失败：区分争用 Busy 与锁失效 Unavailable，不改既有错误映射或重读。
            #[cfg(debug_assertions)]
            eprintln!(
                "state-access-lock error={_error:?} thread={:?}",
                std::thread::current().id()
            );
            AppError::new(AppErrorCode::StorageFailed).with_detail(ErrorDetail::Busy)
        })
    }
    fn load_or_initialize(&self) -> Result<AppState, AppError> {
        if self.store.has_snapshot_or_backup()? {
            return self.store.load().map_err(Into::into);
        }
        let state = crate::domain::builtin_defaults::new_builtin_state()?;
        self.store.save(&state)?;
        Ok(state)
    }
    /// P2-06交接只在writer关闭后持有此短gate复制有界材料；不跨child生命周期。
    pub(crate) fn with_runtime_version<T>(
        &self,
        expected: &crate::domain::SnapshotVersion,
        action: impl FnOnce() -> T,
    ) -> Result<T, AppError> {
        let _guard = self.lock()?;
        let state = self.load_or_initialize()?;
        state.config_version().0.require(&expected.config.0)?;
        state.selection_version().0.require(&expected.selection.0)?;
        Ok(action())
    }
    /// 短gate只保护本地落盘；RPC期间靠持久fence阻止所有JsonStateStore writer。
    pub fn begin_remote_selection(
        &self,
        request: &crate::storage::RemoteSelection,
    ) -> Result<crate::storage::SelectionFence, AppError> {
        let _guard = self.lock()?;
        self.store.begin_remote(request).map_err(Into::into)
    }
    pub fn remote_selection_fence(
        &self,
    ) -> Result<Option<crate::storage::SelectionFence>, AppError> {
        self.store.selection_fence().map_err(Into::into)
    }
    pub(crate) fn confirm_remote_selection(
        &self,
        fence: &crate::storage::SelectionFence,
        actual: &NodeId,
    ) -> Result<AppState, AppError> {
        let _guard = self.lock()?;
        self.store.confirm_remote(fence, actual).map_err(Into::into)
    }
    pub(crate) fn finish_remote_selection(
        &self,
        fence: &crate::storage::SelectionFence,
        confirmed: &SnapshotVersion,
    ) -> Result<(), AppError> {
        let _guard = self.lock()?;
        self.store
            .finish_remote(fence, confirmed)
            .map_err(Into::into)
    }
    pub fn snapshot(&self) -> Result<AppState, AppError> {
        let _guard = self.lock()?;
        self.load_or_initialize()
    }
    /// PUT/GET 与配置、选择两个 CAS 同处短 writer gate，防止成员保存插入间隙。
    pub(crate) fn with_controller_write<T>(
        &self,
        config: &ConfigVersion,
        selection: &SelectionVersion,
        write: impl FnOnce() -> T,
    ) -> Result<T, AppError> {
        let _guard = self.lock()?;
        let _writer = self.store.lock_unfenced_writer()?;
        let state = self.load_or_initialize()?;
        state.config_version().0.require(&config.0)?;
        state.selection_version().0.require(&selection.0)?;
        Ok(write())
    }
    pub fn append(
        &self,
        expected: ConfigVersion,
        batch: SubscriptionAppend,
    ) -> Result<SaveOutcome<AppState, SnapshotVersion>, AppError> {
        let _guard = self.lock()?;
        let mut next = self.load_or_initialize()?;
        next.config_version().0.require(&expected.0)?;
        next.subscriptions.extend(batch.subscriptions);
        next.providers.extend(batch.providers);
        next.nodes.extend(batch.nodes);
        next.profile.validate()?;
        next.validate()
            .map_err(|_| AppError::validation(FieldPath::Snapshot))?;
        let saved = self.store.commit(&next)?;
        Ok(SaveOutcome {
            version: saved.version(),
            value: saved,
            effect: ApplyEffect::SavedOnly,
        })
    }
    /// 组编辑器保存唯一业务快照；不启动内核，不覆盖并发偏好/选择。
    pub fn save_groups(
        &self,
        expected: ConfigVersion,
        groups: Vec<crate::domain::NodeGroup>,
    ) -> Result<GroupSaveOutcome, GroupSaveError> {
        let _guard = self.lock().map_err(GroupSaveError::Storage)?;
        let mut next = self.load_or_initialize().map_err(GroupSaveError::Storage)?;
        next.config_version()
            .0
            .require(&expected.0)
            .map_err(GroupSaveError::Storage)?;
        for group in &groups {
            if group.rule == crate::domain::GroupRule::Failover
                && group.lanes.len() < 2
                && !next
                    .groups
                    .iter()
                    .any(|old| old.id == group.id && old.rule == crate::domain::GroupRule::Failover)
            {
                return Err(GroupSaveError::Invalid(
                    crate::domain::GroupIssue::InvalidSettings(group.id.clone()),
                ));
            }
        }
        let old_selectors = next.runtime_groups();
        next.groups = groups;
        // 未知 pending 必须仍可按实际 Controller 值核对；拒绝破坏它的配置编辑。
        let selectors = next.runtime_groups();
        for (id, selection) in &next.group_selections {
            if let Some(pending) = &selection.pending
                && !selectors.iter().any(|g| {
                    g.id == *id
                        && matches!(g.selection, SelectionPolicy::Manual { .. })
                        && g.members.contains(&pending.member)
                        // 保留旧确认/编译默认的完整成员与顺序，不能把未知旧值变成第三值。
                        && old_selectors.iter().any(|old| old.id == *id && old == g)
                })
            {
                return Err(GroupSaveError::Invalid(
                    crate::domain::GroupIssue::InvalidSettings(id.clone()),
                ));
            }
        }
        next.group_selections.retain(|id, selection| {
            let Some(selector) = selectors
                .iter()
                .find(|g| g.id == *id && matches!(g.selection, SelectionPolicy::Manual { .. }))
            else {
                return false;
            };
            if selection
                .selected
                .as_ref()
                .is_some_and(|m| !selector.members.contains(m))
            {
                selection.selected = None;
                selection.mode = crate::domain::GroupSelectionMode::Auto;
            }
            true
        });
        next.validate_groups().map_err(GroupSaveError::Invalid)?;
        next.validate()
            .map_err(|_| GroupSaveError::Storage(AppError::validation(FieldPath::Snapshot)))?;
        let saved = self
            .store
            .commit(&next)
            .map_err(|e| GroupSaveError::Storage(e.into()))?;
        Ok(GroupSaveOutcome {
            version: saved.version(),
            dropped: saved.dropped_groups(),
            value: saved,
            effect: ApplyEffect::SavedOnly,
        })
    }
    /// 目标分流 CRUD/命中排序保存同一 ProfileRouting 数组；只反馈 Saved，不驱动 Runtime。
    pub fn save_routing(
        &self,
        expected: ConfigVersion,
        routing: crate::domain::ProfileRouting,
    ) -> Result<SaveOutcome<AppState, ConfigVersion>, AppError> {
        self.save_routing_with_order(expected, routing, None)
    }
    /// 只恢复目标分流及其显示顺序；保留订阅、出站、其它Profile和偏好。
    pub fn restore_target_routing(
        &self,
        expected: ConfigVersion,
    ) -> Result<SaveOutcome<AppState, ConfigVersion>, AppError> {
        self.save_routing_with_order(
            expected,
            crate::domain::builtin_defaults::default_target_routing(),
            Some(crate::domain::builtin_defaults::default_strategy_order()),
        )
    }
    fn save_routing_with_order(
        &self,
        expected: ConfigVersion,
        routing: crate::domain::ProfileRouting,
        restored_order: Option<Vec<String>>,
    ) -> Result<SaveOutcome<AppState, ConfigVersion>, AppError> {
        let _guard = self.lock()?;
        let mut next = self.load_or_initialize()?;
        next.config_version().0.require(&expected.0)?;
        let old_selectors = next.runtime_groups();
        let old_policy_ids = next
            .profile
            .routing
            .selectors()
            .map(|p| p.id.clone())
            .collect::<std::collections::HashSet<_>>();
        next.profile.routing = routing;
        let selectors = next.runtime_groups();
        for (id, selection) in &next.group_selections {
            if selection.pending.is_some()
                && !selectors
                    .iter()
                    .any(|g| &g.id == id && old_selectors.iter().any(|old| old == g))
            {
                return Err(AppError::validation(FieldPath::Routing));
            }
        }
        if let Some(order) = restored_order {
            let reset_ids = next
                .profile
                .routing
                .selectors()
                .map(|p| p.id.clone())
                .collect::<std::collections::HashSet<_>>();
            // 即使重复恢复同一份配置，也不能清掉尚未确认的 Controller 选择。
            if next.group_selections.iter().any(|(id, selection)| {
                selection.pending.is_some()
                    && (old_policy_ids.contains(id) || reset_ids.contains(id))
            }) {
                return Err(AppError::validation(FieldPath::Routing));
            }
            next.group_selections
                .retain(|id, _| !old_policy_ids.contains(id) && !reset_ids.contains(id));
            next.app_config.behavior.proxy_view.strategy_order = order;
        }
        next.group_selections.retain(|id, selection| {
            let Some(selector) = selectors.iter().find(|g| &g.id == id) else {
                return false;
            };
            if selection
                .selected
                .as_ref()
                .is_some_and(|m| !selector.members.contains(m))
            {
                selection.selected = None;
                selection.mode = crate::domain::GroupSelectionMode::Auto;
            }
            true
        });
        next.validate()
            .map_err(|_| AppError::validation(FieldPath::Routing))?;
        let saved = self.store.commit(&next)?;
        Ok(SaveOutcome {
            version: saved.config_version(),
            // 同一 gate 返回已提交权威快照；UI 不再二次读取并把 Busy 冒充保存失败。
            value: saved,
            effect: ApplyEffect::SavedOnly,
        })
    }
    /// 分享配置使用同一个 CAS/原子快照，监听与连接从不进入持久事实。
    /// 五种入站只保存配置；启用表示参与下次编译，不声称 Runtime 已经 Applied。
    pub fn edit_shared_server(
        &self,
        expected: ConfigVersion,
        command: super::shared_inbounds::SharedServerCommand,
    ) -> Result<SaveOutcome<AppState, SnapshotVersion>, AppError> {
        use super::shared_inbounds::SharedServerCommand;
        let _guard = self.lock()?;
        let mut next = self.load_or_initialize()?;
        next.config_version().0.require(&expected.0)?;
        let servers = &mut next.app_config.shared_servers;
        let invalid = || AppError::validation(FieldPath::Snapshot);
        match command {
            SharedServerCommand::Create(value) => {
                if servers.iter().any(|s| s.id == value.id) {
                    return Err(invalid());
                }
                servers.push(value);
            }
            SharedServerCommand::Update(value) => {
                let old = servers
                    .iter_mut()
                    .find(|s| s.id == value.id)
                    .ok_or_else(invalid)?;
                *old = value;
            }
            SharedServerCommand::SetEnabled { id, enabled } => {
                servers
                    .iter_mut()
                    .find(|s| s.id == id)
                    .ok_or_else(invalid)?
                    .enabled = enabled;
            }
            SharedServerCommand::Reorder(ids) => {
                if ids.len() != servers.len() {
                    return Err(invalid());
                }
                let mut ordered = Vec::with_capacity(ids.len());
                for id in ids {
                    let index = servers
                        .iter()
                        .position(|s| s.id == id)
                        .ok_or_else(invalid)?;
                    ordered.push(servers.remove(index));
                }
                *servers = ordered;
            }
            SharedServerCommand::Delete { id } => {
                let index = servers
                    .iter()
                    .position(|s| s.id == id)
                    .ok_or_else(invalid)?;
                servers.remove(index);
            }
        }
        next.validate().map_err(|_| invalid())?;
        let saved = self.store.commit(&next)?;
        Ok(SaveOutcome {
            version: saved.version(),
            value: saved,
            effect: ApplyEffect::SavedOnly,
        })
    }
    /// 分享 HTTP 配置使用同一个 CAS/原子快照。
    pub fn save_shares(
        &self,
        expected: ConfigVersion,
        shares: Vec<crate::domain::SubscriptionShare>,
    ) -> Result<AppState, AppError> {
        let _guard = self.lock()?;
        let mut next = self.load_or_initialize()?;
        next.config_version().0.require(&expected.0)?;
        next.app_config.subscription_shares = shares;
        next.validate()
            .map_err(|_| AppError::validation(FieldPath::Snapshot))?;
        self.store.commit(&next).map_err(Into::into)
    }
    /// Restores business facts only. Imported concurrency tokens are discarded; both counters
    /// start at zero in the new epoch. A mismatch on either current counter rejects replacement.
    pub fn replace(
        &self,
        expected: SnapshotVersion,
        mut replacement: AppState,
    ) -> Result<SaveOutcome<AppState, SnapshotVersion>, AppError> {
        let _guard = self.lock()?;
        let current = self.load_or_initialize()?;
        current.config_version().0.require(&expected.config.0)?;
        current
            .selection_version()
            .0
            .require(&expected.selection.0)?;
        // 新 state space 不携带旧实例尚未确认的意图。
        for pool in &mut replacement.pools {
            if let SelectionPolicy::Manual {
                pending_node_id, ..
            } = &mut pool.selection
            {
                *pending_node_id = None;
            }
        }
        for selection in replacement.group_selections.values_mut() {
            selection.pending = None;
        }
        replacement.profile.validate()?;
        replacement.state_epoch = StateEpoch::fresh()?;
        replacement.config_revision = 0;
        replacement.selection_revision = 0;
        replacement
            .validate()
            .map_err(|_| AppError::validation(FieldPath::Snapshot))?;
        self.store.replace_at(&expected, &replacement)?;
        Ok(SaveOutcome {
            version: replacement.version(),
            value: replacement,
            effect: ApplyEffect::SavedOnly,
        })
    }
}

pub struct ProfileService {
    snapshots: SnapshotService,
}
impl ProfileService {
    pub fn new(snapshots: SnapshotService) -> Self {
        Self { snapshots }
    }
    pub fn patch(
        &self,
        expected: ConfigVersion,
        patch: ProfilePatch,
    ) -> Result<SaveOutcome<Profile, ConfigVersion>, AppError> {
        let _guard = self.snapshots.lock()?;
        let mut next = self.snapshots.load_or_initialize()?;
        next.config_version().0.require(&expected.0)?;
        next.profile = next.profile.patched(patch)?;
        next.validate()
            .map_err(|_| AppError::validation(FieldPath::RoutingCustomRules))?;
        let saved = self.snapshots.store.commit(&next)?;
        // This card only proves persistence. No runtime/check/manifest side effect is claimed.
        Ok(SaveOutcome {
            value: saved.profile.clone(),
            version: saved.config_version(),
            effect: ApplyEffect::SavedOnly,
        })
    }
}

/// Persists manual selection intent. Runtime control/confirmation is owned by P2;
/// this service never claims that a controller accepted the intent.
pub struct SelectionService {
    snapshots: SnapshotService,
}
enum ManualStage {
    Begin,
    Confirm,
    Clear,
}
impl SelectionService {
    pub fn new(snapshots: SnapshotService) -> Self {
        Self { snapshots }
    }
    pub fn select_manual(
        &self,
        expected: SelectionVersion,
        pool_id: PoolId,
        node_id: Option<NodeId>,
    ) -> Result<SaveOutcome<Option<NodeId>, SelectionVersion>, AppError> {
        let _guard = self.snapshots.lock()?;
        let mut next = self.snapshots.load_or_initialize()?;
        next.selection_version().0.require(&expected.0)?;
        let pool = next
            .pools
            .iter_mut()
            .find(|v| v.id == pool_id)
            .ok_or_else(|| AppError::validation(FieldPath::Selection))?;
        let SelectionPolicy::Manual {
            selected_node_id,
            pending_node_id,
        } = &mut pool.selection
        else {
            return Err(AppError::validation(FieldPath::Selection));
        };
        if pending_node_id.is_some() {
            return Err(AppError::validation(FieldPath::Selection));
        }
        *selected_node_id = node_id.clone();
        next.validate()
            .map_err(|_| AppError::validation(FieldPath::Selection))?;
        let saved = self.snapshots.store.commit(&next)?;
        Ok(SaveOutcome {
            value: node_id,
            version: saved.selection_version(),
            effect: ApplyEffect::SavedOnly,
        })
    }
    /// 先持久化意图，保持最后确认值；拒绝覆盖尚未核对的请求。
    pub fn begin_manual_pending(
        &self,
        expected: SelectionVersion,
        pool: PoolId,
        requested: NodeId,
    ) -> Result<SaveOutcome<NodeId, SelectionVersion>, AppError> {
        self.stage_manual(expected, pool, requested, ManualStage::Begin, None)
    }
    /// 只有同版本、同 pending 请求才允许提交 controller 的确认结果。
    pub fn confirm_manual_pending(
        &self,
        expected: SelectionVersion,
        pool: PoolId,
        requested: NodeId,
    ) -> Result<SaveOutcome<NodeId, SelectionVersion>, AppError> {
        self.stage_manual(expected, pool, requested, ManualStage::Confirm, None)
    }
    /// 调用者须已读回旧 confirmed；CAS 防止清掉另一请求。
    pub fn clear_manual_pending(
        &self,
        expected: SelectionVersion,
        pool: PoolId,
        requested: NodeId,
    ) -> Result<SaveOutcome<NodeId, SelectionVersion>, AppError> {
        self.stage_manual(expected, pool, requested, ManualStage::Clear, None)
    }
    /// Group/lane 的 pending/confirm/clear 使用同一快照 CAS；mode 只在 confirm 后生效。
    pub(crate) fn stage_group(
        &self,
        config: ConfigVersion,
        expected: SelectionVersion,
        selector: PoolId,
        requested: crate::domain::GroupSelectionIntent,
        confirm: Option<bool>,
    ) -> Result<SelectionVersion, AppError> {
        let _guard = self.snapshots.lock()?;
        let mut next = self.snapshots.load_or_initialize()?;
        next.config_version().0.require(&config.0)?;
        next.selection_version().0.require(&expected.0)?;
        if !next.runtime_groups().iter().any(|g| {
            g.id == selector
                && g.members.contains(&requested.member)
                && matches!(g.selection, SelectionPolicy::Manual { .. })
        }) {
            return Err(AppError::validation(FieldPath::Snapshot));
        }
        if confirm.is_none()
            && (next.group_selections.values().any(|s| s.pending.is_some())
                || next.pools.iter().any(|p| {
                    matches!(
                        p.selection,
                        SelectionPolicy::Manual {
                            pending_node_id: Some(_),
                            ..
                        }
                    )
                }))
        {
            return Err(AppError::new(AppErrorCode::RevisionConflict));
        }
        let entry = next.group_selections.entry(selector).or_default();
        if let Some(confirm) = confirm {
            if entry.pending.as_ref() != Some(&requested) {
                return Err(AppError::new(AppErrorCode::RevisionConflict));
            }
            if confirm {
                entry.selected = Some(requested.member);
                entry.mode = requested.mode;
            }
            entry.pending = None;
        } else {
            entry.pending = Some(requested);
        }
        let saved = self.snapshots.store.commit(&next)?;
        Ok(saved.selection_version())
    }
    pub(crate) fn stage_node(
        &self,
        config: &ConfigVersion,
        expected: SelectionVersion,
        pool: PoolId,
        node: NodeId,
        confirm: Option<bool>,
    ) -> Result<SaveOutcome<NodeId, SelectionVersion>, AppError> {
        self.stage_manual(
            expected,
            pool,
            node,
            match confirm {
                None => ManualStage::Begin,
                Some(true) => ManualStage::Confirm,
                Some(false) => ManualStage::Clear,
            },
            Some(config),
        )
    }
    fn stage_manual(
        &self,
        expected: SelectionVersion,
        pool_id: PoolId,
        requested: NodeId,
        stage: ManualStage,
        config: Option<&ConfigVersion>,
    ) -> Result<SaveOutcome<NodeId, SelectionVersion>, AppError> {
        let _guard = self.snapshots.lock()?;
        let mut next = self.snapshots.load_or_initialize()?;
        next.selection_version().0.require(&expected.0)?;
        if let Some(config) = config {
            next.config_version().0.require(&config.0)?;
        }
        let pool = next
            .pools
            .iter_mut()
            .find(|p| p.id == pool_id)
            .ok_or_else(|| AppError::validation(FieldPath::Selection))?;
        let SelectionPolicy::Manual {
            selected_node_id,
            pending_node_id,
        } = &mut pool.selection
        else {
            return Err(AppError::validation(FieldPath::Selection));
        };
        match stage {
            ManualStage::Begin if pending_node_id.is_none() => {
                *pending_node_id = Some(requested.clone())
            }
            ManualStage::Confirm | ManualStage::Clear
                if pending_node_id.as_ref() == Some(&requested) =>
            {
                if matches!(stage, ManualStage::Confirm) {
                    *selected_node_id = Some(requested.clone());
                }
                *pending_node_id = None;
            }
            _ => return Err(AppError::validation(FieldPath::Selection)),
        }
        next.validate()
            .map_err(|_| AppError::validation(FieldPath::Selection))?;
        let saved = self.snapshots.store.commit(&next)?;
        Ok(SaveOutcome {
            value: requested,
            version: saved.selection_version(),
            effect: ApplyEffect::SavedOnly,
        })
    }
}

#[cfg(test)]
mod tests;

/// Typed local visual preference transaction. Never applies runtime configuration.
#[derive(Clone)]
pub struct DesktopPreferencesService {
    snapshots: SnapshotService,
    behavior_changes: tokio::sync::broadcast::Sender<BehaviorChange>,
}
impl DesktopPreferencesService {
    pub fn new(snapshots: SnapshotService) -> Self {
        Self {
            snapshots,
            behavior_changes: tokio::sync::broadcast::channel(64).0,
        }
    }
    pub fn save(
        &self,
        expected: ConfigVersion,
        visual: crate::domain::DesktopVisualPreferences,
    ) -> Result<SaveOutcome<AppState, ConfigVersion>, AppError> {
        visual.validate()?;
        let _guard = self.snapshots.lock()?;
        let mut next = self.snapshots.load_or_initialize()?;
        next.config_version().0.require(&expected.0)?;
        next.app_config.visual = visual;
        let saved = self.snapshots.store.commit(&next)?;
        Ok(SaveOutcome {
            version: saved.config_version(),
            value: saved,
            effect: ApplyEffect::SavedOnly,
        })
    }
}

/// Latest typed behavior value plus exact changed fields. No JSON or runtime effect.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BehaviorChange {
    pub version: ConfigVersion,
    pub value: crate::domain::DesktopBehaviorPreferences,
    pub fields: Vec<crate::domain::BehaviorField>,
}
impl DesktopPreferencesService {
    pub fn behavior_snapshot(&self) -> Result<BehaviorChange, AppError> {
        let state = self.snapshots.snapshot()?;
        Ok(BehaviorChange {
            version: state.config_version(),
            value: state.app_config.behavior,
            fields: vec![],
        })
    }
    pub fn subscribe_behavior(&self) -> tokio::sync::broadcast::Receiver<BehaviorChange> {
        self.behavior_changes.subscribe()
    }
    pub fn patch_behavior(
        &self,
        expected: ConfigVersion,
        patch: crate::domain::DesktopBehaviorPreferencesPatch,
    ) -> Result<SaveOutcome<AppState, ConfigVersion>, AppError> {
        let _guard = self.snapshots.lock()?;
        let mut next = self.snapshots.load_or_initialize()?;
        next.config_version().0.require(&expected.0)?;
        let previous = next.app_config.behavior.clone();
        next.app_config.behavior = patch.apply(&previous);
        next.app_config.behavior.validate()?;
        let fields = crate::domain::DesktopBehaviorPreferencesPatch::between(
            &previous,
            &next.app_config.behavior,
        )
        .changed_fields();
        let saved = self.snapshots.store.commit(&next)?;
        if !fields.is_empty() {
            // Under the same gate as commit, so concurrent writers cannot reorder notifications.
            let _ = self.behavior_changes.send(BehaviorChange {
                version: saved.config_version(),
                value: saved.app_config.behavior.clone(),
                fields,
            });
        }
        Ok(SaveOutcome {
            version: saved.config_version(),
            value: saved,
            effect: ApplyEffect::SavedOnly,
        })
    }
}

#[cfg(test)]
mod behavior_tests {
    use super::*;
    use crate::domain::*;
    fn services() -> (
        std::path::PathBuf,
        SnapshotService,
        DesktopPreferencesService,
    ) {
        let root = std::env::temp_dir().join(format!(
            "veyra-behavior-{}-{:?}",
            std::process::id(),
            StateEpoch::fresh().unwrap()
        ));
        let snapshots = SnapshotService::new(
            JsonStateStore::new(root.join("state.json")).unwrap(),
            StateAccessGate::default(),
        );
        let prefs = DesktopPreferencesService::new(snapshots.clone());
        (root, snapshots, prefs)
    }
    // 新安装与新 AppState 的 UI 默认 HTTPS 实际落盘/重开；Runtime/group health 不随之改变。
    #[test]
    fn new_ui_latency_https_default_survives_real_store_reopen() {
        let fresh = AppState::try_empty().unwrap();
        assert_eq!(
            fresh.app_config.behavior.latency.test_url.0,
            "https://www.gstatic.com/generate_204"
        );
        let (root, snapshots, preferences) = services();
        let initial = snapshots.snapshot().unwrap(); // 首次生产初始化真实写入 state.json。
        assert_eq!(
            initial.app_config.behavior.latency,
            fresh.app_config.behavior.latency
        );
        assert_eq!(initial.app_config.behavior.latency.timeout_ms, 5000);
        assert_eq!(
            initial.profile.test_url.as_str(),
            "http://www.gstatic.com/generate_204"
        );
        assert_eq!(initial.groups, fresh.groups);
        assert!(initial.groups.is_empty());
        let path = root.join("state.json");
        let bytes = std::fs::read(&path).unwrap();
        let modified = std::fs::metadata(&path).unwrap().modified().unwrap();
        drop(preferences);
        drop(snapshots);
        let store = JsonStateStore::new(path.clone()).unwrap();
        assert_eq!(store.load().unwrap(), initial);
        let reopened = SnapshotService::new(store, StateAccessGate::default());
        assert_eq!(reopened.snapshot().unwrap(), initial);
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        assert_eq!(
            std::fs::metadata(&path).unwrap().modified().unwrap(),
            modified
        );
        drop(reopened);
        std::fs::remove_dir_all(root).unwrap();
    }

    // 已真正保存的 HTTP 不迁移；读、无关行为 patch 和语言保存后重建服务仍保原值。
    #[test]
    fn saved_http_latency_survives_reopen_unrelated_patch_and_language_save() {
        let (root, snapshots, preferences) = services();
        let path = root.join("state.json");
        let mut old = AppState::try_empty().unwrap();
        old.app_config.behavior.latency.test_url =
            UiLatencyUrl("http://www.gstatic.com/generate_204?user=saved".into());
        old.groups = default_groups();
        old.groups
            .iter_mut()
            .find(|g| g.id == PoolId("all-auto".into()))
            .unwrap()
            .test_url = "http://group.example.invalid/health".into();
        JsonStateStore::new(path.clone())
            .unwrap()
            .save(&old)
            .unwrap();
        drop(preferences);
        drop(snapshots);
        let bytes = std::fs::read(&path).unwrap();
        let modified = std::fs::metadata(&path).unwrap().modified().unwrap();
        let store = JsonStateStore::new(path.clone()).unwrap();
        assert_eq!(store.load().unwrap(), old);
        let reopened = SnapshotService::new(store, StateAccessGate::default());
        assert_eq!(reopened.snapshot().unwrap(), old);
        let preferences = DesktopPreferencesService::new(reopened.clone());
        assert_eq!(
            preferences
                .behavior_snapshot()
                .unwrap()
                .value
                .latency
                .test_url,
            old.app_config.behavior.latency.test_url
        );
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        assert_eq!(
            std::fs::metadata(&path).unwrap().modified().unwrap(),
            modified
        );
        let patched = preferences
            .patch_behavior(
                old.config_version(),
                DesktopBehaviorPreferencesPatch {
                    hide_unavailable: Some(true),
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(patched.effect, ApplyEffect::SavedOnly);
        let mut expected = old.clone();
        expected.app_config.behavior.proxy_view.hide_unavailable = true;
        expected.config_revision += 1;
        assert_eq!(patched.value, expected);
        for language in [
            DesktopLanguage::English,
            DesktopLanguage::TraditionalChinese,
        ] {
            let mut visual = expected.app_config.visual.clone();
            visual.language = language;
            let saved = preferences
                .save(expected.config_version(), visual.clone())
                .unwrap();
            expected.app_config.visual = visual;
            expected.config_revision += 1;
            assert_eq!(saved.value, expected);
        }
        drop(preferences);
        drop(reopened);
        let bytes = std::fs::read(&path).unwrap();
        let modified = std::fs::metadata(&path).unwrap().modified().unwrap();
        let store = JsonStateStore::new(path.clone()).unwrap();
        assert_eq!(store.load().unwrap(), expected);
        let reopened = SnapshotService::new(store, StateAccessGate::default());
        let restored = reopened.snapshot().unwrap();
        assert_eq!(restored, expected);
        assert_eq!(
            restored.app_config.behavior.latency,
            old.app_config.behavior.latency
        );
        assert_eq!(restored.profile, old.profile);
        assert_eq!(restored.groups, old.groups);
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        assert_eq!(
            std::fs::metadata(&path).unwrap().modified().unwrap(),
            modified
        );
        drop(reopened);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn save_reload_versions_noop_and_stale_cas() {
        let (root, s, p) = services();
        let before = s.snapshot().unwrap();
        let out = p
            .patch_behavior(
                before.config_version(),
                DesktopBehaviorPreferencesPatch {
                    timeout_ms: Some(9000),
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(out.effect, ApplyEffect::SavedOnly);
        assert_eq!(out.value.config_revision, before.config_revision + 1);
        assert_eq!(out.value.selection_revision, before.selection_revision);
        assert_eq!(out.value.state_epoch, before.state_epoch);
        assert_eq!(out.value.profile, before.profile);
        assert_eq!(out.value.app_config.visual, before.app_config.visual);
        assert_eq!(
            out.value.app_config.check_updates_on_start,
            before.app_config.check_updates_on_start
        );
        assert_eq!(s.snapshot().unwrap(), out.value);
        assert_eq!(
            p.patch_behavior(
                out.version.clone(),
                DesktopBehaviorPreferencesPatch {
                    timeout_ms: Some(9000),
                    ..Default::default()
                }
            )
            .unwrap()
            .version,
            out.version
        );
        assert_eq!(
            p.patch_behavior(before.config_version(), Default::default())
                .unwrap_err()
                .code(),
            AppErrorCode::RevisionConflict
        );
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn visual_and_behavior_interleaving_never_overwrites_other_family() {
        let (root, s, p) = services();
        let initial = s.snapshot().unwrap();
        let mut visual = initial.app_config.visual.clone();
        visual.theme_mode = DesktopThemeMode::Dark;
        let a = p.save(initial.config_version(), visual).unwrap();
        let b = p
            .patch_behavior(
                a.version,
                DesktopBehaviorPreferencesPatch {
                    timeout_ms: Some(8000),
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(b.value.app_config.visual.theme_mode, DesktopThemeMode::Dark);
        let c = p
            .patch_behavior(
                b.version.clone(),
                DesktopBehaviorPreferencesPatch {
                    group_columns: Some(3),
                    ..Default::default()
                },
            )
            .unwrap();
        let mut visual = c.value.app_config.visual;
        visual.global_radius = 9;
        assert_eq!(
            p.save(b.version, visual.clone()).unwrap_err().code(),
            AppErrorCode::RevisionConflict
        );
        let d = p.save(c.version, visual).unwrap();
        assert_eq!(
            (
                d.value.app_config.behavior.latency.timeout_ms,
                d.value.app_config.behavior.proxy_view.group_columns
            ),
            (8000, 3)
        );
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn diagnostics_ipv6_and_profile_ipv6_are_independent_in_both_directions() {
        let (root, s, p) = services();
        let profile = ProfileService::new(s.clone());
        let a = s.snapshot().unwrap();
        let a = profile
            .patch(
                a.config_version(),
                ProfilePatch {
                    ipv6: Patch::Value(false),
                    ..Default::default()
                },
            )
            .unwrap();
        let b = p
            .patch_behavior(
                a.version,
                DesktopBehaviorPreferencesPatch {
                    ipv6_test: Some(true),
                    ..Default::default()
                },
            )
            .unwrap();
        assert!(!b.value.profile.ipv6);
        assert!(b.value.app_config.behavior.diagnostics.ipv6_test);
        profile
            .patch(
                b.version,
                ProfilePatch {
                    ipv6: Patch::Value(true),
                    ..Default::default()
                },
            )
            .unwrap();
        let c = s.snapshot().unwrap();
        assert!(c.profile.ipv6);
        assert!(c.app_config.behavior.diagnostics.ipv6_test);
        std::fs::remove_dir_all(root).unwrap();
    }
    // 保护手输宽度：真实偏好提交/事件/磁盘重建都保留3030等值，且不修改其它配置或选择。
    #[test]
    fn policy85_typed_width_persists_without_legacy_bounds() {
        let (root, snapshots, preferences) = services();
        let mut state = snapshots.snapshot().unwrap();
        let mut events = preferences.subscribe_behavior();
        for width in [1, 99, 321, 3030, 30300, 303030, u32::MAX] {
            let saved = preferences
                .patch_behavior(
                    state.config_version(),
                    DesktopBehaviorPreferencesPatch {
                        node_card_min_width: Some(width),
                        ..Default::default()
                    },
                )
                .unwrap();
            state.app_config.behavior.proxy_view.node_card_min_width = width;
            state.config_revision += 1;
            assert_eq!(saved.value, state);
            let event = events.try_recv().unwrap();
            assert_eq!(event.fields, [BehaviorField::NodeWidth]);
            assert_eq!(event.value.proxy_view.node_card_min_width, width);
            let reopened = JsonStateStore::new(root.join("state.json")).unwrap();
            assert_eq!(reopened.load().unwrap(), state);
        }
        let bytes = std::fs::read(root.join("state.json")).unwrap();
        assert!(
            preferences
                .patch_behavior(
                    state.config_version(),
                    DesktopBehaviorPreferencesPatch {
                        node_card_min_width: Some(0),
                        ..Default::default()
                    }
                )
                .is_err()
        );
        assert_eq!(std::fs::read(root.join("state.json")).unwrap(), bytes);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn notification_delivers_committed_typed_value_and_noop_is_silent() {
        let (root, s, p) = services();
        let a = s.snapshot().unwrap();
        let mut rx = p.subscribe_behavior();
        let b = p
            .patch_behavior(
                a.config_version(),
                DesktopBehaviorPreferencesPatch {
                    group_columns: Some(3),
                    ..Default::default()
                },
            )
            .unwrap();
        let event = rx.try_recv().unwrap();
        assert_eq!(event.version, b.version);
        assert_eq!(event.value, b.value.app_config.behavior);
        assert_eq!(event.fields, [BehaviorField::ProxyColumns]);
        p.patch_behavior(b.version, Default::default()).unwrap();
        assert!(rx.try_recv().is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
}

/// Future page consumers subscribe to committed typed values, never invent per-page defaults.
#[derive(Clone, Copy, Debug)]
pub enum BehaviorConsumer {
    Proxies,
    Overview,
    Connections,
    Diagnostics,
}
impl BehaviorConsumer {
    pub fn accepts(self, event: &BehaviorChange) -> bool {
        event.fields.iter().any(|field| match self {
            Self::Proxies => matches!(
                field,
                BehaviorField::LatencyUrl
                    | BehaviorField::LatencyTimeout
                    | BehaviorField::LatencyLow
                    | BehaviorField::LatencyMedium
                    | BehaviorField::ProxyColumns
                    | BehaviorField::HideUnavailable
                    | BehaviorField::NodeSort
                    | BehaviorField::GroupByProvider
                    | BehaviorField::NodeWidth
                    | BehaviorField::StrategyOrder
            ),
            Self::Overview => matches!(
                field,
                BehaviorField::LatencyLow
                    | BehaviorField::LatencyMedium
                    | BehaviorField::TestSites
                    | BehaviorField::IpInfo
            ),
            Self::Connections => matches!(
                field,
                BehaviorField::LatencyUrl
                    | BehaviorField::LatencyTimeout
                    | BehaviorField::LatencyLow
                    | BehaviorField::LatencyMedium
                    | BehaviorField::IpInfo
            ),
            Self::Diagnostics => matches!(
                field,
                BehaviorField::Ipv6Test
                    | BehaviorField::LatencyUrl
                    | BehaviorField::LatencyTimeout
                    | BehaviorField::IpInfo
            ),
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GroupSaveError {
    Invalid(crate::domain::GroupIssue),
    Storage(AppError),
}

/// 空动态组已保存，但不会进入配置；悬空引用通过 GroupSaveError 明确拒绝。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GroupSaveOutcome {
    pub value: AppState,
    pub version: SnapshotVersion,
    pub effect: ApplyEffect,
    pub dropped: Vec<PoolId>,
}
