//! Independent behavior draft/CAS namespace. Only explicitly edited fields are replayed.
#[cfg(test)]
use crate::preferences::SaveStatus;
use veyra_core::domain::*;
#[derive(Clone, Debug)]
pub struct BehaviorSave {
    pub generation: u64,
    pub expected: ConfigVersion,
    pub patch: DesktopBehaviorPreferencesPatch,
}
#[derive(Default)]
pub struct BehaviorCoordinator {
    pub draft: DesktopBehaviorPreferences,
    pub pending: DesktopBehaviorPreferencesPatch,
    pub generation: u64,
    pub version: Option<ConfigVersion>,
    pub status: BehaviorStatus,
    in_flight: Option<BehaviorSave>,
}
// Separate default to avoid changing the visual coordinator's existing behavior.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum BehaviorStatus {
    #[default]
    Idle,
    Pending,
    Saving,
    Failed(AppError),
}
impl BehaviorCoordinator {
    pub fn rebase(&mut self, state: &AppState) {
        self.version = Some(state.config_version());
        self.draft = self.pending.apply(&state.app_config.behavior);
    }
    pub fn edit(&mut self, patch: DesktopBehaviorPreferencesPatch) {
        self.pending.merge(&patch);
        self.draft = patch.apply(&self.draft);
        self.generation += 1;
        self.status = BehaviorStatus::Pending;
    }
    pub fn submit(&mut self) -> Option<BehaviorSave> {
        if self.in_flight.is_some() {
            return None;
        }
        self.generation += 1;
        let request = BehaviorSave {
            generation: self.generation,
            expected: self.version.clone()?,
            patch: self.pending.clone(),
        };
        self.in_flight = Some(request.clone());
        self.status = BehaviorStatus::Saving;
        Some(request)
    }
    pub fn accepts_completion(&self, generation: u64) -> bool {
        self.in_flight
            .as_ref()
            .is_some_and(|r| r.generation == generation)
    }
    pub fn complete(&mut self, generation: u64, result: Result<&AppState, AppError>) -> bool {
        if !self.accepts_completion(generation) {
            return false;
        }
        let request = self.in_flight.take().unwrap();
        let result = result.and_then(|saved| {
            if saved.state_epoch != request.expected.0.epoch
                || saved.config_revision < request.expected.0.revision
            {
                return Err(AppError::new(AppErrorCode::RevisionConflict));
            }
            Ok(saved)
        });
        match result {
            Ok(saved) => {
                self.pending.acknowledge(&request.patch);
                self.rebase(saved);
                self.status = if self.pending.changed_fields().is_empty() {
                    BehaviorStatus::Idle
                } else {
                    BehaviorStatus::Pending
                };
            }
            Err(error) => self.status = BehaviorStatus::Failed(error),
        }
        generation == self.generation
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use veyra_core::application::state_service::BehaviorConsumer;
    #[test]
    fn failed_save_retains_draft_and_retry_success() {
        let mut state = AppState::empty();
        let mut c = BehaviorCoordinator::default();
        c.rebase(&state);
        c.edit(DesktopBehaviorPreferencesPatch {
            timeout_ms: Some(8000),
            ..Default::default()
        });
        let request = c.submit().unwrap();
        assert!(c.complete(
            request.generation,
            Err(AppError::new(AppErrorCode::StorageFailed))
        ));
        assert_eq!(c.draft.latency.timeout_ms, 8000);
        assert_eq!(state.app_config.behavior.latency.timeout_ms, 5000);
        let retry = c.submit().unwrap();
        state.app_config.behavior = retry.patch.apply(&state.app_config.behavior);
        state.config_revision += 1;
        assert!(c.complete(retry.generation, Ok(&state)));
        assert_eq!(c.status, BehaviorStatus::Idle);
        assert!(c.pending.changed_fields().is_empty());
    }
    #[test]
    fn conflict_rebase_replays_only_edited_fields() {
        let mut state = AppState::empty();
        let mut c = BehaviorCoordinator::default();
        c.rebase(&state);
        c.edit(DesktopBehaviorPreferencesPatch {
            timeout_ms: Some(7000),
            ..Default::default()
        });
        let old = c.submit().unwrap();
        state.app_config.behavior.proxy_view.group_columns = 3;
        state.app_config.visual.global_radius = 8;
        state.config_revision += 1;
        c.complete(
            old.generation,
            Err(AppError::new(AppErrorCode::RevisionConflict)),
        );
        c.rebase(&state);
        assert_eq!(c.draft.proxy_view.group_columns, 3);
        let retry = c.submit().unwrap();
        assert_eq!(retry.expected, state.config_version());
        assert_eq!(
            retry.patch.changed_fields(),
            [BehaviorField::LatencyTimeout]
        );
        state.app_config.behavior = retry.patch.apply(&state.app_config.behavior);
        c.complete(retry.generation, Ok(&state));
        assert_eq!(
            (
                c.draft.latency.timeout_ms,
                c.draft.proxy_view.group_columns,
                state.app_config.visual.global_radius
            ),
            (7000, 3, 8)
        );
    }
    #[test]
    fn stale_completion_preserves_newer_edits() {
        let mut c = BehaviorCoordinator::default();
        let mut state = AppState::empty();
        c.rebase(&state);
        c.edit(DesktopBehaviorPreferencesPatch {
            timeout_ms: Some(6000),
            ..Default::default()
        });
        let old = c.submit().unwrap();
        c.edit(DesktopBehaviorPreferencesPatch {
            timeout_ms: Some(9000),
            ..Default::default()
        });
        assert!(!c.complete(old.generation + 10, Ok(&state)));
        state.app_config.behavior = old.patch.apply(&state.app_config.behavior);
        state.config_revision += 1;
        assert!(!c.complete(old.generation, Ok(&state)));
        assert_eq!(c.draft.latency.timeout_ms, 9000);
        assert_eq!(c.status, BehaviorStatus::Pending);
        assert_eq!(c.submit().unwrap().patch.timeout_ms, Some(9000));
    }
    #[test]
    fn restart_projection_and_visual_namespace_are_independent() {
        let mut state = AppState::empty();
        state.app_config.behavior.proxy_view.group_columns = 3;
        let mut c = BehaviorCoordinator::default();
        c.rebase(&state);
        assert_eq!(c.draft, state.app_config.behavior);
        let mut visual = crate::preferences::SaveCoordinator::default();
        let mut draft = visual.draft.clone();
        draft.global_radius = 9;
        visual.edit(draft, 0);
        let a = visual.take_due(400, state.config_version()).unwrap();
        c.edit(DesktopBehaviorPreferencesPatch {
            ipv6_test: Some(true),
            ..Default::default()
        });
        let b = c.submit().unwrap();
        c.complete(
            b.generation,
            Err(AppError::new(AppErrorCode::RevisionConflict)),
        );
        assert_eq!(visual.status, SaveStatus::Saving);
        assert!(visual.complete(a.generation, Ok(())));
        assert!(matches!(c.status, BehaviorStatus::Failed(_)));
    }
    #[test]
    fn replacement_epoch_completion_cannot_acknowledge_current_draft() {
        let state = AppState::empty();
        let mut c = BehaviorCoordinator::default();
        c.rebase(&state);
        c.edit(DesktopBehaviorPreferencesPatch {
            timeout_ms: Some(6500),
            ..Default::default()
        });
        let request = c.submit().unwrap();
        let replacement = AppState::empty();
        c.complete(request.generation, Ok(&replacement));
        assert!(
            matches!(c.status,BehaviorStatus::Failed(ref e) if e.code()==AppErrorCode::RevisionConflict)
        );
        assert_eq!(c.pending.timeout_ms, Some(6500));
        assert_eq!(c.draft.latency.timeout_ms, 6500);
        assert_eq!(c.version, Some(state.config_version()));
    }
    #[test]
    fn four_subscriptions_receive_typed_values_without_local_defaults() {
        use veyra_core::{
            application::{
                state_access::StateAccessGate,
                state_service::{DesktopPreferencesService, SnapshotService},
            },
            storage::JsonStateStore,
        };
        let root = std::env::temp_dir().join(format!(
            "veyra-consumers-{:?}",
            StateEpoch::fresh().unwrap()
        ));
        let snapshots = SnapshotService::new(
            JsonStateStore::new(root.join("state.json")).unwrap(),
            StateAccessGate::default(),
        );
        let service = DesktopPreferencesService::new(snapshots);
        let state = service.behavior_snapshot().unwrap();
        let mut receivers = [
            BehaviorConsumer::Proxies,
            BehaviorConsumer::Overview,
            BehaviorConsumer::Connections,
            BehaviorConsumer::Diagnostics,
        ]
        .map(|consumer| (consumer, service.subscribe_behavior()));
        let out = service
            .patch_behavior(
                state.version,
                DesktopBehaviorPreferencesPatch {
                    timeout_ms: Some(7000),
                    low_ms: Some(300),
                    ipv6_test: Some(true),
                    ..Default::default()
                },
            )
            .unwrap();
        for (consumer, rx) in &mut receivers {
            let event = rx.try_recv().unwrap();
            assert!(consumer.accepts(&event));
            assert_eq!(event.version, out.version);
            assert_eq!(event.value.latency.timeout_ms, 7000);
            assert!(event.value.diagnostics.ipv6_test);
        }
        std::fs::remove_dir_all(root).unwrap();
    }
}
