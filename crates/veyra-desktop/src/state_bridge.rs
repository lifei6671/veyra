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
    Groups {
        request: u64,
        result: Result<Box<AppState>, veyra_core::application::state_service::GroupSaveError>,
        saved: bool,
        snapshot: Option<Box<AppState>>,
    },
    BackendProfile {
        request: crate::backend_profile::Request,
        result: Result<
            veyra_core::application::state_service::SaveOutcome<
                veyra_core::domain::Profile,
                veyra_core::domain::ConfigVersion,
            >,
            AppError,
        >,
        snapshot: Result<Box<AppState>, AppError>,
    },
    Runtime(crate::runtime_service::RuntimeEvent),
    Subscriptions {
        request: crate::subscriptions::Request,
        result: Result<
            crate::subscriptions::Completion,
            veyra_core::application::subscription_management::SubscriptionOperationError,
        >,
    },
    DefaultBackground {
        request: crate::ui::background::Request,
        bytes: Vec<u8>,
    },
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
    pub runtime: Option<veyra_core::application::manual_runtime::ManualRuntimeSnapshot>,
    pub runtime_request: u64,
    pub runtime_sequence: u64,
    pub runtime_operation: Option<veyra_core::application::manual_runtime::RuntimeCommand>,
    pub runtime_result: Option<
        Result<
            veyra_core::application::manual_runtime::RuntimeResult,
            veyra_core::application::manual_runtime::RuntimeError,
        >,
    >,
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
    /// 页面已经接收的权威快照也能完成首次读取；作废旧全局请求后仍须进入 Ready。
    /// 返回首次 Runtime 观测请求，避免启动时先导航导致观测永远没有发出。
    pub fn accept_page_snapshot(&mut self, state: Box<AppState>) -> Option<u64> {
        self.leave_page();
        self.snapshot = Some(state);
        self.load = LoadState::Ready;
        self.begin_initial_runtime_refresh(Disposition::Accepted)
    }
    /// 首次有效快照到达后才开始 Runtime 观测；错误/陈旧回调不启动，成功仅启动一次。
    pub fn begin_initial_runtime_refresh(&mut self, disposition: Disposition) -> Option<u64> {
        if disposition != Disposition::Accepted
            || !matches!(self.load, LoadState::Ready)
            || self.runtime_request != 0
        {
            return None;
        }
        Some(self.begin_runtime(veyra_core::application::manual_runtime::RuntimeCommand::Refresh))
    }
    pub fn begin_runtime(
        &mut self,
        command: veyra_core::application::manual_runtime::RuntimeCommand,
    ) -> u64 {
        self.runtime_request += 1;
        // 新 request 替换旧操作的视觉归属；旧 completion 仍由 receive 拒绝。
        self.runtime_operation = Some(command);
        self.runtime_result = None;
        self.runtime_request
    }
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
            AppEvent::Runtime(event) => {
                if event.request != self.runtime_request {
                    return Disposition::StaleRequest;
                }
                if event.sequence <= self.runtime_sequence {
                    return Disposition::OldInstance;
                }
                self.runtime_sequence = event.sequence;
                match event.snapshot {
                    Ok(snapshot) => self.runtime = Some(snapshot),
                    Err(error) => self.runtime_result = Some(Err(error)),
                }
                if let Some(result) = event.result {
                    self.runtime_result = Some(result);
                    self.runtime_operation = None;
                }
                Disposition::Accepted
            }
            AppEvent::Groups { .. }
            | AppEvent::Subscriptions { .. }
            | AppEvent::BehaviorSaved { .. }
            | AppEvent::BackendProfile { .. }
            | AppEvent::PlatformSaved { .. }
            | AppEvent::ExternalPreferenceWrite { .. }
            | AppEvent::VisualSaved { .. }
            | AppEvent::AssetPrepared { .. }
            | AppEvent::BackgroundPreview { .. }
            | AppEvent::DefaultBackground { .. } => {
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
    /// 保护真实启动编排的同一个判定入口：失败和过期读取不启动 Runtime。
    #[test]
    fn initial_runtime_refresh_waits_for_accepted_ready_and_runs_once() {
        let mut b = StateBridge::default();
        assert!(
            b.begin_initial_runtime_refresh(Disposition::Accepted)
                .is_none()
        );
        let old = b.begin();
        let current = b.begin();
        let disposition = b.receive(event(old));
        assert!(b.begin_initial_runtime_refresh(disposition).is_none());
        let disposition = b.receive(AppEvent::Snapshot {
            request: current,
            result: Err(AppError::new(
                veyra_core::domain::AppErrorCode::StorageFailed,
            )),
        });
        assert!(b.begin_initial_runtime_refresh(disposition).is_none());
        let retry = b.begin();
        let disposition = b.receive(event(retry));
        assert_eq!(b.begin_initial_runtime_refresh(disposition), Some(1));
        let next = b.begin();
        let disposition = b.receive(event(next));
        assert!(b.begin_initial_runtime_refresh(disposition).is_none());
        assert!(matches!(b.load, LoadState::Ready));
    }
    /// 保护慢启动时先进入订阅页：页面读取成功后不再卡住全局 Loading。
    #[test]
    fn page_snapshot_finishes_startup_before_stale_global_completion() {
        let mut b = StateBridge::default();
        let pending = b.begin();
        b.leave_page();
        let pending_after_navigation = b.begin();
        assert_eq!(b.accept_page_snapshot(Box::new(AppState::empty())), Some(1));
        assert!(matches!(b.load, LoadState::Ready));
        for request in [pending, pending_after_navigation] {
            let disposition = b.receive(event(request));
            assert_eq!(disposition, Disposition::StaleGeneration);
            assert!(b.begin_initial_runtime_refresh(disposition).is_none());
        }
        assert!(
            b.accept_page_snapshot(Box::new(AppState::empty()))
                .is_none()
        );
        assert!(matches!(b.load, LoadState::Ready));
        assert_eq!(b.runtime_request, 1);
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

#[cfg(test)]
mod runtime_tests {
    use super::*;
    use crate::runtime_service::RuntimeEvent;
    use veyra_core::application::manual_runtime::{RuntimeCommand, RuntimeError, RuntimeResult};
    #[test]
    fn old_runtime_result_cannot_clear_new_operation_busy() {
        let mut b = StateBridge::default();
        let old = b.begin_runtime(RuntimeCommand::Start);
        assert_eq!(b.runtime_operation, Some(RuntimeCommand::Start));
        let latest = b.begin_runtime(RuntimeCommand::Stop);
        let event = |request, sequence| {
            AppEvent::Runtime(RuntimeEvent {
                request,
                sequence,
                snapshot: Err(RuntimeError::StateUnavailable),
                result: Some(Ok(RuntimeResult::Stopped)),
            })
        };
        assert_eq!(b.receive(event(old, 1)), Disposition::StaleRequest);
        assert_eq!(b.runtime_operation, Some(RuntimeCommand::Stop));
        assert_eq!(b.receive(event(latest, 3)), Disposition::Accepted);
        assert_eq!(b.runtime_operation, None);
        assert_eq!(b.receive(event(latest, 2)), Disposition::OldInstance);
    }
    // 保护 Restart 被旧 Start 回调覆盖，以及当前失败 completion 的释放。
    #[test]
    fn restart_operation_survives_old_start_and_clears_on_current_failure() {
        let mut b = StateBridge::default();
        let old = b.begin_runtime(RuntimeCommand::Start);
        let current = b.begin_runtime(RuntimeCommand::Restart);
        let completion = |request, sequence| {
            AppEvent::Runtime(RuntimeEvent {
                request,
                sequence,
                snapshot: Err(RuntimeError::CandidateFailed),
                result: Some(Err(RuntimeError::CandidateFailed)),
            })
        };
        assert_eq!(b.receive(completion(old, 1)), Disposition::StaleRequest);
        assert_eq!(b.runtime_operation, Some(RuntimeCommand::Restart));
        assert_eq!(b.receive(completion(current, 2)), Disposition::Accepted);
        assert_eq!(b.runtime_operation, None);
    }
}
