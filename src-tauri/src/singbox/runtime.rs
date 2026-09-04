//! 受管 sing-box sidecar 的纯内存事务边界。
//!
//! 实际进程、路径和配置文件由后续 Platform Port 实现；本模块只允许固定的
//! `check`/`run` 语义，且从不接收命令、路径、PID 或配置原文。

#![allow(dead_code)]

use std::num::NonZeroU16;
use thiserror::Error;

use super::{GeneratedConfig, clash_api::ClashRuntimeObservation};

/// Platform 层只能报告无敏感内容的 sidecar 操作失败。
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
#[error("sidecar port operation failed")]
pub(crate) struct SidecarPortError;

/// 仅由 [`SidecarPort::run`] 产生的受管实例身份。
///
/// 其值不包含 PID，调用方不能构造任意待停止的进程身份。
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ManagedSidecar {
    identity: u64,
}

impl ManagedSidecar {
    /// 仅 Platform Port 可用的内部实例标识，不是操作系统 PID。
    pub(crate) fn from_port_identity(identity: u64) -> Self {
        Self { identity }
    }

    pub(crate) fn identity(&self) -> u64 {
        self.identity
    }
}

/// 固定 sidecar 操作的封闭 Port。
///
/// 实现负责把这组语义映射到应用拥有的目录、已验证的 sidecar 身份以及固定参数；
/// Runtime 不提供执行任意命令、访问任意路径或停止任意 PID 的入口。
pub(crate) trait SidecarPort {
    fn check(&mut self, candidate: &GeneratedConfig) -> Result<(), SidecarPortError>;
    fn prepare(&mut self, candidate: &GeneratedConfig) -> Result<(), SidecarPortError>;
    fn run(&mut self) -> Result<ManagedSidecar, SidecarPortError>;
    fn ready(&mut self, instance: &ManagedSidecar) -> Result<(), SidecarPortError>;
    fn stop(&mut self, instance: &ManagedSidecar) -> Result<(), SidecarPortError>;
    /// 清理由 Port 持有、尚未成为运行实例的候选及 check 子进程。
    fn cancel_pending(&mut self) -> Result<(), SidecarPortError>;
    /// 未确认退出或未完成清理的资源阻止后续启动。
    fn has_pending_cleanup(&self) -> bool;
}

/// 只允许 active Runtime 的后端 owner 读取当前受管 child 的已脱敏观测摘要。
///
/// 此 trait 不接受 endpoint、header、secret、路径、PID 或其它调用方参数。
pub(crate) trait RuntimeObservationSidecarPort {
    fn read_runtime_observation(
        &mut self,
        instance: &ManagedSidecar,
    ) -> Result<ClashRuntimeObservation, SidecarPortError>;
}

/// 不暴露配置原文的运行时快照。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SidecarSnapshot {
    pub lifecycle: SidecarLifecycle,
    pub has_active_config: bool,
    pub has_candidate_config: bool,
}

/// sidecar 的已验证生命周期状态。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SidecarLifecycle {
    Stopped,
    Ready,
    RecoveryRequired,
}

/// 不携带配置、凭据、路径或平台错误文本的运行时失败。
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub(crate) enum SidecarError {
    #[error("sidecar candidate check failed")]
    CandidateCheck,
    #[error("sidecar candidate preparation failed")]
    CandidatePrepare,
    #[error("sidecar active instance could not be stopped")]
    ActiveStop,
    #[error("sidecar candidate could not be started")]
    CandidateStart,
    #[error("sidecar candidate did not become ready")]
    CandidateReady,
    #[error("sidecar candidate could not be stopped")]
    CandidateStop,
    #[error("sidecar resources require an explicit stop")]
    RecoveryRequired,
}

#[derive(Default)]
struct ConfigSlots {
    candidate: Option<GeneratedConfig>,
    active: Option<GeneratedConfig>,
}

/// 单一受管 sidecar 的配置替换事务。
pub(crate) struct SidecarRuntime<P> {
    port: P,
    mixed_port: Option<NonZeroU16>,
    configs: ConfigSlots,
    child: Option<ManagedSidecar>,
    recovery_required: bool,
}

