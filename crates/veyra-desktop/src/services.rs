use crate::state_bridge::{AppEvent, Request};
use std::{path::PathBuf, sync::Arc};
use tokio::{
    runtime::{Builder, Runtime},
    sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel},
};
use veyra_core::{
    application::{
        state_access::StateAccessGate,
        state_service::{ProfileService, SelectionService, SnapshotService},
    },
    storage::JsonStateStore,
};

pub struct AppServices {
    pub snapshots: Arc<SnapshotService>,
    // Composition only: P1-03 has no profile or selection write UI.
    pub _profiles: Arc<ProfileService>,
    pub _selections: Arc<SelectionService>,
    runtime: Runtime,
    sender: UnboundedSender<AppEvent>,
}
impl AppServices {
    pub fn new(
        root: PathBuf,
    ) -> Result<(Self, UnboundedReceiver<AppEvent>), Box<dyn std::error::Error>> {
        let snapshots = SnapshotService::new(
            JsonStateStore::new(root.join("state.json"))?,
            StateAccessGate::default(),
        );
        let (sender, receiver) = unbounded_channel();
        let services = Self {
            _profiles: Arc::new(ProfileService::new(snapshots.clone())),
            _selections: Arc::new(SelectionService::new(snapshots.clone())),
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
