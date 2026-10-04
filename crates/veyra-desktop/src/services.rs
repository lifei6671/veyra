use crate::state_bridge::{AppEvent, Request};
use std::{path::PathBuf, sync::Arc};
use tokio::{
    runtime::{Builder, Runtime},
    sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel},
};
use veyra_core::{
    application::{
        state_access::StateAccessGate,
        state_service::{
            DesktopPreferencesService, ProfileService, SelectionService, SnapshotService,
        },
    },
    storage::JsonStateStore,
};

pub struct AppServices {
    pub snapshots: Arc<SnapshotService>,
    // Composition only: P1-03 has no profile or selection write UI.
    pub _profiles: Arc<ProfileService>,
    pub _selections: Arc<SelectionService>,
    pub preferences: Arc<DesktopPreferencesService>,
    pub assets: Arc<crate::visual_assets::VisualAssetStore>,
    root: PathBuf,
    gate: StateAccessGate,
    runtime: Runtime,
    sender: UnboundedSender<AppEvent>,
}
impl AppServices {
    pub fn new(
        root: PathBuf,
    ) -> Result<(Self, UnboundedReceiver<AppEvent>), Box<dyn std::error::Error>> {
        let gate = StateAccessGate::default();
        let snapshots =
            SnapshotService::new(JsonStateStore::new(root.join("state.json"))?, gate.clone());
        let (sender, receiver) = unbounded_channel();
        let services = Self {
            _profiles: Arc::new(ProfileService::new(snapshots.clone())),
            _selections: Arc::new(SelectionService::new(snapshots.clone())),
            preferences: Arc::new(DesktopPreferencesService::new(snapshots.clone())),
            assets: Arc::new(crate::visual_assets::VisualAssetStore::new(
                root.join("visual-assets"),
            )),
            root,
            gate,
            snapshots: Arc::new(snapshots),
            runtime: Builder::new_multi_thread()
                .worker_threads(2)
                .max_blocking_threads(1)
                .enable_time()
                .build()?,
            sender,
        };
        Ok((services, receiver))
    }
    pub fn save_visual(&self, request: crate::preferences::VisualSave) {
        let preferences = self.preferences.clone();
        let sender = self.sender.clone();
        self.runtime.spawn_blocking(move || {
            let result = preferences
                .save(request.expected, request.draft)
                .map(|outcome| Box::new(outcome.value));
            let _ = sender.send(AppEvent::VisualSaved {
                generation: request.generation,
                result,
            });
        });
    }
    pub fn import_background(&self, generation: u64, path: PathBuf) {
        let assets = self.assets.clone();
        let sender = self.sender.clone();
        self.runtime.spawn_blocking(move || {
            let result = assets.import(&path);
            let _ = sender.send(AppEvent::AssetPrepared { generation, result });
        });
    }
    pub fn background_preview(
        &self,
        generation: u64,
        id: veyra_core::domain::VisualAssetId,
        blur: u8,
    ) {
        let assets = self.assets.clone();
        let sender = self.sender.clone();
        self.runtime.spawn_blocking(move || {
            let result = assets.preview(&id, blur);
            let _ = sender.send(AppEvent::BackgroundPreview { generation, result });
        });
    }
    pub fn collect_asset_orphans(&self, draft: veyra_core::domain::DesktopBackground) {
        let assets = self.assets.clone();
        let root = self.root.clone();
        let gate = self.gate.clone();
        self.runtime.spawn_blocking(move || {
            let Ok(_access) = gate.try_lock() else {
                eprintln!("orphan cleanup deferred: state access busy");
                return;
            };
            let read = |name| {
                std::fs::read(root.join(name)).ok().and_then(|bytes| {
                    serde_json::from_slice::<veyra_core::domain::AppState>(&bytes).ok()
                })
            };
            // Current state, recovery backup and the pending draft are live references.
            // Keep assets if a reference cannot be decoded; never race a core writer.
            if let (Some(current), Some(backup)) = (read("state.json"), read("state.json.bak"))
                && let Err(error) = assets.collect_orphans(&[
                    current.app_config.visual.background,
                    backup.app_config.visual.background,
                    draft,
                ])
            {
                eprintln!("orphan cleanup deferred: {error}");
            }
        });
    }
    pub fn refresh(&self, request: Request) {
        self.read(request, 0);
    }
    #[cfg(debug_assertions)]
    pub fn evidence_refresh(&self, request: Request, delay_ms: u64) {
        self.read(request, delay_ms.min(3000));
    }
    fn read(&self, request: Request, delay_ms: u64) {
        let snapshots = self.snapshots.clone();
        let sender = self.sender.clone();
        let handle = self.runtime.handle().clone();
        // This future captures core services + typed values only. No GPUI imports here.
        self.runtime.spawn(async move {
            let result = handle
                .spawn_blocking(move || snapshots.snapshot().map(Box::new))
                .await
                .expect("snapshot worker panicked");
            // Delay delivery AFTER the real read so A is genuinely an older result.
            #[cfg(debug_assertions)]
            tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
            #[cfg(not(debug_assertions))]
            let _ = delay_ms;
            let _ = sender.send(AppEvent::Snapshot { request, result });
        });
    }
    #[cfg(debug_assertions)]
    pub fn synthetic_event(&self, event: AppEvent) {
        let _ = self.sender.send(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_bridge::{Disposition, LoadState, StateBridge};
    // Protect real StateStore errors/retry through the production channel and runtime.
    #[test]
    fn real_store_empty_error_retry_and_late_delivery() {
        let root = std::env::temp_dir().join(format!(
            "veyra-p1-03-test-{:?}",
            veyra_core::domain::StateEpoch::fresh().unwrap()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let (services, mut rx) = AppServices::new(root.clone()).unwrap();
        let mut bridge = StateBridge::default();
        let request = bridge.begin();
        services.refresh(request);
        assert_eq!(
            bridge.receive(rx.blocking_recv().unwrap()),
            Disposition::Accepted
        );
        assert!(bridge.snapshot.as_ref().unwrap().nodes.is_empty());
        std::fs::write(root.join("state.json"), b"{malformed").unwrap();
        let request = bridge.begin();
        services.refresh(request);
        bridge.receive(rx.blocking_recv().unwrap());
        assert!(matches!(bridge.load, LoadState::Error(_)));
        std::fs::remove_file(root.join("state.json")).unwrap();
        let request = bridge.begin();
        services.refresh(request);
        bridge.receive(rx.blocking_recv().unwrap());
        assert!(matches!(bridge.load, LoadState::Ready));
        let a = bridge.begin();
        services.evidence_refresh(a, 500);
        let b = bridge.begin();
        let id = b.id;
        services.evidence_refresh(b, 50);
        assert_eq!(
            bridge.receive(rx.blocking_recv().unwrap()),
            Disposition::Accepted
        );
        assert_eq!(
            bridge.receive(rx.blocking_recv().unwrap()),
            Disposition::StaleRequest
        );
        assert_eq!(bridge.accepted_request, Some(id));
        drop(services);
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[cfg(test)]
mod visual_tests {
    use super::*;
    use crate::preferences::{SaveCoordinator, VisualSave};
    use veyra_core::domain::*;
    #[test]
    fn prepared_background_failed_commit_retry_and_backup_reference_are_atomic() {
        let root = std::env::temp_dir().join(format!(
            "veyra-p1-04a-atomic-{:?}",
            StateEpoch::fresh().unwrap()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let a = root.join("a.png");
        let b = root.join("b.png");
        image::RgbaImage::from_pixel(2, 2, image::Rgba([0, 180, 100, 255]))
            .save(&a)
            .unwrap();
        image::RgbaImage::from_pixel(2, 2, image::Rgba([180, 100, 0, 255]))
            .save(&b)
            .unwrap();
        let (services, mut rx) = AppServices::new(root.clone()).unwrap();
        let before = services.snapshots.snapshot().unwrap();
        let id_a = services.assets.import(&a).unwrap();
        let id_b = services.assets.import(&b).unwrap();
        let mut draft = before.app_config.visual.clone();
        draft.background = DesktopBackground::ManagedAsset(id_a.clone());
        let saved_a = services
            .preferences
            .save(before.config_version(), draft.clone())
            .unwrap()
            .value;
        std::fs::create_dir(root.join("state.tmp")).unwrap();
        draft.background = DesktopBackground::ManagedAsset(id_b.clone());
        services.save_visual(VisualSave {
            generation: 1,
            expected: saved_a.config_version(),
            draft: draft.clone(),
        });
        let AppEvent::VisualSaved { result, .. } = rx.blocking_recv().unwrap() else {
            panic!("typed result")
        };
        assert_eq!(result.unwrap_err().code(), AppErrorCode::StorageFailed);
        assert_eq!(services.snapshots.snapshot().unwrap(), saved_a);
        assert!(services.assets.path(&id_a).exists());
        assert!(services.assets.path(&id_b).exists());
        std::fs::remove_dir(root.join("state.tmp")).unwrap();
        services.save_visual(VisualSave {
            generation: 2,
            expected: saved_a.config_version(),
            draft: draft.clone(),
        });
        let AppEvent::VisualSaved { result, .. } = rx.blocking_recv().unwrap() else {
            panic!("typed result")
        };
        let saved_b = result.unwrap();
        assert_eq!(saved_b.app_config.visual, draft);
        assert_eq!(saved_b.state_epoch, before.state_epoch);
        assert_eq!(saved_b.selection_revision, before.selection_revision);
        assert!(
            services.assets.path(&id_a).exists(),
            "backup still references A"
        );
        let backup: AppState =
            serde_json::from_slice(&std::fs::read(root.join("state.json.bak")).unwrap()).unwrap();
        assert_eq!(
            backup.app_config.visual.background,
            DesktopBackground::ManagedAsset(id_a.clone())
        );
        draft.global_radius = 8;
        let current = services
            .preferences
            .save(saved_b.config_version(), draft.clone())
            .unwrap()
            .value;
        let backup: AppState =
            serde_json::from_slice(&std::fs::read(root.join("state.json.bak")).unwrap()).unwrap();
        services
            .assets
            .collect_orphans(&[
                current.app_config.visual.background.clone(),
                backup.app_config.visual.background,
            ])
            .unwrap();
        assert!(!services.assets.path(&id_a).exists());
        assert!(services.assets.path(&id_b).exists());
        let preview = services.assets.preview(&id_b, 10).unwrap();
        assert!(preview.exists());
        assert_eq!(services.assets.preview(&id_b, 10).unwrap(), preview);
        drop(services);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn rapid_slider_coalesces_real_commit_and_restart_projection() {
        let root = std::env::temp_dir().join(format!(
            "veyra-p1-04a-coalesce-{:?}",
            StateEpoch::fresh().unwrap()
        ));
        let (services, mut rx) = AppServices::new(root.clone()).unwrap();
        let before = services.snapshots.snapshot().unwrap();
        let mut coordinator = SaveCoordinator::default();
        coordinator.restore(&before.app_config.visual);
        for (i, value) in [10, 20, 30, 40].into_iter().enumerate() {
            let mut draft = coordinator.draft.clone();
            draft.background_opacity = value;
            coordinator.edit(draft, i as u64 * 10);
        }
        services.save_visual(coordinator.take_due(430, before.config_version()).unwrap());
        let AppEvent::VisualSaved { generation, result } = rx.blocking_recv().unwrap() else {
            panic!("typed result")
        };
        let saved = result.unwrap();
        coordinator.complete(generation, Ok(()));
        assert_eq!(saved.config_revision, before.config_revision + 1);
        assert_eq!(saved.app_config.visual.background_opacity, 40);
        drop(services);
        let (restarted, _) = AppServices::new(root.clone()).unwrap();
        let restored = restarted.snapshots.snapshot().unwrap();
        let mut projection = SaveCoordinator::default();
        projection.restore(&restored.app_config.visual);
        assert_eq!(projection.draft, saved.app_config.visual);
        assert_eq!(restored.state_epoch, before.state_epoch);
        assert_eq!(restored.selection_revision, before.selection_revision);
        drop(restarted);
        std::fs::remove_dir_all(root).unwrap();
    }
}
