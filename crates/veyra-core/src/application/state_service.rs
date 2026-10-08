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
        self.gate
            .try_lock()
            .map_err(|_| AppError::new(AppErrorCode::StorageFailed).with_detail(ErrorDetail::Busy))
    }
    fn load_or_initialize(&self) -> Result<AppState, AppError> {
        if self.store.has_snapshot_or_backup()? {
            return self.store.load().map_err(Into::into);
        }
        let state = AppState::try_empty()?;
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
    /// Runtime selector 写入的窄范围例外：版本/epoch核对与有超时的PUT/GET共用业务写gate。
    /// 不能复读后释放再PUT，否则新pending可在间隙发布。此锁不覆盖child生命周期或cache复制。
    pub(crate) fn with_selection_write<T>(
        &self,
        expected: &SelectionVersion,
        write: impl FnOnce() -> T,
    ) -> Result<T, AppError> {
        let _guard = self.lock()?;
        let _writer = self.store.lock_unfenced_writer()?;
        self.load_or_initialize()?
            .selection_version()
            .0
            .require(&expected.0)?;
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
        next.groups = groups;
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
        self.stage_manual(expected, pool, requested, ManualStage::Begin)
    }
    /// 只有同版本、同 pending 请求才允许提交 controller 的确认结果。
    pub fn confirm_manual_pending(
        &self,
        expected: SelectionVersion,
        pool: PoolId,
        requested: NodeId,
    ) -> Result<SaveOutcome<NodeId, SelectionVersion>, AppError> {
        self.stage_manual(expected, pool, requested, ManualStage::Confirm)
    }
    /// 调用者须已读回旧 confirmed；CAS 防止清掉另一请求。
    pub fn clear_manual_pending(
        &self,
        expected: SelectionVersion,
        pool: PoolId,
        requested: NodeId,
    ) -> Result<SaveOutcome<NodeId, SelectionVersion>, AppError> {
        self.stage_manual(expected, pool, requested, ManualStage::Clear)
    }
    fn stage_manual(
        &self,
        expected: SelectionVersion,
        pool_id: PoolId,
        requested: NodeId,
        stage: ManualStage,
    ) -> Result<SaveOutcome<NodeId, SelectionVersion>, AppError> {
        let _guard = self.snapshots.lock()?;
        let mut next = self.snapshots.load_or_initialize()?;
        next.selection_version().0.require(&expected.0)?;
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
