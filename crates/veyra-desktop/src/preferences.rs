//! Single draft coordinator. Saved facts remain in the Core snapshot.
use veyra_core::domain::{AppError, ConfigVersion, DesktopVisualPreferences};
pub const DEBOUNCE_MS: u64 = 400;
#[derive(Clone, Debug, PartialEq)]
pub enum SaveStatus {
    Idle,
    Pending,
    Saving,
    Failed(AppError),
}
#[derive(Clone, Debug)]
pub struct VisualSave {
    pub generation: u64,
    pub expected: ConfigVersion,
    pub draft: DesktopVisualPreferences,
}
pub struct SaveCoordinator {
    pub draft: DesktopVisualPreferences,
    pub generation: u64,
    pub status: SaveStatus,
    in_flight: Option<u64>,
    due_ms: u64,
    pub changes: u64,
    pub submissions: u64,
}
impl Default for SaveCoordinator {
    fn default() -> Self {
        Self {
            draft: Default::default(),
            generation: 0,
            status: SaveStatus::Idle,
            in_flight: None,
            due_ms: 0,
            changes: 0,
            submissions: 0,
        }
    }
}
impl SaveCoordinator {
    pub fn restore(&mut self, value: &DesktopVisualPreferences) {
        if self.generation == 0 {
            self.draft = value.clone();
        }
    }
    pub fn edit(&mut self, value: DesktopVisualPreferences, now_ms: u64) -> u64 {
        self.draft = value;
        self.generation += 1;
        self.changes += 1;
        self.due_ms = now_ms + DEBOUNCE_MS;
        self.status = SaveStatus::Pending;
        self.generation
    }
    pub fn take_due(&mut self, now_ms: u64, expected: ConfigVersion) -> Option<VisualSave> {
        if self.in_flight.is_some() || now_ms < self.due_ms || self.status != SaveStatus::Pending {
            return None;
        }
        self.in_flight = Some(self.generation);
        self.status = SaveStatus::Saving;
        self.submissions += 1;
        Some(VisualSave {
            generation: self.generation,
            expected,
            draft: self.draft.clone(),
        })
    }
    pub fn accepts_completion(&self, generation: u64) -> bool {
        self.in_flight == Some(generation)
    }
    /// A stale result cannot replace a newer draft. Its committed snapshot still updates Core projection.
    pub fn complete(&mut self, generation: u64, result: Result<(), AppError>) -> bool {
        if self.in_flight != Some(generation) {
            return false;
        }
        self.in_flight = None;
        if generation != self.generation {
            self.status = SaveStatus::Pending;
            return false;
        }
        self.status = match result {
            Ok(()) => SaveStatus::Idle,
            Err(error) => SaveStatus::Failed(error),
        };
        true
    }
    pub fn retry(&mut self, now_ms: u64) {
        self.generation += 1;
        self.due_ms = now_ms;
        self.status = SaveStatus::Pending;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use veyra_core::domain::{AppErrorCode, AppState};
    #[test]
    fn coalesces_last_value_and_ignores_stale_completion() {
        let mut c = SaveCoordinator::default();
        let version = AppState::empty().config_version();
        for (i, value) in [10, 20, 30, 40].into_iter().enumerate() {
            let mut draft = c.draft.clone();
            draft.background_opacity = value;
            c.edit(draft, i as u64 * 10);
        }
        assert!(c.take_due(429, version.clone()).is_none());
        let save = c.take_due(430, version.clone()).unwrap();
        assert_eq!(save.draft.background_opacity, 40);
        assert_eq!(c.submissions, 1);
        let mut next = c.draft.clone();
        next.background_opacity = 50;
        c.edit(next, 440);
        assert!(!c.complete(save.generation, Ok(())));
        assert_eq!(c.draft.background_opacity, 50);
        assert!(!c.complete(save.generation, Ok(())));
        assert_eq!(c.status, SaveStatus::Pending);
        let next = c.take_due(840, version).unwrap();
        assert!(c.complete(next.generation, Ok(())));
        assert_eq!(c.status, SaveStatus::Idle);
    }
    #[test]
    fn failure_retains_draft_retry_converges() {
        let mut c = SaveCoordinator::default();
        let version = AppState::empty().config_version();
        let mut draft = c.draft.clone();
        draft.global_radius = 7;
        c.edit(draft, 0);
        let save = c.take_due(400, version.clone()).unwrap();
        assert!(c.complete(
            save.generation,
            Err(AppError::new(AppErrorCode::StorageFailed))
        ));
        assert_eq!(c.draft.global_radius, 7);
        assert!(matches!(c.status, SaveStatus::Failed(_)));
        c.retry(500);
        let retry = c.take_due(500, version).unwrap();
        assert_eq!(retry.draft.global_radius, 7);
        c.complete(retry.generation, Ok(()));
        assert_eq!(c.status, SaveStatus::Idle);
    }
}
