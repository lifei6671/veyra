use super::state_access::StateAccessGate;
use crate::{
    domain::{
        AppError, AppErrorCode, AppState, ConfigVersion, ErrorDetail, FieldPath, NodeId, PoolId,
        Profile, ProfilePatch, Provider, ProxyNode, SelectionPolicy, SelectionVersion,
        SnapshotVersion, StateEpoch, Subscription,
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
    pub fn snapshot(&self) -> Result<AppState, AppError> {
        let _guard = self.lock()?;
        self.load_or_initialize()
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
        replacement.profile.validate()?;
        replacement.state_epoch = StateEpoch::fresh()?;
        replacement.config_revision = 0;
        replacement.selection_revision = 0;
        replacement
            .validate()
            .map_err(|_| AppError::validation(FieldPath::Snapshot))?;
        self.store.save(&replacement)?;
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
        let SelectionPolicy::Manual { selected_node_id } = &mut pool.selection else {
            return Err(AppError::validation(FieldPath::Selection));
        };
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
}

#[cfg(test)]
mod tests;

/// Typed local visual preference transaction. Never applies runtime configuration.
#[derive(Clone)]
pub struct DesktopPreferencesService {
    snapshots: SnapshotService,
}
impl DesktopPreferencesService {
    pub fn new(snapshots: SnapshotService) -> Self {
        Self { snapshots }
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