impl<P> SidecarRuntime<P>
where
    P: SidecarPort,
{
    /// `mixed_port` 只能由后端构造期固定，不从 UI 或运行时字符串取得。
    pub(crate) fn new(port: P, mixed_port: NonZeroU16) -> Self {
        Self {
            port,
            mixed_port: Some(mixed_port),
            configs: ConfigSlots::default(),
            child: None,
            recovery_required: false,
        }
    }

    /// 观测专用实例不提供 mixed listener；它只能使用配置中已有的固定 Clash API。
    pub(crate) fn new_observation_only(port: P) -> Self {
        Self {
            port,
            mixed_port: None,
            configs: ConfigSlots::default(),
            child: None,
            recovery_required: false,
        }
    }

    pub(crate) fn snapshot(&self) -> SidecarSnapshot {
        let lifecycle = if self.recovery_required {
            SidecarLifecycle::RecoveryRequired
        } else if self.child.is_some() {
            SidecarLifecycle::Ready
        } else {
            SidecarLifecycle::Stopped
        };
        SidecarSnapshot {
            lifecycle,
            has_active_config: self.configs.active.is_some(),
            has_candidate_config: self.configs.candidate.is_some(),
        }
    }

    /// 仅在 sidecar 已通过 Ready 判据后暴露固定 loopback mixed port。
    pub(crate) fn mixed_port(&self) -> Option<NonZeroU16> {
        (self.snapshot().lifecycle == SidecarLifecycle::Ready)
            .then_some(self.mixed_port)
            .flatten()
    }

    /// 检查成功后才停止旧实例；启动失败仅清理候选，不重新启动旧配置。
    pub(crate) fn start_or_replace(
        &mut self,
        candidate: GeneratedConfig,
    ) -> Result<(), SidecarError> {
        if self.recovery_required || self.port.has_pending_cleanup() {
            self.recovery_required = true;
            return Err(SidecarError::RecoveryRequired);
        }
        if self.port.check(&candidate).is_err() {
            self.recovery_required = self.port.has_pending_cleanup();
            return Err(SidecarError::CandidateCheck);
        }
        if self.port.prepare(&candidate).is_err() {
            self.recovery_required = self.port.cancel_pending().is_err();
            return Err(SidecarError::CandidatePrepare);
        }
        self.configs.candidate = Some(candidate);
        if let Some(child) = self.child.as_ref() {
            if self.port.stop(child).is_err() {
                self.recovery_required = true;
                self.configs.candidate = None;
                // 两类失败都保留 recovery，候选清理失败由 Port 继续持有资源。
                return Err(if self.port.cancel_pending().is_err() {
                    SidecarError::CandidateStop
                } else {
                    SidecarError::ActiveStop
                });
            }
            self.child = None;
        }
        self.configs.active = None;
        let candidate_child = match self.port.run() {
            Ok(child) => child,
            Err(_) => {
                self.configs.candidate = None;
                self.recovery_required = self.port.has_pending_cleanup();
                return Err(SidecarError::CandidateStart);
            }
        };
        if self.port.ready(&candidate_child).is_err() {
            self.configs.candidate = None;
            if self.port.stop(&candidate_child).is_err() {
                self.child = Some(candidate_child);
                self.recovery_required = true;
                return Err(SidecarError::CandidateStop);
            }
            return Err(SidecarError::CandidateReady);
        }
        self.child = Some(candidate_child);
        self.configs.active = self.configs.candidate.take();
        self.recovery_required = false;
        Ok(())
    }

    /// 只停止已拥有的 child 与 pending；未确认清理时保留归属供手动 Stop。
    pub(crate) fn stop(&mut self) -> Result<(), SidecarError> {
        if let Some(child) = self.child.as_ref() {
            if self.port.stop(child).is_err() {
                self.recovery_required = true;
                return Err(SidecarError::ActiveStop);
            }
            self.child = None;
        }
        self.configs = ConfigSlots::default();
        if self.port.cancel_pending().is_err() {
            self.recovery_required = true;
            return Err(SidecarError::CandidateStop);
        }
        self.recovery_required = false;
        Ok(())
    }

    /// 观测确认当前实例失效时清理已拥有资源；仅清理未确认才保留 recovery。
    pub(crate) fn recover_active_failure(&mut self) -> Result<(), SidecarError> {
        self.stop()
    }

    /// 仅在 Runtime 当前持有 active child 时执行后端受控操作；没有 active child 不触发 I/O。
    pub(crate) fn with_active_port<T>(
        &mut self,
        operation: impl FnOnce(&mut P, &ManagedSidecar) -> Result<T, SidecarPortError>,
    ) -> Result<Option<T>, SidecarPortError> {
        if self.snapshot().lifecycle != SidecarLifecycle::Ready {
            return Ok(None);
        }
        let Some(child) = self.child.clone() else {
            return Ok(None);
        };
        operation(&mut self.port, &child).map(Some)
    }

    pub(crate) fn into_port(self) -> P {
        self.port
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::num::NonZeroU16;

    use super::*;
    use crate::{
        domain::{NodeId, ProtocolOptions, ProviderId, ProxyNode, ProxyProtocol, RuntimeIntent},
        singbox::{ConfigCompiler, SingBoxCompiler},
    };

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum Event {
        Check,
        Prepare,
        Run,
        Ready(u64),
        Stop(u64),
    }

    #[derive(Default)]
    struct MockPort {
        events: Vec<Event>,
        check: VecDeque<Result<(), SidecarPortError>>,
        prepare: VecDeque<Result<(), SidecarPortError>>,
        run: VecDeque<Result<ManagedSidecar, SidecarPortError>>,
        ready: VecDeque<Result<(), SidecarPortError>>,
        stop: VecDeque<Result<(), SidecarPortError>>,
        next_identity: u64,
        cancel: VecDeque<Result<(), SidecarPortError>>,
        pending_cleanup: bool,
        check_cleanup_unconfirmed: bool,
    }

    impl MockPort {
        fn result(
            queue: &mut VecDeque<Result<(), SidecarPortError>>,
        ) -> Result<(), SidecarPortError> {
            queue.pop_front().unwrap_or(Ok(()))
        }
    }

    impl SidecarPort for MockPort {
        fn check(&mut self, _: &GeneratedConfig) -> Result<(), SidecarPortError> {
            self.events.push(Event::Check);
            let result = Self::result(&mut self.check);
            self.pending_cleanup = result.is_err() && self.check_cleanup_unconfirmed;
            result
        }

        fn prepare(&mut self, _: &GeneratedConfig) -> Result<(), SidecarPortError> {
            self.events.push(Event::Prepare);
            Self::result(&mut self.prepare)
        }

        fn run(&mut self) -> Result<ManagedSidecar, SidecarPortError> {
            self.events.push(Event::Run);
            self.run.pop_front().unwrap_or_else(|| {
                self.next_identity += 1;
                Ok(ManagedSidecar::from_port_identity(self.next_identity))
            })
        }

        fn ready(&mut self, instance: &ManagedSidecar) -> Result<(), SidecarPortError> {
            self.events.push(Event::Ready(instance.identity()));
            Self::result(&mut self.ready)
        }

        fn stop(&mut self, instance: &ManagedSidecar) -> Result<(), SidecarPortError> {
            self.events.push(Event::Stop(instance.identity()));
            Self::result(&mut self.stop)
        }

        fn cancel_pending(&mut self) -> Result<(), SidecarPortError> {
            let result = Self::result(&mut self.cancel);
            self.pending_cleanup = result.is_err();
            result
        }
        fn has_pending_cleanup(&self) -> bool {
            self.pending_cleanup
        }
    }

    fn config() -> GeneratedConfig {
        SingBoxCompiler
            .compile(
                &RuntimeIntent {
                    nodes: vec![ProxyNode {
                        id: NodeId("node".to_owned()),
                        provider_id: ProviderId("provider".to_owned()),
                        name: "fixture node".to_owned(),
                        protocol: ProxyProtocol::Vless,
                        server: "example.invalid".to_owned(),
                        port: 443,
                        options: ProtocolOptions::Vless {
                            uuid: "00000000-0000-4000-8000-000000000001".to_owned(),
                            flow: None,
                        },
                        transport: None,
                        tls: None,
                    }],
                    pools: vec![crate::domain::RuntimePool {
                        id: crate::domain::PoolId("main".to_owned()),
                        members: vec![NodeId("node".to_owned())],
                        selection: crate::domain::SelectionPolicy::Manual {
                            selected_node_id: None,
                        },
                    }],
                    routes: Vec::new(),
                },
                &crate::domain::RouteTarget::Pool(crate::domain::PoolId("main".to_owned())),
                crate::domain::DnsPolicy::System,
                crate::singbox::RuntimeProfile::ObservationOnly,
            )
            .expect("fixture plan")
            .finalize(&crate::singbox::managed_sidecar::test_api_secret())
            .expect("fixture configuration")
    }

    fn mixed_port() -> NonZeroU16 {
        NonZeroU16::new(20_890).expect("non-zero fixture port")
    }

    #[test]
    fn starts_in_fixed_check_prepare_run_ready_order() {
        let mut runtime = SidecarRuntime::new(MockPort::default(), mixed_port());

        runtime.start_or_replace(config()).expect("start");

        let port = runtime.into_port();
        assert_eq!(
            port.events,
            vec![Event::Check, Event::Prepare, Event::Run, Event::Ready(1)]
        );
    }

    #[test]
    fn fixed_mixed_port_is_hidden_until_ready() {
        let mut runtime = SidecarRuntime::new(MockPort::default(), mixed_port());

        assert_eq!(runtime.mixed_port(), None);
        runtime.start_or_replace(config()).expect("start");

        assert_eq!(runtime.mixed_port(), Some(mixed_port()));
    }

    #[test]
    fn observation_only_runtime_never_exposes_a_mixed_port() {
        let runtime = SidecarRuntime::new_observation_only(MockPort::default());

        assert_eq!(runtime.mixed_port(), None);
        assert_eq!(runtime.snapshot().lifecycle, SidecarLifecycle::Stopped);
    }

    #[test]
    fn check_failure_keeps_active_config_and_child() {
        let mut runtime = SidecarRuntime::new(MockPort::default(), mixed_port());
        runtime.start_or_replace(config()).expect("start stable");
        runtime.port.check.push_back(Err(SidecarPortError));

        let error = runtime
            .start_or_replace(config())
            .expect_err("check failure");

        assert_eq!(error, SidecarError::CandidateCheck);
        assert_eq!(runtime.snapshot().lifecycle, SidecarLifecycle::Ready);
        assert!(runtime.snapshot().has_active_config);
        assert!(!error.to_string().contains("fixture-secret"));
    }

    #[test]
    fn ready_candidate_replaces_active_without_retaining_previous_secret() {
        let mut runtime = SidecarRuntime::new(MockPort::default(), mixed_port());
        runtime.start_or_replace(config()).expect("start stable");

        runtime.start_or_replace(config()).expect("replace stable");

        assert_eq!(runtime.snapshot().lifecycle, SidecarLifecycle::Ready);
        assert!(runtime.snapshot().has_active_config);
        assert!(!runtime.snapshot().has_candidate_config);
        assert_eq!(
            runtime.port.events,
            vec![
                Event::Check,
                Event::Prepare,
                Event::Run,
                Event::Ready(1),
                Event::Check,
                Event::Prepare,
                Event::Stop(1),
                Event::Run,
                Event::Ready(2),
            ]
        );
    }

    #[test]
    fn ready_failure_stops_candidate_without_restarting_previous() {
        let mut runtime = SidecarRuntime::new(MockPort::default(), mixed_port());
        runtime.start_or_replace(config()).expect("start stable");
        runtime.port.ready.push_back(Err(SidecarPortError));

        let error = runtime
            .start_or_replace(config())
            .expect_err("candidate ready failure");

        assert_eq!(error, SidecarError::CandidateReady);
        assert_eq!(runtime.snapshot().lifecycle, SidecarLifecycle::Stopped);
        assert!(!runtime.snapshot().has_active_config);
        assert!(!runtime.snapshot().has_candidate_config);
        assert_eq!(
            runtime.port.events,
            vec![
                Event::Check,
                Event::Prepare,
                Event::Run,
                Event::Ready(1),
                Event::Check,
                Event::Prepare,
                Event::Stop(1),
                Event::Run,
                Event::Ready(2),
                Event::Stop(2),
            ]
        );
    }

    #[test]
    fn stop_uses_only_the_current_managed_instance() {
        let mut runtime = SidecarRuntime::new(MockPort::default(), mixed_port());
        runtime.start_or_replace(config()).expect("start stable");

        runtime.stop().expect("stop managed instance");
        runtime.stop().expect("stopped runtime is a no-op");

        let port = runtime.into_port();
        assert_eq!(
            port.events,
            vec![
                Event::Check,
                Event::Prepare,
                Event::Run,
                Event::Ready(1),
                Event::Stop(1),
            ]
        );
    }

    #[test]
    fn failed_candidate_stop_requires_recovery_without_leaking_config() {
        let mut runtime = SidecarRuntime::new(MockPort::default(), mixed_port());
        runtime.port.ready.push_back(Err(SidecarPortError));
        runtime.port.stop.push_back(Err(SidecarPortError));

        let error = runtime
            .start_or_replace(config())
            .expect_err("candidate stop failure");

        assert_eq!(error, SidecarError::CandidateStop);
        assert_eq!(
            runtime.snapshot().lifecycle,
            SidecarLifecycle::RecoveryRequired
        );
        assert!(!error.to_string().contains("fixture-secret"));
    }

    #[test]
    fn active_observation_failure_clears_owned_resources_and_reports_stopped() {
        let mut runtime = SidecarRuntime::new(MockPort::default(), mixed_port());
        runtime.start_or_replace(config()).expect("start stable");

        runtime
            .recover_active_failure()
            .expect("stop owned child for recovery");

        assert_eq!(runtime.snapshot().lifecycle, SidecarLifecycle::Stopped);
        assert_eq!(
            runtime.port.events,
            vec![
                Event::Check,
                Event::Prepare,
                Event::Run,
                Event::Ready(1),
                Event::Stop(1)
            ]
        );

        runtime.stop().expect("explicitly clear recovery");
        assert_eq!(runtime.snapshot().lifecycle, SidecarLifecycle::Stopped);
    }
    #[test]
    fn preparation_failure_preserves_the_ready_child_and_cancels_candidate() {
        let mut runtime = SidecarRuntime::new_observation_only(MockPort::default());
        runtime.start_or_replace(config()).unwrap();
        runtime.port.prepare.push_back(Err(SidecarPortError));
        assert_eq!(
            runtime.start_or_replace(config()),
            Err(SidecarError::CandidatePrepare)
        );
        assert_eq!(runtime.child.as_ref().unwrap().identity(), 1);
        assert_eq!(runtime.snapshot().lifecycle, SidecarLifecycle::Ready);
        assert_eq!(
            runtime
                .port
                .events
                .iter()
                .filter(|event| matches!(event, Event::Run))
                .count(),
            1
        );
        assert!(!runtime.snapshot().has_candidate_config);
    }

    #[test]
    fn failed_old_stop_keeps_identity_blocks_sampling_and_never_runs_candidate() {
        let mut runtime = SidecarRuntime::new_observation_only(MockPort::default());
        runtime.start_or_replace(config()).unwrap();
        runtime.port.stop.push_back(Err(SidecarPortError));
        assert_eq!(
            runtime.start_or_replace(config()),
            Err(SidecarError::ActiveStop)
        );
        assert_eq!(runtime.child.as_ref().unwrap().identity(), 1);
        assert_eq!(
            runtime.snapshot().lifecycle,
            SidecarLifecycle::RecoveryRequired
        );
        assert_eq!(
            runtime
                .port
                .events
                .iter()
                .filter(|event| matches!(event, Event::Run))
                .count(),
            1
        );
        assert_eq!(
            runtime.with_active_port(|_, _| panic!("recovery must not sample")),
            Ok(None::<()>)
        );
        let events = runtime.port.events.clone();
        assert_eq!(
            runtime.start_or_replace(config()),
            Err(SidecarError::RecoveryRequired)
        );
        assert_eq!(runtime.port.events, events);
        runtime.stop().unwrap();
        assert_eq!(runtime.snapshot().lifecycle, SidecarLifecycle::Stopped);
        assert!(!runtime.snapshot().has_active_config);
    }

    #[test]
    fn failed_spawn_clears_configs_without_restarting_previous() {
        let mut runtime = SidecarRuntime::new_observation_only(MockPort::default());
        runtime.start_or_replace(config()).unwrap();
        runtime.port.run.push_back(Err(SidecarPortError));
        assert_eq!(
            runtime.start_or_replace(config()),
            Err(SidecarError::CandidateStart)
        );
        assert_eq!(runtime.snapshot().lifecycle, SidecarLifecycle::Stopped);
        assert!(!runtime.snapshot().has_active_config);
        assert!(!runtime.snapshot().has_candidate_config);
        assert!(runtime.child.is_none());
        assert_eq!(
            runtime
                .port
                .events
                .iter()
                .filter(|event| matches!(event, Event::Run))
                .count(),
            2
        );
    }

    #[test]
    fn pending_cleanup_failure_is_owned_until_manual_stop() {
        let mut runtime = SidecarRuntime::new_observation_only(MockPort::default());
        runtime.port.prepare.push_back(Err(SidecarPortError));
        runtime.port.cancel.push_back(Err(SidecarPortError));
        assert_eq!(
            runtime.start_or_replace(config()),
            Err(SidecarError::CandidatePrepare)
        );
        assert_eq!(
            runtime.snapshot().lifecycle,
            SidecarLifecycle::RecoveryRequired
        );
        assert!(runtime.port.has_pending_cleanup());
        assert_eq!(
            runtime.start_or_replace(config()),
            Err(SidecarError::RecoveryRequired)
        );
        runtime.stop().unwrap();
        assert!(!runtime.port.has_pending_cleanup());
        assert_eq!(runtime.snapshot().lifecycle, SidecarLifecycle::Stopped);
    }

    #[test]
    fn failed_ready_stop_keeps_candidate_identity_until_manual_stop() {
        let mut runtime = SidecarRuntime::new_observation_only(MockPort::default());
        runtime.start_or_replace(config()).unwrap();
        runtime.port.ready.push_back(Err(SidecarPortError));
        runtime.port.stop.extend([Ok(()), Err(SidecarPortError)]);
        assert_eq!(
            runtime.start_or_replace(config()),
            Err(SidecarError::CandidateStop)
        );
        assert_eq!(runtime.child.as_ref().unwrap().identity(), 2);
        assert!(!runtime.snapshot().has_active_config);
        assert!(!runtime.snapshot().has_candidate_config);
        assert_eq!(
            runtime.snapshot().lifecycle,
            SidecarLifecycle::RecoveryRequired
        );
        runtime.stop().unwrap();
        assert_eq!(runtime.snapshot().lifecycle, SidecarLifecycle::Stopped);
        assert_eq!(
            runtime
                .port
                .events
                .iter()
                .filter(|event| matches!(event, Event::Run))
                .count(),
            2
        );
    }
    #[test]
    fn unconfirmed_check_exit_preserves_owner_and_blocks_start_until_stop() {
        let mut runtime = SidecarRuntime::new_observation_only(MockPort::default());
        runtime.port.check.push_back(Err(SidecarPortError));
        runtime.port.check_cleanup_unconfirmed = true;
        assert_eq!(
            runtime.start_or_replace(config()),
            Err(SidecarError::CandidateCheck)
        );
        assert!(runtime.port.has_pending_cleanup());
        assert_eq!(
            runtime.snapshot().lifecycle,
            SidecarLifecycle::RecoveryRequired
        );
        assert_eq!(
            runtime.start_or_replace(config()),
            Err(SidecarError::RecoveryRequired)
        );
        assert_eq!(runtime.port.events, vec![Event::Check]);
        runtime.stop().unwrap();
        assert_eq!(runtime.snapshot().lifecycle, SidecarLifecycle::Stopped);
        assert!(!runtime.port.has_pending_cleanup());
        runtime.start_or_replace(config()).unwrap();
        assert_eq!(runtime.snapshot().lifecycle, SidecarLifecycle::Ready);
    }

    #[test]
    fn active_observation_identity_changes_only_after_ready_and_is_unavailable_after_stop() {
        let mut runtime = SidecarRuntime::new_observation_only(MockPort::default());
        assert_eq!(
            runtime.with_active_port(|_, _| panic!("stopped must not sample")),
            Ok(None::<u64>)
        );
        runtime.start_or_replace(config()).expect("first ready");
        let first = runtime
            .with_active_port(|_, child| Ok(child.identity()))
            .expect("first sample")
            .expect("active child");
        runtime
            .start_or_replace(config())
            .expect("replacement ready");
        let second = runtime
            .with_active_port(|_, child| Ok(child.identity()))
            .expect("second sample")
            .expect("active child");
        assert_ne!(first, second);
        assert_eq!(
            runtime
                .port
                .events
                .iter()
                .filter(|event| **event == Event::Stop(first))
                .count(),
            1
        );
        runtime.port.ready.push_back(Err(SidecarPortError));
        assert_eq!(
            runtime.start_or_replace(config()),
            Err(SidecarError::CandidateReady)
        );
        assert_eq!(
            runtime
                .with_active_port(|_, _| panic!("failed replacement must not sample old identity")),
            Ok(None::<u64>)
        );
        runtime.start_or_replace(config()).expect("manual start");
        let third = runtime
            .with_active_port(|_, child| Ok(child.identity()))
            .expect("new sample")
            .expect("active child");
        assert_ne!(third, first);
        assert_ne!(third, second);
        runtime.stop().expect("confirmed stop");
        assert_eq!(
            runtime.with_active_port(|_, _| panic!("stopped must not publish late work")),
            Ok(None::<u64>)
        );
        assert!(!runtime.snapshot().has_active_config);
    }
}
