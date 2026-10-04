//! Only core values and request metadata cross the Tokio → foreground boundary.
use std::collections::VecDeque;
use veyra_core::application::runtime_snapshot::InstanceId;
use veyra_core::domain::{AppError, AppState, StateEpoch};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Request {
    pub id: u64,
    pub generation: u64,
    pub epoch: Option<StateEpoch>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstanceToken {
    pub id: InstanceId,
    pub generation: u64,
}
pub enum AppEvent {
    PlatformSaved {
        result: Result<(), crate::platform::PlatformError>,
    },

    BehaviorSaved {
        generation: u64,
        result: Result<Box<AppState>, AppError>,
    },
    ExternalPreferenceWrite {
        result: Result<veyra_core::domain::ConfigVersion, AppError>,
    },
    BackgroundPreview {
        generation: u64,
        result: Result<std::path::PathBuf, AppError>,
    },
    VisualSaved {
        generation: u64,
        result: Result<Box<AppState>, AppError>,
    },
    AssetPrepared {
        generation: u64,
        result: Result<veyra_core::domain::VisualAssetId, AppError>,
    },
    Snapshot {
        request: Request,
        result: Result<Box<AppState>, AppError>,
    },
    /// No Runtime owner exists in P1-03. This is explicitly synthetic evidence only.
    SyntheticInstance {
        instance: InstanceToken,
        epoch: StateEpoch,
    },
}
#[derive(Debug, Eq, PartialEq)]
pub enum Disposition {
    Accepted,
    StaleRequest,
    StaleGeneration,
    StaleEpoch,
    OldInstance,
}
#[derive(Default)]
pub enum LoadState {
    #[default]
    Idle,
    Busy,
    Ready,
    Error(AppError),
}
#[derive(Default)]
pub struct StateBridge {
    /// Read-only projection of the one core snapshot, never edited in desktop.
    pub snapshot: Option<Box<AppState>>,
    pub load: LoadState,
    pub accepted_request: Option<u64>,
    pub history: VecDeque<String>,
    next_request: u64,
    generation: u64,
    pending: Option<Request>,
    pub synthetic_instance: Option<InstanceToken>,
    pub synthetic_accepted: u64,
}
impl StateBridge {
    pub fn begin(&mut self) -> Request {
        self.next_request += 1;
        let request = Request {
            id: self.next_request,
            generation: self.generation,
            epoch: self.snapshot.as_ref().map(|s| s.state_epoch.clone()),
        };
        self.pending = Some(request.clone());
        self.load = LoadState::Busy;
        self.record(format!(
            "request={} generation={} busy",
            request.id, request.generation
        ));
        request
    }
    pub fn leave_page(&mut self) {
        self.generation += 1;
        self.pending = None;
        if matches!(self.load, LoadState::Busy) {
            self.load = if self.snapshot.is_some() {
                LoadState::Ready
            } else {
                LoadState::Idle
            };
        }
    }
    fn record(&mut self, line: String) {
        eprintln!("bridge {line}");
        if self.history.len() == 6 {
            self.history.pop_front();
        }
        self.history.push_back(line);
    }
    pub fn receive(&mut self, event: AppEvent) -> Disposition {
        match event {
            AppEvent::BehaviorSaved { .. }
            | AppEvent::PlatformSaved { .. }
            | AppEvent::ExternalPreferenceWrite { .. }
            | AppEvent::VisualSaved { .. }
            | AppEvent::AssetPrepared { .. }
            | AppEvent::BackgroundPreview { .. } => {
                unreachable!("composition root owns preference results")
            }
            AppEvent::Snapshot { request, result } => {
                let decision = if request.generation != self.generation {
                    Disposition::StaleGeneration
                } else if self.pending.as_ref().map(|r| r.id) != Some(request.id) {
                    Disposition::StaleRequest
                } else if request.epoch != self.snapshot.as_ref().map(|s| s.state_epoch.clone()) {
                    Disposition::StaleEpoch
                } else {
                    Disposition::Accepted
                };
                self.record(format!(
                    "request={} generation={} {:?}",
                    request.id, request.generation, decision
                ));
                if decision == Disposition::Accepted {
                    self.pending = None;
                    match result {
                        Ok(snapshot) => {
                            // A latest authoritative read may discover a replacement epoch.
                            // Requests captured before that adoption can never write over it.
                            self.snapshot = Some(snapshot);
                            self.accepted_request = Some(request.id);
                            self.load = LoadState::Ready;
                        }
                        Err(error) => self.load = LoadState::Error(error),
                    }
                }
                decision
            }
            AppEvent::SyntheticInstance { instance, epoch } => {
                let decision = if self.synthetic_instance.as_ref() != Some(&instance) {
                    Disposition::OldInstance
                } else if self.snapshot.as_ref().map(|s| &s.state_epoch) != Some(&epoch) {
                    Disposition::StaleEpoch
                } else {
                    self.synthetic_accepted += 1;
                    Disposition::Accepted
                };
                self.record(format!(
                    "synthetic instance={} generation={} {:?}",
                    instance.id.0, instance.generation, decision
                ));
                decision
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn event(request: Request) -> AppEvent {
        AppEvent::Snapshot {
            request,
            result: Ok(Box::new(AppState::empty())),
        }
    }
    // Protect navigation, out-of-order reads and replacement-state isolation.
    #[test]
    fn latest_request_wins() {
        let mut b = StateBridge::default();
        let a = b.begin();
        let newer = b.begin();
        assert_eq!(b.receive(event(newer.clone())), Disposition::Accepted);
        assert_eq!(b.receive(event(a)), Disposition::StaleRequest);
        assert_eq!(b.accepted_request, Some(newer.id));
    }
    #[test]
    fn leaving_page_invalidates_generation() {
        let mut b = StateBridge::default();
        let a = b.begin();
        b.leave_page();
        assert_eq!(b.receive(event(a)), Disposition::StaleGeneration);
        assert!(b.snapshot.is_none());
    }
    #[test]
    fn replaced_epoch_rejects_old_read() {
        let mut b = StateBridge {
            snapshot: Some(Box::new(AppState::empty())),
            ..Default::default()
        };
        let a = b.begin();
        b.snapshot = Some(Box::new(AppState::empty()));
        assert_eq!(b.receive(event(a)), Disposition::StaleEpoch);
    }
    // Protect busy/error/retry without faking core success.
    #[test]
    fn busy_error_retry_ready() {
        let mut b = StateBridge::default();
        let a = b.begin();
        assert!(matches!(b.load, LoadState::Busy));
        b.receive(AppEvent::Snapshot {
            request: a,
            result: Err(AppError::new(
                veyra_core::domain::AppErrorCode::StorageFailed,
            )),
        });
        assert!(matches!(b.load, LoadState::Error(_)));
        let retry = b.begin();
        assert!(matches!(b.load, LoadState::Busy));
        b.receive(event(retry));
        assert!(matches!(b.load, LoadState::Ready));
    }
    #[test]
    fn stale_failure_cannot_clear_new_busy() {
        let mut b = StateBridge::default();
        let a = b.begin();
        b.begin();
        b.receive(AppEvent::Snapshot {
            request: a,
            result: Err(AppError::new(
                veyra_core::domain::AppErrorCode::StorageFailed,
            )),
        });
        assert!(matches!(b.load, LoadState::Busy));
    }
    // Protect instance identity AND generation; these are synthetic, not kernel tests.
    #[test]
    fn instance_and_epoch_gate() {
        let mut b = StateBridge::default();
        let state = AppState::empty();
        let epoch = state.state_epoch.clone();
        b.snapshot = Some(Box::new(state));
        let current = InstanceToken {
            id: InstanceId("B".into()),
            generation: 2,
        };
        b.synthetic_instance = Some(current.clone());
        for instance in [
            InstanceToken {
                id: InstanceId("A".into()),
                generation: 1,
            },
            InstanceToken {
                id: InstanceId("B".into()),
                generation: 1,
            },
        ] {
            assert_eq!(
                b.receive(AppEvent::SyntheticInstance {
                    instance,
                    epoch: epoch.clone()
                }),
                Disposition::OldInstance
            );
        }
        assert_eq!(
            b.receive(AppEvent::SyntheticInstance {
                instance: current.clone(),
                epoch: StateEpoch::fresh().unwrap()
            }),
            Disposition::StaleEpoch
        );
        assert_eq!(
            b.receive(AppEvent::SyntheticInstance {
                instance: current,
                epoch
            }),
            Disposition::Accepted
        );
        assert_eq!(b.synthetic_accepted, 1);
    }
}
